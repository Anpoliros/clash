//! 运行配置读取：从用户 mihomo 配置解析 controller、secret 和 Provider 顺序。

use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_yaml::{Mapping, Value};

#[derive(Clone)]
pub struct RuntimeConfig {
    pub controller: String,
    pub secret: Option<String>,
    pub provider_names: Vec<String>,
}

// #----读取配置----
pub fn prepare(source: &Path) -> Result<RuntimeConfig> {
    let raw = fs::read_to_string(source).context("读取用户配置失败")?;
    let doc: Value = serde_yaml::from_str(&raw).context("解析 YAML 配置失败")?;
    let root = doc
        .as_mapping()
        .context("mihomo 配置顶层必须是 YAML 对象")?;

    let controller =
        get_string(root, "external-controller").unwrap_or_else(|| "127.0.0.1:9090".into());
    let secret = get_string(root, "secret");
    let provider_names = provider_names_from_text(&raw).unwrap_or_else(|| provider_names(root));

    Ok(RuntimeConfig {
        controller: normalize_controller(&controller),
        secret,
        provider_names,
    })
}

// #----工具函数----
fn get_string(map: &Mapping, key: &str) -> Option<String> {
    let key = Value::from(key);
    map.get(&key).and_then(Value::as_str).map(ToOwned::to_owned)
}

fn provider_names(map: &Mapping) -> Vec<String> {
    let key = Value::from("proxy-providers");
    map.get(&key)
        .and_then(Value::as_mapping)
        .map(|providers| {
            providers
                .keys()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn provider_names_from_text(raw: &str) -> Option<Vec<String>> {
    let mut in_block = false;
    let mut names = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if !in_block {
            if trimmed == "proxy-providers:" {
                in_block = true;
            }
            continue;
        }

        if !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 2 && trimmed.ends_with(':') {
            names.push(trimmed.trim_end_matches(':').to_string());
        }
    }

    if names.is_empty() {
        None
    } else {
        Some(names)
    }
}

fn normalize_controller(controller: &str) -> String {
    if controller.starts_with("http://") || controller.starts_with("https://") {
        controller.trim_end_matches('/').to_string()
    } else {
        format!("http://{}", controller.trim_end_matches('/'))
    }
}
