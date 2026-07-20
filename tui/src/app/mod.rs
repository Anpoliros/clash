//! 应用状态与事件循环：明确拆分 UI state、runtime state 与 mihomo state。

use std::{collections::HashMap, io};

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use unicode_width::UnicodeWidthStr;

use crate::{
    config::{app_config, rules_config, rules_config::RuleGroup},
    events::{self, AppEvent},
    mihomo::models::{DelayHistory, ProviderItem, ProxyItem},
    runtime::bootstrap::BootContext,
};

const DIRECT_GROUP: &str = "GLOBAL";
const MIN_NODE_COLUMNS: usize = 1;
const MAX_NODE_COLUMNS: usize = 3;
const NODE_INDENT_WIDTH: u16 = 8;
const PROXY_LIST_CONTENT_TOP: u16 = 6;
const GENERAL_MAX_CURSOR: usize = 12;
const GENERAL_CONTENT_TOP: u16 = 3;
const GENERAL_TOTAL_ROWS: usize = 18;
const GENERAL_CURSOR_ROWS: [usize; GENERAL_MAX_CURSOR + 1] =
    [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 16, 17];
const RULES_LIST_HEADER_ROWS: usize = 6;
const RULES_DETAIL_HEADER_ROWS: usize = 7;

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
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub node_columns: usize,
    pub config_edit: Option<ConfigField>,
    pub input_buffer: String,
    pub input_cursor: usize,
    pub ticks: u64,
    pub logs_open: bool,
    pub help_open: bool,
    pub status: String,
    pub rule_input: Option<RuleInput>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigField {
    NodeNameWidth,
    NodeItemMinWidth,
    NodeMinGapWidth,
    NodeReserveWidth,
    NodeColumnGapWidth,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleInput {
    Search,
    NewGroup,
    RenameGroup(usize),
    EditTarget(usize),
    AddRule(usize),
    EditRule(usize, usize),
}

#[derive(Clone, Copy, Debug)]
pub struct NodeLayout {
    pub name_width: u16,
    pub item_width: u16,
    pub min_gap_width: u16,
    pub reserve_width: u16,
}

impl ConfigField {
    fn label(self) -> &'static str {
        match self {
            ConfigField::NodeNameWidth => "节点名宽度",
            ConfigField::NodeItemMinWidth => "节点项最小宽度",
            ConfigField::NodeMinGapWidth => "最小间隔",
            ConfigField::NodeReserveWidth => "尾部保留宽度",
            ConfigField::NodeColumnGapWidth => "列间宽度",
        }
    }
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

#[derive(Clone, Debug, Default)]
pub struct RulesState {
    pub groups: Vec<RuleGroup>,
    pub detail_group: Option<usize>,
    pub comma_display: bool,
    pub search: String,
    pub config_backup_done: bool,
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
    pub rules: RulesState,
    pub rows: Vec<RowRef>,
    pub logs: Vec<String>,
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

    let mut app = App::new(boot, event_tx);
    app.refresh().await;

    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    loop {
        let size = terminal.size()?;
        app.set_viewport(size.width, size.height);
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
    pub fn new(boot: BootContext, event_tx: mpsc::UnboundedSender<AppEvent>) -> Self {
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
                terminal_width: 80,
                terminal_height: 24,
                node_columns: 2,
                config_edit: None,
                input_buffer: String::new(),
                input_cursor: 0,
                ticks: 0,
                logs_open: false,
                help_open: false,
                status: "Ready".into(),
                rule_input: None,
            },
            mihomo: MihomoState::default(),
            rules: RulesState {
                groups: rules_config::load(&boot.source_config, &boot.work_dir)
                    .map(|config| config.groups)
                    .unwrap_or_default(),
                detail_group: None,
                comma_display: false,
                search: String::new(),
                config_backup_done: false,
            },
            rows: Vec::new(),
            logs: Vec::new(),
            boot,
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
        if self.ui.rule_input.is_some() {
            return self.handle_rule_input(key).await;
        }
        if self.ui.config_edit.is_some() {
            return self.handle_config_input(key);
        }
        if self.ui.help_open {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
            ) {
                self.ui.help_open = false;
            }
            return Ok(false);
        }
        if self.ui.logs_open {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                self.ui.logs_open = false;
            }
            return Ok(false);
        }

        match key.code {
            KeyCode::Char('q')
                if self.ui.page == Page::Rules && self.rules.detail_group.is_some() =>
            {
                self.rules_back_or_clear_search()
            }
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('?') => self.ui.help_open = true,
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
                let result = if self.ui.page == Page::Rules {
                    self.toggle_selected_rule_group().await
                } else {
                    self.activate().await
                };
                self.handle_action_result(result);
            }
            KeyCode::Char('r') | KeyCode::Char('R') if self.ui.page == Page::Proxies => {
                let result = self.refresh_selected_provider().await;
                self.handle_action_result(result);
            }
            KeyCode::Char('p') | KeyCode::Char('P') if self.ui.page == Page::Proxies => {
                let result = self.ping_selected_provider().await;
                self.handle_action_result(result);
            }
            KeyCode::Char('s') | KeyCode::Char('S') if self.ui.page == Page::Proxies => {
                self.sort_selected_provider()
            }
            KeyCode::Char('/') if self.ui.page == Page::Rules => self.start_rule_search(),
            KeyCode::Esc if self.ui.page == Page::Rules => self.rules_back_or_clear_search(),
            KeyCode::Char('n') | KeyCode::Char('N') if self.ui.page == Page::Rules => {
                self.ui.cursor = 0;
                self.ui.scroll = 0;
                self.start_rule_input(RuleInput::NewGroup, "")
            }
            KeyCode::Char('e') | KeyCode::Char('E') if self.ui.page == Page::Rules => {
                self.start_edit_selected_rule_item()
            }
            KeyCode::Char('m') | KeyCode::Char('M') if self.ui.page == Page::Rules => {
                self.start_rename_selected_rule_group()
            }
            KeyCode::Char('x') | KeyCode::Char('X') if self.ui.page == Page::Rules => {
                let result = self.delete_selected_rule_item().await;
                self.handle_action_result(result);
            }
            KeyCode::Char('t') | KeyCode::Char('T') if self.ui.page == Page::Rules => {
                self.rules.comma_display = !self.rules.comma_display;
                self.ui.status = if self.rules.comma_display {
                    "Rules 已切换为逗号原文显示".into()
                } else {
                    "Rules 已切换为制表显示".into()
                };
            }
            KeyCode::Char('a') | KeyCode::Char('A') if self.ui.page == Page::Rules => {
                self.start_add_rule()
            }
            KeyCode::Char('J') if self.ui.page == Page::Rules => {
                let result = self.move_selected_rule_item(1).await;
                self.handle_action_result(result);
            }
            KeyCode::Char('K') if self.ui.page == Page::Rules => {
                let result = self.move_selected_rule_item(-1).await;
                self.handle_action_result(result);
            }
            _ => {}
        }
        Ok(false)
    }

    async fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                if mouse.row == 0 {
                    if let Some(page) = tab_page_from_column(mouse.column) {
                        self.ui.page = page;
                        self.ui.cursor = 0;
                        self.ui.scroll = 0;
                        self.ui.node_col = 0;
                    }
                } else if self.ui.page == Page::Proxies && mouse.row >= PROXY_LIST_CONTENT_TOP {
                    self.ui.cursor = self
                        .ui
                        .scroll
                        .saturating_add(
                            (mouse.row as usize).saturating_sub(PROXY_LIST_CONTENT_TOP as usize),
                        )
                        .min(self.rows.len().saturating_sub(1));
                    self.ui.node_col = self.node_col_from_mouse(mouse.column);
                    self.fix_node_col();
                    self.keep_cursor_visible();
                } else if self.ui.page == Page::General && mouse.row >= GENERAL_CONTENT_TOP {
                    let display_row = self
                        .ui
                        .scroll
                        .saturating_add(mouse.row.saturating_sub(GENERAL_CONTENT_TOP) as usize);
                    if let Some(cursor) = general_cursor_from_display_row(display_row) {
                        self.ui.cursor = cursor;
                        self.keep_cursor_visible();
                    }
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
                6 => self.start_config_input(ConfigField::NodeNameWidth),
                7 => self.start_config_input(ConfigField::NodeItemMinWidth),
                8 => self.start_config_input(ConfigField::NodeMinGapWidth),
                9 => self.start_config_input(ConfigField::NodeReserveWidth),
                10 => self.start_config_input(ConfigField::NodeColumnGapWidth),
                11 => self.ui.logs_open = true,
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
            Page::Rules => {
                if self.rules.detail_group.is_some() {
                    self.start_edit_selected_rule_item();
                } else {
                    self.enter_selected_rule_group();
                }
            }
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
        // 外部工具可能修改 Provider，刷新时同步配置中的最新顺序。
        if let Ok(config) = crate::config::runtime_config::prepare(&self.boot.source_config) {
            self.boot.config.provider_names = config.provider_names;
        }
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

    // #----Rules 动作----
    fn enter_selected_rule_group(&mut self) {
        if self.rules.detail_group.is_some() {
            return;
        }
        let Some(idx) = self.filtered_group_indices().get(self.ui.cursor).copied() else {
            return;
        };
        self.rules.detail_group = Some(idx);
        self.ui.cursor = 0;
        self.ui.scroll = 0;
        self.ui.status = format!("进入规则分组：{}", self.rules.groups[idx].name);
    }

    fn rules_back_or_clear_search(&mut self) {
        if self.rules.detail_group.is_some() {
            self.rules.detail_group = None;
            self.ui.cursor = 0;
            self.ui.scroll = 0;
            self.ui.status = "已返回 Rules 分组列表".into();
        } else if !self.rules.search.is_empty() {
            self.rules.search.clear();
            self.ui.cursor = 0;
            self.ui.scroll = 0;
            self.ui.status = "已清除 Rules 搜索".into();
        }
    }

    fn start_rule_search(&mut self) {
        self.ui.rule_input = Some(RuleInput::Search);
        self.ui.input_buffer = self.rules.search.clone();
        self.ui.input_cursor = char_len(&self.ui.input_buffer);
        self.ui.status = "输入搜索关键字，Enter 应用，Esc 取消".into();
    }

    fn start_rule_input(&mut self, mode: RuleInput, value: &str) {
        self.ui.rule_input = Some(mode);
        self.ui.input_buffer = value.to_string();
        self.ui.input_cursor = char_len(&self.ui.input_buffer);
        self.ui.status = "输入内容后按 Enter 保存，Esc 取消".into();
    }

    fn start_edit_selected_rule_item(&mut self) {
        if let Some(group_idx) = self.rules.detail_group {
            let Some(rule_idx) = self
                .filtered_rule_indices(group_idx)
                .get(self.ui.cursor)
                .copied()
            else {
                return;
            };
            let value = self.rules.groups[group_idx].rules[rule_idx].clone();
            self.start_rule_input(RuleInput::EditRule(group_idx, rule_idx), &value);
            return;
        }
        let Some(group_idx) = self.filtered_group_indices().get(self.ui.cursor).copied() else {
            return;
        };
        let value = self.rules.groups[group_idx].target.clone();
        self.start_rule_input(RuleInput::EditTarget(group_idx), &value);
    }

    fn start_rename_selected_rule_group(&mut self) {
        if self.rules.detail_group.is_some() {
            self.ui.status = "请返回分组列表后再改名".into();
            return;
        }
        let Some(group_idx) = self.filtered_group_indices().get(self.ui.cursor).copied() else {
            return;
        };
        let value = self.rules.groups[group_idx].name.clone();
        self.start_rule_input(RuleInput::RenameGroup(group_idx), &value);
    }

    fn start_add_rule(&mut self) {
        let Some(group_idx) = self.rules.detail_group else {
            self.ui.status = "先进入一个分组再添加规则".into();
            return;
        };
        self.ui.cursor = self.filtered_rule_indices(group_idx).len();
        self.keep_cursor_visible();
        self.start_rule_input(RuleInput::AddRule(group_idx), "");
        self.ui.status = "输入规则，例如 DOMAIN-SUFFIX,google.com".into();
    }

    async fn toggle_selected_rule_group(&mut self) -> Result<()> {
        if self.rules.detail_group.is_some() {
            return Ok(());
        }
        let Some(group_idx) = self.filtered_group_indices().get(self.ui.cursor).copied() else {
            return Ok(());
        };
        self.rules.groups[group_idx].active = !self.rules.groups[group_idx].active;
        self.save_rules_and_reload().await?;
        Ok(())
    }

    async fn delete_selected_rule_item(&mut self) -> Result<()> {
        if let Some(group_idx) = self.rules.detail_group {
            let Some(rule_idx) = self
                .filtered_rule_indices(group_idx)
                .get(self.ui.cursor)
                .copied()
            else {
                return Ok(());
            };
            self.rules.groups[group_idx].rules.remove(rule_idx);
            self.ui.cursor = self.ui.cursor.min(
                self.filtered_rule_indices(group_idx)
                    .len()
                    .saturating_sub(1),
            );
            self.save_rules_and_reload().await?;
            self.ui.status = "规则已删除并热加载".into();
            return Ok(());
        }

        let Some(group_idx) = self.filtered_group_indices().get(self.ui.cursor).copied() else {
            return Ok(());
        };
        let group = self.rules.groups.remove(group_idx);
        rules_config::backup_rule_file(&group)?;
        self.ui.cursor = self
            .ui
            .cursor
            .min(self.filtered_group_indices().len().saturating_sub(1));
        self.save_rules_and_reload().await?;
        self.ui.status = format!("分组 {} 已删除，规则文件已备份", group.name);
        Ok(())
    }

    async fn move_selected_rule_item(&mut self, delta: isize) -> Result<()> {
        if let Some(group_idx) = self.rules.detail_group {
            let Some(rule_idx) = self
                .filtered_rule_indices(group_idx)
                .get(self.ui.cursor)
                .copied()
            else {
                return Ok(());
            };
            let next = move_index(rule_idx, delta, self.rules.groups[group_idx].rules.len());
            self.rules.groups[group_idx].rules.swap(rule_idx, next);
            self.ui.cursor = self
                .filtered_rule_indices(group_idx)
                .iter()
                .position(|item| *item == next)
                .unwrap_or(self.ui.cursor);
            self.save_rules_and_reload().await?;
            self.ui.status = "规则顺序已更新并热加载".into();
            return Ok(());
        }

        let Some(group_idx) = self.filtered_group_indices().get(self.ui.cursor).copied() else {
            return Ok(());
        };
        let next = move_index(group_idx, delta, self.rules.groups.len());
        self.rules.groups.swap(group_idx, next);
        self.ui.cursor = self
            .filtered_group_indices()
            .iter()
            .position(|item| *item == next)
            .unwrap_or(self.ui.cursor);
        self.save_rules_and_reload().await?;
        self.ui.status = "分组顺序已更新并热加载".into();
        Ok(())
    }

    async fn handle_rule_input(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.ui.rule_input = None;
                self.ui.input_buffer.clear();
                self.ui.input_cursor = 0;
                self.ui.status = "已取消输入".into();
            }
            KeyCode::Enter => {
                if let Err(err) = self.commit_rule_input().await {
                    self.ui.status = format!("操作失败：{err}");
                }
            }
            KeyCode::Backspace => {
                if self.ui.input_cursor > 0 {
                    self.ui.input_cursor -= 1;
                    remove_char_at(&mut self.ui.input_buffer, self.ui.input_cursor);
                }
            }
            KeyCode::Delete => {
                remove_char_at(&mut self.ui.input_buffer, self.ui.input_cursor);
            }
            KeyCode::Left => {
                self.ui.input_cursor = self.ui.input_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                self.ui.input_cursor = self
                    .ui
                    .input_cursor
                    .saturating_add(1)
                    .min(char_len(&self.ui.input_buffer));
            }
            KeyCode::Home => {
                self.ui.input_cursor = 0;
            }
            KeyCode::End => {
                self.ui.input_cursor = char_len(&self.ui.input_buffer);
            }
            KeyCode::Char(ch) => {
                if self.ui.input_buffer.len() < 240 {
                    insert_char_at(&mut self.ui.input_buffer, self.ui.input_cursor, ch);
                    self.ui.input_cursor += 1;
                }
            }
            _ => {}
        }
        Ok(false)
    }

    async fn commit_rule_input(&mut self) -> Result<()> {
        let Some(mode) = self.ui.rule_input.clone() else {
            return Ok(());
        };
        let value = self.ui.input_buffer.trim().to_string();
        match mode {
            RuleInput::Search => {
                self.rules.search = value;
                self.ui.cursor = 0;
                self.ui.scroll = 0;
                self.ui.status = if self.rules.search.is_empty() {
                    "已清除 Rules 搜索".into()
                } else {
                    format!("Rules 搜索：{}", self.rules.search)
                };
            }
            RuleInput::NewGroup => {
                rules_config::validate_group_name(&value)?;
                if self.rules.groups.iter().any(|group| group.name == value) {
                    return Err(anyhow::anyhow!("分组已存在"));
                }
                self.rules
                    .groups
                    .push(rules_config::create_group(&self.boot.work_dir, &value)?);
                self.save_rules_and_reload().await?;
                self.ui.status = format!("分组 {value} 已创建");
            }
            RuleInput::RenameGroup(idx) => {
                rules_config::validate_group_name(&value)?;
                if self
                    .rules
                    .groups
                    .iter()
                    .enumerate()
                    .any(|(item_idx, group)| item_idx != idx && group.name == value)
                {
                    return Err(anyhow::anyhow!("分组已存在"));
                }
                if let Some(group) = self.rules.groups.get_mut(idx) {
                    group.name = value.clone();
                    group.path = self
                        .boot
                        .work_dir
                        .join("rules")
                        .join(format!("{value}.yaml"));
                }
                self.save_rules_and_reload().await?;
                self.ui.status = format!("分组已改名为 {value}");
            }
            RuleInput::EditTarget(idx) => {
                if value.is_empty() {
                    return Err(anyhow::anyhow!("目标策略不能为空"));
                }
                if let Some(group) = self.rules.groups.get_mut(idx) {
                    group.target = value.clone();
                }
                self.save_rules_and_reload().await?;
                self.ui.status = format!("目标策略已改为 {value}");
            }
            RuleInput::AddRule(group_idx) => {
                if value.is_empty() {
                    return Err(anyhow::anyhow!("规则不能为空"));
                }
                if let Some(group) = self.rules.groups.get_mut(group_idx) {
                    group.rules.push(value);
                }
                self.save_rules_and_reload().await?;
                self.ui.status = "规则已添加并热加载".into();
            }
            RuleInput::EditRule(group_idx, rule_idx) => {
                if value.is_empty() {
                    return Err(anyhow::anyhow!("规则不能为空"));
                }
                if let Some(rule) = self
                    .rules
                    .groups
                    .get_mut(group_idx)
                    .and_then(|group| group.rules.get_mut(rule_idx))
                {
                    *rule = value;
                }
                self.save_rules_and_reload().await?;
                self.ui.status = "规则已更新并热加载".into();
            }
        }
        self.ui.rule_input = None;
        self.ui.input_buffer.clear();
        self.ui.input_cursor = 0;
        self.keep_cursor_visible();
        Ok(())
    }

    async fn save_rules_and_reload(&mut self) -> Result<()> {
        rules_config::save(
            &self.boot.source_config,
            &self.boot.work_dir,
            &rules_config::RulesConfig {
                groups: self.rules.groups.clone(),
            },
            !self.rules.config_backup_done,
        )?;
        self.rules.config_backup_done = true;
        let runtime_config = crate::config::runtime_config::prepare(&self.boot.source_config)?;
        self.boot.config = runtime_config;
        self.boot
            .client
            .reload_config(&self.boot.source_config.to_string_lossy())
            .await?;
        self.refresh().await;
        Ok(())
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
                for node_idx in (0..provider.nodes.len()).step_by(self.ui.node_columns) {
                    self.rows.push(RowRef::NodeRow(provider_idx, node_idx));
                }
            }
        }
        if self.ui.page == Page::Proxies {
            self.ui.cursor = self.ui.cursor.min(self.rows.len().saturating_sub(1));
            self.fix_node_col();
            self.keep_cursor_visible();
        }
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
            Page::General => GENERAL_MAX_CURSOR,
            Page::Proxies => self.rows.len().saturating_sub(1),
            Page::Rules => self.rules_cursor_len().saturating_sub(1),
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

    fn selected_node_index(&self, provider_idx: usize, row_start: usize) -> Option<usize> {
        let provider = self.mihomo.providers.get(provider_idx)?;
        let idx = row_start + self.ui.node_col.min(self.ui.node_columns.saturating_sub(1));
        if idx < provider.nodes.len() {
            Some(idx)
        } else {
            Some(row_start)
        }
    }

    pub fn filtered_group_indices(&self) -> Vec<usize> {
        self.rules
            .groups
            .iter()
            .enumerate()
            .filter(|(_, group)| {
                self.rules.search.is_empty()
                    || group.name.contains(&self.rules.search)
                    || group.target.contains(&self.rules.search)
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    pub fn filtered_rule_indices(&self, group_idx: usize) -> Vec<usize> {
        self.rules
            .groups
            .get(group_idx)
            .map(|group| {
                group
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(_, rule)| {
                        self.rules.search.is_empty() || rule.contains(&self.rules.search)
                    })
                    .map(|(idx, _)| idx)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn handle_horizontal(&mut self, delta: isize) {
        if self.ui.page == Page::General {
            self.adjust_node_item_width(delta);
            return;
        }
        if self.ui.page == Page::Rules {
            self.handle_rules_horizontal(delta);
            return;
        }

        match self.rows.get(self.ui.cursor).cloned() {
            Some(RowRef::Provider(idx)) => {
                if let Some(provider) = self.mihomo.providers.get_mut(idx) {
                    provider.expanded = delta.is_positive();
                    self.rebuild_rows();
                }
            }
            Some(RowRef::NodeRow(provider_idx, _)) => {
                if delta.is_negative() && self.ui.node_col == 0 {
                    self.collapse_provider(provider_idx);
                } else {
                    self.move_node_col(delta);
                }
            }
            _ => {}
        }
    }

    fn handle_rules_horizontal(&mut self, delta: isize) {
        if self.rules.detail_group.is_some() {
            if delta.is_negative() {
                self.rules_back_or_clear_search();
            }
            return;
        }
        let Some(group_idx) = self.filtered_group_indices().get(self.ui.cursor).copied() else {
            return;
        };
        if let Some(group) = self.rules.groups.get_mut(group_idx) {
            group.expanded = delta.is_positive();
        }
    }

    fn move_node_col(&mut self, delta: isize) {
        if !matches!(self.rows.get(self.ui.cursor), Some(RowRef::NodeRow(_, _))) {
            return;
        }
        self.ui.node_col = if delta.is_negative() {
            self.ui.node_col.saturating_sub(1)
        } else {
            self.ui.node_col.saturating_add(1)
        };
        self.fix_node_col();
    }

    fn collapse_provider(&mut self, provider_idx: usize) {
        if let Some(provider) = self.mihomo.providers.get_mut(provider_idx) {
            provider.expanded = false;
        }
        self.rebuild_rows();
        if let Some(idx) = self
            .rows
            .iter()
            .position(|row| matches!(row, RowRef::Provider(idx) if *idx == provider_idx))
        {
            self.ui.cursor = idx;
        }
        self.fix_node_col();
        self.keep_cursor_visible();
    }

    fn fix_node_col(&mut self) {
        if let Some(RowRef::NodeRow(provider_idx, row_start)) = self.rows.get(self.ui.cursor) {
            let len = self
                .mihomo
                .providers
                .get(*provider_idx)
                .map(|provider| provider.nodes.len())
                .unwrap_or_default();
            let max_col = self.ui.node_columns.saturating_sub(1);
            self.ui.node_col = self.ui.node_col.min(max_col);
            if row_start + self.ui.node_col >= len {
                self.ui.node_col = 0;
            }
        } else {
            self.ui.node_col = 0;
        }
    }

    fn keep_cursor_visible(&mut self) {
        let height = self.ui_visible_rows();
        let cursor_row = self.cursor_display_row();
        if cursor_row < self.ui.scroll {
            self.ui.scroll = cursor_row;
        } else if cursor_row >= self.ui.scroll.saturating_add(height) {
            self.ui.scroll = cursor_row.saturating_sub(height.saturating_sub(1));
        }
        self.ui.scroll = self.ui.scroll.min(self.max_scroll());
    }

    fn cursor_display_row(&self) -> usize {
        match self.ui.page {
            Page::General => GENERAL_CURSOR_ROWS[self.ui.cursor.min(GENERAL_MAX_CURSOR)],
            Page::Proxies | Page::Rules => self.ui.cursor,
        }
    }

    fn ui_visible_rows(&self) -> usize {
        match self.ui.page {
            Page::General => self.ui.terminal_height.saturating_sub(5).max(1) as usize,
            Page::Proxies => self.ui.terminal_height.saturating_sub(8).max(1) as usize,
            Page::Rules => {
                let header = if self.rules.detail_group.is_some() {
                    RULES_DETAIL_HEADER_ROWS
                } else {
                    RULES_LIST_HEADER_ROWS
                };
                self.ui.terminal_height.saturating_sub(header as u16).max(1) as usize
            }
        }
    }

    fn max_scroll(&self) -> usize {
        match self.ui.page {
            Page::General => GENERAL_TOTAL_ROWS.saturating_sub(self.ui_visible_rows()),
            Page::Proxies => self.rows.len().saturating_sub(self.ui_visible_rows()),
            Page::Rules => self
                .rules_cursor_len()
                .saturating_sub(self.ui_visible_rows()),
        }
    }

    fn rules_cursor_len(&self) -> usize {
        if let Some(group_idx) = self.rules.detail_group {
            self.filtered_rule_indices(group_idx).len()
        } else {
            self.filtered_group_indices().len()
        }
    }

    fn node_col_from_mouse(&self, column: u16) -> usize {
        let node_start = NODE_INDENT_WIDTH.saturating_add(1);
        let rel = column.saturating_sub(node_start);
        let step = self
            .node_layout()
            .item_width
            .saturating_add(self.boot.app_config.node_column_gap_width);
        (rel / step).min(self.ui.node_columns.saturating_sub(1) as u16) as usize
    }

    fn set_viewport(&mut self, width: u16, height: u16) {
        let old_columns = self.ui.node_columns;
        self.ui.terminal_width = width;
        self.ui.terminal_height = height;
        self.ui.node_columns = self.calculate_node_columns(width);
        if self.ui.node_columns != old_columns {
            self.rebuild_rows();
        } else {
            self.fix_node_col();
            self.keep_cursor_visible();
        }
    }

    fn calculate_node_columns(&self, width: u16) -> usize {
        let content_width = width.saturating_sub(2).saturating_sub(NODE_INDENT_WIDTH);
        let step = self
            .min_node_item_width()
            .saturating_add(self.boot.app_config.node_column_gap_width);
        let columns = content_width
            .saturating_add(self.boot.app_config.node_column_gap_width)
            .checked_div(step.max(1))
            .unwrap_or(1) as usize;
        columns.clamp(MIN_NODE_COLUMNS, MAX_NODE_COLUMNS)
    }

    fn adjust_node_item_width(&mut self, delta: isize) {
        let Some(field) = self.config_field_at_cursor() else {
            return;
        };
        let step = if delta.is_negative() { -1 } else { 1 };
        self.adjust_config_field(field, step);
    }

    fn config_field_at_cursor(&self) -> Option<ConfigField> {
        match self.ui.cursor {
            6 => Some(ConfigField::NodeNameWidth),
            7 => Some(ConfigField::NodeItemMinWidth),
            8 => Some(ConfigField::NodeMinGapWidth),
            9 => Some(ConfigField::NodeReserveWidth),
            10 => Some(ConfigField::NodeColumnGapWidth),
            _ => None,
        }
    }

    fn adjust_config_field(&mut self, field: ConfigField, delta: i16) {
        match field {
            ConfigField::NodeNameWidth => self.boot.app_config.adjust_node_name_width(delta),
            ConfigField::NodeItemMinWidth => self.boot.app_config.adjust_node_item_min_width(delta),
            ConfigField::NodeMinGapWidth => self.boot.app_config.adjust_node_min_gap_width(delta),
            ConfigField::NodeReserveWidth => self.boot.app_config.adjust_node_reserve_width(delta),
            ConfigField::NodeColumnGapWidth => {
                self.boot.app_config.adjust_node_column_gap_width(delta)
            }
        }
        self.save_app_config(field.label());
    }

    fn start_config_input(&mut self, field: ConfigField) {
        self.ui.config_edit = Some(field);
        self.ui.input_buffer = self.config_field_value(field).to_string();
        self.ui.status = format!("输入 {} 后按 Enter 保存，Esc 取消", field.label());
    }

    fn handle_config_input(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.ui.config_edit = None;
                self.ui.input_buffer.clear();
                self.ui.status = "已取消输入".into();
            }
            KeyCode::Enter => self.commit_config_input(),
            KeyCode::Backspace => {
                self.ui.input_buffer.pop();
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                if self.ui.input_buffer.len() < 4 {
                    self.ui.input_buffer.push(ch);
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn commit_config_input(&mut self) {
        let Some(field) = self.ui.config_edit else {
            return;
        };
        let Ok(value) = self.ui.input_buffer.parse::<u16>() else {
            self.ui.status = "请输入数字".into();
            return;
        };
        match field {
            ConfigField::NodeNameWidth => self.boot.app_config.set_node_name_width(value),
            ConfigField::NodeItemMinWidth => self.boot.app_config.set_node_item_min_width(value),
            ConfigField::NodeMinGapWidth => self.boot.app_config.set_node_min_gap_width(value),
            ConfigField::NodeReserveWidth => self.boot.app_config.set_node_reserve_width(value),
            ConfigField::NodeColumnGapWidth => {
                self.boot.app_config.set_node_column_gap_width(value)
            }
        }
        self.ui.config_edit = None;
        self.ui.input_buffer.clear();
        self.save_app_config(field.label());
    }

    fn config_field_value(&self, field: ConfigField) -> u16 {
        match field {
            ConfigField::NodeNameWidth => self.boot.app_config.node_name_width,
            ConfigField::NodeItemMinWidth => self.boot.app_config.node_item_min_width,
            ConfigField::NodeMinGapWidth => self.boot.app_config.node_min_gap_width,
            ConfigField::NodeReserveWidth => self.boot.app_config.node_reserve_width,
            ConfigField::NodeColumnGapWidth => self.boot.app_config.node_column_gap_width,
        }
    }

    fn save_app_config(&mut self, label: &str) {
        if let Err(err) = app_config::save(&self.boot.app_config) {
            self.ui.status = format!("保存 TUI 偏好失败：{err}");
            return;
        }
        self.ui.node_columns = self.calculate_node_columns(self.ui.terminal_width);
        self.rebuild_rows();
        self.ui.status = format!("{label}：{}", self.config_field_value_by_label(label));
    }

    fn config_field_value_by_label(&self, label: &str) -> u16 {
        match label {
            "节点名宽度" => self.boot.app_config.node_name_width,
            "节点项最小宽度" => self.boot.app_config.node_item_min_width,
            "最小间隔" => self.boot.app_config.node_min_gap_width,
            "尾部保留宽度" => self.boot.app_config.node_reserve_width,
            "列间宽度" => self.boot.app_config.node_column_gap_width,
            _ => 0,
        }
    }

    pub fn node_layout(&self) -> NodeLayout {
        let max_delay_width = self.max_delay_width();
        let full_name_width = self.max_full_node_name_width();
        let content_width = self
            .ui
            .terminal_width
            .saturating_sub(2)
            .saturating_sub(NODE_INDENT_WIDTH);
        let available_item_width = content_width
            .saturating_sub(
                self.boot
                    .app_config
                    .node_column_gap_width
                    .saturating_mul(self.ui.node_columns.saturating_sub(1) as u16),
            )
            .checked_div(self.ui.node_columns.max(1) as u16)
            .unwrap_or(self.min_node_item_width());
        let min_required = self.min_node_item_width();
        let full_required = full_name_width
            .saturating_add(self.boot.app_config.node_min_gap_width)
            .saturating_add(max_delay_width);
        let item_width = if self.ui.node_columns == MAX_NODE_COLUMNS {
            available_item_width.clamp(min_required, full_required.max(min_required))
        } else {
            min_required
        };
        let name_width = item_width
            .saturating_sub(max_delay_width)
            .saturating_sub(self.boot.app_config.node_min_gap_width)
            .clamp(
                self.boot.app_config.node_name_width,
                full_name_width.max(self.boot.app_config.node_name_width),
            );

        NodeLayout {
            name_width,
            item_width: item_width.max(
                name_width
                    .saturating_add(self.boot.app_config.node_min_gap_width)
                    .saturating_add(max_delay_width),
            ),
            min_gap_width: self.boot.app_config.node_min_gap_width,
            reserve_width: self.boot.app_config.node_reserve_width,
        }
    }

    fn min_node_item_width(&self) -> u16 {
        self.boot.app_config.node_item_min_width.max(
            self.boot
                .app_config
                .node_name_width
                .saturating_add(self.boot.app_config.node_min_gap_width)
                .saturating_add(self.max_delay_width()),
        )
    }

    fn max_delay_width(&self) -> u16 {
        self.mihomo
            .providers
            .iter()
            .flat_map(|provider| provider.nodes.iter())
            .map(|node| delay_text(node.delay))
            .map(|text| UnicodeWidthStr::width(text.as_str()) as u16)
            .max()
            .unwrap_or(UnicodeWidthStr::width("--ms") as u16)
    }

    fn max_full_node_name_width(&self) -> u16 {
        self.mihomo
            .providers
            .iter()
            .flat_map(|provider| provider.nodes.iter())
            .map(|node| bracketed_node_width(&clean_node_name_for_layout(&node.name)))
            .max()
            .unwrap_or(self.boot.app_config.node_name_width)
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
    let mut ordered = Vec::with_capacity(providers.len());
    for name in preferred_order {
        if let Some(provider) = providers.get(name) {
            ordered.push((name.clone(), provider.clone()));
        }
    }

    // 配置顺序只用于排序，不能成为白名单，否则外部新增 Provider 会被隐藏。
    let mut remaining: Vec<_> = providers
        .iter()
        .filter(|(name, _)| !preferred_order.contains(name))
        .map(|(name, provider)| (name.clone(), provider.clone()))
        .collect();
    remaining.sort_by(|a, b| a.0.cmp(&b.0));
    ordered.extend(remaining);

    let list: Vec<_> = ordered
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

// #----显示宽度----
fn general_cursor_from_display_row(row: usize) -> Option<usize> {
    GENERAL_CURSOR_ROWS.iter().position(|item| *item == row)
}

fn tab_page_from_column(column: u16) -> Option<Page> {
    match column {
        1..=7 => Some(Page::Proxies),
        10..=16 => Some(Page::General),
        19..=23 => Some(Page::Rules),
        _ => None,
    }
}

fn move_index(idx: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return idx;
    }
    if delta.is_negative() {
        idx.saturating_sub((-delta) as usize)
    } else {
        idx.saturating_add(delta as usize)
    }
    .min(len.saturating_sub(1))
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

fn byte_index_at(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

fn insert_char_at(text: &mut String, char_idx: usize, ch: char) {
    let byte_idx = byte_index_at(text, char_idx);
    text.insert(byte_idx, ch);
}

fn remove_char_at(text: &mut String, char_idx: usize) {
    let start = byte_index_at(text, char_idx);
    if start >= text.len() {
        return;
    }
    let end = byte_index_at(text, char_idx + 1);
    text.replace_range(start..end, "");
}

fn delay_text(delay: Option<u64>) -> String {
    delay.map_or("--ms".into(), |value| format!("{value}ms"))
}

fn bracketed_node_width(name: &str) -> u16 {
    UnicodeWidthStr::width(name) as u16 + 2
}

fn clean_node_name_for_layout(name: &str) -> String {
    let mut text = String::new();
    for ch in name.chars() {
        if !is_emoji_char_for_layout(ch) {
            text.push(ch);
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_emoji_char_for_layout(ch: char) -> bool {
    let value = ch as u32;
    matches!(
        value,
        0x1F000..=0x1FAFF
            | 0x2600..=0x27BF
            | 0xFE00..=0xFE0F
            | 0x200D
            | 0x20E3
    )
}

// #----测试----
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{build_from_real_providers, ProviderItem, ProxyItem};

    fn provider(name: &str) -> ProviderItem {
        ProviderItem {
            name: name.into(),
            proxies: vec![ProxyItem {
                name: format!("{name}-node"),
                ..ProxyItem::default()
            }],
        }
    }

    #[test]
    fn provider_order_does_not_hide_new_runtime_provider() {
        let providers = HashMap::from([
            ("old".into(), provider("old")),
            ("new".into(), provider("new")),
        ]);

        let views = build_from_real_providers(providers, &["old".into()], "");
        let names: Vec<_> = views
            .iter()
            .map(|provider| provider.name.as_str())
            .collect();

        assert_eq!(names, ["old", "new"]);
    }

    #[test]
    fn providers_without_preferred_order_are_stable() {
        let providers = HashMap::from([
            ("zeta".into(), provider("zeta")),
            ("alpha".into(), provider("alpha")),
        ]);

        let views = build_from_real_providers(providers, &[], "");
        let names: Vec<_> = views
            .iter()
            .map(|provider| provider.name.as_str())
            .collect();

        assert_eq!(names, ["alpha", "zeta"]);
    }
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
