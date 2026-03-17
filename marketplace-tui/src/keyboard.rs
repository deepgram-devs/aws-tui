use crate::app::{
    App, AppState, CommandAction, CreateProductField, CreateProductStep, CreateVersionField,
    CreateVersionStep, DetailTab, PricingField, VersionMetadataField,
};
use crate::app::CreateVersionStep::{VersionInfo, Instructions, IOProperties, Review as VersionReview};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Returns true if the application should quit.
pub fn handle_key_event(app: &mut App, key: KeyEvent) -> bool {
    // Ctrl+C always quits
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return true;
    }

    match &app.state.clone() {
        AppState::ProductList => handle_product_list(app, key),
        AppState::ProductDetail => handle_product_detail(app, key),
        AppState::VersionList => handle_version_list(app, key),
        AppState::CreateProductWizard => handle_create_product(app, key),
        AppState::CreateVersionWizard => handle_create_version(app, key),
        AppState::WhitelistManager => handle_whitelist(app, key),
        AppState::PricingEditor => handle_pricing(app, key),
        AppState::MetadataEditor => handle_metadata_editor(app, key),
        AppState::VersionMetadataEditor => handle_version_metadata_editor(app, key),
        AppState::RestrictVersionConfirm => handle_restrict_confirm(app, key),
        AppState::ChangeSetStatusView => handle_changeset_view(app, key),
        AppState::CommandPalette => handle_command_palette(app, key),
        AppState::Help => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') => app.close_overlay(),
                KeyCode::Up | KeyCode::Char('k') => {
                    app.help_scroll = app.help_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.help_scroll = app.help_scroll.saturating_add(1);
                }
                KeyCode::PageUp => {
                    app.help_scroll = app.help_scroll.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    app.help_scroll = app.help_scroll.saturating_add(10);
                }
                _ => {}
            }
            false
        }
        AppState::Logs => handle_logs(app, key),
    }
}

// ─── Product List ─────────────────────────────────────────────────────────────

fn handle_product_list(app: &mut App, key: KeyEvent) -> bool {
    // ── Filter input mode ────────────────────────────────────────────────────
    // Entered with `/`. All printable characters go to the filter; shortcuts
    // are suppressed so the user can type any character including q, j, k, etc.
    if app.filter_input_mode {
        match key.code {
            KeyCode::Esc => {
                app.filter_input_mode = false;
                app.product_filter.clear();
                app.apply_product_filter();
            }
            KeyCode::Enter => {
                // Confirm filter and return to navigation mode
                app.filter_input_mode = false;
            }
            KeyCode::Backspace => {
                app.product_filter.pop();
                app.apply_product_filter();
                if app.product_filter.is_empty() {
                    app.filter_input_mode = false;
                }
            }
            // Navigation still works while filtering
            KeyCode::Up => app.product_list_up(),
            KeyCode::Down => app.product_list_down(),
            KeyCode::PageUp => app.product_list_page_up(10),
            KeyCode::PageDown => app.product_list_page_down(10),
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.product_filter.push(c);
                app.apply_product_filter();
            }
            _ => {}
        }
        return false;
    }

    // ── Navigation mode ───────────────────────────────────────────────────────
    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('?') => app.open_help(),
        KeyCode::Char('l') => app.open_logs(),
        KeyCode::Char('s') => app.open_changeset_view(),
        KeyCode::Char(':') | KeyCode::Char(' ') => app.open_command_palette(),
        KeyCode::Char('R') => {
            app.pending_refresh = true;
        }
        KeyCode::Char('c') => {
            app.create_product = crate::app::CreateProductWizardState::new();
            app.state = AppState::CreateProductWizard;
        }
        KeyCode::Char('y') => {
            if let Some(id) = app.selected_product().map(|p| p.entity_id.clone()) {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(id.clone());
                    app.set_status(format!("Copied entity ID: {}", id), false);
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => app.product_list_up(),
        KeyCode::Down | KeyCode::Char('j') => app.product_list_down(),
        KeyCode::PageUp => app.product_list_page_up(10),
        KeyCode::PageDown => app.product_list_page_down(10),
        KeyCode::Enter => {
            app.open_product_detail();
        }
        // `/` enters filter input mode — all subsequent characters go to filter
        KeyCode::Char('/') => {
            app.filter_input_mode = true;
            app.product_filter.clear();
            app.apply_product_filter();
        }
        KeyCode::Esc => {
            if !app.product_filter.is_empty() {
                app.product_filter.clear();
                app.apply_product_filter();
            }
        }
        _ => {}
    }
    false
}

// ─── Product Detail ───────────────────────────────────────────────────────────

fn handle_product_detail(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Esc => {
            app.state = AppState::ProductList;
        }
        KeyCode::Char('?') => app.open_help(),
        KeyCode::Char('l') => app.open_logs(),
        KeyCode::Char('s') => app.open_changeset_view(),
        KeyCode::Char(':') | KeyCode::Char(' ') => app.open_command_palette(),
        KeyCode::Tab | KeyCode::Right => {
            let next_idx = (app.detail_tab.index() + 1) % 4;
            app.detail_tab = DetailTab::from_index(next_idx);
        }
        KeyCode::BackTab | KeyCode::Left => {
            let cur = app.detail_tab.index();
            let prev = if cur == 0 { 3 } else { cur - 1 };
            app.detail_tab = DetailTab::from_index(prev);
        }
        KeyCode::Char('e') => {
            if app.current_product_detail.is_some() {
                app.open_metadata_editor();
            }
        }
        KeyCode::Char('v') => {
            if app.current_product_detail.is_some() {
                if let Some(entity_id) = app.current_product_detail.as_ref().map(|d| d.entity_id.clone()) {
                    app.create_version = crate::app::CreateVersionWizardState::new(entity_id);
                    app.state = AppState::CreateVersionWizard;
                }
            }
        }
        KeyCode::Char('V') => {
            if app.current_product_detail.is_some() {
                app.open_version_list();
            }
        }
        KeyCode::Char('w') => {
            if app.current_product_detail.is_some() {
                app.open_whitelist_manager();
            }
        }
        KeyCode::Char('p') => {
            if app.current_product_detail.is_some() {
                app.open_pricing_editor();
            }
        }
        KeyCode::Char('y') => {
            if let Some(detail) = &app.current_product_detail {
                let id = detail.entity_id.clone();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(id.clone());
                    app.set_status(format!("Copied entity ID: {}", id), false);
                }
            }
        }
        KeyCode::Char('R') => {
            if let Some(detail) = &app.current_product_detail {
                let entity_id = detail.entity_id.clone();
                app.pending_load_detail = Some(entity_id);
            }
        }
        _ => {}
    }
    false
}

// ─── Version List ─────────────────────────────────────────────────────────────

fn handle_version_list(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Esc => {
            app.state = AppState::ProductDetail;
        }
        KeyCode::Char('?') => app.open_help(),
        KeyCode::Char('l') => app.open_logs(),
        KeyCode::Up | KeyCode::Char('k') => app.version_list_up(),
        KeyCode::Down | KeyCode::Char('j') => app.version_list_down(),
        KeyCode::PageUp => {
            for _ in 0..10 { app.version_list_up(); }
        }
        KeyCode::PageDown => {
            for _ in 0..10 { app.version_list_down(); }
        }
        KeyCode::Char('e') => {
            app.open_version_metadata_editor();
        }
        KeyCode::Char('r') => {
            if app.selected_version().is_some() {
                app.previous_state = AppState::VersionList;
                app.state = AppState::RestrictVersionConfirm;
            }
        }
        KeyCode::Char('y') => {
            if let Some(ver) = app.selected_version() {
                let id = ver.id.clone();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(id.clone());
                    app.set_status(format!("Copied version ID: {}", id), false);
                }
            }
        }
        _ => {}
    }
    false
}

// ─── Create Product Wizard ────────────────────────────────────────────────────

fn handle_create_product(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.state = AppState::ProductList;
            return false;
        }
        KeyCode::Tab => {
            app.create_product.advance_field();
        }
        KeyCode::Char(c) => {
            app.create_product.active_field_mut().push(c);
        }
        KeyCode::Backspace => {
            app.create_product.active_field_mut().pop();
        }
        KeyCode::Enter => {
            match app.create_product.step.clone() {
                CreateProductStep::BasicInfo => {
                    match app.create_product.validate_step() {
                        Ok(()) => {
                            app.create_product.step = CreateProductStep::Descriptions;
                            app.create_product.focused_field = CreateProductField::LongDescription;
                            app.create_product.validation_error = None;
                        }
                        Err(e) => {
                            app.create_product.validation_error = Some(e);
                        }
                    }
                }
                CreateProductStep::Descriptions => {
                    match app.create_product.validate_step() {
                        Ok(()) => {
                            app.create_product.step = CreateProductStep::Support;
                            app.create_product.focused_field = CreateProductField::SupportDescription;
                            app.create_product.validation_error = None;
                        }
                        Err(e) => {
                            app.create_product.validation_error = Some(e);
                        }
                    }
                }
                CreateProductStep::Support => {
                    app.create_product.step = CreateProductStep::Review;
                    app.create_product.validation_error = None;
                }
                CreateProductStep::Review => {
                    app.pending_create_product = true;
                }
            }
        }
        KeyCode::BackTab => {
            // Go back a step
            match app.create_product.step.clone() {
                CreateProductStep::Descriptions => {
                    app.create_product.step = CreateProductStep::BasicInfo;
                    app.create_product.focused_field = CreateProductField::Title;
                }
                CreateProductStep::Support => {
                    app.create_product.step = CreateProductStep::Descriptions;
                    app.create_product.focused_field = CreateProductField::LongDescription;
                }
                CreateProductStep::Review => {
                    app.create_product.step = CreateProductStep::Support;
                    app.create_product.focused_field = CreateProductField::SupportDescription;
                }
                _ => {}
            }
        }
        _ => {}
    }
    false
}

// ─── Create Version Wizard ────────────────────────────────────────────────────

fn handle_create_version(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.state = AppState::ProductDetail;
            return false;
        }
        KeyCode::Tab => {
            app.create_version.advance_field();
        }
        KeyCode::Char(c) => {
            app.create_version.active_field_mut().push(c);
        }
        KeyCode::Backspace => {
            app.create_version.active_field_mut().pop();
        }
        KeyCode::Enter => {
            match app.create_version.validate_step() {
                Err(e) => {
                    app.create_version.validation_error = Some(e);
                }
                Ok(()) => {
                    app.create_version.validation_error = None;
                    match app.create_version.step.clone() {
                        VersionInfo => {
                            app.create_version.step = Instructions;
                            app.create_version.focused_field = CreateVersionField::UsageInstructions;
                        }
                        Instructions => {
                            app.create_version.step = IOProperties;
                            app.create_version.focused_field = CreateVersionField::InputDescription;
                        }
                        IOProperties => {
                            app.create_version.step = VersionReview;
                        }
                        VersionReview => {
                            app.pending_create_version = true;
                        }
                    }
                }
            }
        }
        KeyCode::BackTab => {
            app.create_version.validation_error = None;
            match app.create_version.step.clone() {
                Instructions => {
                    app.create_version.step = VersionInfo;
                    app.create_version.focused_field = CreateVersionField::Title;
                }
                IOProperties => {
                    app.create_version.step = Instructions;
                    app.create_version.focused_field = CreateVersionField::UsageInstructions;
                }
                VersionReview => {
                    app.create_version.step = IOProperties;
                    app.create_version.focused_field = CreateVersionField::InputDescription;
                }
                _ => {}
            }
        }
        _ => {}
    }
    false
}

// ─── Whitelist Manager ────────────────────────────────────────────────────────

fn handle_whitelist(app: &mut App, key: KeyEvent) -> bool {
    if app.whitelist_editing {
        match key.code {
            KeyCode::Esc => {
                app.whitelist_input.clear();
                app.whitelist_input_valid = None;
                app.whitelist_editing = false;
            }
            KeyCode::Enter => {
                let valid = app.whitelist_input_valid.unwrap_or(false);
                if valid {
                    let account_id = app.whitelist_input.clone();
                    if let Some(detail) = &mut app.current_product_detail {
                        if !detail.allowed_accounts.contains(&account_id) {
                            detail.allowed_accounts.push(account_id.clone());
                            let accounts = detail.allowed_accounts.clone();
                            app.pending_save_whitelist = Some(accounts);
                        }
                    }
                    app.whitelist_input.clear();
                    app.whitelist_editing = false;
                    app.whitelist_input_valid = None;
                } else {
                    app.set_status("Account ID must be exactly 12 digits".to_string(), true);
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if app.whitelist_input.len() < 12 {
                    app.whitelist_input.push(c);
                    app.validate_whitelist_input();
                }
            }
            KeyCode::Backspace => {
                app.whitelist_input.pop();
                app.validate_whitelist_input();
            }
            _ => {}
        }
        return false;
    }

    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Esc => {
            app.state = AppState::ProductDetail;
        }
        KeyCode::Char('?') => app.open_help(),
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(detail) = &app.current_product_detail {
                let n = detail.allowed_accounts.len();
                if n > 0 {
                    let cur = app.whitelist_selected.unwrap_or(0);
                    let next = if cur == 0 { n - 1 } else { cur - 1 };
                    app.whitelist_selected = Some(next);
                    app.whitelist_list_state.select(Some(next));
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(detail) = &app.current_product_detail {
                let n = detail.allowed_accounts.len();
                if n > 0 {
                    let cur = app.whitelist_selected.unwrap_or(0);
                    let next = if cur >= n - 1 { 0 } else { cur + 1 };
                    app.whitelist_selected = Some(next);
                    app.whitelist_list_state.select(Some(next));
                }
            }
        }
        KeyCode::Char('a') | KeyCode::Char('n') => {
            app.whitelist_editing = true;
            app.whitelist_input.clear();
            app.whitelist_input_valid = None;
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if let (Some(idx), Some(detail)) =
                (app.whitelist_selected, &mut app.current_product_detail)
            {
                if idx < detail.allowed_accounts.len() {
                    detail.allowed_accounts.remove(idx);
                    let accounts = detail.allowed_accounts.clone();
                    app.pending_save_whitelist = Some(accounts);
                    let new_n = detail.allowed_accounts.len();
                    if new_n == 0 {
                        app.whitelist_selected = None;
                        app.whitelist_list_state.select(None);
                    } else {
                        let new_idx = idx.min(new_n - 1);
                        app.whitelist_selected = Some(new_idx);
                        app.whitelist_list_state.select(Some(new_idx));
                    }
                }
            }
        }
        _ => {}
    }
    false
}

// ─── Pricing Editor ───────────────────────────────────────────────────────────

fn handle_pricing(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        // No `q` or `?` shortcuts here — all typed characters belong to the
        // active field. Use Esc to leave the pricing editor.
        KeyCode::Esc => {
            app.state = AppState::ProductDetail;
        }
        KeyCode::Tab => {
            let next = app.pricing_focused_field.next();
            app.pricing_focused_field = next;
        }
        KeyCode::Enter => {
            app.validate_pricing_input();
            if app.pricing_input_valid.unwrap_or(false) {
                app.pending_save_pricing = true;
            } else {
                app.set_status("Invalid price value — must be a non-negative number".to_string(), true);
            }
        }
        KeyCode::Char(c) => {
            match app.pricing_focused_field {
                PricingField::Price => {
                    if c.is_ascii_digit() || c == '.' {
                        app.pricing_price.push(c);
                        app.validate_pricing_input();
                    }
                }
                PricingField::Currency => {
                    if c.is_ascii_uppercase() || c.is_ascii_lowercase() {
                        if app.pricing_currency.len() < 3 {
                            app.pricing_currency.push(c.to_ascii_uppercase());
                        }
                    }
                }
                PricingField::Dimension => {
                    app.pricing_dimension.push(c);
                }
            }
        }
        KeyCode::Backspace => match app.pricing_focused_field {
            PricingField::Price => {
                app.pricing_price.pop();
                app.validate_pricing_input();
            }
            PricingField::Currency => {
                app.pricing_currency.pop();
            }
            PricingField::Dimension => {
                app.pricing_dimension.pop();
            }
        },
        _ => {}
    }
    false
}

// ─── Metadata Editor ──────────────────────────────────────────────────────────

fn handle_metadata_editor(app: &mut App, key: KeyEvent) -> bool {
    let editing = app.metadata_editor.as_ref().map(|e| e.editing).unwrap_or(false);

    if editing {
        match key.code {
            KeyCode::Esc => {
                if let Some(editor) = &mut app.metadata_editor {
                    editor.editing = false;
                    editor.input_buffer.clear();
                }
            }
            KeyCode::Enter => {
                if let Some(editor) = &mut app.metadata_editor {
                    let idx = editor.selected_field;
                    if idx < editor.fields.len() {
                        editor.fields[idx].1 = editor.input_buffer.clone();
                    }
                    editor.editing = false;
                    editor.input_buffer.clear();
                    editor.validation_error = None;
                }
            }
            KeyCode::Char(c) => {
                if let Some(editor) = &mut app.metadata_editor {
                    editor.input_buffer.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some(editor) = &mut app.metadata_editor {
                    editor.input_buffer.pop();
                }
            }
            _ => {}
        }
        return false;
    }

    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Esc => {
            app.metadata_editor = None;
            app.state = AppState::ProductDetail;
        }
        KeyCode::Char('?') => app.open_help(),
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(editor) = &mut app.metadata_editor {
                let n = editor.fields.len();
                if n > 0 {
                    editor.selected_field = if editor.selected_field == 0 {
                        n - 1
                    } else {
                        editor.selected_field - 1
                    };
                    editor.fields_state.select(Some(editor.selected_field));
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(editor) = &mut app.metadata_editor {
                let n = editor.fields.len();
                if n > 0 {
                    editor.selected_field = (editor.selected_field + 1) % n;
                    editor.fields_state.select(Some(editor.selected_field));
                }
            }
        }
        KeyCode::Enter => {
            if let Some(editor) = &mut app.metadata_editor {
                let idx = editor.selected_field;
                if idx < editor.fields.len() {
                    editor.input_buffer = editor.fields[idx].1.clone();
                    editor.editing = true;
                }
            }
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.pending_save_metadata = true;
        }
        _ => {}
    }
    false
}

// ─── Version Metadata Editor ─────────────────────────────────────────────────

fn handle_version_metadata_editor(app: &mut App, key: KeyEvent) -> bool {
    let editing = app
        .version_metadata_editor
        .as_ref()
        .map(|e| e.editing)
        .unwrap_or(false);

    if editing {
        match key.code {
            KeyCode::Esc => {
                if let Some(editor) = &mut app.version_metadata_editor {
                    editor.editing = false;
                    editor.input_buffer.clear();
                }
            }
            KeyCode::Enter => {
                if let Some(editor) = &mut app.version_metadata_editor {
                    let idx = editor.selected_field;
                    if idx < editor.fields.len() {
                        editor.fields[idx].1 = editor.input_buffer.clone();
                    }
                    editor.editing = false;
                    editor.input_buffer.clear();
                    editor.validation_error = None;
                }
            }
            KeyCode::Char(c) => {
                if let Some(editor) = &mut app.version_metadata_editor {
                    editor.input_buffer.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some(editor) = &mut app.version_metadata_editor {
                    editor.input_buffer.pop();
                }
            }
            _ => {}
        }
        return false;
    }

    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Esc => {
            app.version_metadata_editor = None;
            app.state = AppState::VersionList;
        }
        KeyCode::Char('?') => app.open_help(),
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(editor) = &mut app.version_metadata_editor {
                let n = editor.fields.len();
                if n > 0 {
                    editor.selected_field =
                        if editor.selected_field == 0 { n - 1 } else { editor.selected_field - 1 };
                    editor.fields_state.select(Some(editor.selected_field));
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(editor) = &mut app.version_metadata_editor {
                let n = editor.fields.len();
                if n > 0 {
                    editor.selected_field = (editor.selected_field + 1) % n;
                    editor.fields_state.select(Some(editor.selected_field));
                }
            }
        }
        KeyCode::Enter => {
            if let Some(editor) = &mut app.version_metadata_editor {
                let idx = editor.selected_field;
                if idx < editor.fields.len() {
                    editor.input_buffer = editor.fields[idx].1.clone();
                    editor.editing = true;
                }
            }
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Validate before saving
            if let Some(editor) = &app.version_metadata_editor {
                let title = editor
                    .fields
                    .iter()
                    .find(|(f, _)| *f == VersionMetadataField::Title)
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("");
                if title.trim().is_empty() {
                    if let Some(editor) = &mut app.version_metadata_editor {
                        editor.validation_error = Some("Version title is required".to_string());
                    }
                } else {
                    app.pending_save_version_metadata = true;
                }
            }
        }
        _ => {}
    }
    false
}

// ─── Restrict Version Confirm ────────────────────────────────────────────────

fn handle_restrict_confirm(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(ver_id) = app.selected_version().map(|v| v.id.clone()) {
                app.pending_restrict_version_id = Some(ver_id);
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.state = app.previous_state.clone();
        }
        _ => {}
    }
    false
}

// ─── Changeset Status View ───────────────────────────────────────────────────

fn handle_changeset_view(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Esc => {
            app.close_overlay();
        }
        KeyCode::Char('R') => {
            app.pending_poll_changesets = true;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let n = app.pending_changesets.len();
            if n > 0 {
                let cur = app.changesets_list_state.selected().unwrap_or(0);
                let next = if cur == 0 { n - 1 } else { cur - 1 };
                app.changesets_list_state.select(Some(next));
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let n = app.pending_changesets.len();
            if n > 0 {
                let cur = app.changesets_list_state.selected().unwrap_or(0);
                let next = if cur >= n - 1 { 0 } else { cur + 1 };
                app.changesets_list_state.select(Some(next));
            }
        }
        _ => {}
    }
    false
}

// ─── Command Palette ──────────────────────────────────────────────────────────

/// Returns true if the application should quit.
pub fn handle_command_palette(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.command_palette_input.clear();
            app.state = if app.previous_state != AppState::CommandPalette {
                app.previous_state.clone()
            } else {
                AppState::ProductList
            };
        }
        KeyCode::Enter => {
            if let Some(sel) = app.command_palette_state.selected() {
                if let Some(&item_idx) = app.filtered_commands.get(sel) {
                    let action = app.command_palette_items[item_idx].action.clone();
                    app.command_palette_input.clear();
                    app.state = if app.previous_state != AppState::CommandPalette {
                        app.previous_state.clone()
                    } else {
                        AppState::ProductList
                    };
                    return execute_command_action(app, action);
                }
            }
        }
        KeyCode::Up => {
            let n = app.filtered_commands.len();
            if n > 0 {
                let cur = app.command_palette_state.selected().unwrap_or(0);
                let next = if cur == 0 { n - 1 } else { cur - 1 };
                app.command_palette_state.select(Some(next));
            }
        }
        KeyCode::Down => {
            let n = app.filtered_commands.len();
            if n > 0 {
                let cur = app.command_palette_state.selected().unwrap_or(0);
                let next = if cur >= n - 1 { 0 } else { cur + 1 };
                app.command_palette_state.select(Some(next));
            }
        }
        KeyCode::Char(c) => {
            app.command_palette_input.push(c);
            app.apply_command_filter();
        }
        KeyCode::Backspace => {
            app.command_palette_input.pop();
            app.apply_command_filter();
        }
        _ => {}
    }
    false
}

fn execute_command_action(app: &mut App, action: CommandAction) -> bool {
    match action {
        CommandAction::CreateProduct => {
            app.create_product = crate::app::CreateProductWizardState::new();
            app.state = AppState::CreateProductWizard;
        }
        CommandAction::CreateVersion => {
            if let Some(entity_id) =
                app.current_product_detail.as_ref().map(|d| d.entity_id.clone())
            {
                app.create_version = crate::app::CreateVersionWizardState::new(entity_id);
                app.state = AppState::CreateVersionWizard;
            }
        }
        CommandAction::Refresh => {
            app.pending_refresh = true;
        }
        CommandAction::EditMetadata => {
            if app.current_product_detail.is_some() {
                app.open_metadata_editor();
            }
        }
        CommandAction::ManageWhitelist => {
            if app.current_product_detail.is_some() {
                app.open_whitelist_manager();
            }
        }
        CommandAction::EditPricing => {
            if app.current_product_detail.is_some() {
                app.open_pricing_editor();
            }
        }
        CommandAction::RestrictVersion => {
            if app.selected_version().is_some() {
                app.previous_state = app.state.clone();
                app.state = AppState::RestrictVersionConfirm;
            }
        }
        CommandAction::ViewChangeSets => {
            app.open_changeset_view();
        }
        CommandAction::ShowHelp => {
            app.open_help();
        }
        CommandAction::ShowLogs => {
            app.open_logs();
        }
        CommandAction::CopyAllProductIds => {
            if !app.products.is_empty() {
                let text = app
                    .products
                    .iter()
                    .map(|p| format!("{}\t{}", p.entity_id, p.visibility))
                    .collect::<Vec<_>>()
                    .join("\n");
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(text);
                    app.set_status(
                        format!("Copied {} product IDs to clipboard", app.products.len()),
                        false,
                    );
                }
            } else {
                app.set_status("No products loaded".to_string(), true);
            }
        }
        CommandAction::CopyEntityId => {
            let id = app
                .current_product_detail
                .as_ref()
                .map(|d| d.entity_id.clone())
                .or_else(|| app.selected_product().map(|p| p.entity_id.clone()));
            if let Some(id) = id {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(id.clone());
                    app.set_status(format!("Copied entity ID: {}", id), false);
                }
            }
        }
        CommandAction::Quit => return true,
    }
    false
}

// ─── Logs Overlay ─────────────────────────────────────────────────────────────

fn handle_logs(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('l') => {
            app.close_overlay();
        }
        KeyCode::Char('q') => return true,
        KeyCode::Up | KeyCode::Char('k') => {
            app.log_scroll = app.log_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.log_scroll = app.log_scroll.saturating_add(1);
        }
        KeyCode::PageUp => {
            app.log_scroll = app.log_scroll.saturating_sub(20);
        }
        KeyCode::PageDown => {
            app.log_scroll = app.log_scroll.saturating_add(20);
        }
        KeyCode::Char('G') => {
            // Jump to end
            let count = app.log.lock().map(|l| l.len()).unwrap_or(0) as u16;
            app.log_scroll = count.saturating_sub(1);
        }
        _ => {}
    }
    false
}
