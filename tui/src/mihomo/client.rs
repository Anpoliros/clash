//! mihomo API 客户端：提供 runtime-first 的节点、配置、日志和测速操作。

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::{Client, RequestBuilder};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

use super::models::*;

#[derive(Clone)]
pub struct MihomoClient {
    base: String,
    secret: Option<String>,
    http: Client,
}

impl MihomoClient {
    pub fn new(base: String, secret: Option<String>) -> Self {
        Self {
            base,
            secret,
            http: Client::builder()
                .timeout(Duration::from_secs(3))
                .no_proxy()
                .build()
                .expect("reqwest client build should not fail"),
        }
    }

    // #----基础接口----
    pub async fn version(&self) -> Result<Option<String>> {
        let resp = self
            .auth(self.http.get(self.url("/version")))
            .send()
            .await?;
        let body: VersionResponse = resp.error_for_status()?.json().await?;
        Ok(body.version)
    }

    pub async fn configs(&self) -> Result<ConfigsResponse> {
        let resp = self
            .auth(self.http.get(self.url("/configs")))
            .send()
            .await?;
        resp.error_for_status()?
            .json()
            .await
            .context("解析 /configs 失败")
    }

    // #----代理接口----
    pub async fn proxies(&self) -> Result<ProxiesResponse> {
        let resp = self
            .auth(self.http.get(self.url("/proxies")))
            .send()
            .await?;
        resp.error_for_status()?
            .json()
            .await
            .context("解析 /proxies 失败")
    }

    pub async fn providers(&self) -> Result<ProvidersResponse> {
        let resp = self
            .auth(self.http.get(self.url("/providers/proxies")))
            .send()
            .await?;
        resp.error_for_status()?
            .json()
            .await
            .context("解析 /providers/proxies 失败")
    }

    pub async fn select_proxy(&self, group: &str, proxy: &str) -> Result<()> {
        let path = format!("/proxies/{}", urlencoding::encode(group));
        let resp = self
            .auth(self.http.put(self.url(&path)))
            .json(&json!({ "name": proxy }))
            .send()
            .await?;
        Self::ensure_success(resp).await?;
        Ok(())
    }

    pub async fn delay(&self, proxy: &str) -> Result<u64> {
        let path = format!(
            "/proxies/{}/delay?url={}&timeout=5000",
            urlencoding::encode(proxy),
            urlencoding::encode("https://cp.cloudflare.com")
        );
        let body: DelayResponse = self
            .auth(self.http.get(self.url(&path)))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(body.delay)
    }

    pub async fn refresh_provider(&self, provider: &str) -> Result<()> {
        let path = format!("/providers/proxies/{}", urlencoding::encode(provider));
        let resp = self.auth(self.http.put(self.url(&path))).send().await?;
        Self::ensure_success(resp).await?;
        Ok(())
    }

    pub async fn healthcheck_provider(&self, provider: &str) -> Result<()> {
        let path = format!(
            "/providers/proxies/{}/healthcheck",
            urlencoding::encode(provider)
        );
        self.auth(self.http.get(self.url(&path)))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    // #----日志流----
    pub async fn stream_logs(&self, tx: mpsc::UnboundedSender<String>) -> Result<()> {
        let mut url = self.base.replacen("http://", "ws://", 1);
        url = url.replacen("https://", "wss://", 1);
        url.push_str("/logs");
        if let Some(secret) = &self.secret {
            url.push_str("?token=");
            url.push_str(&urlencoding::encode(secret));
        }

        let (stream, _) = connect_async(url.as_str())
            .await
            .context("连接 /logs websocket 失败")?;
        let (_, mut read) = stream.split();
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if msg.is_text() {
                let _ = tx.send(msg.into_text()?);
            }
        }
        Ok(())
    }

    // #----内部工具----
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn auth(&self, req: RequestBuilder) -> RequestBuilder {
        match &self.secret {
            Some(secret) if !secret.is_empty() => req.bearer_auth(secret),
            _ => req,
        }
    }

    async fn ensure_success(resp: reqwest::Response) -> Result<()> {
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let text = resp.text().await.unwrap_or_default();
        Err(anyhow!("mihomo API {status}: {text}"))
    }
}
