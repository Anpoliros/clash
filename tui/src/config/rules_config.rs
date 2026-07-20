//! Rules 配置管理：维护本地 rule-providers、规则文件和启用顺序。

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use serde_yaml::{Mapping, Value};

const RULE_PROVIDERS: &str = "rule-providers";
const RULES: &str = "rules";
const RULES_DIR: &str = "rules";
const DEFAULT_TARGET: &str = "PROXY";

#[derive(Clone, Debug, Default)]
pub struct RulesConfig {
    pub groups: Vec<RuleGroup>,
}

#[derive(Clone, Debug)]
pub struct RuleGroup {
    pub name: String,
    pub target: String,
    pub active: bool,
    pub expanded: bool,
    pub path: PathBuf,
    pub rules: Vec<String>,
}

// #----加载保存----
pub fn load(config_path: &Path, work_dir: &Path) -> Result<RulesConfig> {
    let raw = fs::read_to_string(config_path).context("读取 mihomo 配置失败")?;
    let doc: Value = serde_yaml::from_str(&raw).context("解析 mihomo 配置失败")?;
    let root = doc
        .as_mapping()
        .ok_or_else(|| anyhow!("mihomo 配置顶层必须是 YAML 对象"))?;

    let active = active_rule_sets(root);
    let providers = root
        .get(&Value::from(RULE_PROVIDERS))
        .and_then(Value::as_mapping);

    let mut groups = providers
        .map(|items| {
            items
                .iter()
                .filter_map(|(key, value)| {
                    let name = key.as_str()?.to_string();
                    let provider = value.as_mapping()?;
                    if !is_managed_provider(provider) {
                        return None;
                    }
                    let path = provider
                        .get(&Value::from("path"))
                        .and_then(Value::as_str)
                        .map(|item| resolve_rule_path(work_dir, item))
                        .unwrap_or_else(|| rule_file_path(work_dir, &name));
                    let rules = load_rule_file(&path).unwrap_or_default();
                    let target = active
                        .iter()
                        .find(|item| item.name == name)
                        .map(|item| item.target.clone())
                        .unwrap_or_else(|| DEFAULT_TARGET.into());
                    Some(RuleGroup {
                        name,
                        target,
                        active: active.iter().any(|item| item.name == key.as_str().unwrap()),
                        expanded: false,
                        path,
                        rules,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    groups.sort_by_key(|group| {
        active
            .iter()
            .position(|item| item.name == group.name)
            .unwrap_or(usize::MAX)
    });
    Ok(RulesConfig { groups })
}

pub fn save(
    config_path: &Path,
    work_dir: &Path,
    config: &RulesConfig,
    create_backup: bool,
) -> Result<()> {
    let raw = fs::read_to_string(config_path).context("读取 mihomo 配置失败")?;
    let mut doc: Value = serde_yaml::from_str(&raw).context("解析 mihomo 配置失败")?;
    if create_backup {
        backup_file(config_path)?;
    }

    let root = ensure_mapping(&mut doc);
    save_rule_files(&config.groups)?;
    write_providers(root, work_dir, &config.groups);
    write_rules(root, &config.groups);

    fs::write(config_path, serde_yaml::to_string(&doc)?).context("写入 mihomo 配置失败")
}

pub fn create_group(work_dir: &Path, name: &str) -> Result<RuleGroup> {
    validate_group_name(name)?;
    Ok(RuleGroup {
        name: name.into(),
        target: DEFAULT_TARGET.into(),
        active: false,
        expanded: false,
        path: rule_file_path(work_dir, name),
        rules: Vec::new(),
    })
}

pub fn backup_rule_file(group: &RuleGroup) -> Result<()> {
    if !group.path.exists() {
        return Ok(());
    }
    let backup =
        group
            .path
            .with_file_name(format!("{}.bak.{}", file_name(&group.path), timestamp()));
    fs::rename(&group.path, backup).context("备份规则文件失败")
}

pub fn validate_group_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
    if valid {
        Ok(())
    } else {
        Err(anyhow!("分组名只能包含英文、数字、-、_、."))
    }
}

// #----YAML 写入----
fn write_providers(root: &mut Mapping, work_dir: &Path, groups: &[RuleGroup]) {
    let key = Value::from(RULE_PROVIDERS);
    if !matches!(root.get(&key), Some(Value::Mapping(_))) {
        root.insert(key.clone(), Value::Mapping(Mapping::new()));
    }
    let providers = root.get_mut(&key).and_then(Value::as_mapping_mut).unwrap();
    let managed_names = providers
        .iter()
        .filter_map(|(name, value)| {
            let provider = value.as_mapping()?;
            is_managed_provider(provider).then(|| name.clone())
        })
        .collect::<Vec<_>>();
    for name in managed_names {
        providers.remove(&name);
    }
    for group in groups {
        providers.insert(
            Value::from(group.name.clone()),
            provider_value(work_dir, group),
        );
    }
}

fn write_rules(root: &mut Mapping, groups: &[RuleGroup]) {
    let key = Value::from(RULES);
    let old_rules = root
        .get(&key)
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    let names = groups
        .iter()
        .map(|group| group.name.as_str())
        .collect::<Vec<_>>();
    let mut kept = old_rules
        .into_iter()
        .filter(|item| {
            item.as_str()
                .and_then(parse_rule_set)
                .map(|(name, _)| !names.iter().any(|item| *item == name))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    let active = groups
        .iter()
        .filter(|group| group.active)
        .map(|group| Value::from(format!("RULE-SET,{},{}", group.name, group.target)))
        .collect::<Vec<_>>();
    kept.splice(0..0, active);
    root.insert(key, Value::Sequence(kept));
}

fn provider_value(work_dir: &Path, group: &RuleGroup) -> Value {
    let mut map = Mapping::new();
    map.insert(Value::from("type"), Value::from("file"));
    map.insert(
        Value::from("path"),
        Value::from(relative_rule_path(work_dir, group)),
    );
    map.insert(Value::from("behavior"), Value::from("classical"));
    map.insert(Value::from("format"), Value::from("yaml"));
    Value::Mapping(map)
}

fn save_rule_files(groups: &[RuleGroup]) -> Result<()> {
    for group in groups {
        if let Some(parent) = group.path.parent() {
            fs::create_dir_all(parent).context("创建规则目录失败")?;
        }
        let mut map = Mapping::new();
        map.insert(
            Value::from("payload"),
            Value::Sequence(group.rules.iter().cloned().map(Value::from).collect()),
        );
        fs::write(&group.path, serde_yaml::to_string(&Value::Mapping(map))?)
            .with_context(|| format!("写入规则文件失败：{}", group.path.display()))?;
    }
    Ok(())
}

// #----解析工具----
fn load_rule_file(path: &Path) -> Result<Vec<String>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path).context("读取规则文件失败")?;
    let doc: Value = serde_yaml::from_str(&raw).context("解析规则文件失败")?;
    let rules = doc
        .as_mapping()
        .and_then(|map| map.get(&Value::from("payload")))
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok(rules)
}

fn active_rule_sets(root: &Mapping) -> Vec<ActiveRuleSet> {
    root.get(&Value::from(RULES))
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter_map(parse_rule_set)
                .map(|(name, target)| ActiveRuleSet {
                    name: name.to_string(),
                    target: target.to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_rule_set(rule: &str) -> Option<(&str, &str)> {
    let mut parts = rule.split(',').map(str::trim);
    (parts.next()? == "RULE-SET").then_some(())?;
    let name = parts.next()?;
    let target = parts.next()?;
    Some((name, target))
}

fn is_managed_provider(provider: &Mapping) -> bool {
    let is_file = provider
        .get(&Value::from("type"))
        .and_then(Value::as_str)
        .map(|item| item == "file")
        .unwrap_or(false);
    let in_rules_dir = provider
        .get(&Value::from("path"))
        .and_then(Value::as_str)
        .map(|path| {
            path == RULES_DIR
                || path.starts_with("rules/")
                || path.starts_with("./rules/")
                || path.starts_with("rules\\")
                || path.starts_with(".\\rules\\")
        })
        .unwrap_or(false);
    is_file && in_rules_dir
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

fn rule_file_path(work_dir: &Path, name: &str) -> PathBuf {
    work_dir.join(RULES_DIR).join(format!("{name}.yaml"))
}

fn resolve_rule_path(work_dir: &Path, path: &str) -> PathBuf {
    let path = path.trim_start_matches("./");
    work_dir.join(path)
}

fn relative_rule_path(work_dir: &Path, group: &RuleGroup) -> String {
    group
        .path
        .strip_prefix(work_dir)
        .ok()
        .map(|path| format!("./{}", path.to_string_lossy()))
        .unwrap_or_else(|| group.path.to_string_lossy().to_string())
}

fn backup_file(path: &Path) -> Result<()> {
    let backup = path.with_file_name(format!("{}.bak.{}", file_name(path), timestamp()));
    fs::copy(path, backup).context("备份 mihomo 配置失败")?;
    Ok(())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.yaml")
        .to_string()
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|item| item.as_secs())
        .unwrap_or_default()
}

#[derive(Clone, Debug)]
struct ActiveRuleSet {
    name: String,
    target: String,
}
