mod app;
mod aws;
mod keyboard;
mod models;
mod ui;

use anyhow::Result;
use app::{App, AppLog, AppState};
use aws::AwsManager;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use models::LogLevel;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup
    let log: AppLog = Arc::new(Mutex::new(Vec::new()));
    let mut app = App::new(log.clone());

    // Initialize AWS manager
    let mut aws_manager = AwsManager::new(&app.current_region).await?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initial load
    app.log(LogLevel::Info, format!("Starting DynamoDB TUI in region {}", app.current_region));
    load_tables(&mut app, &aws_manager).await;

    // Main loop
    let result = run_app(&mut terminal, &mut app, &mut aws_manager).await;

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    aws_manager: &mut AwsManager,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let should_quit = keyboard::handle_key_event(app, key);

                if should_quit {
                    return Ok(());
                }

                // Handle state-specific actions that require async operations
                match &app.state {
                    AppState::RegularTablesView => {
                        // Check if user requested refresh
                        if app.pending_refresh {
                            app.pending_refresh = false;
                            app.log(LogLevel::Info, "Refreshing tables...".to_string());
                            load_tables(app, aws_manager).await;
                        }
                        // Check if we just switched regions
                        if aws_manager.current_region() != app.current_region {
                            app.log(LogLevel::Info, format!("Switching to region {}", app.current_region));
                            app.is_loading = true;
                            if let Err(e) = aws_manager.switch_region(&app.current_region).await {
                                app.log(LogLevel::Error, format!("Failed to switch region: {}", e));
                            } else {
                                load_tables(app, aws_manager).await;
                            }
                            app.is_loading = false;
                        }
                    }
                    AppState::TableItemsView => {
                        // Load items if we just entered this view
                        if app.current_items.is_empty() && !app.is_loading {
                            if let Some(table) = app.selected_table() {
                                let table_name = table.name.clone();
                                app.is_loading = true;
                                load_items(app, aws_manager, &table_name).await;
                                app.is_loading = false;
                            }
                        }
                    }
                    AppState::TableCreationWizard => {
                        // Check if we're on the review step and Enter was pressed
                        if app.wizard_state.step == app::WizardStep::Review {
                            // Check if previous key was Enter (simplified - we'd need better state management)
                            let config = app.wizard_state.to_table_config();
                            app.log(LogLevel::Info, format!("Creating table '{}'...", config.name));
                            app.is_loading = true;

                            match aws_manager.create_table(config).await {
                                Ok(_) => {
                                    app.log(LogLevel::Info, "Table created successfully".to_string());
                                    app.state = AppState::RegularTablesView;
                                    load_tables(app, aws_manager).await;
                                }
                                Err(e) => {
                                    app.log(LogLevel::Error, format!("Failed to create table: {}", e));
                                    app.state = AppState::RegularTablesView;
                                }
                            }
                            app.is_loading = false;
                        }
                    }
                    AppState::ItemEditor => {
                        // Check for save action
                        if app.pending_save_item {
                            app.pending_save_item = false;
                            save_item(app, aws_manager).await;
                        }
                    }
                    AppState::DeleteConfirmation(_target) => {
                        // Handle item deletion
                        if app.pending_delete_item {
                            app.pending_delete_item = false;
                            delete_item(app, aws_manager).await;
                        }
                        // Handle table deletion
                        if let Some(table_name) = app.pending_delete_table.clone() {
                            app.pending_delete_table = None;
                            delete_table(app, aws_manager, &table_name).await;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Load table details on demand
        if let Some(selected_idx) = app.selected_table_index {
            if let Some(None) = app.tables.get(selected_idx) {
                if let Some(table_name) = app.table_names.get(selected_idx).cloned() {
                    load_table_details(app, aws_manager, &table_name, selected_idx).await;
                }
            }
        }
    }
}

async fn load_tables(app: &mut App, aws_manager: &AwsManager) {
    app.log(LogLevel::Info, format!("Loading tables in {}...", app.current_region));
    app.is_loading = true;

    match aws_manager.list_table_names().await {
        Ok(names) => {
            app.log(LogLevel::Info, format!("Found {} tables", names.len()));
            app.table_names = names.clone();
            app.tables = vec![None; names.len()];

            // Select first table if available
            if !names.is_empty() {
                app.selected_table_index = Some(0);
                app.table_state.select(Some(0));
            } else {
                app.selected_table_index = None;
                app.table_state.select(None);
            }
        }
        Err(e) => {
            app.log(LogLevel::Error, format!("Failed to list tables: {}", e));
            app.table_names.clear();
            app.tables.clear();
        }
    }

    app.is_loading = false;
}

async fn load_table_details(
    app: &mut App,
    aws_manager: &AwsManager,
    table_name: &str,
    index: usize,
) {
    match aws_manager.describe_table(table_name).await {
        Ok(table) => {
            if let Some(slot) = app.tables.get_mut(index) {
                *slot = Some(table);
            }
        }
        Err(e) => {
            app.log(
                LogLevel::Warning,
                format!("Failed to load details for {}: {}", table_name, e),
            );
        }
    }
}

async fn load_items(app: &mut App, aws_manager: &AwsManager, table_name: &str) {
    app.log(LogLevel::Info, format!("Loading items from {}...", table_name));

    match aws_manager
        .scan_table(
            table_name,
            app.pagination.items_per_page as i32,
            app.pagination.last_evaluated_key.clone(),
        )
        .await
    {
        Ok((items, last_key)) => {
            app.log(LogLevel::Info, format!("Loaded {} items", items.len()));
            app.current_items = items;
            app.pagination.last_evaluated_key = last_key.clone();
            app.pagination.has_more_pages = last_key.is_some();

            if !app.current_items.is_empty() {
                app.selected_item_index = Some(0);
                app.items_table_state.select(Some(0));
            }
        }
        Err(e) => {
            app.log(LogLevel::Error, format!("Failed to load items: {}", e));
            app.current_items.clear();
        }
    }
}

async fn save_item(app: &mut App, aws_manager: &AwsManager) {
    if let Some(editor) = &app.item_editor {
        let table_name = editor.table_name.clone();
        let item: std::collections::HashMap<String, models::AttributeValue> = editor
            .attributes
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        app.log(LogLevel::Info, format!("Saving item to {}...", table_name));

        match aws_manager.put_item(&table_name, item).await {
            Ok(_) => {
                app.log(LogLevel::Info, "Item saved successfully".to_string());
                app.exit_item_editor();
                // Reload items
                load_items(app, aws_manager, &table_name).await;
            }
            Err(e) => {
                app.log(LogLevel::Error, format!("Failed to save item: {}", e));
            }
        }
    }
}

async fn delete_item(app: &mut App, aws_manager: &AwsManager) {
    if let (Some(table), Some(item)) = (app.selected_table(), app.selected_item()) {
        let table_name = table.name.clone();
        let hash_key_name = table.hash_key.name.clone();
        let range_key_name = table.range_key.as_ref().map(|k| k.name.clone());

        let mut key = std::collections::HashMap::new();
        key.insert(hash_key_name.clone(), item.hash_key_value.clone());
        if let (Some(range_name), Some(range_value)) = (range_key_name, &item.range_key_value) {
            key.insert(range_name, range_value.clone());
        }

        app.log(LogLevel::Info, format!("Deleting item from {}...", table_name));

        match aws_manager.delete_item(&table_name, key).await {
            Ok(_) => {
                app.log(LogLevel::Info, "Item deleted successfully".to_string());
                app.state = AppState::TableItemsView;
                // Reload items
                load_items(app, aws_manager, &table_name).await;
            }
            Err(e) => {
                app.log(LogLevel::Error, format!("Failed to delete item: {}", e));
                app.state = AppState::TableItemsView;
            }
        }
    }
}

async fn delete_table(app: &mut App, aws_manager: &AwsManager, table_name: &str) {
    app.log(LogLevel::Info, format!("Deleting table {}...", table_name));
    app.is_loading = true;

    match aws_manager.delete_table(table_name).await {
        Ok(_) => {
            app.log(LogLevel::Info, format!("Table {} deleted successfully", table_name));
            app.state = AppState::RegularTablesView;
            load_tables(app, aws_manager).await;
        }
        Err(e) => {
            app.log(LogLevel::Error, format!("Failed to delete table: {}", e));
            app.state = AppState::RegularTablesView;
        }
    }

    app.is_loading = false;
}
