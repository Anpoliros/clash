//! 启动编排：查找配置、读取 controller，并准备 API 与进程管理上下文。

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{
    config::app_config::{self, AppConfig},
    config::runtime_config::{self, RuntimeConfig},
    mihomo::{client::MihomoClient, process::MihomoProcess},
};

#[derive(Clone)]
pub struct BootContext {
    pub config: RuntimeConfig,
    pub app_config: AppConfig,
    pub client: MihomoClient,
    pub process: MihomoProcess,
    pub work_dir: PathBuf,
    pub source_config: PathBuf,
}

// #----启动准备----
pub async fn bootstrap(work_dir: PathBuf) -> Result<BootContext> {
    let source_config = find_config(&work_dir)?;
    let app_config = app_config::load(&work_dir)?;
    let runtime_config = runtime_config::prepare(&source_config)?;
    let process = MihomoProcess::new(work_dir.clone());
    let client = MihomoClient::new(
        runtime_config.controller.clone(),
        runtime_config.secret.clone(),
    );

    Ok(BootContext {
        config: runtime_config,
        app_config,
        client,
        process,
        work_dir,
        source_config,
    })
}

// #----文件发现----
fn find_config(dir: &Path) -> Result<PathBuf> {
    for name in ["config.yaml", "config.yml"] {
        let path = dir.join(name);
        if path.is_file() {
            return Ok(path);
        }
    }

    let mut configs: Vec<_> = fs::read_dir(dir)
        .context("读取工作目录失败")?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|v| v.to_str()),
                Some("yaml" | "yml")
            )
        })
        .collect();
    configs.sort();
    configs
        .into_iter()
        .next()
        .context("未找到 mihomo YAML 配置")
}
