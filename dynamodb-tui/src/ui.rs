use crate::app::{App, AppState, DeleteTarget, WizardField, WizardStep};
use crate::models::*;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    match &app.state {
        AppState::RegularTablesView => draw_tables_view(f, app),
        AppState::TableItemsView => draw_items_view(f, app),
        AppState::RegionSelector => draw_region_selector(f, app),
        AppState::TableCreationWizard => draw_table_wizard(f, app),
        AppState::ItemEditor => draw_item_editor(f, app),
        AppState::DeleteConfirmation(target) => draw_delete_confirmation(f, app, target),
        AppState::Help => draw_help(f, app),
    }
}

fn draw_tables_view(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(8),
            Constraint::Length(6),
        ])
        .split(f.area());

    // Tables list
    draw_tables_list(f, app, chunks[0]);

    // Details panel
    draw_table_details(f, app, chunks[1]);

    // Logs
    draw_logs(f, app, chunks[2]);
}

fn draw_tables_list(f: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = ["Name", "Hash Key", "Range Key", "Billing", "Items"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app
        .tables
        .iter()
        .enumerate()
        .map(|(i, table_opt)| {
            if let Some(table) = table_opt {
                let billing_info = match &table.billing_mode {
                    BillingMode::OnDemand => "On-Demand".to_string(),
                    BillingMode::Provisioned => {
                        format!(
                            "Provisioned (R:{} W:{})",
                            table.read_capacity.unwrap_or(0),
                            table.write_capacity.unwrap_or(0)
                        )
                    }
                };

                let range_key = table
                    .range_key
                    .as_ref()
                    .map(|k| format!("{}({})", k.name, k.key_type.to_aws_type()))
                    .unwrap_or_else(|| "-".to_string());

                let items = table
                    .item_count
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".to_string());

                Row::new(vec![
                    Cell::from(table.name.clone()),
                    Cell::from(format!(
                        "{}({})",
                        table.hash_key.name,
                        table.hash_key.key_type.to_aws_type()
                    )),
                    Cell::from(range_key),
                    Cell::from(billing_info),
                    Cell::from(items),
                ])
            } else {
                // Not yet loaded
                Row::new(vec![
                    Cell::from(app.table_names.get(i).cloned().unwrap_or_default()),
                    Cell::from("Loading..."),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ])
            }
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("DynamoDB Tables ({})", app.current_region)),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_table_details(f: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(table) = app.selected_table() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Table: ", Style::default().fg(Color::Cyan)),
                Span::raw(&table.name),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::raw(&table.table_status),
            ]),
            Line::from(vec![
                Span::styled("Hash Key: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!(
                    "{} ({})",
                    table.hash_key.name,
                    table.hash_key.key_type.to_aws_type()
                )),
            ]),
        ];

        if let Some(range_key) = &table.range_key {
            lines.push(Line::from(vec![
                Span::styled("Range Key: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} ({})", range_key.name, range_key.key_type.to_aws_type())),
            ]));
        }

        lines
    } else if app.table_names.is_empty() {
        vec![
            Line::from("No tables in this region."),
            Line::from(""),
            Line::from(vec![
                Span::raw("Press "),
                Span::styled("c", Style::default().fg(Color::Green)),
                Span::raw(" to create a new table."),
            ]),
        ]
    } else {
        vec![Line::from("Select a table to view details")]
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_items_view(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(10),
            Constraint::Length(6),
        ])
        .split(f.area());

    // Items list
    draw_items_list(f, app, chunks[0]);

    // Item details
    draw_item_details(f, app, chunks[1]);

    // Logs
    draw_logs(f, app, chunks[2]);
}

fn draw_items_list(f: &mut Frame, app: &mut App, area: Rect) {
    let table_name = app
        .selected_table()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    if app.current_items.is_empty() {
        let message = if app.is_loading {
            "Loading items..."
        } else {
            "No items in this table. Press 'i' to add an item."
        };

        let paragraph = Paragraph::new(message).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Items in '{}'", table_name)),
        );
        f.render_widget(paragraph, area);
        return;
    }

    // Determine columns from first item
    let table = app.selected_table();
    let hash_key_name = table.map(|t| &t.hash_key.name);
    let range_key_name = table.and_then(|t| t.range_key.as_ref().map(|k| &k.name));

    let mut column_names = Vec::new();
    if let Some(first_item) = app.current_items.first() {
        // Add keys first
        if let Some(hash_name) = hash_key_name {
            column_names.push(hash_name.clone());
        }
        if let Some(range_name) = range_key_name {
            column_names.push(range_name.clone());
        }

        // Add other attributes (limit to 5 total columns)
        for key in first_item.attributes.keys() {
            if column_names.len() >= 5 {
                break;
            }
            if !column_names.contains(key) {
                column_names.push(key.clone());
            }
        }
    }

    let header_cells: Vec<Cell> = column_names
        .iter()
        .map(|h| Cell::from(h.as_str()).style(Style::default().fg(Color::Yellow)))
        .collect();
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app
        .current_items
        .iter()
        .map(|item| {
            let cells: Vec<Cell> = column_names
                .iter()
                .map(|col| {
                    item.attributes
                        .get(col)
                        .map(|v| Cell::from(v.display_short(30)))
                        .unwrap_or_else(|| Cell::from("-"))
                })
                .collect();
            Row::new(cells)
        })
        .collect();

    let widths: Vec<Constraint> = (0..column_names.len())
        .map(|_| Constraint::Percentage(100 / column_names.len() as u16))
        .collect();

    let table_widget = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default().borders(Borders::ALL).title(format!(
                "Items in '{}' (Page {})",
                table_name, app.pagination.current_page
            )),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(table_widget, area, &mut app.items_table_state);
}

fn draw_item_details(f: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(item) = app.selected_item() {
        let mut lines = vec![Line::from(vec![
            Span::styled("Attributes:", Style::default().fg(Color::Cyan)),
        ])];

        // Sort attributes to show keys first
        let mut sorted_attrs: Vec<_> = item.attributes.iter().collect();
        let table = app.selected_table();
        let hash_key_name = table.map(|t| &t.hash_key.name);
        let range_key_name = table.and_then(|t| t.range_key.as_ref().map(|k| &k.name));

        sorted_attrs.sort_by(|a, b| {
            let a_is_key = hash_key_name.map(|h| h == a.0).unwrap_or(false)
                || range_key_name.map(|r| r == a.0).unwrap_or(false);
            let b_is_key = hash_key_name.map(|h| h == b.0).unwrap_or(false)
                || range_key_name.map(|r| r == b.0).unwrap_or(false);

            match (a_is_key, b_is_key) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.cmp(b.0),
            }
        });

        for (key, value) in sorted_attrs {
            lines.push(Line::from(format!("  {}: {}", key, value.display_full())));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("[i]", Style::default().fg(Color::Green)),
            Span::raw(" Add  "),
            Span::styled("[e]", Style::default().fg(Color::Green)),
            Span::raw(" Edit  "),
            Span::styled("[d]", Style::default().fg(Color::Green)),
            Span::raw(" Delete  "),
            Span::styled("[Esc]", Style::default().fg(Color::Green)),
            Span::raw(" Back to Tables"),
        ]));

        lines
    } else {
        vec![Line::from("Select an item to view details")]
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Item Details"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_region_selector(f: &mut Frame, app: &mut App) {
    // Draw tables view as background
    draw_tables_view(f, app);

    // Calculate popup size
    let area = f.area();
    let popup_width = 40;
    let popup_height = 20;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area
    f.render_widget(Block::default().style(Style::default().bg(Color::Black)), popup_area);

    // Create layout for filter and list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(popup_area);

    // Filter input
    let filter_text = vec![Line::from(vec![
        Span::raw("Filter: "),
        Span::styled(&app.region_filter, Style::default().fg(Color::Yellow)),
    ])];
    let filter_widget = Paragraph::new(filter_text)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(filter_widget, chunks[0]);

    // Region list
    let items: Vec<ListItem> = app
        .filtered_regions
        .iter()
        .map(|r| {
            let style = if r == &app.current_region {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            ListItem::new(r.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select Region"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[1], &mut app.region_selector_state.clone());
}

fn draw_table_wizard(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let popup_width = 60;
    let popup_height = 20;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Block::default().style(Style::default().bg(Color::Black)), popup_area);

    match app.wizard_state.step {
        WizardStep::BasicConfig => draw_wizard_basic_config(f, app, popup_area),
        WizardStep::BillingConfig => draw_wizard_billing_config(f, app, popup_area),
        WizardStep::Review => draw_wizard_review(f, app, popup_area),
    }
}

fn draw_wizard_basic_config(f: &mut Frame, app: &App, area: Rect) {
    let ws = &app.wizard_state;

    let lines = vec![
        Line::from(vec![
            Span::raw("Table Name: "),
            if ws.current_field == WizardField::TableName {
                Span::styled(
                    format!("[{}█]", &ws.table_name),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                )
            } else {
                Span::styled(&ws.table_name, Style::default())
            },
        ]),
        Line::from(""),
        Line::from("Partition Key:"),
        Line::from(vec![
            Span::raw("  Name: "),
            if ws.current_field == WizardField::HashKeyName {
                Span::styled(
                    format!("[{}█]", &ws.hash_key_name),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                )
            } else {
                Span::styled(&ws.hash_key_name, Style::default())
            },
        ]),
        Line::from(vec![
            Span::raw("  Type: "),
            Span::styled(
                match ws.hash_key_type {
                    AttributeType::String => "String",
                    AttributeType::Number => "Number",
                    AttributeType::Binary => "Binary",
                },
                if ws.current_field == WizardField::HashKeyType {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            ),
            if ws.current_field == WizardField::HashKeyType {
                Span::styled(" ←", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Sort Key: "),
            Span::styled(
                if ws.has_range_key { "[x]" } else { "[ ]" },
                if ws.current_field == WizardField::HasRangeKey {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                }
            ),
            if ws.current_field == WizardField::HasRangeKey {
                Span::styled(" ← Press Space to toggle", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
        ]),
    ];

    let mut all_lines = lines;

    if ws.has_range_key {
        all_lines.extend(vec![
            Line::from(vec![
                Span::raw("  Name: "),
                if ws.current_field == WizardField::RangeKeyName {
                    Span::styled(
                        format!("[{}█]", &ws.range_key_name),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    )
                } else {
                    Span::styled(&ws.range_key_name, Style::default())
                },
            ]),
            Line::from(vec![
                Span::raw("  Type: "),
                Span::styled(
                    match ws.range_key_type {
                        AttributeType::String => "String",
                        AttributeType::Number => "Number",
                        AttributeType::Binary => "Binary",
                    },
                    if ws.current_field == WizardField::RangeKeyType {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                if ws.current_field == WizardField::RangeKeyType {
                    Span::styled(" ←", Style::default().fg(Color::Yellow))
                } else {
                    Span::raw("")
                },
            ]),
        ]);
    }

    all_lines.push(Line::from(""));
    all_lines.push(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Green)),
        Span::raw(" Next Field  "),
        Span::styled("[Enter]", Style::default().fg(Color::Green)),
        Span::raw(" Next Step  "),
        Span::styled("[Esc]", Style::default().fg(Color::Green)),
        Span::raw(" Cancel"),
    ]));

    let paragraph = Paragraph::new(all_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Create Table (Step 1/3) - Basic Configuration"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_wizard_billing_config(f: &mut Frame, app: &App, area: Rect) {
    let ws = &app.wizard_state;

    let mut lines = vec![
        Line::from(vec![
            Span::raw("Billing Mode:"),
            if ws.current_field == WizardField::BillingMode {
                Span::styled(" ← Use O for On-Demand, P for Provisioned", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                if ws.billing_mode == BillingMode::OnDemand {
                    "◉"
                } else {
                    "○"
                },
                if ws.current_field == WizardField::BillingMode {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                }
            ),
            Span::raw(" On-Demand"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                if ws.billing_mode == BillingMode::Provisioned {
                    "◉"
                } else {
                    "○"
                },
                if ws.current_field == WizardField::BillingMode {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                }
            ),
            Span::raw(" Provisioned"),
        ]),
        Line::from(""),
    ];

    if ws.billing_mode == BillingMode::Provisioned {
        lines.extend(vec![
            Line::from(vec![
                Span::raw("Read Capacity Units: "),
                if ws.current_field == WizardField::ReadCapacity {
                    Span::styled(
                        format!("[{}█]", &ws.read_capacity),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    )
                } else {
                    Span::styled(&ws.read_capacity, Style::default())
                },
            ]),
            Line::from(vec![
                Span::raw("Write Capacity Units: "),
                if ws.current_field == WizardField::WriteCapacity {
                    Span::styled(
                        format!("[{}█]", &ws.write_capacity),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    )
                } else {
                    Span::styled(&ws.write_capacity, Style::default())
                },
            ]),
            Line::from(""),
        ]);
    }

    lines.push(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Green)),
        Span::raw(" Next Field  "),
        Span::styled("[Enter]", Style::default().fg(Color::Green)),
        Span::raw(" Next Step  "),
        Span::styled("[Backspace]", Style::default().fg(Color::Green)),
        Span::raw(" Back  "),
        Span::styled("[Esc]", Style::default().fg(Color::Green)),
        Span::raw(" Cancel"),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Create Table (Step 2/3) - Billing Configuration"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_wizard_review(f: &mut Frame, app: &App, area: Rect) {
    let ws = &app.wizard_state;

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Table Name: ", Style::default().fg(Color::Cyan)),
            Span::raw(&ws.table_name),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Partition Key: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{} ({})",
                ws.hash_key_name,
                ws.hash_key_type.to_aws_type()
            )),
        ]),
    ];

    if ws.has_range_key {
        lines.push(Line::from(vec![
            Span::styled("Sort Key: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{} ({})",
                ws.range_key_name,
                ws.range_key_type.to_aws_type()
            )),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Billing Mode: ", Style::default().fg(Color::Cyan)),
        Span::raw(match ws.billing_mode {
            BillingMode::OnDemand => "On-Demand",
            BillingMode::Provisioned => "Provisioned",
        }),
    ]));

    if ws.billing_mode == BillingMode::Provisioned {
        lines.push(Line::from(vec![
            Span::styled("Read Capacity: ", Style::default().fg(Color::Cyan)),
            Span::raw(&ws.read_capacity),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Write Capacity: ", Style::default().fg(Color::Cyan)),
            Span::raw(&ws.write_capacity),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(Color::Green)),
        Span::raw(" Create Table  "),
        Span::styled("[Backspace]", Style::default().fg(Color::Green)),
        Span::raw(" Back  "),
        Span::styled("[Esc]", Style::default().fg(Color::Green)),
        Span::raw(" Cancel"),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Create Table (Step 3/3) - Review"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_item_editor(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup_width = area.width.saturating_sub(10).min(80);
    let popup_height = area.height.saturating_sub(4).min(30);
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Block::default().style(Style::default().bg(Color::Black)), popup_area);

    if let Some(editor) = &app.item_editor {
        let title = match editor.mode {
            EditorMode::Create => format!("Add New Item to '{}'", editor.table_name),
            EditorMode::Edit => format!("Edit Item in '{}'", editor.table_name),
        };

        let mut lines = vec![Line::from(vec![
            Span::styled("Primary Keys:", Style::default().fg(Color::Yellow)),
        ])];

        // Show attributes
        for (i, (key, value)) in editor.attributes.iter().enumerate() {
            let is_hash_key = key == &editor.hash_key_schema.name;
            let is_range_key = editor
                .range_key_schema
                .as_ref()
                .map(|k| key == &k.name)
                .unwrap_or(false);
            let is_key = is_hash_key || is_range_key;

            let prefix = if i == editor.selected_attribute_index {
                "> "
            } else {
                "  "
            };

            let key_marker = if is_key {
                if editor.mode == EditorMode::Edit {
                    " [READ-ONLY]"
                } else {
                    " *"
                }
            } else {
                ""
            };

            let style = if i == editor.selected_attribute_index {
                Style::default().fg(Color::Yellow)
            } else if is_key {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            // Check if we're currently editing this attribute
            let is_editing_key = i == editor.selected_attribute_index
                && matches!(editor.editing_field, Some(EditingField::Key));
            let is_editing_value = i == editor.selected_attribute_index
                && matches!(editor.editing_field, Some(EditingField::Value));

            let key_display = if is_editing_key {
                format!("[{}]", editor.input_buffer)
            } else {
                key.clone()
            };

            let value_display = if is_editing_value {
                format!("[{}]", editor.input_buffer)
            } else {
                value.display_short(40)
            };

            // Show attribute type
            let type_str = match value {
                crate::models::AttributeValue::String(_) => " (String)",
                crate::models::AttributeValue::Number(_) => " (Number)",
                crate::models::AttributeValue::Binary(_) => " (Binary)",
                crate::models::AttributeValue::Boolean(_) => " (Boolean)",
                crate::models::AttributeValue::Null => " (Null)",
                crate::models::AttributeValue::StringSet(_) => " (SS)",
                crate::models::AttributeValue::NumberSet(_) => " (NS)",
                crate::models::AttributeValue::BinarySet(_) => " (BS)",
                crate::models::AttributeValue::List(_) => " (List)",
                crate::models::AttributeValue::Map(_) => " (Map)",
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}{}: ", prefix, key_display),
                    if is_editing_key {
                        Style::default().fg(Color::Green).add_modifier(Modifier::UNDERLINED)
                    } else {
                        style
                    }
                ),
                Span::styled(
                    value_display,
                    if is_editing_value {
                        Style::default().fg(Color::Green).add_modifier(Modifier::UNDERLINED)
                    } else {
                        Style::default()
                    }
                ),
                Span::styled(type_str, Style::default().fg(Color::DarkGray)),
                Span::styled(key_marker, Style::default().fg(Color::Red)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  + Add Attribute", Style::default().fg(Color::Green)),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("[↑↓]", Style::default().fg(Color::Green)),
            Span::raw(" Navigate  "),
            Span::styled("[Enter]", Style::default().fg(Color::Green)),
            Span::raw(" Edit Value  "),
            Span::styled("[Ctrl+K]", Style::default().fg(Color::Green)),
            Span::raw(" Edit Name  "),
            Span::styled("[Ctrl+T]", Style::default().fg(Color::Green)),
            Span::raw(" Change Type"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("[Ctrl+N]", Style::default().fg(Color::Green)),
            Span::raw(" New Attr  "),
            Span::styled("[Ctrl+D]", Style::default().fg(Color::Green)),
            Span::raw(" Delete Attr  "),
            Span::styled("[Ctrl+S]", Style::default().fg(Color::Green)),
            Span::raw(" Save  "),
            Span::styled("[Esc]", Style::default().fg(Color::Green)),
            Span::raw(" Cancel"),
        ]));

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, popup_area);
    }
}

fn draw_delete_confirmation(f: &mut Frame, _app: &App, target: &DeleteTarget) {
    // We can't easily draw the background here without mutable access
    // The background will be drawn in the main draw function before calling this

    let area = f.area();
    let popup_width = 50;
    let popup_height = 8;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Block::default().style(Style::default().bg(Color::Black)), popup_area);

    let message = match target {
        DeleteTarget::Table(name) => format!("Delete table '{}'?", name),
        DeleteTarget::Item => "Delete selected item?".to_string(),
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Warning: ", Style::default().fg(Color::Red)),
            Span::raw(&message),
        ]),
        Line::from(""),
        Line::from("This action cannot be undone."),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y]", Style::default().fg(Color::Red)),
            Span::raw(" Yes, delete  "),
            Span::styled("[n]", Style::default().fg(Color::Green)),
            Span::raw(" No, cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Confirm Deletion")
                .border_style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, popup_area);
}

fn draw_help(f: &mut Frame, _app: &App) {
    let help_text = vec![
        Line::from(vec![
            Span::styled("DynamoDB TUI - Help", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tables View:", Style::default().fg(Color::Yellow)),
        ]),
        Line::from("  ↑/↓        - Navigate table list"),
        Line::from("  Enter      - View items in selected table"),
        Line::from("  r          - Refresh table status and details"),
        Line::from("  g          - Open region selector"),
        Line::from("  c          - Create new table"),
        Line::from("  d          - Delete selected table"),
        Line::from("  ?          - Show this help"),
        Line::from("  q/Esc      - Quit application"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Items View:", Style::default().fg(Color::Yellow)),
        ]),
        Line::from("  ↑/↓        - Navigate item list"),
        Line::from("  i          - Add new item"),
        Line::from("  e/Enter    - Edit selected item"),
        Line::from("  d          - Delete selected item"),
        Line::from("  Esc        - Back to tables view"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Item Editor:", Style::default().fg(Color::Yellow)),
        ]),
        Line::from("  ↑/↓        - Navigate attributes"),
        Line::from("  Enter      - Edit attribute value"),
        Line::from("  Ctrl+K     - Edit attribute name"),
        Line::from("  Ctrl+T     - Change attribute type (String/Number/Binary)"),
        Line::from("  Ctrl+N     - Add new attribute"),
        Line::from("  Ctrl+D     - Delete attribute"),
        Line::from("  Ctrl+S     - Save item"),
        Line::from("  Esc        - Cancel without saving"),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Press any key to close]", Style::default().fg(Color::Green)),
        ]),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, f.area());
}

fn draw_logs(f: &mut Frame, app: &App, area: Rect) {
    let logs = if let Ok(log) = app.log.lock() {
        log.iter()
            .rev()
            .take(4)
            .rev()
            .map(|entry| {
                let level_style = match entry.level {
                    LogLevel::Info => Style::default().fg(Color::Cyan),
                    LogLevel::Warning => Style::default().fg(Color::Yellow),
                    LogLevel::Error => Style::default().fg(Color::Red),
                };
                Line::from(vec![
                    Span::styled(entry.timestamp.clone(), Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(
                        match entry.level {
                            LogLevel::Info => "[INFO]",
                            LogLevel::Warning => "[WARN]",
                            LogLevel::Error => "[ERROR]",
                        },
                        level_style,
                    ),
                    Span::raw(" "),
                    Span::raw(entry.message.clone()),
                ])
            })
            .collect()
    } else {
        vec![]
    };

    let paragraph = Paragraph::new(logs)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}
