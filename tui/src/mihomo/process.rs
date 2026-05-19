//! mihomo 进程状态：只基于工作目录中的 mihomo.pid 识别后端状态。

use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct MihomoProcess {
    work_dir: PathBuf,
}

impl MihomoProcess {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
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
}
