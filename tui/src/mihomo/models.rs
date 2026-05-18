//! API 数据模型：只覆盖 MVP 用到的 mihomo 字段，其他字段交给 serde 忽略。

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ConfigsResponse {
    pub port: Option<u16>,
    #[serde(rename = "mixed-port")]
    pub mixed_port: Option<u16>,
    pub tun: Option<TunConfig>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TunConfig {
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProxiesResponse {
    pub proxies: HashMap<String, ProxyItem>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProxyItem {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub now: Option<String>,
    #[serde(default)]
    pub all: Vec<String>,
    #[serde(default)]
    pub history: Vec<DelayHistory>,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DelayHistory {
    pub delay: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProvidersResponse {
    pub providers: HashMap<String, ProviderItem>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProviderItem {
    pub name: String,
    #[serde(default)]
    pub proxies: Vec<ProxyItem>,
}

#[derive(Debug, Deserialize)]
pub struct DelayResponse {
    pub delay: u64,
}

#[derive(Debug, Deserialize)]
pub struct VersionResponse {
    pub version: Option<String>,
}
