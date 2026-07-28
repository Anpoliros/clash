//! API 数据模型：只覆盖 TUI 用到的 mihomo 字段，其他字段交给 serde 忽略。
//! 修改时间：2026-07-28 18:15:12 +08:00

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
    #[serde(default, rename = "subscriptionInfo", alias = "subscription-info")]
    pub subscription_info: Option<SubscriptionInfo>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SubscriptionInfo {
    #[serde(default, alias = "Upload", alias = "UPLOAD")]
    pub upload: i64,
    #[serde(default, alias = "Download", alias = "DOWNLOAD")]
    pub download: i64,
    #[serde(default, alias = "Total", alias = "TOTAL")]
    pub total: i64,
    #[serde(default, alias = "Expire", alias = "EXPIRE")]
    pub expire: i64,
}

#[derive(Debug, Deserialize)]
pub struct DelayResponse {
    pub delay: u64,
}

#[derive(Debug, Deserialize)]
pub struct VersionResponse {
    pub version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::ProvidersResponse;

    #[test]
    fn provider_subscription_info_accepts_current_mihomo_fields() {
        let response: ProvidersResponse = serde_json::from_str(
            r#"{
                "providers": {
                    "airport": {
                        "name": "airport",
                        "proxies": [],
                        "subscriptionInfo": {
                            "Upload": 100,
                            "Download": 200,
                            "Total": 1000,
                            "Expire": 1788134400
                        }
                    }
                }
            }"#,
        )
        .unwrap();
        let info = response.providers["airport"]
            .subscription_info
            .as_ref()
            .unwrap();

        assert_eq!(info.upload, 100);
        assert_eq!(info.download, 200);
        assert_eq!(info.total, 1000);
        assert_eq!(info.expire, 1788134400);
    }

    #[test]
    fn provider_subscription_info_accepts_legacy_field_name() {
        let response: ProvidersResponse = serde_json::from_str(
            r#"{
                "providers": {
                    "airport": {
                        "name": "airport",
                        "proxies": [],
                        "subscription-info": {
                            "upload": 100,
                            "download": 200,
                            "total": 1000,
                            "expire": 1788134400
                        }
                    }
                }
            }"#,
        )
        .unwrap();

        assert!(response.providers["airport"].subscription_info.is_some());
    }
}
