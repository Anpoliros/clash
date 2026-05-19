//! runtime.yaml 管理：复制用户基础配置，并补齐 external-controller。

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde_yaml::{Mapping, Value};

#[derive(Clone)]
pub struct RuntimeConfig {
    pub controller: String,
    pub secret: Option<String>,
    pub provider_names: Vec<String>,
}

// #----生成配置----
pub fn prepare(source: &Path) -> Result<RuntimeConfig> {
    let raw = fs::read_to_string(source).context("读取用户配置失败")?;
    let mut doc: Value = serde_yaml::from_str(&raw).context("解析 YAML 配置失败")?;
    let root = ensure_mapping(&mut doc);

    let controller = get_string(root, "external-controller").unwrap_or_else(|| {
        let value = "127.0.0.1:9090".to_string();
        root.insert(
            Value::from("external-controller"),
            Value::from(value.clone()),
        );
        value
    });
    let secret = get_string(root, "secret");
    let provider_names = provider_names_from_text(&raw).unwrap_or_else(|| provider_names(root));

    let path = runtime_dir()?.join("runtime.yaml");
    fs::create_dir_all(path.parent().expect("runtime path has parent"))
        .context("创建 runtime 目录失败")?;
    fs::write(&path, serde_yaml::to_string(&doc)?).context("写入 runtime.yaml 失败")?;

    Ok(RuntimeConfig {
        controller: normalize_controller(&controller),
        secret,
        provider_names,
    })
}

// #----工具函数----
fn runtime_dir() -> Result<PathBuf> {
    let home = env::var_os("HOME").context("无法获取 HOME 环境变量")?;
    Ok(PathBuf::from(home).join(".config/clash-tui"))
}

fn ensure_mapping(value: &mut Value) -> &mut Mapping {
    if !matches!(value, Value::Mapping(_)) {
        *value = Value::Mapping(Mapping::new());
    }
    match value {
        Value::Mapping(map) => map,
        _ => unreachable!(),
    }
}

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
