//! mihomo 进程管理：负责按用户工作目录启动、停止并识别后端状态。

use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{Context, Result};
use tokio::{process::Command, sync::mpsc};

#[derive(Clone)]
pub struct MihomoProcess {
    work_dir: PathBuf,
    binary: PathBuf,
    runtime_config: PathBuf,
}

impl MihomoProcess {
    pub fn new(work_dir: PathBuf, binary: PathBuf, runtime_config: PathBuf) -> Self {
        Self {
            work_dir,
            binary,
            runtime_config,
        }
    }

    // #----状态查询----
    pub fn pid(&self) -> Option<u32> {
        let pid_path = self.work_dir.join("mihomo.pid");
        fs::read_to_string(pid_path).ok()?.trim().parse().ok()
    }

    pub fn is_running(&self) -> bool {
        self.pid()
            .map(|pid| Path::new("/proc").join(pid.to_string()).exists())
            .unwrap_or(false)
    }

    // #----启停控制----
    pub async fn start(&self, log_tx: mpsc::UnboundedSender<String>, sudo: bool) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }

        let mut command = if sudo {
            let mut command = Command::new("sudo");
            command.arg("-n").arg(&self.binary);
            command
        } else {
            Command::new(&self.binary)
        };

        let mut child = command
            .arg("-d")
            .arg(&self.work_dir)
            .arg("-f")
            .arg(&self.runtime_config)
            .current_dir(&self.work_dir)
            .env_remove("http_proxy")
            .env_remove("https_proxy")
            .env_remove("all_proxy")
            .env_remove("no_proxy")
            .env_remove("HTTP_PROXY")
            .env_remove("HTTPS_PROXY")
            .env_remove("ALL_PROXY")
            .env_remove("NO_PROXY")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("启动 mihomo 失败")?;

        let pid = child.id().unwrap_or_default();
        fs::write(self.work_dir.join("mihomo.pid"), pid.to_string())
            .context("写入 mihomo.pid 失败")?;

        if let Some(stdout) = child.stdout.take() {
            crate::events::spawn_reader(stdout, log_tx.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            crate::events::spawn_reader(stderr, log_tx);
        }

        tokio::spawn(async move {
            let _ = child.wait().await;
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        if let Some(pid) = self.pid() {
            let _ = Command::new("kill").arg(pid.to_string()).status().await;
        }
        let _ = fs::remove_file(self.work_dir.join("mihomo.pid"));
        Ok(())
    }
}
