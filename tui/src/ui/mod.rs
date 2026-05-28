//! UI 渲染入口：组合顶部标签、页面内容与全屏日志浮层。

pub mod theme;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::{App, ConfigField, NodeLayout, Page, RowRef};

use self::theme::Theme;

// #----主渲染----
pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let theme = Theme::default();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
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
    let tab = |page, text: &'static str| {
        let style = if app.ui.page == page {
            theme.active.add_modifier(Modifier::BOLD)
        } else {
            theme.text
        };
        Span::styled(text, style)
    };
    let line = Line::from(vec![
        Span::raw(" "),
        tab(Page::Proxies, "Proxies"),
        Span::raw("  "),
        tab(Page::General, "General"),
        Span::raw("  "),
        tab(Page::Rules, "Rules"),
    ]);
    frame.render_widget(
        Paragraph::new(line)
            .block(Block::default().borders(Borders::BOTTOM))
            .style(Style::default()),
        area,
    );
}

// #----General 页面----
fn draw_general(frame: &mut Frame<'_>, area: Rect, app: &App, theme: &Theme) {
    let rows = vec![
        Line::from(Span::styled("Runtime", theme.header)),
        selectable_line(
            &format!(
                "Proxy        {}",
                if app.runtime.proxy_enabled {
                    "on"
                } else {
                    "off"
                }
            ),
            app.ui.cursor == 0,
            theme,
        ),
        selectable_line(
            &format!(
                "TUN          {}",
                if app.runtime.tun_enabled { "on" } else { "off" }
            ),
            app.ui.cursor == 1,
            theme,
        ),
        selectable_line(
            &format!(
                "Proxy Port   {}",
                app.runtime.proxy_port.map_or("-".into(), |v| v.to_string())
            ),
            app.ui.cursor == 2,
            theme,
        ),
        selectable_line(
            &format!("Manage Port  {}", app.runtime.manage_port),
            app.ui.cursor == 3,
            theme,
        ),
        selectable_line(
            &format!("Node  {}", clean_node_name(&app.runtime.active_node)),
            app.ui.cursor == 4,
            theme,
        ),
        selectable_line(
            &format!(
                "Pid   {}",
                app.runtime.pid.map_or("-".into(), |v| v.to_string())
            ),
            app.ui.cursor == 5,
            theme,
        ),
        Line::raw(""),
        Line::from(Span::styled("Layout", theme.header)),
        editable_line(
            "Node Name Width",
            ConfigField::NodeNameWidth,
            app.boot.app_config.node_name_width,
            app.ui.cursor == 6,
            app,
            theme,
        ),
        editable_line(
            "Item Min Width",
            ConfigField::NodeItemMinWidth,
            app.boot.app_config.node_item_min_width,
            app.ui.cursor == 7,
            app,
            theme,
        ),
        editable_line(
            "Min Gap Width",
            ConfigField::NodeMinGapWidth,
            app.boot.app_config.node_min_gap_width,
            app.ui.cursor == 8,
            app,
            theme,
        ),
        editable_line(
            "Reserve Width",
            ConfigField::NodeReserveWidth,
            app.boot.app_config.node_reserve_width,
            app.ui.cursor == 9,
            app,
            theme,
        ),
        editable_line(
            "Column Gap",
            ConfigField::NodeColumnGapWidth,
            app.boot.app_config.node_column_gap_width,
            app.ui.cursor == 10,
            app,
            theme,
        ),
        Line::raw(""),
        Line::from(Span::styled("Actions", theme.header)),
        selectable_line("Log          Enter 查看日志", app.ui.cursor == 11, theme),
        selectable_line(
            &format!(
                "mihomo {}",
                app.mihomo
                    .version
                    .clone()
                    .unwrap_or_else(|| "未连接".into())
            ),
            app.ui.cursor == 12,
            theme,
        ),
    ];
    let visible_rows = area.height.saturating_sub(2).max(1) as usize;
    let rows = rows
        .into_iter()
        .skip(app.ui.scroll)
        .take(visible_rows)
        .collect::<Vec<_>>();
    let panel = Paragraph::new(rows)
        .block(Block::default().title(" General ").borders(Borders::ALL))
        .style(theme.text);
    frame.render_widget(panel, area);
}

fn selectable_line(text: &str, selected: bool, theme: &Theme) -> Line<'static> {
    if selected {
        Line::from(Span::styled(text.to_string(), theme.hover))
    } else {
        Line::raw(text.to_string())
    }
}

fn editable_line(
    label: &str,
    field: ConfigField,
    value: u16,
    selected: bool,
    app: &App,
    theme: &Theme,
) -> Line<'static> {
    let value = if app.ui.config_edit == Some(field) {
        format!("{}_", app.ui.input_buffer)
    } else {
        value.to_string()
    };
    selectable_line(&format!("{label:<16} {value:>4}"), selected, theme)
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

    let visible_rows = chunks[1].height.saturating_sub(2).max(1) as usize;
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
                let mut spans = vec![Span::raw("        ")];
                for col in 0..app.ui.node_columns {
                    if col > 0 {
                        spans.push(Span::raw(
                            " ".repeat(app.boot.app_config.node_column_gap_width as usize),
                        ));
                    }
                    spans.push(node_span(
                        provider.nodes.get(row_start + col),
                        idx == app.ui.cursor && app.ui.node_col == col,
                        app.node_layout(),
                        theme,
                    ));
                }
                ListItem::new(Line::from(spans))
            }
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title(" Providers ").borders(Borders::ALL))
        .style(theme.text);
    frame.render_widget(list, chunks[1]);
}

fn node_span<'a>(
    node: Option<&crate::app::NodeView>,
    selected: bool,
    layout: NodeLayout,
    theme: &Theme,
) -> Span<'a> {
    let Some(node) = node else {
        return Span::raw(" ".repeat(layout.item_width as usize));
    };
    let delay = node.delay.map_or("--ms".into(), |v| format!("{v}ms"));
    let delay_width = UnicodeWidthStr::width(delay.as_str());
    let name = fit_middle_display_width(
        &clean_node_name(&node.name),
        layout.name_width.saturating_sub(2) as usize,
        layout.reserve_width as usize,
    );
    let name_box = pad_display_width(&format!("[{name}]"), layout.name_width as usize);
    let gap_width = (layout.item_width as usize)
        .saturating_sub(layout.name_width as usize)
        .saturating_sub(delay_width)
        .max(layout.min_gap_width as usize);
    let text = format!("{name_box}{}{}", " ".repeat(gap_width), delay);
    let style = if selected {
        theme.hover
    } else if node.active {
        theme.active.add_modifier(Modifier::BOLD)
    } else {
        theme.text
    };
    Span::styled(text, style)
}

fn fit_middle_display_width(text: &str, width: usize, reserve_width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return pad_display_width(text, width);
    }

    let ellipsis_width = 1;
    let suffix = tail_by_display_width(
        text,
        reserve_width.min(width.saturating_sub(ellipsis_width)),
    );
    let suffix_width = UnicodeWidthStr::width(suffix.as_str());
    let prefix_width = width
        .saturating_sub(ellipsis_width)
        .saturating_sub(suffix_width);
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > prefix_width {
            break;
        }
        out.push(ch);
        used += ch_width;
    }
    out.push('…');
    out.push_str(&suffix);
    pad_display_width(&out, width)
}

fn tail_by_display_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut chars = Vec::new();
    let mut used = 0;
    for ch in text.chars().rev() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > width {
            break;
        }
        chars.push(ch);
        used += ch_width;
    }
    chars.into_iter().rev().collect()
}

fn pad_display_width(text: &str, width: usize) -> String {
    let used = UnicodeWidthStr::width(text);
    if used >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - used))
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
        " {} | Tab 切换页面 | j/k 移动 | h/l 展开或列移动 | Enter 操作 | q 退出",
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
