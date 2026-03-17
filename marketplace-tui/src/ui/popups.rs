use super::common::*;
use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

// ─── Help Popup ───────────────────────────────────────────────────────────────

pub fn draw_help(f: &mut Frame, app: &App) {
    let area = centered_rect(90, 42, f.area());
    f.render_widget(Clear, area);

    let help_lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "  AWS Marketplace TUI — Keyboard Shortcuts",
            Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
        )]),
        Line::raw(""),
        section("Product List"),
        shortcut("↑ / ↓  or  j / k", "Navigate products"),
        shortcut("PgUp / PgDn", "Page through list"),
        shortcut("Enter", "Open product detail"),
        shortcut("c", "Create new product"),
        shortcut("R", "Refresh product list"),
        shortcut("y", "Copy entity ID to clipboard"),
        shortcut("/", "Enter filter mode (all characters go to filter, including q, j, k…)"),
        shortcut("Enter  (filter mode)", "Confirm filter, return to navigation"),
        shortcut("Esc   (filter mode)", "Clear filter and exit filter mode"),
        shortcut("Esc   (nav mode)", "Clear active filter"),
        Line::raw(""),
        section("Product Detail"),
        shortcut("Tab / Shift+Tab", "Switch tabs: Overview / Versions / Pricing / Allowlist"),
        shortcut("e", "Edit product metadata"),
        shortcut("v", "Add new version"),
        shortcut("V", "Open version list"),
        shortcut("w", "Manage allowlist"),
        shortcut("p", "Edit pricing"),
        shortcut("y", "Copy entity ID to clipboard"),
        shortcut("R", "Refresh product detail"),
        shortcut("Esc", "Back to product list"),
        Line::raw(""),
        section("Version List"),
        shortcut("↑ / ↓", "Navigate versions"),
        shortcut("PgUp / PgDn", "Page through list"),
        shortcut("e", "Edit selected version metadata"),
        shortcut("r", "Restrict (deprecate) selected version"),
        shortcut("y", "Copy version ID to clipboard"),
        shortcut("Esc", "Back"),
        Line::raw(""),
        section("Version / Product Metadata Editor"),
        shortcut("↑ / ↓", "Navigate fields"),
        shortcut("Enter", "Edit selected field"),
        shortcut("Ctrl+S", "Save all changes"),
        shortcut("Esc", "Cancel"),
        Line::raw(""),
        section("Allowlist Manager"),
        shortcut("a / n", "Add a new account ID"),
        shortcut("d / Del", "Remove selected account ID"),
        shortcut("↑ / ↓", "Navigate accounts"),
        shortcut("Enter", "Confirm input"),
        shortcut("Esc", "Back"),
        Line::raw(""),
        section("Pricing Editor"),
        shortcut("Tab", "Switch between fields"),
        shortcut("Enter", "Save pricing changes"),
        shortcut("Esc", "Cancel and go back"),
        shortcut("(any character)", "Type into active field — no shortcut conflicts"),
        Line::raw(""),
        section("Wizards (Create Product / Create Version)"),
        shortcut("Tab", "Move to next field"),
        shortcut("Shift+Tab", "Go back a step"),
        shortcut("Enter", "Advance to next step / submit"),
        shortcut("Esc", "Cancel wizard"),
        Line::raw(""),
        section("Global"),
        shortcut(": or Space", "Open command palette"),
        shortcut("?", "Show / close this help"),
        shortcut("↑ / ↓  PgUp / PgDn", "Scroll help"),
        shortcut("l", "Toggle log viewer"),
        shortcut("s", "View change set status"),
        shortcut("q", "Quit"),
        shortcut("Ctrl+C", "Force quit"),
        Line::raw(""),
        section("Mouse"),
        shortcut("Scroll", "Navigate lists, scroll log / help"),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  [Esc] or [?] to close",
            Style::default().fg(SUBTLE),
        )]),
    ];

    let total_lines = help_lines.len() as u16;
    let inner_height = area.height.saturating_sub(2); // subtract borders
    let max_scroll = total_lines.saturating_sub(inner_height);
    let scroll = app.help_scroll.min(max_scroll);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BRAND_ORANGE))
        .title(Span::styled(
            " Help — Keyboard Shortcuts ",
            Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
        ));

    let para = Paragraph::new(help_lines)
        .block(block)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);

    // Scrollbar
    if total_lines > inner_height {
        let mut scrollbar_state =
            ScrollbarState::default().content_length(total_lines as usize).position(scroll as usize);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut scrollbar_state,
        );
    }
}

// ─── Logs Overlay ─────────────────────────────────────────────────────────────

pub fn draw_logs(f: &mut Frame, app: &App) {
    let area = centered_rect(100, 35, f.area());
    f.render_widget(Clear, area);

    let log = match app.log.lock() {
        Ok(l) => l.clone(),
        Err(_) => return,
    };

    let lines: Vec<Line> = log
        .iter()
        .rev()
        .skip(app.log_scroll as usize)
        .take(area.height.saturating_sub(4) as usize)
        .map(|entry| {
            let (level_str, level_color) = match entry.level {
                crate::models::LogLevel::Info => ("INFO ", Color::Cyan),
                crate::models::LogLevel::Warning => ("WARN ", WARNING_YELLOW),
                crate::models::LogLevel::Error => ("ERROR", ERROR_RED),
            };
            Line::from(vec![
                Span::styled(
                    format!(" {} ", entry.timestamp),
                    Style::default().fg(SUBTLE),
                ),
                Span::styled(
                    format!("[{}] ", level_str),
                    Style::default().fg(level_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(entry.message.clone(), Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let title = format!(" Application Log ({} entries) ", log.len());

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK))
                .title(Span::styled(
                    title,
                    Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);

    // Scrollbar
    if log.len() > (area.height as usize).saturating_sub(4) {
        let mut scrollbar_state =
            ScrollbarState::default().content_length(log.len()).position(app.log_scroll as usize);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut scrollbar_state,
        );
    }
}

// ─── Change Set Status View ───────────────────────────────────────────────────

pub fn draw_changeset_status(f: &mut Frame, app: &App) {
    let (header_area, content_area, status_area) = base_layout(f);
    draw_header(f, header_area, app, "Change Set Status");

    let items: Vec<ListItem> = if app.pending_changesets.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  No change sets tracked in this session.",
            Style::default().fg(SUBTLE),
        )]))]
    } else {
        app.pending_changesets
            .iter()
            .map(|cs| {
                let status_color = match cs.status.color_hint() {
                    "green" => SUCCESS_GREEN,
                    "red" => ERROR_RED,
                    "yellow" => WARNING_YELLOW,
                    "cyan" => Color::Cyan,
                    _ => SUBTLE,
                };

                let failure_line = cs
                    .failure_description
                    .as_deref()
                    .map(|f| {
                        Line::from(vec![
                            Span::raw("    "),
                            Span::styled(truncate(f, 80), Style::default().fg(ERROR_RED)),
                        ])
                    });

                let spinner = if !cs.status.is_terminal() {
                    format!("{} ", app.spinner_char())
                } else {
                    String::new()
                };

                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(spinner, Style::default().fg(Color::Cyan)),
                        Span::styled(
                            format!("{:<45}", cs.change_set_name),
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{}", cs.status),
                            Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            format!("  Operation: {:<30}  ", cs.operation),
                            Style::default().fg(SUBTLE),
                        ),
                        Span::styled(
                            format!("Submitted: {}", cs.submitted_at),
                            Style::default().fg(SUBTLE),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            format!("  ID: {}", truncate(&cs.change_set_id, 50)),
                            Style::default().fg(SUBTLE),
                        ),
                    ]),
                ];

                if let Some(fl) = failure_line {
                    lines.push(fl);
                }
                lines.push(Line::raw(""));

                ListItem::new(lines)
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK))
                .title(Span::styled(
                    format!(" Change Sets ({}) ", app.pending_changesets.len()),
                    Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(Style::default().bg(SELECTED_BG));

    let mut state = app.changesets_list_state.clone();
    f.render_stateful_widget(list, content_area, &mut state);

    draw_status_bar(
        f,
        status_area,
        app,
        "[↑↓] Navigate  [R] Force poll status  [Esc] Back",
    );
}

// ─── Restrict Version Confirmation ────────────────────────────────────────────

pub fn draw_restrict_confirm(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 10, f.area());
    f.render_widget(Clear, area);

    let version_title = app
        .selected_version()
        .map(|v| v.title.clone())
        .unwrap_or_else(|| "selected version".to_string());

    let lines: Vec<Line> = vec![
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  Restrict (deprecate) version:",
            Style::default().fg(Color::White),
        )]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                &version_title,
                Style::default().fg(WARNING_YELLOW).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  This action cannot be undone.",
            Style::default().fg(ERROR_RED),
        )]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "  [y] Confirm  [n/Esc] Cancel",
                Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ERROR_RED))
                .title(Span::styled(
                    " Confirm Restriction ",
                    Style::default().fg(ERROR_RED).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

// ─── Command Palette ──────────────────────────────────────────────────────────

pub fn draw_command_palette(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 30, f.area());
    f.render_widget(Clear, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Search input
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BRAND_ORANGE))
        .title(Span::styled(
            " Command Palette ",
            Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
        ));

    let input_para = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled("🔍 ", Style::default().fg(SUBTLE)),
        Span::styled(
            format!("{}_", app.command_palette_input),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(input_block);

    f.render_widget(input_para, layout[0]);

    // Results list
    let items: Vec<ListItem> = app
        .filtered_commands
        .iter()
        .map(|&i| {
            let item = &app.command_palette_items[i];
            let shortcut_span = item
                .shortcut
                .as_deref()
                .map(|s| {
                    Span::styled(
                        format!("  [{}]", s),
                        Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                    )
                })
                .unwrap_or_else(|| Span::raw("      "));

            ListItem::new(Line::from(vec![
                shortcut_span,
                Span::styled(
                    format!("  {:.<30} ", item.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    item.description.clone(),
                    Style::default().fg(SUBTLE),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK))
                .title(Span::styled(
                    format!(" {} commands ", app.filtered_commands.len()),
                    Style::default().fg(SUBTLE),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(SELECTED_BG)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = app.command_palette_state.clone();
    f.render_stateful_widget(list, layout[1], &mut state);
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn section<'a>(title: &str) -> Line<'a> {
    Line::from(vec![Span::styled(
        format!("  ─── {} ───", title),
        Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
    )])
}

fn shortcut<'a>(keys: &str, description: &str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("    {:.<24} ", keys),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(description.to_string(), Style::default().fg(Color::White)),
    ])
}
