//! clash-tui 入口：解析工作目录，初始化运行时配置，并启动终端界面。

mod app;
mod config;
mod events;
mod mihomo;
mod runtime;
mod ui;

use std::{env, path::PathBuf};

use anyhow::{bail, Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let work_dir = parse_work_dir()?;
    let boot = runtime::bootstrap::bootstrap(work_dir).await?;
    app::run(boot).await
}

// #----参数解析----
fn parse_work_dir() -> Result<PathBuf> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-d" | "--dir" => {
                let dir = args.next().context("-d/--dir 需要指定 mihomo 工作目录")?;
                return Ok(PathBuf::from(dir)
                    .canonicalize()
                    .context("工作目录不存在")?);
            }
            "-h" | "--help" => {
                println!("Usage: clash-tui -d <mihomo-work-dir>");
                std::process::exit(0);
            }
            other => bail!("未知参数：{other}，仅支持 -d/--dir"),
        }
    }

    bail!("缺少启动参数：clash-tui -d <mihomo-work-dir>")
}
