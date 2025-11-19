use crate::app::{App, AppState, DeleteTarget, WizardField, WizardStep};
use crate::models::AttributeType;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key_event(app: &mut App, key: KeyEvent) -> bool {
    match &app.state {
        AppState::RegularTablesView => handle_tables_view_keys(app, key),
        AppState::TableItemsView => handle_items_view_keys(app, key),
        AppState::RegionSelector => handle_region_selector_keys(app, key),
        AppState::TableCreationWizard => handle_wizard_keys(app, key),
        AppState::ItemEditor => handle_item_editor_keys(app, key),
        AppState::DeleteConfirmation(_) => handle_delete_confirmation_keys(app, key),
        AppState::Help => {
            // Any key closes help
            app.state = AppState::RegularTablesView;
            false
        }
    }
}

fn handle_tables_view_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => return true, // Quit
        KeyCode::Char('?') => {
            app.state = AppState::Help;
        }
        KeyCode::Char('g') => {
            app.state = AppState::RegionSelector;
            app.region_filter.clear();
            app.update_region_filter(String::new());
        }
        KeyCode::Char('r') => {
            // Refresh table status and details
            app.pending_refresh = true;
        }
        KeyCode::Char('c') => {
            app.wizard_state = crate::app::WizardState::new();
            app.state = AppState::TableCreationWizard;
        }
        KeyCode::Char('d') => {
            if let Some(table) = app.selected_table() {
                let table_name = table.name.clone();
                app.state = AppState::DeleteConfirmation(DeleteTarget::Table(table_name));
            }
        }
        KeyCode::Up => {
            if !app.table_names.is_empty() {
                let current = app.selected_table_index.unwrap_or(0);
                let new_index = if current > 0 {
                    current - 1
                } else {
                    app.table_names.len() - 1
                };
                app.selected_table_index = Some(new_index);
                app.table_state.select(Some(new_index));
            }
        }
        KeyCode::Down => {
            if !app.table_names.is_empty() {
                let current = app.selected_table_index.unwrap_or(0);
                let new_index = if current < app.table_names.len() - 1 {
                    current + 1
                } else {
                    0
                };
                app.selected_table_index = Some(new_index);
                app.table_state.select(Some(new_index));
            }
        }
        KeyCode::Enter => {
            if app.selected_table().is_some() {
                app.enter_items_view();
                return false; // Signal to load items
            }
        }
        _ => {}
    }
    false
}

fn handle_items_view_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.exit_items_view();
        }
        KeyCode::Char('i') => {
            if let Some(table) = app.selected_table().cloned() {
                app.start_create_item(&table);
            }
        }
        KeyCode::Char('e') | KeyCode::Enter => {
            if let (Some(table), Some(item)) =
                (app.selected_table().cloned(), app.selected_item().cloned())
            {
                app.start_edit_item(&table, &item);
            }
        }
        KeyCode::Char('d') => {
            if app.selected_item().is_some() {
                app.state = AppState::DeleteConfirmation(DeleteTarget::Item);
            }
        }
        KeyCode::Up => {
            if !app.current_items.is_empty() {
                let current = app.selected_item_index.unwrap_or(0);
                let new_index = if current > 0 {
                    current - 1
                } else {
                    app.current_items.len() - 1
                };
                app.selected_item_index = Some(new_index);
                app.items_table_state.select(Some(new_index));
            }
        }
        KeyCode::Down => {
            if !app.current_items.is_empty() {
                let current = app.selected_item_index.unwrap_or(0);
                let new_index = if current < app.current_items.len() - 1 {
                    current + 1
                } else {
                    0
                };
                app.selected_item_index = Some(new_index);
                app.items_table_state.select(Some(new_index));
            }
        }
        _ => {}
    }
    false
}

fn handle_region_selector_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.state = AppState::RegularTablesView;
        }
        KeyCode::Enter => {
            if let Some(selected) = app.region_selector_state.selected() {
                if let Some(region) = app.filtered_regions.get(selected) {
                    app.current_region = region.clone();
                    app.state = AppState::RegularTablesView;
                    return false; // Signal to reload tables
                }
            }
        }
        KeyCode::Up => {
            if !app.filtered_regions.is_empty() {
                let current = app.region_selector_state.selected().unwrap_or(0);
                let new_index = if current > 0 {
                    current - 1
                } else {
                    app.filtered_regions.len() - 1
                };
                app.region_selector_state.select(Some(new_index));
            }
        }
        KeyCode::Down => {
            if !app.filtered_regions.is_empty() {
                let current = app.region_selector_state.selected().unwrap_or(0);
                let new_index = if current < app.filtered_regions.len() - 1 {
                    current + 1
                } else {
                    0
                };
                app.region_selector_state.select(Some(new_index));
            }
        }
        KeyCode::Char(c) => {
            app.region_filter.push(c);
            app.update_region_filter(app.region_filter.clone());
        }
        KeyCode::Backspace => {
            app.region_filter.pop();
            app.update_region_filter(app.region_filter.clone());
        }
        _ => {}
    }
    false
}

fn handle_wizard_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.state = AppState::RegularTablesView;
            return false;
        }
        KeyCode::Enter => {
            match app.wizard_state.step {
                WizardStep::BasicConfig => {
                    if let Err(e) = app.wizard_state.validate_basic_config() {
                        app.status_message = e;
                    } else {
                        app.wizard_state.step = WizardStep::BillingConfig;
                    }
                }
                WizardStep::BillingConfig => {
                    if let Err(e) = app.wizard_state.validate_billing_config() {
                        app.status_message = e;
                    } else {
                        app.wizard_state.step = WizardStep::Review;
                    }
                }
                WizardStep::Review => {
                    // Signal to create table
                    return false;
                }
            }
        }
        KeyCode::Tab => {
            handle_wizard_tab(app);
        }
        KeyCode::Char(c) => {
            handle_wizard_char_input(app, c);
        }
        KeyCode::Backspace => {
            // Check if current field is a text input field
            use WizardField::*;
            let is_text_field = matches!(
                app.wizard_state.current_field,
                TableName | HashKeyName | RangeKeyName | ReadCapacity | WriteCapacity
            );

            if is_text_field {
                // Backspace deletes character from text field
                handle_wizard_backspace(app);
            } else if key.modifiers.is_empty() {
                // Navigate back a step (only when not on text fields)
                match app.wizard_state.step {
                    WizardStep::BillingConfig => {
                        app.wizard_state.step = WizardStep::BasicConfig;
                    }
                    WizardStep::Review => {
                        app.wizard_state.step = WizardStep::BillingConfig;
                    }
                    _ => {}
                }
            }
            return false;
        }
        _ => {}
    }
    false
}

fn handle_wizard_tab(app: &mut App) {
    use WizardField::*;
    let ws = &mut app.wizard_state;

    match ws.step {
        WizardStep::BasicConfig => {
            ws.current_field = match ws.current_field {
                TableName => HashKeyName,
                HashKeyName => HashKeyType,
                HashKeyType => HasRangeKey,
                HasRangeKey => {
                    if ws.has_range_key {
                        RangeKeyName
                    } else {
                        TableName
                    }
                }
                RangeKeyName => RangeKeyType,
                RangeKeyType => TableName,
                _ => TableName,
            };
        }
        WizardStep::BillingConfig => {
            ws.current_field = match ws.current_field {
                BillingMode => {
                    if ws.billing_mode == crate::models::BillingMode::Provisioned {
                        ReadCapacity
                    } else {
                        BillingMode
                    }
                }
                ReadCapacity => WriteCapacity,
                WriteCapacity => BillingMode,
                _ => BillingMode,
            };
        }
        WizardStep::Review => {}
    }
}

fn handle_wizard_char_input(app: &mut App, c: char) {
    use WizardField::*;
    let ws = &mut app.wizard_state;

    match ws.current_field {
        TableName => ws.table_name.push(c),
        HashKeyName => ws.hash_key_name.push(c),
        HashKeyType => {
            ws.hash_key_type = match c {
                's' | 'S' => AttributeType::String,
                'n' | 'N' => AttributeType::Number,
                'b' | 'B' => AttributeType::Binary,
                _ => ws.hash_key_type.clone(),
            };
        }
        HasRangeKey => {
            if c == ' ' || c == 'x' || c == 'X' {
                ws.has_range_key = !ws.has_range_key;
            }
        }
        RangeKeyName => ws.range_key_name.push(c),
        RangeKeyType => {
            ws.range_key_type = match c {
                's' | 'S' => AttributeType::String,
                'n' | 'N' => AttributeType::Number,
                'b' | 'B' => AttributeType::Binary,
                _ => ws.range_key_type.clone(),
            };
        }
        BillingMode => {
            match c {
                'o' | 'O' | '1' => ws.billing_mode = crate::models::BillingMode::OnDemand,
                'p' | 'P' | '2' => ws.billing_mode = crate::models::BillingMode::Provisioned,
                _ => {}
            }
        }
        ReadCapacity => {
            if c.is_ascii_digit() {
                ws.read_capacity.push(c);
            }
        }
        WriteCapacity => {
            if c.is_ascii_digit() {
                ws.write_capacity.push(c);
            }
        }
    }
}

fn handle_wizard_backspace(app: &mut App) {
    use WizardField::*;
    let ws = &mut app.wizard_state;

    match ws.current_field {
        TableName => {
            ws.table_name.pop();
        }
        HashKeyName => {
            ws.hash_key_name.pop();
        }
        RangeKeyName => {
            ws.range_key_name.pop();
        }
        ReadCapacity => {
            ws.read_capacity.pop();
        }
        WriteCapacity => {
            ws.write_capacity.pop();
        }
        _ => {}
    }
}

fn handle_item_editor_keys(app: &mut App, key: KeyEvent) -> bool {
    // Check if we're currently editing a field
    let is_editing = app.item_editor.as_ref()
        .and_then(|e| e.editing_field.as_ref())
        .cloned();

    if let Some(field) = is_editing {
        return handle_attribute_editing(app, key, field);
    }

    // Not editing, handle navigation
    match key.code {
        KeyCode::Esc => {
            app.exit_item_editor();
        }
        KeyCode::Up => {
            if let Some(editor) = &mut app.item_editor {
                if editor.selected_attribute_index > 0 {
                    editor.selected_attribute_index -= 1;
                }
            }
        }
        KeyCode::Down => {
            if let Some(editor) = &mut app.item_editor {
                if editor.selected_attribute_index < editor.attributes.len() {
                    editor.selected_attribute_index += 1;
                }
            }
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Signal to save item
            app.pending_save_item = true;
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Add new attribute - default to String type
            if let Some(editor) = &mut app.item_editor {
                editor
                    .attributes
                    .push(("new_attribute".to_string(), crate::models::AttributeValue::String(String::new())));
                editor.selected_attribute_index = editor.attributes.len() - 1;
            }
        }
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Change attribute type (cycles through String -> Number -> Binary)
            if let Some(editor) = &mut app.item_editor {
                let idx = editor.selected_attribute_index;
                if idx < editor.attributes.len() {
                    let (key, value) = &editor.attributes[idx];

                    // Check if this is a primary key (can't change type)
                    let is_hash_key = key == &editor.hash_key_schema.name;
                    let is_range_key = editor
                        .range_key_schema
                        .as_ref()
                        .map(|k| key == &k.name)
                        .unwrap_or(false);

                    if !is_hash_key && !is_range_key {
                        // Cycle through types, preserving value where possible
                        let new_value = match value {
                            crate::models::AttributeValue::String(s) => {
                                // String -> Number (keep value if it looks like a number)
                                crate::models::AttributeValue::Number(s.clone())
                            }
                            crate::models::AttributeValue::Number(n) => {
                                // Number -> Binary (clear value)
                                crate::models::AttributeValue::Binary(Vec::new())
                            }
                            crate::models::AttributeValue::Binary(_) => {
                                // Binary -> String (clear value)
                                crate::models::AttributeValue::String(String::new())
                            }
                            _ => {
                                // For complex types, reset to String
                                crate::models::AttributeValue::String(String::new())
                            }
                        };

                        let key_clone = key.clone();
                        editor.attributes[idx] = (key_clone, new_value);
                    }
                }
            }
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Delete selected attribute (not keys)
            if let Some(editor) = &mut app.item_editor {
                let idx = editor.selected_attribute_index;
                if idx < editor.attributes.len() {
                    let (key, _) = &editor.attributes[idx];
                    let is_hash_key = key == &editor.hash_key_schema.name;
                    let is_range_key = editor
                        .range_key_schema
                        .as_ref()
                        .map(|k| key == &k.name)
                        .unwrap_or(false);

                    // Don't allow deleting primary keys
                    if !is_hash_key && !is_range_key {
                        editor.attributes.remove(idx);
                        if editor.selected_attribute_index >= editor.attributes.len() && !editor.attributes.is_empty() {
                            editor.selected_attribute_index = editor.attributes.len() - 1;
                        }
                    }
                }
            }
        }
        KeyCode::Enter => {
            // Start editing selected attribute value
            if let Some(editor) = &mut app.item_editor {
                if editor.selected_attribute_index < editor.attributes.len() {
                    let (_, value) = &editor.attributes[editor.selected_attribute_index];
                    // Initialize input buffer with current value
                    editor.input_buffer = match value {
                        crate::models::AttributeValue::String(s) => s.clone(),
                        crate::models::AttributeValue::Number(n) => n.clone(),
                        crate::models::AttributeValue::Boolean(b) => b.to_string(),
                        crate::models::AttributeValue::Binary(b) => {
                            // Display as hex string for editing
                            b.iter().map(|byte| format!("{:02x}", byte)).collect::<String>()
                        }
                        _ => String::new(),
                    };
                    editor.editing_field = Some(crate::models::EditingField::Value);
                }
            }
        }
        KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Edit attribute key/name
            if let Some(editor) = &mut app.item_editor {
                if editor.selected_attribute_index < editor.attributes.len() {
                    let (key, _) = &editor.attributes[editor.selected_attribute_index];

                    // Check if this is a primary key (can't rename those)
                    let is_hash_key = key == &editor.hash_key_schema.name;
                    let is_range_key = editor
                        .range_key_schema
                        .as_ref()
                        .map(|k| key == &k.name)
                        .unwrap_or(false);

                    if !is_hash_key && !is_range_key {
                        editor.input_buffer = key.clone();
                        editor.editing_field = Some(crate::models::EditingField::Key);
                    }
                }
            }
        }
        _ => {}
    }
    false
}

fn handle_attribute_editing(app: &mut App, key: KeyEvent, field: crate::models::EditingField) -> bool {
    match key.code {
        KeyCode::Esc => {
            // Cancel editing
            if let Some(editor) = &mut app.item_editor {
                editor.editing_field = None;
                editor.input_buffer.clear();
            }
            return false;
        }
        KeyCode::Enter => {
            // Save the edited value
            if let Some(editor) = &mut app.item_editor {
                let idx = editor.selected_attribute_index;
                if idx < editor.attributes.len() {
                    let new_value = match field {
                        crate::models::EditingField::Value => {
                            // Determine the type and parse accordingly
                            let (_, current_value) = &editor.attributes[idx];
                            match current_value {
                                crate::models::AttributeValue::String(_) => {
                                    crate::models::AttributeValue::String(editor.input_buffer.clone())
                                }
                                crate::models::AttributeValue::Number(_) => {
                                    // Validate it's a number
                                    crate::models::AttributeValue::Number(editor.input_buffer.clone())
                                }
                                crate::models::AttributeValue::Boolean(_) => {
                                    let val = editor.input_buffer.to_lowercase();
                                    let bool_val = val == "true" || val == "1" || val == "yes";
                                    crate::models::AttributeValue::Boolean(bool_val)
                                }
                                crate::models::AttributeValue::Binary(_) => {
                                    // Parse hex string to binary
                                    let hex_str = editor.input_buffer.trim();
                                    let mut bytes = Vec::new();

                                    // Parse pairs of hex digits
                                    for i in (0..hex_str.len()).step_by(2) {
                                        if i + 1 < hex_str.len() {
                                            if let Ok(byte) = u8::from_str_radix(&hex_str[i..i+2], 16) {
                                                bytes.push(byte);
                                            }
                                        }
                                    }

                                    crate::models::AttributeValue::Binary(bytes)
                                }
                                _ => crate::models::AttributeValue::String(editor.input_buffer.clone()),
                            }
                        }
                        crate::models::EditingField::Key => {
                            // Edit attribute name
                            let (_, value) = editor.attributes[idx].clone();
                            editor.attributes[idx] = (editor.input_buffer.clone(), value);
                            editor.editing_field = None;
                            editor.input_buffer.clear();
                            return false;
                        }
                        _ => {
                            editor.editing_field = None;
                            editor.input_buffer.clear();
                            return false;
                        }
                    };

                    if matches!(field, crate::models::EditingField::Value) {
                        let (key, _) = editor.attributes[idx].clone();
                        editor.attributes[idx] = (key, new_value);
                    }
                }
                editor.editing_field = None;
                editor.input_buffer.clear();
            }
            return false;
        }
        KeyCode::Char(c) => {
            if let Some(editor) = &mut app.item_editor {
                editor.input_buffer.push(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(editor) = &mut app.item_editor {
                editor.input_buffer.pop();
            }
        }
        _ => {}
    }
    false
}

fn handle_delete_confirmation_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            // Signal to perform deletion
            match &app.state {
                AppState::DeleteConfirmation(DeleteTarget::Table(name)) => {
                    app.pending_delete_table = Some(name.clone());
                }
                AppState::DeleteConfirmation(DeleteTarget::Item) => {
                    app.pending_delete_item = true;
                }
                _ => {}
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            // Cancel deletion
            match &app.state {
                AppState::DeleteConfirmation(DeleteTarget::Table(_)) => {
                    app.state = AppState::RegularTablesView;
                }
                AppState::DeleteConfirmation(DeleteTarget::Item) => {
                    app.state = AppState::TableItemsView;
                }
                _ => {}
            }
        }
        _ => {}
    }
    false
}
