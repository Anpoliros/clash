//! UI 渲染入口：组合顶部标签、页面内容与全屏日志浮层。

pub mod theme;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Frame,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs},
};

use crate::app::{App, Page, RowRef};

use self::theme::Theme;

// #----主渲染----
pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let theme = Theme::default();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_tabs(frame, root[0], app, &theme);
    match app.ui.page {
        Page::General => draw_general(frame, root[1], app, &theme),
        Page::Proxies => draw_proxies(frame, root[1], app, &theme),
        Page::Rules => draw_rules(frame, root[1], &theme),
    }
    draw_status(frame, root[2], app, &theme);

    if app.ui.logs_open {
        draw_logs(frame, frame.area(), app, &theme);
    }
}

fn draw_tabs(frame: &mut Frame<'_>, area: Rect, app: &App, theme: &Theme) {
    let selected = match app.ui.page {
        Page::Proxies => 0,
        Page::General => 1,
        Page::Rules => 2,
    };
    let tabs = Tabs::new(["Proxies", "General", "Rules"])
        .select(selected)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(theme.text)
        .highlight_style(theme.active.add_modifier(Modifier::BOLD));
    frame.render_widget(tabs, area);
}

// #----General 页面----
fn draw_general(frame: &mut Frame<'_>, area: Rect, app: &App, theme: &Theme) {
    let rows = vec![
        line_toggle(
            "Proxy",
            app.runtime.proxy_enabled,
            app.ui.cursor == 0,
            theme,
        ),
        line_toggle("TUN", app.runtime.tun_enabled, app.ui.cursor == 1, theme),
        Line::raw(""),
        Line::raw(format!(
            "Proxy Port   {}",
            app.runtime.proxy_port.map_or("-".into(), |v| v.to_string())
        )),
        Line::raw(format!("Manage Port  {}", app.runtime.manage_port)),
        Line::raw(""),
        Line::raw(format!(
            "Node  {}",
            clean_node_name(&app.runtime.active_node)
        )),
        Line::raw(format!(
            "Pid   {}",
            app.runtime.pid.map_or("-".into(), |v| v.to_string())
        )),
        selectable_line("Log   Enter 查看日志", app.ui.cursor == 5, theme),
        Line::raw(""),
        Line::raw(format!(
            "mihomo {}",
            app.mihomo
                .version
                .clone()
                .unwrap_or_else(|| "未连接".into())
        )),
    ];
    let panel = Paragraph::new(rows)
        .block(Block::default().title(" General ").borders(Borders::ALL))
        .style(theme.text);
    frame.render_widget(panel, area);
}

fn line_toggle(label: &str, enabled: bool, selected: bool, theme: &Theme) -> Line<'static> {
    let mark = if enabled { "[x]" } else { "[ ]" };
    selectable_line(&format!("{label:<8} {mark}"), selected, theme)
}

fn selectable_line(text: &str, selected: bool, theme: &Theme) -> Line<'static> {
    if selected {
        Line::from(Span::styled(text.to_string(), theme.hover))
    } else {
        Line::raw(text.to_string())
    }
}

// #----Proxies 页面----
fn draw_proxies(frame: &mut Frame<'_>, area: Rect, app: &App, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let auto = Paragraph::new(format!(
        "Auto Select    当前节点：{}",
        clean_node_name(&app.runtime.active_node)
    ))
    .block(Block::default().title(" Proxies ").borders(Borders::ALL))
    .style(theme.text);
    frame.render_widget(auto, chunks[0]);

    let visible_rows = chunks[1].height.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .rows
        .iter()
        .enumerate()
        .skip(app.ui.scroll)
        .take(visible_rows)
        .map(|(idx, row)| match row {
            RowRef::AutoSelect => {
                let style = if idx == app.ui.cursor {
                    theme.hover
                } else if app
                    .mihomo
                    .auto_group
                    .as_ref()
                    .map(|name| name == &app.runtime.active_node)
                    .unwrap_or(false)
                {
                    theme.active.add_modifier(Modifier::BOLD)
                } else {
                    theme.active
                };
                let name = app.mihomo.auto_group.as_deref().unwrap_or("Auto Select");
                ListItem::new(Line::from(vec![
                    Span::styled("[Auto] ", style),
                    Span::styled(clean_node_name(name), style),
                ]))
            }
            RowRef::Provider(provider_idx) => {
                let provider = &app.mihomo.providers[*provider_idx];
                let icon = if provider.expanded { "▼" } else { "▶" };
                let style = if idx == app.ui.cursor {
                    theme.hover
                } else {
                    theme.header
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{icon}] {}", provider.name), style),
                    Span::raw("  "),
                    Span::styled("[Refresh]", theme.action),
                    Span::raw(" "),
                    Span::styled("[Ping]", theme.action),
                    Span::raw(" "),
                    Span::styled("[Sort]", theme.action),
                ]))
            }
            RowRef::NodeRow(provider_idx, row_start) => {
                let provider = &app.mihomo.providers[*provider_idx];
                let left = provider.nodes.get(*row_start);
                let right = provider.nodes.get(row_start + 1);
                ListItem::new(Line::from(vec![
                    Span::raw("        "),
                    node_span(left, idx == app.ui.cursor && app.ui.node_col == 0, theme),
                    Span::raw("  "),
                    node_span(right, idx == app.ui.cursor && app.ui.node_col == 1, theme),
                ]))
            }
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title(" Providers ").borders(Borders::ALL))
        .style(theme.text);
    frame.render_widget(list, chunks[1]);
}

fn node_span<'a>(node: Option<&crate::app::NodeView>, selected: bool, theme: &Theme) -> Span<'a> {
    let Some(node) = node else {
        return Span::raw(format!("{:<28}", ""));
    };
    let delay = node.delay.map_or("--ms".into(), |v| format!("{v}ms"));
    let text = format!(
        "[{:<16}] {:>5}",
        compact_node_name(&clean_node_name(&node.name)),
        delay
    );
    let style = if node.active {
        theme.active.add_modifier(Modifier::BOLD)
    } else if selected {
        theme.hover
    } else {
        theme.text
    };
    Span::styled(format!("{text:<28}"), style)
}

fn compact_node_name(name: &str) -> String {
    let mut chars = name.chars();
    let short: String = chars.by_ref().take(16).collect();
    if chars.next().is_some() {
        format!("{short}…")
    } else {
        short
    }
}

fn clean_node_name(name: &str) -> String {
    let mut text = String::new();
    for ch in name.chars() {
        if !is_emoji_char(ch) {
            text.push(ch);
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_emoji_char(ch: char) -> bool {
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

fn draw_rules(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let paragraph = Paragraph::new("Rules 页面暂不实现。MVP 聚焦 General 与 Proxies。")
        .alignment(Alignment::Center)
        .block(Block::default().title(" Rules ").borders(Borders::ALL))
        .style(theme.muted);
    frame.render_widget(paragraph, area);
}

fn draw_status(frame: &mut Frame<'_>, area: Rect, app: &App, theme: &Theme) {
    let text = format!(
        " {} | Tab 切换页面 | j/k 移动 | Enter 操作 | q 退出",
        app.ui.status
    );
    frame.render_widget(Paragraph::new(text).style(theme.status), area);
}

// #----日志浮层----
fn draw_logs(frame: &mut Frame<'_>, area: Rect, app: &App, theme: &Theme) {
    let popup = centered_rect(area, 92, 86);
    frame.render_widget(Clear, popup);
    let height = popup.height.saturating_sub(2) as usize;
    let end = app.logs.len().saturating_sub(app.ui.log_scroll);
    let start = end.saturating_sub(height);
    let lines = app.logs[start..end].to_vec();
    let title = if app.ui.log_scroll == 0 {
        " Logs  Esc/q 退出  鼠标滚轮查看 "
    } else {
        " Logs  Esc/q 退出  滚轮向下回到底部 "
    };
    let paragraph = Paragraph::new(lines.join("\n"))
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(theme.text);
    frame.render_widget(paragraph, popup);
}

fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
