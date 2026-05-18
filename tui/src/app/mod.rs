//! 应用状态与事件循环：明确拆分 UI state、runtime state 与 mihomo state。

use std::{collections::HashMap, io};

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::{process::Command, sync::mpsc, time::Duration};

use crate::{
    config::runtime_config,
    events::{self, AppEvent},
    mihomo::models::{DelayHistory, ProviderItem, ProxyItem},
    runtime::bootstrap::BootContext,
};

const DIRECT_GROUP: &str = "GLOBAL";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    General,
    Proxies,
    Rules,
}

#[derive(Clone, Debug)]
pub struct UiState {
    pub page: Page,
    pub cursor: usize,
    pub node_col: usize,
    pub scroll: usize,
    pub log_scroll: usize,
    pub ticks: u64,
    pub logs_open: bool,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct RuntimeState {
    pub proxy_enabled: bool,
    pub tun_enabled: bool,
    pub proxy_port: Option<u16>,
    pub manage_port: u16,
    pub active_node: String,
    pub pid: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct MihomoState {
    pub version: Option<String>,
    pub root_group: Option<String>,
    pub node_group: Option<String>,
    pub auto_group: Option<String>,
    pub providers: Vec<ProviderView>,
}

#[derive(Clone, Debug)]
pub struct ProviderView {
    pub name: String,
    pub expanded: bool,
    pub nodes: Vec<NodeView>,
}

#[derive(Clone, Debug)]
pub struct NodeView {
    pub name: String,
    pub delay: Option<u64>,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub enum RowRef {
    AutoSelect,
    Provider(usize),
    NodeRow(usize, usize),
}

pub struct App {
    pub boot: BootContext,
    pub ui: UiState,
    pub runtime: RuntimeState,
    pub mihomo: MihomoState,
    pub rows: Vec<RowRef>,
    pub logs: Vec<String>,
    log_tx: mpsc::UnboundedSender<String>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
}

// #----运行入口----
pub async fn run(boot: BootContext) -> Result<()> {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let (log_tx, log_rx) = mpsc::unbounded_channel();
    events::spawn_input(event_tx.clone());
    events::spawn_log_bridge(log_rx, event_tx.clone());

    let log_client = boot.client.clone();
    let log_event_tx = log_tx.clone();
    tokio::spawn(async move {
        let _ = log_client.stream_logs(log_event_tx).await;
    });

    let mut app = App::new(boot, log_tx, event_tx);
    app.refresh().await;

    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    loop {
        terminal.draw(|frame| crate::ui::draw(frame, &app))?;
        if let Some(event) = event_rx.recv().await {
            if app.handle(event).await? {
                break;
            }
        }
    }

    Ok(())
}

impl App {
    pub fn new(
        boot: BootContext,
        log_tx: mpsc::UnboundedSender<String>,
        event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Self {
        let manage_port = boot
            .config
            .controller
            .rsplit(':')
            .next()
            .and_then(|v| v.parse().ok())
            .unwrap_or(9090);

        Self {
            runtime: RuntimeState {
                proxy_enabled: boot.process.is_running(),
                tun_enabled: false,
                proxy_port: None,
                manage_port,
                active_node: "-".into(),
                pid: boot.process.pid(),
            },
            ui: UiState {
                page: Page::Proxies,
                cursor: 0,
                node_col: 0,
                scroll: 0,
                log_scroll: 0,
                ticks: 0,
                logs_open: false,
                status: "Ready".into(),
            },
            mihomo: MihomoState::default(),
            rows: Vec::new(),
            logs: Vec::new(),
            boot,
            log_tx,
            event_tx,
        }
    }

    // #----事件处理----
    async fn handle(&mut self, event: AppEvent) -> Result<bool> {
        match event {
            AppEvent::Key(key) => return self.handle_key(key).await,
            AppEvent::Mouse(mouse) => self.handle_mouse(mouse).await?,
            AppEvent::Log(line) => self.push_log(line),
            AppEvent::DelayResult {
                provider,
                node,
                delay,
            } => self.apply_delay(&provider, &node, delay),
            AppEvent::ProviderPingDone(name) => {
                self.ui.status = format!("{name} 测速完成");
            }
            AppEvent::Tick => {
                self.ui.ticks = self.ui.ticks.saturating_add(1);
                self.runtime.proxy_enabled = self.boot.process.is_running();
                self.runtime.pid = self.boot.process.pid();
                if self.runtime.proxy_enabled && self.ui.ticks % 40 == 0 {
                    self.refresh().await;
                }
            }
        }
        Ok(false)
    }

    async fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            return Ok(true);
        }
        if self.ui.logs_open {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                self.ui.logs_open = false;
            }
            return Ok(false);
        }

        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('1') => self.ui.page = Page::Proxies,
            KeyCode::Char('2') => self.ui.page = Page::General,
            KeyCode::Char('3') => self.ui.page = Page::Rules,
            KeyCode::Tab => self.next_page(),
            KeyCode::BackTab => self.prev_page(),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::Left | KeyCode::Char('h') => self.handle_horizontal(-1),
            KeyCode::Right | KeyCode::Char('l') => self.handle_horizontal(1),
            KeyCode::Enter => {
                let result = self.activate().await;
                self.handle_action_result(result);
            }
            KeyCode::Char(' ') => {
                let result = self.activate().await;
                self.handle_action_result(result);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                let result = self.refresh_selected_provider().await;
                self.handle_action_result(result);
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                let result = self.ping_selected_provider().await;
                self.handle_action_result(result);
            }
            KeyCode::Char('s') | KeyCode::Char('S') => self.sort_selected_provider(),
            _ => {}
        }
        Ok(false)
    }

    async fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                if mouse.row == 1 {
                    self.ui.page = match mouse.column {
                        0..=12 => Page::Proxies,
                        13..=25 => Page::General,
                        _ => Page::Rules,
                    };
                } else if self.ui.page == Page::Proxies && mouse.row > 4 {
                    self.ui.cursor = self
                        .ui
                        .scroll
                        .saturating_add((mouse.row as usize).saturating_sub(5))
                        .min(self.rows.len().saturating_sub(1));
                    self.ui.node_col = self.node_col_from_mouse(mouse.column);
                    let result = self.handle_proxy_click(mouse.column).await;
                    self.handle_action_result(result);
                }
            }
            MouseEventKind::ScrollDown if self.ui.logs_open => self.scroll_logs(-3),
            MouseEventKind::ScrollUp if self.ui.logs_open => self.scroll_logs(3),
            MouseEventKind::ScrollDown => self.move_cursor(1),
            MouseEventKind::ScrollUp => self.move_cursor(-1),
            _ => {}
        }
        Ok(())
    }

    // #----业务动作----
    async fn activate(&mut self) -> Result<()> {
        match self.ui.page {
            Page::General => match self.ui.cursor {
                0 => self.toggle_proxy().await?,
                1 => self.toggle_tun().await?,
                5 => self.ui.logs_open = true,
                _ => {}
            },
            Page::Proxies => {
                if let Some(row) = self.rows.get(self.ui.cursor).cloned() {
                    match row {
                        RowRef::AutoSelect => self.select_auto().await?,
                        RowRef::Provider(idx) => {
                            if let Some(provider) = self.mihomo.providers.get_mut(idx) {
                                provider.expanded = !provider.expanded;
                                self.rebuild_rows();
                            }
                        }
                        RowRef::NodeRow(provider_idx, row_start) => {
                            if let Some(node_idx) =
                                self.selected_node_index(provider_idx, row_start)
                            {
                                self.select_node(provider_idx, node_idx).await?;
                            }
                        }
                    }
                }
            }
            Page::Rules => {}
        }
        Ok(())
    }

    async fn select_auto(&mut self) -> Result<()> {
        let Some(root_group) = self.mihomo.root_group.clone() else {
            self.ui.status = "未找到根选择组".into();
            return Ok(());
        };
        let Some(auto_group) = self.mihomo.auto_group.clone() else {
            self.ui.status = "未找到自动选择组".into();
            return Ok(());
        };

        self.boot
            .client
            .select_proxy(&root_group, &auto_group)
            .await?;
        self.runtime.active_node = auto_group;
        self.ui.status = "已切回自动选择".into();
        self.refresh().await;
        Ok(())
    }

    async fn toggle_proxy(&mut self) -> Result<()> {
        if self.runtime.proxy_enabled {
            self.boot.process.stop().await?;
            self.ui.status = "mihomo stopped".into();
        } else {
            let sudo = self.runtime.tun_enabled;
            if sudo {
                self.sudo_validate().await?;
            }
            self.boot.process.start(self.log_tx.clone(), sudo).await?;
            self.ui.status = "mihomo starting".into();
            self.wait_for_api().await;
        }
        self.runtime.proxy_enabled = self.boot.process.is_running();
        self.runtime.pid = self.boot.process.pid();
        Ok(())
    }

    async fn toggle_tun(&mut self) -> Result<()> {
        let enabled = !self.runtime.tun_enabled;
        if enabled {
            self.sudo_validate().await?;
        }
        runtime_config::set_tun_enabled(&self.boot.config.path, enabled)?;
        if let Err(err) = self
            .boot
            .client
            .reload_config(self.boot.config.path.to_string_lossy().as_ref())
            .await
        {
            self.ui.status = format!("TUN 配置已写入，reload 失败：{err}");
            return Ok(());
        }
        self.runtime.tun_enabled = enabled;
        self.ui.status = if enabled {
            "TUN enabled"
        } else {
            "TUN disabled"
        }
        .into();
        Ok(())
    }

    async fn select_node(&mut self, provider_idx: usize, node_idx: usize) -> Result<()> {
        let Some(node_group) = self.mihomo.node_group.clone() else {
            self.ui.status = "未找到可切换的 Selector 组".into();
            return Ok(());
        };
        let Some(node) = self
            .mihomo
            .providers
            .get(provider_idx)
            .and_then(|provider| provider.nodes.get(node_idx))
            .cloned()
        else {
            return Ok(());
        };

        self.boot
            .client
            .select_proxy(&node_group, &node.name)
            .await?;
        if let Some(root_group) = self.mihomo.root_group.clone() {
            if root_group != node_group {
                self.boot
                    .client
                    .select_proxy(&root_group, &node_group)
                    .await?;
            }
        }
        self.runtime.active_node = node.name.clone();
        self.ui.status = "节点已切换".into();
        self.refresh().await;
        Ok(())
    }

    pub async fn refresh(&mut self) {
        if let Ok(version) = self.boot.client.version().await {
            self.mihomo.version = version;
        }
        if let Ok(configs) = self.boot.client.configs().await {
            self.runtime.proxy_port = configs.mixed_port.or(configs.port);
            self.runtime.tun_enabled = configs.tun.and_then(|tun| tun.enable).unwrap_or(false);
        }
        match self.load_providers().await {
            Ok(()) => self.ui.status = "已同步 mihomo 状态".into(),
            Err(err) => self.ui.status = format!("等待 mihomo API：{err}"),
        }
    }

    async fn refresh_selected_provider(&mut self) -> Result<()> {
        if let Some(name) = self.selected_provider_name() {
            self.boot.client.refresh_provider(&name).await?;
            self.ui.status = format!("{name} 已刷新");
            self.refresh().await;
        }
        Ok(())
    }

    async fn ping_selected_provider(&mut self) -> Result<()> {
        if let Some(name) = self.selected_provider_name() {
            let nodes = self
                .mihomo
                .providers
                .iter()
                .find(|item| item.name == name)
                .map(|provider| {
                    provider
                        .nodes
                        .iter()
                        .map(|node| node.name.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let client = self.boot.client.clone();
            let tx = self.event_tx.clone();
            self.ui.status = format!("{name} 测速中...");
            tokio::spawn(async move {
                let _ = client.healthcheck_provider(&name).await;
                for node in nodes {
                    let delay = client.delay(&node).await.ok();
                    let _ = tx.send(AppEvent::DelayResult {
                        provider: name.clone(),
                        node,
                        delay,
                    });
                }
                let _ = tx.send(AppEvent::ProviderPingDone(name));
            });
        }
        Ok(())
    }

    fn sort_selected_provider(&mut self) {
        if let Some(name) = self.selected_provider_name() {
            if let Some(provider) = self
                .mihomo
                .providers
                .iter_mut()
                .find(|item| item.name == name)
            {
                provider
                    .nodes
                    .sort_by_key(|node| node.delay.unwrap_or(u64::MAX));
                self.ui.status = format!("{name} 已按延迟排序");
                self.rebuild_rows();
            }
        }
    }

    // #----状态同步----
    async fn load_providers(&mut self) -> Result<()> {
        let proxies = self.boot.client.proxies().await?;
        let root_group = choose_root_group(&proxies.proxies);
        let node_group = choose_node_group(&proxies.proxies).or_else(|| root_group.clone());
        let auto_group = choose_auto_group(root_group.as_deref(), &node_group, &proxies.proxies);
        self.mihomo.root_group = root_group.clone();
        self.mihomo.node_group = node_group.clone();
        self.mihomo.auto_group = auto_group;
        let root_now = root_group
            .as_ref()
            .and_then(|name| proxies.proxies.get(name))
            .and_then(|item| item.now.clone());
        self.runtime.active_node = match (root_now, node_group.as_deref()) {
            (Some(root_now), Some(node_group)) if root_now != node_group => root_now,
            (_, Some(node_group)) => proxies
                .proxies
                .get(node_group)
                .and_then(|item| item.now.clone())
                .unwrap_or_else(|| "-".into()),
            (Some(root_now), None) => root_now,
            _ => "-".into(),
        };

        let providers = match self.boot.client.providers().await {
            Ok(resp) if !resp.providers.is_empty() => build_from_real_providers(
                resp.providers,
                &self.boot.config.provider_names,
                &self.runtime.active_node,
            ),
            _ => build_from_proxy_groups(&proxies.proxies, &self.runtime.active_node),
        };

        let expanded: HashMap<_, _> = self
            .mihomo
            .providers
            .iter()
            .map(|item| (item.name.clone(), item.expanded))
            .collect();
        self.mihomo.providers = providers
            .into_iter()
            .map(|mut provider| {
                if let Some(was_expanded) = expanded.get(&provider.name) {
                    provider.expanded = *was_expanded;
                }
                provider
            })
            .collect();
        self.rebuild_rows();
        Ok(())
    }

    fn rebuild_rows(&mut self) {
        self.rows.clear();
        if self.mihomo.auto_group.is_some() {
            self.rows.push(RowRef::AutoSelect);
        }
        for (provider_idx, provider) in self.mihomo.providers.iter().enumerate() {
            self.rows.push(RowRef::Provider(provider_idx));
            if provider.expanded {
                for node_idx in (0..provider.nodes.len()).step_by(2) {
                    self.rows.push(RowRef::NodeRow(provider_idx, node_idx));
                }
            }
        }
        self.ui.cursor = self.ui.cursor.min(self.rows.len().saturating_sub(1));
        self.fix_node_col();
        self.keep_cursor_visible();
    }

    // #----UI 工具----
    fn next_page(&mut self) {
        self.ui.page = match self.ui.page {
            Page::Proxies => Page::General,
            Page::General => Page::Rules,
            Page::Rules => Page::Proxies,
        };
        self.ui.cursor = 0;
        self.ui.scroll = 0;
        self.ui.node_col = 0;
    }

    fn prev_page(&mut self) {
        self.ui.page = match self.ui.page {
            Page::Proxies => Page::Rules,
            Page::General => Page::Proxies,
            Page::Rules => Page::General,
        };
        self.ui.cursor = 0;
        self.ui.scroll = 0;
        self.ui.node_col = 0;
    }

    fn move_cursor(&mut self, delta: isize) {
        let max = match self.ui.page {
            Page::General => 5,
            Page::Proxies => self.rows.len().saturating_sub(1),
            Page::Rules => 0,
        };
        self.ui.cursor = if delta.is_negative() {
            self.ui.cursor.saturating_sub((-delta) as usize)
        } else {
            self.ui.cursor.saturating_add(delta as usize)
        }
        .min(max);
        self.fix_node_col();
        self.keep_cursor_visible();
    }

    fn push_log(&mut self, line: String) {
        self.logs.push(line);
        if self.logs.len() > 1000 {
            let drain = self.logs.len() - 1000;
            self.logs.drain(0..drain);
            self.ui.log_scroll = self.ui.log_scroll.saturating_sub(drain);
        }
        self.ui.log_scroll = self.ui.log_scroll.min(self.max_log_scroll());
    }

    fn selected_provider_name(&self) -> Option<String> {
        match self.rows.get(self.ui.cursor)? {
            RowRef::AutoSelect => None,
            RowRef::Provider(idx) => self
                .mihomo
                .providers
                .get(*idx)
                .map(|item| item.name.clone()),
            RowRef::NodeRow(idx, _) => self
                .mihomo
                .providers
                .get(*idx)
                .map(|item| item.name.clone()),
        }
    }

    async fn handle_proxy_click(&mut self, column: u16) -> Result<()> {
        match self.rows.get(self.ui.cursor).cloned() {
            Some(RowRef::AutoSelect) => self.activate().await?,
            Some(RowRef::Provider(_)) if column >= 56 => self.sort_selected_provider(),
            Some(RowRef::Provider(_)) if column >= 47 => self.ping_selected_provider().await?,
            Some(RowRef::Provider(_)) if column >= 36 => self.refresh_selected_provider().await?,
            _ => self.activate().await?,
        }
        Ok(())
    }

    fn selected_node_index(&self, provider_idx: usize, row_start: usize) -> Option<usize> {
        let provider = self.mihomo.providers.get(provider_idx)?;
        let idx = row_start + self.ui.node_col.min(1);
        if idx < provider.nodes.len() {
            Some(idx)
        } else {
            Some(row_start)
        }
    }

    fn move_node_col(&mut self, delta: isize) {
        if !matches!(self.rows.get(self.ui.cursor), Some(RowRef::NodeRow(_, _))) {
            return;
        }
        self.ui.node_col = if delta.is_negative() { 0 } else { 1 };
        self.fix_node_col();
    }

    fn handle_horizontal(&mut self, delta: isize) {
        match self.rows.get(self.ui.cursor).cloned() {
            Some(RowRef::Provider(idx)) => {
                if let Some(provider) = self.mihomo.providers.get_mut(idx) {
                    provider.expanded = delta.is_positive();
                    self.rebuild_rows();
                }
            }
            Some(RowRef::NodeRow(_, _)) => self.move_node_col(delta),
            _ => {}
        }
    }

    fn fix_node_col(&mut self) {
        if let Some(RowRef::NodeRow(provider_idx, row_start)) = self.rows.get(self.ui.cursor) {
            let len = self
                .mihomo
                .providers
                .get(*provider_idx)
                .map(|provider| provider.nodes.len())
                .unwrap_or_default();
            if row_start + self.ui.node_col >= len {
                self.ui.node_col = 0;
            }
        } else {
            self.ui.node_col = 0;
        }
    }

    fn keep_cursor_visible(&mut self) {
        let height = self.ui_visible_rows();
        if self.ui.cursor < self.ui.scroll {
            self.ui.scroll = self.ui.cursor;
        } else if self.ui.cursor >= self.ui.scroll.saturating_add(height) {
            self.ui.scroll = self.ui.cursor.saturating_sub(height.saturating_sub(1));
        }
    }

    fn ui_visible_rows(&self) -> usize {
        12
    }

    fn node_col_from_mouse(&self, column: u16) -> usize {
        if column > 42 {
            1
        } else {
            0
        }
    }

    fn handle_action_result(&mut self, result: Result<()>) {
        if let Err(err) = result {
            self.ui.status = format!("操作失败：{err}");
        }
    }

    fn scroll_logs(&mut self, delta: isize) {
        self.ui.log_scroll = if delta.is_negative() {
            self.ui.log_scroll.saturating_sub((-delta) as usize)
        } else {
            self.ui.log_scroll.saturating_add(delta as usize)
        }
        .min(self.max_log_scroll());
    }

    fn max_log_scroll(&self) -> usize {
        self.logs.len().saturating_sub(1)
    }

    async fn wait_for_api(&mut self) {
        for _ in 0..20 {
            if self.boot.client.version().await.is_ok() {
                self.refresh().await;
                return;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        self.ui.status = "mihomo 已启动，等待 external-controller 就绪".into();
    }

    async fn sudo_validate(&mut self) -> Result<()> {
        self.ui.status = "需要 sudo 权限，正在切换到终端提示".into();
        TerminalGuard::leave();
        let status = Command::new("sudo").arg("-v").status().await;
        TerminalGuard::restore()?;
        match status {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(anyhow::anyhow!("sudo 验证失败：{status}")),
            Err(err) => Err(err).context("无法执行 sudo -v"),
        }
    }

    fn apply_delay(&mut self, provider: &str, node: &str, delay: Option<u64>) {
        if let Some(item) = self
            .mihomo
            .providers
            .iter_mut()
            .find(|item| item.name == provider)
            .and_then(|provider| provider.nodes.iter_mut().find(|item| item.name == node))
        {
            item.delay = delay;
        }
    }
}

// #----数据转换----
fn choose_root_group(proxies: &std::collections::HashMap<String, ProxyItem>) -> Option<String> {
    ["节点选择", "Proxy", DIRECT_GROUP]
        .iter()
        .find(|name| {
            proxies
                .get(**name)
                .map(|item| !item.all.is_empty())
                .unwrap_or(false)
        })
        .map(|name| (*name).to_string())
        .or_else(|| {
            proxies
                .values()
                .find(|item| item.kind == "Selector" && !item.all.is_empty())
                .map(|item| item.name.clone())
        })
}

fn choose_node_group(proxies: &std::collections::HashMap<String, ProxyItem>) -> Option<String> {
    ["全部节点", "GLOBAL"]
        .iter()
        .find(|name| {
            proxies
                .get(**name)
                .map(|item| item.kind == "Selector" && has_real_nodes(item, proxies))
                .unwrap_or(false)
        })
        .map(|name| (*name).to_string())
        .or_else(|| {
            proxies
                .values()
                .find(|item| item.kind == "Selector" && has_real_nodes(item, proxies))
                .map(|item| item.name.clone())
        })
}

fn choose_auto_group(
    root_group: Option<&str>,
    node_group: &Option<String>,
    proxies: &std::collections::HashMap<String, ProxyItem>,
) -> Option<String> {
    let root = root_group.and_then(|name| proxies.get(name))?;
    let node_group = node_group.as_deref();
    ["自动选择", "Auto", "AUTO", "UrlTest", "Fallback"]
        .iter()
        .find(|name| root.all.iter().any(|item| item == **name))
        .map(|name| (*name).to_string())
        .or_else(|| {
            root.all
                .iter()
                .filter(|name| Some(name.as_str()) != node_group)
                .filter_map(|name| proxies.get(name))
                .find(|item| matches!(item.kind.as_str(), "URLTest" | "Fallback" | "LoadBalance"))
                .map(|item| item.name.clone())
        })
}

fn build_from_real_providers(
    providers: std::collections::HashMap<String, ProviderItem>,
    preferred_order: &[String],
    active: &str,
) -> Vec<ProviderView> {
    let ordered: Vec<_> = if preferred_order.is_empty() {
        providers
            .iter()
            .map(|(name, provider)| (name.clone(), provider.clone()))
            .collect()
    } else {
        preferred_order
            .iter()
            .filter_map(|name| {
                providers
                    .get(name)
                    .cloned()
                    .map(|provider| (name.clone(), provider))
            })
            .collect()
    };

    let mut list: Vec<_> = ordered
        .into_iter()
        .map(|(fallback_name, provider)| ProviderView {
            name: if provider.name.is_empty() {
                fallback_name
            } else {
                provider.name
            },
            expanded: true,
            nodes: provider
                .proxies
                .into_iter()
                .map(|proxy| node_from_proxy(proxy, active))
                .collect(),
        })
        .filter(|provider| !provider.nodes.is_empty())
        .collect();
    if preferred_order.is_empty() {
        list.sort_by(|a, b| a.name.cmp(&b.name));
    }
    list
}

fn build_from_proxy_groups(
    proxies: &std::collections::HashMap<String, ProxyItem>,
    active: &str,
) -> Vec<ProviderView> {
    let mut providers: Vec<_> = proxies
        .values()
        .filter(|item| !item.hidden && !item.all.is_empty())
        .map(|group| ProviderView {
            name: group.name.clone(),
            expanded: group.name != "GLOBAL",
            nodes: group
                .all
                .iter()
                .filter_map(|name| proxies.get(name).cloned())
                .filter(|proxy| !is_builtin_proxy(&proxy.name))
                .map(|proxy| node_from_proxy(proxy, active))
                .collect(),
        })
        .filter(|provider| !provider.nodes.is_empty())
        .collect();
    providers.sort_by(|a, b| a.name.cmp(&b.name));
    providers
}

fn node_from_proxy(proxy: ProxyItem, active: &str) -> NodeView {
    NodeView {
        delay: latest_delay(&proxy.history),
        active: proxy.name == active,
        name: proxy.name,
    }
}

fn latest_delay(history: &[DelayHistory]) -> Option<u64> {
    history
        .iter()
        .rev()
        .find_map(|item| item.delay)
        .filter(|delay| *delay > 0)
}

fn is_builtin_proxy(name: &str) -> bool {
    matches!(
        name,
        "DIRECT" | "REJECT" | "REJECT-DROP" | "PASS" | "COMPATIBLE"
    )
}

fn has_real_nodes(
    group: &ProxyItem,
    proxies: &std::collections::HashMap<String, ProxyItem>,
) -> bool {
    group
        .all
        .iter()
        .filter_map(|name| proxies.get(name))
        .any(|proxy| proxy.all.is_empty() && !is_builtin_proxy(&proxy.name))
}

// #----终端守卫----
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        Self::restore()?;
        Ok(Self)
    }

    fn restore() -> Result<()> {
        enable_raw_mode().context("无法进入 raw mode，请在交互式终端中运行 clash-tui")?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)
            .context("无法进入 TUI alternate screen，请确认 stdout 是终端")?;
        Ok(())
    }

    fn leave() {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        TerminalGuard::leave();
    }
}
