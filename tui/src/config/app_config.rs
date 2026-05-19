//! TUI 偏好配置：加载并保存用户界面相关的小型持久化设置。

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_DIR: &str = "tui/config";
const CONFIG_FILE: &str = "ui.yaml";
const DEFAULT_NODE_NAME_WIDTH: u16 = 22;
const DEFAULT_NODE_ITEM_MIN_WIDTH: u16 = 30;
const DEFAULT_NODE_MIN_GAP_WIDTH: u16 = 2;
const DEFAULT_NODE_RESERVE_WIDTH: u16 = 2;
const DEFAULT_NODE_COLUMN_GAP_WIDTH: u16 = 4;

const MIN_NODE_NAME_WIDTH: u16 = 8;
const MAX_NODE_NAME_WIDTH: u16 = 80;
const MIN_NODE_ITEM_WIDTH: u16 = 14;
const MAX_NODE_ITEM_WIDTH: u16 = 120;
const MIN_NODE_GAP_WIDTH: u16 = 1;
const MAX_NODE_GAP_WIDTH: u16 = 16;
const MIN_NODE_RESERVE_WIDTH: u16 = 0;
const MAX_NODE_RESERVE_WIDTH: u16 = 12;
const MIN_NODE_COLUMN_GAP_WIDTH: u16 = 1;
const MAX_NODE_COLUMN_GAP_WIDTH: u16 = 24;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(alias = "node_item_width")]
    pub node_name_width: u16,
    pub node_item_min_width: u16,
    pub node_min_gap_width: u16,
    pub node_reserve_width: u16,
    pub node_column_gap_width: u16,

    #[serde(skip)]
    pub path: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            node_name_width: DEFAULT_NODE_NAME_WIDTH,
            node_item_min_width: DEFAULT_NODE_ITEM_MIN_WIDTH,
            node_min_gap_width: DEFAULT_NODE_MIN_GAP_WIDTH,
            node_reserve_width: DEFAULT_NODE_RESERVE_WIDTH,
            node_column_gap_width: DEFAULT_NODE_COLUMN_GAP_WIDTH,
            path: PathBuf::new(),
        }
    }
}

// #----加载保存----
pub fn load(work_dir: &Path) -> Result<AppConfig> {
    let path = config_path(work_dir);
    if !path.is_file() {
        let config = AppConfig {
            path,
            ..AppConfig::default()
        };
        save(&config)?;
        return Ok(config);
    }

    let text = fs::read_to_string(&path).context("读取 TUI 偏好配置失败")?;
    let mut config: AppConfig = serde_yaml::from_str(&text).context("解析 TUI 偏好配置失败")?;
    config.path = path;
    config.normalize();
    save(&config)?;
    Ok(config)
}

pub fn save(config: &AppConfig) -> Result<()> {
    if let Some(parent) = config.path.parent() {
        fs::create_dir_all(parent).context("创建 TUI 配置目录失败")?;
    }
    let text = serde_yaml::to_string(&SerializableConfig::from(config))
        .context("序列化 TUI 偏好配置失败")?;
    fs::write(&config.path, text).context("写入 TUI 偏好配置失败")
}

// #----配置规则----
impl AppConfig {
    pub fn normalize(&mut self) {
        self.node_name_width = self
            .node_name_width
            .clamp(MIN_NODE_NAME_WIDTH, MAX_NODE_NAME_WIDTH);
        self.node_item_min_width = self
            .node_item_min_width
            .clamp(MIN_NODE_ITEM_WIDTH, MAX_NODE_ITEM_WIDTH);
        self.node_min_gap_width = self
            .node_min_gap_width
            .clamp(MIN_NODE_GAP_WIDTH, MAX_NODE_GAP_WIDTH);
        self.node_reserve_width = self
            .node_reserve_width
            .clamp(MIN_NODE_RESERVE_WIDTH, MAX_NODE_RESERVE_WIDTH);
        self.node_column_gap_width = self
            .node_column_gap_width
            .clamp(MIN_NODE_COLUMN_GAP_WIDTH, MAX_NODE_COLUMN_GAP_WIDTH);
    }

    pub fn adjust_node_name_width(&mut self, delta: i16) {
        self.node_name_width = clamp_delta(
            self.node_name_width,
            delta,
            MIN_NODE_NAME_WIDTH,
            MAX_NODE_NAME_WIDTH,
        );
    }

    pub fn adjust_node_item_min_width(&mut self, delta: i16) {
        self.node_item_min_width = clamp_delta(
            self.node_item_min_width,
            delta,
            MIN_NODE_ITEM_WIDTH,
            MAX_NODE_ITEM_WIDTH,
        );
    }

    pub fn adjust_node_min_gap_width(&mut self, delta: i16) {
        self.node_min_gap_width = clamp_delta(
            self.node_min_gap_width,
            delta,
            MIN_NODE_GAP_WIDTH,
            MAX_NODE_GAP_WIDTH,
        );
    }

    pub fn adjust_node_reserve_width(&mut self, delta: i16) {
        self.node_reserve_width = clamp_delta(
            self.node_reserve_width,
            delta,
            MIN_NODE_RESERVE_WIDTH,
            MAX_NODE_RESERVE_WIDTH,
        );
    }

    pub fn adjust_node_column_gap_width(&mut self, delta: i16) {
        self.node_column_gap_width = clamp_delta(
            self.node_column_gap_width,
            delta,
            MIN_NODE_COLUMN_GAP_WIDTH,
            MAX_NODE_COLUMN_GAP_WIDTH,
        );
    }

    pub fn set_node_name_width(&mut self, value: u16) {
        self.node_name_width = value.clamp(MIN_NODE_NAME_WIDTH, MAX_NODE_NAME_WIDTH);
    }

    pub fn set_node_item_min_width(&mut self, value: u16) {
        self.node_item_min_width = value.clamp(MIN_NODE_ITEM_WIDTH, MAX_NODE_ITEM_WIDTH);
    }

    pub fn set_node_min_gap_width(&mut self, value: u16) {
        self.node_min_gap_width = value.clamp(MIN_NODE_GAP_WIDTH, MAX_NODE_GAP_WIDTH);
    }

    pub fn set_node_reserve_width(&mut self, value: u16) {
        self.node_reserve_width = value.clamp(MIN_NODE_RESERVE_WIDTH, MAX_NODE_RESERVE_WIDTH);
    }

    pub fn set_node_column_gap_width(&mut self, value: u16) {
        self.node_column_gap_width =
            value.clamp(MIN_NODE_COLUMN_GAP_WIDTH, MAX_NODE_COLUMN_GAP_WIDTH);
    }
}

fn config_path(work_dir: &Path) -> PathBuf {
    work_dir.join(CONFIG_DIR).join(CONFIG_FILE)
}

#[derive(Serialize)]
struct SerializableConfig {
    node_name_width: u16,
    node_item_min_width: u16,
    node_min_gap_width: u16,
    node_reserve_width: u16,
    node_column_gap_width: u16,
}

impl From<&AppConfig> for SerializableConfig {
    fn from(config: &AppConfig) -> Self {
        Self {
            node_name_width: config.node_name_width,
            node_item_min_width: config.node_item_min_width,
            node_min_gap_width: config.node_min_gap_width,
            node_reserve_width: config.node_reserve_width,
            node_column_gap_width: config.node_column_gap_width,
        }
    }
}

fn clamp_delta(value: u16, delta: i16, min: u16, max: u16) -> u16 {
    let next = value as i16 + delta;
    next.clamp(min as i16, max as i16) as u16
}
