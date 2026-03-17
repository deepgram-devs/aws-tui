use super::common::*;
use crate::app::{App, DetailTab, PricingField};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Tabs, Wrap,
    },
    Frame,
};

// ─── Product List ─────────────────────────────────────────────────────────────

pub fn draw_product_list(f: &mut Frame, app: &App) {
    let (header_area, content_area, status_area) = base_layout(f);
    draw_header(f, header_area, app, "Machine Learning Products");

    let content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(content_area);

    draw_product_filter(f, content_layout[0], app);
    draw_product_table(f, content_layout[1], app);
    draw_status_bar(
        f,
        status_area,
        app,
        if app.filter_input_mode {
            "[↑↓] Navigate  [Enter] Confirm filter  [Esc] Clear & exit filter"
        } else {
            "[↑↓] Navigate  [Enter] Open  [/] Filter  [c] Create  [R] Refresh  [y] Copy ID"
        },
    );
}

fn draw_product_filter(f: &mut Frame, area: Rect, app: &App) {
    let (border_color, title_color, hint) = if app.filter_input_mode {
        (BRAND_ORANGE, BRAND_ORANGE, " [Enter] Confirm  [Esc] Clear & exit ")
    } else {
        (BRAND_DARK, SUBTLE, " [/] Filter ")
    };

    let filter_display = if app.filter_input_mode {
        // Show cursor while actively typing
        format!("{}_", app.product_filter)
    } else if app.product_filter.is_empty() {
        "Press / to filter…".to_string()
    } else {
        app.product_filter.clone()
    };

    let count = app.filtered_product_indices.len();
    let total = app.products.len();
    let count_label = if total == 0 {
        "No products".to_string()
    } else if count == total {
        format!("{} products", total)
    } else {
        format!("{} / {} products", count, total)
    };

    let text_color = if app.filter_input_mode {
        Color::White
    } else if app.product_filter.is_empty() {
        SUBTLE
    } else {
        Color::Yellow // active filter but not in input mode
    };

    let filter_text = Line::from(vec![
        Span::styled(" Filter: ", Style::default().fg(title_color)),
        Span::styled(
            filter_display,
            Style::default().fg(text_color).add_modifier(
                if app.filter_input_mode { Modifier::BOLD } else { Modifier::empty() }
            ),
        ),
        Span::raw("  "),
        Span::styled(count_label, Style::default().fg(SUBTLE)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(hint, Style::default().fg(title_color)));

    f.render_widget(Paragraph::new(filter_text).block(block), area);
}

fn draw_product_table(f: &mut Frame, area: Rect, app: &App) {
    let header_cells = ["Name", "Visibility", "Product Code", "Last Modified"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(BRAND_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )
        });

    let header_row = Row::new(header_cells)
        .height(1)
        .style(Style::default().bg(BRAND_DARK));

    let rows: Vec<Row> = app
        .filtered_product_indices
        .iter()
        .map(|&real_idx| {
            let p = &app.products[real_idx];
            let vis_color = visibility_color(&p.visibility);
            // Fixed columns: Visibility(12) + ProductCode(20) + LastModified(20)
            // Plus borders, separators, highlight symbol (~10 chars overhead)
            let name_max = area.width.saturating_sub(12 + 20 + 20 + 10) as usize;
            Row::new(vec![
                Cell::from(truncate(&p.name, name_max.max(20))),
                Cell::from(p.visibility.to_string())
                    .style(Style::default().fg(vis_color)),
                Cell::from(p.product_code.as_deref().unwrap_or("—").to_string()),
                Cell::from(p.last_modified_date.as_deref().unwrap_or("—").to_string()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(30),
            Constraint::Length(12),
            Constraint::Length(20),
            Constraint::Length(20),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BRAND_DARK))
            .title(Span::styled(
                " Products ",
                Style::default()
                    .fg(BRAND_ORANGE)
                    .add_modifier(Modifier::BOLD),
            )),
    )
    .row_highlight_style(
        Style::default()
            .bg(SELECTED_BG)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");

    let mut state = app.products_table_state.clone();
    f.render_stateful_widget(table, area, &mut state);
}

// ─── Product Detail ───────────────────────────────────────────────────────────

pub fn draw_product_detail(f: &mut Frame, app: &App) {
    let (header_area, content_area, status_area) = base_layout(f);

    let title = app
        .current_product_detail
        .as_ref()
        .map(|d| format!("Product: {}", d.title))
        .unwrap_or_else(|| "Product Detail".to_string());

    draw_header(f, header_area, app, &title);

    let content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(content_area);

    draw_detail_tabs(f, content_layout[0], app);

    match &app.detail_tab {
        DetailTab::Overview => draw_detail_overview(f, content_layout[1], app),
        DetailTab::Versions => draw_detail_versions(f, content_layout[1], app),
        DetailTab::Pricing => draw_detail_pricing(f, content_layout[1], app),
        DetailTab::Whitelist => draw_detail_whitelist(f, content_layout[1], app),
    }

    draw_status_bar(
        f,
        status_area,
        app,
        "[Tab] Switch tab  [e] Edit  [v] New version  [w] Allowlist  [p] Pricing  [Esc] Back",
    );
}

fn draw_detail_tabs(f: &mut Frame, area: Rect, app: &App) {
    let tab_titles = vec!["Overview", "Versions", "Pricing", "Allowlist"];
    let tabs = Tabs::new(tab_titles)
        .select(app.detail_tab.index())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK)),
        )
        .highlight_style(
            Style::default()
                .fg(BRAND_ORANGE)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )
        .style(Style::default().fg(SUBTLE));

    f.render_widget(tabs, area);
}

fn draw_detail_overview(f: &mut Frame, area: Rect, app: &App) {
    if let Some(detail) = &app.current_product_detail {
        let fields: Vec<Line> = vec![
            labeled_field("Entity ID", &detail.entity_id),
            labeled_field("Product Code", &detail.product_code),
            labeled_field_colored(
                "Visibility",
                detail.visibility.as_str(),
                visibility_color(&detail.visibility),
            ),
            labeled_field("Title", &detail.title),
            labeled_field("Short Description", &detail.short_description),
            Line::raw(""),
            labeled_field("Long Description", &truncate(&detail.long_description, 120)),
            Line::raw(""),
            labeled_field("Support Description", &truncate(&detail.support_description, 80)),
            labeled_field("Logo URL", &truncate(&detail.logo_url, 80)),
            labeled_field("Refund Policy", &detail.refund_policy),
            labeled_field("Categories", &detail.categories.join(", ")),
            labeled_field("Keywords", &detail.search_keywords.join(", ")),
            labeled_field(
                "Last Modified",
                detail.last_modified_date.as_deref().unwrap_or("—"),
            ),
        ];

        let text = fields
            .into_iter()
            .collect::<Vec<_>>();

        let para = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BRAND_DARK))
                    .title(Span::styled(
                        " Overview ",
                        Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(para, area);
    } else {
        let loading = Paragraph::new(vec![
            Line::raw(""),
            Line::from(Span::styled(
                " Loading product details…",
                Style::default().fg(Color::Cyan),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK)),
        );
        f.render_widget(loading, area);
    }
}

fn draw_detail_versions(f: &mut Frame, area: Rect, app: &App) {
    if let Some(detail) = &app.current_product_detail {
        let items: Vec<ListItem> = detail
            .versions
            .iter()
            .map(|v| {
                let vis_color = visibility_color(&v.visibility);
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:.<40} ", v.title),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("{:<12}", v.visibility),
                        Style::default().fg(vis_color),
                    ),
                    Span::styled(
                        v.created_date.as_deref().unwrap_or("—").to_string(),
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
                        format!(" Versions ({}) ", detail.versions.len()),
                        Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                    )),
            )
            .highlight_style(Style::default().bg(SELECTED_BG).add_modifier(Modifier::BOLD));

        f.render_widget(list, area);
    } else {
        f.render_widget(loading_placeholder("versions"), area);
    }
}

fn draw_detail_pricing(f: &mut Frame, area: Rect, app: &App) {
    if let Some(detail) = &app.current_product_detail {
        let has_terms = !detail.pricing_terms.is_empty();
        let has_dims = !detail.dimensions.is_empty();

        let mut lines: Vec<Line> = vec![Line::raw("")];

        if has_dims {
            lines.push(Line::from(vec![Span::styled(
                "  Pricing Dimensions",
                Style::default().fg(SUBTLE).add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::raw(""));
            for dim in &detail.dimensions {
                lines.push(Line::from(vec![
                    Span::styled("  • ", Style::default().fg(BRAND_ORANGE)),
                    Span::styled(
                        dim.name.clone(),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("      "),
                    Span::styled(
                        format!("Key: {}  Unit: {}", dim.key, dim.unit),
                        Style::default().fg(SUBTLE),
                    ),
                ]));
                if !dim.pricing_types.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            format!("Type: {}", dim.pricing_types.join(", ")),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]));
                }
                lines.push(Line::raw(""));
            }
        }

        if has_terms {
            if has_dims {
                lines.push(Line::from(vec![Span::styled(
                    "  Additional Terms",
                    Style::default().fg(SUBTLE).add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::raw(""));
            }
            for term in &detail.pricing_terms {
                lines.push(Line::from(vec![
                    Span::styled("  • ", Style::default().fg(BRAND_ORANGE)),
                    Span::styled(term.display(), Style::default().fg(Color::White)),
                ]));
            }
        }

        if !has_terms && !has_dims {
            lines.push(Line::from(Span::styled(
                "  No pricing information available",
                Style::default().fg(SUBTLE),
            )));
        }

        let para = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BRAND_DARK))
                    .title(Span::styled(
                        format!(
                            " Pricing ({} dimension{}) ",
                            detail.dimensions.len(),
                            if detail.dimensions.len() == 1 { "" } else { "s" }
                        ),
                        Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(para, area);
    } else {
        f.render_widget(loading_placeholder("pricing"), area);
    }
}

fn draw_detail_whitelist(f: &mut Frame, area: Rect, app: &App) {
    if let Some(detail) = &app.current_product_detail {
        let items: Vec<ListItem> = detail
            .allowed_accounts
            .iter()
            .map(|a| {
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(a, Style::default().fg(Color::White)),
                ]))
            })
            .collect();

        let title = format!(" Allowlist ({}) ", detail.allowed_accounts.len());

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BRAND_DARK))
                    .title(Span::styled(
                        title,
                        Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                    )),
            )
            .highlight_style(Style::default().bg(SELECTED_BG));

        f.render_widget(list, area);
    } else {
        f.render_widget(loading_placeholder("allowlist"), area);
    }
}

// ─── Version List ─────────────────────────────────────────────────────────────

pub fn draw_version_list(f: &mut Frame, app: &App) {
    let (header_area, content_area, status_area) = base_layout(f);

    let title = app
        .current_product_detail
        .as_ref()
        .map(|d| format!("Versions — {}", d.title))
        .unwrap_or_else(|| "Versions".to_string());

    draw_header(f, header_area, app, &title);

    if let Some(detail) = &app.current_product_detail {
        let items: Vec<ListItem> = detail
            .versions
            .iter()
            .map(|v| {
                let vis_color = visibility_color(&v.visibility);
                let arn_hint = v
                    .model_package_arn
                    .as_deref()
                    .map(|a| format!("  ARN: {}", truncate(a, 60)))
                    .unwrap_or_default();

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!(" {:.<50} ", v.title),
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{:<12}", v.visibility),
                            Style::default().fg(vis_color),
                        ),
                        Span::styled(
                            v.created_date.as_deref().unwrap_or("—").to_string(),
                            Style::default().fg(SUBTLE),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            format!("  ID: {}  ", truncate(&v.id, 40)),
                            Style::default().fg(SUBTLE),
                        ),
                        Span::styled(arn_hint, Style::default().fg(SUBTLE)),
                    ]),
                    Line::from(vec![Span::styled(
                        format!(
                            "  Release Notes: {}",
                            truncate(&v.release_notes, 80)
                        ),
                        Style::default().fg(SUBTLE),
                    )]),
                    Line::raw(""),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BRAND_DARK))
                    .title(Span::styled(
                        format!(" Versions ({}) ", detail.versions.len()),
                        Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                    )),
            )
            .highlight_style(
                Style::default()
                    .bg(SELECTED_BG)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = app.versions_list_state.clone();
        f.render_stateful_widget(list, content_area, &mut state);
    } else {
        f.render_widget(loading_placeholder("versions"), content_area);
    }

    draw_status_bar(
        f,
        status_area,
        app,
        "[↑↓] Navigate  [e] Edit version  [r] Restrict version  [y] Copy ID  [Esc] Back",
    );
}

// ─── Whitelist Manager ─────────────────────────────────────────────────────────

pub fn draw_whitelist_manager(f: &mut Frame, app: &App) {
    let (header_area, content_area, status_area) = base_layout(f);

    let title = app
        .current_product_detail
        .as_ref()
        .map(|d| format!("Allowlist — {}", d.title))
        .unwrap_or_else(|| "Allowlist Manager".to_string());

    draw_header(f, header_area, app, &title);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(content_area);

    // Input area
    let input_color = match app.whitelist_input_valid {
        Some(true) => SUCCESS_GREEN,
        Some(false) => ERROR_RED,
        None => if app.whitelist_editing { Color::White } else { SUBTLE },
    };

    let input_display = if app.whitelist_editing {
        format!("{}_", app.whitelist_input)
    } else {
        "Press [a] to add an account ID".to_string()
    };

    let validation_hint = match app.whitelist_input_valid {
        Some(true) => " ✓ Valid 12-digit account ID",
        Some(false) => " ✗ Must be exactly 12 digits",
        None => " Enter a 12-digit AWS account ID",
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(input_color))
        .title(Span::styled(
            " Add Account ID ",
            Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
        ));

    let input_para = Paragraph::new(vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled(input_display, Style::default().fg(input_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(validation_hint, Style::default().fg(SUBTLE)),
        ]),
    ])
    .block(input_block);

    f.render_widget(input_para, layout[0]);

    // Account list
    let accounts = app
        .current_product_detail
        .as_ref()
        .map(|d| d.allowed_accounts.as_slice())
        .unwrap_or(&[]);

    let items: Vec<ListItem> = accounts
        .iter()
        .map(|a| {
            ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(a, Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK))
                .title(Span::styled(
                    format!(" Allowed Accounts ({}) ", accounts.len()),
                    Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(SELECTED_BG)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = app.whitelist_list_state.clone();
    f.render_stateful_widget(list, layout[1], &mut state);

    draw_status_bar(
        f,
        status_area,
        app,
        "[a] Add account  [d/Del] Remove  [Enter] Confirm input  [Esc] Back",
    );
}

// ─── Pricing Editor ───────────────────────────────────────────────────────────

pub fn draw_pricing_editor(f: &mut Frame, app: &App) {
    let (header_area, content_area, status_area) = base_layout(f);

    let title = app
        .current_product_detail
        .as_ref()
        .map(|d| format!("Pricing — {}", d.title))
        .unwrap_or_else(|| "Pricing Editor".to_string());

    draw_header(f, header_area, app, &title);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BRAND_DARK))
        .title(Span::styled(
            " Usage-Based Pricing ",
            Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
        ));

    let fields = [
        (PricingField::Price, "Price per Unit (USD)", &app.pricing_price),
        (PricingField::Currency, "Currency Code", &app.pricing_currency),
        (PricingField::Dimension, "Dimension Key", &app.pricing_dimension),
    ];

    let price_valid_color = match app.pricing_input_valid {
        Some(true) => SUCCESS_GREEN,
        Some(false) => ERROR_RED,
        None => Color::White,
    };

    let lines: Vec<Line> = fields
        .iter()
        .flat_map(|(field, label, value)| {
            let is_focused = app.pricing_focused_field == *field;
            let border_color = if is_focused { BRAND_ORANGE } else { SUBTLE };
            let value_color = if field == &PricingField::Price {
                price_valid_color
            } else if is_focused {
                Color::White
            } else {
                SUBTLE
            };

            let cursor = if is_focused { "_" } else { "" };

            vec![
                Line::from(vec![Span::styled(
                    format!("  {}", label),
                    Style::default().fg(border_color).add_modifier(if is_focused { Modifier::BOLD } else { Modifier::empty() }),
                )]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        format!("{}{}", value, cursor),
                        Style::default().fg(value_color).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::raw(""),
            ]
        })
        .collect();

    let note = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "Note: Changes apply a standard AWS Marketplace EULA automatically.",
            Style::default().fg(SUBTLE),
        ),
    ]);

    let mut all_lines = vec![Line::raw("")];
    all_lines.extend(lines);
    all_lines.push(note);

    let para = Paragraph::new(all_lines).block(block);
    f.render_widget(para, content_area);

    draw_status_bar(
        f,
        status_area,
        app,
        "[Tab] Next field  [Enter] Save  [Esc] Back  (all characters go to the active field)",
    );
}

// ─── Metadata Editor ──────────────────────────────────────────────────────────

pub fn draw_metadata_editor(f: &mut Frame, app: &App) {
    let (header_area, content_area, status_area) = base_layout(f);

    let title = app
        .current_product_detail
        .as_ref()
        .map(|d| format!("Edit Metadata — {}", d.title))
        .unwrap_or_else(|| "Metadata Editor".to_string());

    draw_header(f, header_area, app, &title);

    if let Some(editor) = &app.metadata_editor {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(5)])
            .split(content_area);

        // Field list
        let items: Vec<ListItem> = editor
            .fields
            .iter()
            .enumerate()
            .map(|(i, (field, value))| {
                let is_selected = i == editor.selected_field;
                let label_style = if is_selected {
                    Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(SUBTLE)
                };
                let value_style = if is_selected {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("  {} ", field.label()), label_style),
                    ]),
                    Line::from(vec![
                        Span::raw("    "),
                        Span::styled(truncate(value, 80), value_style),
                    ]),
                    Line::raw(""),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BRAND_DARK))
                    .title(Span::styled(
                        " Fields ",
                        Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                    )),
            )
            .highlight_style(Style::default().bg(SELECTED_BG));

        let mut state = editor.fields_state.clone();
        f.render_stateful_widget(list, layout[0], &mut state);

        // Input area (shown when editing)
        if editor.editing {
            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_ORANGE))
                .title(Span::styled(
                    " Edit Field — Press Enter to save, Esc to cancel ",
                    Style::default().fg(BRAND_ORANGE),
                ));

            let input_para = Paragraph::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("{}_", editor.input_buffer),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ]))
            .block(input_block)
            .wrap(Wrap { trim: false });

            f.render_widget(input_para, layout[1]);
        } else {
            let hint_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK));

            let hint_para = Paragraph::new(Line::from(vec![Span::styled(
                "  [Enter] Edit selected field  [Ctrl+S] Save all changes",
                Style::default().fg(SUBTLE),
            )]))
            .block(hint_block);

            f.render_widget(hint_para, layout[1]);
        }
    }

    draw_status_bar(
        f,
        status_area,
        app,
        "[↑↓] Navigate  [Enter] Edit field  [Ctrl+S] Save  [Esc] Cancel",
    );
}

// ─── Version Metadata Editor ─────────────────────────────────────────────────

pub fn draw_version_metadata_editor(f: &mut Frame, app: &App) {
    let (header_area, content_area, status_area) = base_layout(f);

    let title = app
        .version_metadata_editor
        .as_ref()
        .and_then(|e| {
            app.selected_version().map(|v| format!("Edit Version — {}", v.title))
        })
        .unwrap_or_else(|| "Edit Version".to_string());

    draw_header(f, header_area, app, &title);

    if let Some(editor) = &app.version_metadata_editor {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(5)])
            .split(content_area);

        let items: Vec<ListItem> = editor
            .fields
            .iter()
            .enumerate()
            .map(|(i, (field, value))| {
                let is_selected = i == editor.selected_field;
                let label_style = if is_selected {
                    Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(SUBTLE)
                };
                let value_style = if is_selected {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("  {} ", field.label()), label_style),
                    ]),
                    Line::from(vec![
                        Span::raw("    "),
                        Span::styled(truncate(value, 80), value_style),
                    ]),
                    Line::raw(""),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BRAND_DARK))
                    .title(Span::styled(
                        " Version Fields ",
                        Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                    )),
            )
            .highlight_style(Style::default().bg(SELECTED_BG));

        let mut state = editor.fields_state.clone();
        f.render_stateful_widget(list, layout[0], &mut state);

        if editor.editing {
            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_ORANGE))
                .title(Span::styled(
                    " Edit Field — Enter to save, Esc to cancel ",
                    Style::default().fg(BRAND_ORANGE),
                ));

            let input_para = Paragraph::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("{}_", editor.input_buffer),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ]))
            .block(input_block)
            .wrap(Wrap { trim: false });

            f.render_widget(input_para, layout[1]);
        } else {
            let hint_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK));

            let error_or_hint = editor
                .validation_error
                .as_deref()
                .map(|e| {
                    Span::styled(
                        format!("  ✗ {}", e),
                        Style::default().fg(ERROR_RED).add_modifier(Modifier::BOLD),
                    )
                })
                .unwrap_or_else(|| {
                    Span::styled(
                        "  [Enter] Edit selected field  [Ctrl+S] Save changes",
                        Style::default().fg(SUBTLE),
                    )
                });

            f.render_widget(
                Paragraph::new(Line::from(vec![error_or_hint])).block(hint_block),
                layout[1],
            );
        }
    }

    draw_status_bar(
        f,
        status_area,
        app,
        "[↑↓] Navigate  [Enter] Edit field  [Ctrl+S] Save  [Esc] Cancel",
    );
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn labeled_field<'a>(label: &str, value: &str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:.<28} ", label),
            Style::default().fg(SUBTLE),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

fn labeled_field_colored<'a>(label: &str, value: &str, color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:.<28} ", label),
            Style::default().fg(SUBTLE),
        ),
        Span::styled(
            value.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn loading_placeholder(entity: &str) -> Paragraph<'_> {
    Paragraph::new(vec![
        Line::raw(""),
        Line::from(Span::styled(
            format!("  Loading {}…", entity),
            Style::default().fg(Color::Cyan),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BRAND_DARK)),
    )
}
