mod app;
mod aws;
mod keyboard;
mod models;
mod ui;

use anyhow::Result;
use app::{App, AppLog, AppState};
use aws::AwsManager;
use chrono::Utc;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use models::{ChangesetStatus, LogLevel, PendingChangeset};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write as IoWrite},
    sync::{Arc, Mutex},
    time::Duration,
};

// Poll changeset status every 5 seconds (50 ticks × 100ms)
const CHANGESET_POLL_INTERVAL: u64 = 50;

/// Log the full error cause chain to the app log and show a brief status-bar
/// message directing the user to the log viewer for full details.
///
/// `{:#}` on an `anyhow::Error` prints every cause in the chain separated by
/// ": ", e.g. "update failed: ServiceError { code: … message: … }".
fn log_error(app: &mut App, context: &str, e: &anyhow::Error) {
    let full = format!("{}: {:#}", context, e);
    app.log(LogLevel::Error, full);
    app.set_status(format!("{}: {} — press [l] for details", context, e), true);
}

/// Write `content` to the debug file if one is configured.
/// Appends rather than overwrites so multiple calls accumulate.
fn write_debug(debug_file: &Option<String>, label: &str, content: &str) {
    if let Some(path) = debug_file {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
            let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
            let _ = writeln!(f, "\n=== {} @ {} ===\n{}", label, timestamp, content);
        }
    }
}

/// Parse `--debug-file <path>` from CLI args.
fn parse_debug_file() -> Option<String> {
    let args: Vec<String> = env::args().collect();
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--debug-file" {
            return iter.next().cloned();
        }
        if let Some(path) = arg.strip_prefix("--debug-file=") {
            return Some(path.to_string());
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<()> {
    let debug_file = parse_debug_file();

    let log: AppLog = Arc::new(Mutex::new(Vec::new()));
    let mut app = App::new(log.clone());

    // Initialize AWS manager (Marketplace Catalog is global/us-east-1)
    let mut aws = match AwsManager::new(&app.current_region).await {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to initialize AWS client: {}", e);
            return Err(e);
        }
    };

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    app.log(LogLevel::Info, format!("Starting Marketplace TUI (region: {})", app.current_region));
    if let Some(ref path) = debug_file {
        app.log(LogLevel::Info, format!("Debug output → {}", path));
    }

    // Initial product load
    load_products(&mut app, &aws).await;

    // Run main loop
    let result = run_app(&mut terminal, &mut app, &mut aws, &debug_file).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(ref e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    aws: &mut AwsManager,
    debug_file: &Option<String>,
) -> Result<()> {
    loop {
        app.tick();
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    let should_quit = keyboard::handle_key_event(app, key);
                    if should_quit {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse_event) => {
                    handle_mouse_event(app, mouse_event);
                }
                Event::Resize(_, _) => {
                    // Ratatui handles resize automatically on next draw
                }
                _ => {}
            }
        }

        // ─── Process pending async operations ────────────────────────────

        if app.pending_refresh {
            app.pending_refresh = false;
            app.log(LogLevel::Info, "Refreshing products…".to_string());
            load_products(app, aws).await;
        }

        if let Some(entity_id) = app.pending_load_detail.take() {
            load_product_detail(app, aws, &entity_id, debug_file).await;
        }

        if app.pending_create_product {
            app.pending_create_product = false;
            submit_create_product(app, aws).await;
        }

        if app.pending_create_version {
            app.pending_create_version = false;
            submit_create_version(app, aws).await;
        }

        if app.pending_save_metadata {
            app.pending_save_metadata = false;
            submit_save_metadata(app, aws).await;
        }

        if app.pending_save_version_metadata {
            app.pending_save_version_metadata = false;
            submit_save_version_metadata(app, aws).await;
        }

        if let Some(accounts) = app.pending_save_whitelist.take() {
            submit_save_whitelist(app, aws, accounts).await;
        }

        if app.pending_save_pricing {
            app.pending_save_pricing = false;
            submit_save_pricing(app, aws).await;
        }

        if let Some(version_id) = app.pending_restrict_version_id.take() {
            submit_restrict_version(app, aws, &version_id).await;
        }

        if app.pending_poll_changesets {
            app.pending_poll_changesets = false;
            poll_changesets(app, aws).await;
        }

        // ─── Periodic changeset polling ───────────────────────────────────

        if app.ticker % CHANGESET_POLL_INTERVAL == 0 && app.has_active_changesets() {
            poll_changesets(app, aws).await;
        }
    }
}

// ─── Mouse Event Handler ─────────────────────────────────────────────────────

fn handle_mouse_event(app: &mut App, event: crossterm::event::MouseEvent) {
    use crossterm::event::MouseEventKind;
    match event.kind {
        MouseEventKind::ScrollUp => match app.state {
            AppState::ProductList => app.product_list_up(),
            AppState::VersionList => app.version_list_up(),
            AppState::Logs => {
                app.log_scroll = app.log_scroll.saturating_sub(3);
            }
            AppState::Help => {
                app.help_scroll = app.help_scroll.saturating_sub(3);
            }
            _ => {}
        },
        MouseEventKind::ScrollDown => match app.state {
            AppState::ProductList => app.product_list_down(),
            AppState::VersionList => app.version_list_down(),
            AppState::Logs => {
                app.log_scroll = app.log_scroll.saturating_add(3);
            }
            AppState::Help => {
                app.help_scroll = app.help_scroll.saturating_add(3);
            }
            _ => {}
        },
        _ => {}
    }
}

// ─── Load Products ────────────────────────────────────────────────────────────

async fn load_products(app: &mut App, aws: &AwsManager) {
    app.is_loading = true;
    app.set_status("Loading products…".to_string(), false);

    match aws.list_products().await {
        Ok(products) => {
            let count = products.len();
            app.products = products;
            app.apply_product_filter();
            app.log(LogLevel::Info, format!("Loaded {} products", count));
            app.set_status(format!("Loaded {} products", count), false);
        }
        Err(e) => {
            log_error(app, "Failed to load products", &e);
        }
    }

    app.is_loading = false;
}

// ─── Load Product Detail (lazy) ───────────────────────────────────────────────

async fn load_product_detail(
    app: &mut App,
    aws: &AwsManager,
    entity_id: &str,
    debug_file: &Option<String>,
) {
    app.is_loading = true;
    app.log(LogLevel::Info, format!("Loading detail for {}…", entity_id));

    match aws.describe_product(entity_id).await {
        Ok(detail) => {
            app.log(LogLevel::Info, format!("Loaded detail for '{}'", detail.title));
            // Write raw JSON to the debug file (if configured with --debug-file).
            // The raw JSON is useful for verifying field paths when values appear
            // missing, without bloating the in-app log.
            write_debug(
                debug_file,
                &format!("DescribeEntity {}", entity_id),
                &detail.raw_details,
            );
            app.set_status(format!("Loaded: {}", detail.title), false);
            app.current_product_detail = Some(detail);
        }
        Err(e) => {
            log_error(app, "Failed to load product detail", &e);
        }
    }

    app.is_loading = false;
}

// ─── Create Product ───────────────────────────────────────────────────────────

async fn submit_create_product(app: &mut App, aws: &AwsManager) {
    let config = app.create_product.to_config();
    let name = config.title.clone();

    app.is_loading = true;
    app.log(LogLevel::Info, format!("Creating product '{}'…", name));
    app.set_status(format!("Submitting create product: {}…", name), false);

    match aws.create_product(&config).await {
        Ok(cs_id) => {
            let msg = format!("Create product '{}' submitted (CS: {})", name, &cs_id[..8.min(cs_id.len())]);
            app.log(LogLevel::Info, msg.clone());
            app.set_status(msg, false);

            app.pending_changesets.push(PendingChangeset {
                change_set_id: cs_id,
                change_set_name: format!("Create: {}", name),
                operation: "CreateProduct".to_string(),
                product_entity_id: String::new(),
                submitted_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                status: ChangesetStatus::Preparing,
                failure_description: None,
            });

            app.state = AppState::ProductList;
            // Refresh after a short while — user can press R
            app.pending_refresh = true;
        }
        Err(e) => {
            app.create_product.validation_error = Some(e.to_string());
            log_error(app, "Failed to create product", &e);
        }
    }

    app.is_loading = false;
}

// ─── Create Version ───────────────────────────────────────────────────────────

async fn submit_create_version(app: &mut App, aws: &AwsManager) {
    let config = app.create_version.to_config();
    let entity_id = config.product_entity_id.clone();
    let title = config.title.clone();

    app.is_loading = true;
    app.log(LogLevel::Info, format!("Adding version '{}' to {}…", title, entity_id));

    match aws.add_version(&config).await {
        Ok(cs_id) => {
            let msg = format!("Version '{}' submitted (CS: {})", title, &cs_id[..8.min(cs_id.len())]);
            app.log(LogLevel::Info, msg.clone());
            app.set_status(msg, false);

            app.pending_changesets.push(PendingChangeset {
                change_set_id: cs_id,
                change_set_name: format!("Add version: {}", title),
                operation: "AddDeliveryOptions".to_string(),
                product_entity_id: entity_id.clone(),
                submitted_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                status: ChangesetStatus::Preparing,
                failure_description: None,
            });

            app.state = AppState::ProductDetail;
            app.pending_load_detail = Some(entity_id);
        }
        Err(e) => {
            app.create_version.validation_error = Some(e.to_string());
            log_error(app, "Failed to add version", &e);
        }
    }

    app.is_loading = false;
}

// ─── Save Metadata ────────────────────────────────────────────────────────────

async fn submit_save_metadata(app: &mut App, aws: &AwsManager) {
    let (entity_id, fields_map) = match &app.metadata_editor {
        Some(editor) => {
            let entity_id = editor.product_entity_id.clone();
            let map: std::collections::HashMap<String, String> = editor
                .fields
                .iter()
                .map(|(field, value)| (field.label().to_string(), value.clone()))
                .collect();
            (entity_id, map)
        }
        None => return,
    };

    app.is_loading = true;
    app.log(LogLevel::Info, format!("Saving metadata for {}…", entity_id));

    match aws.update_product_info(&entity_id, &fields_map).await {
        Ok(cs_id) => {
            let msg = format!("Metadata update submitted (CS: {})", &cs_id[..8.min(cs_id.len())]);
            app.log(LogLevel::Info, msg.clone());
            app.set_status(msg, false);

            app.pending_changesets.push(PendingChangeset {
                change_set_id: cs_id,
                change_set_name: "Update metadata".to_string(),
                operation: "UpdateInformation".to_string(),
                product_entity_id: entity_id.clone(),
                submitted_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                status: ChangesetStatus::Preparing,
                failure_description: None,
            });

            app.metadata_editor = None;
            app.state = AppState::ProductDetail;
            app.pending_load_detail = Some(entity_id);
        }
        Err(e) => {
            log_error(app, "Failed to save metadata", &e);
            if let Some(editor) = &mut app.metadata_editor {
                editor.validation_error = Some(e.to_string());
            }
        }
    }

    app.is_loading = false;
}

// ─── Save Version Metadata ────────────────────────────────────────────────────

async fn submit_save_version_metadata(app: &mut App, aws: &AwsManager) {
    let (entity_id, delivery_option_id, title, release_notes) =
        match &app.version_metadata_editor {
            Some(editor) => {
                let title = editor
                    .fields
                    .iter()
                    .find(|(f, _)| *f == crate::app::VersionMetadataField::Title)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                let notes = editor
                    .fields
                    .iter()
                    .find(|(f, _)| *f == crate::app::VersionMetadataField::ReleaseNotes)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                (
                    editor.product_entity_id.clone(),
                    editor.delivery_option_id.clone(),
                    title,
                    notes,
                )
            }
            None => return,
        };

    app.is_loading = true;
    app.log(
        LogLevel::Info,
        format!("Updating version metadata for {}…", delivery_option_id),
    );

    match aws
        .update_version_metadata(&entity_id, &delivery_option_id, &title, &release_notes)
        .await
    {
        Ok(cs_id) => {
            let msg = format!(
                "Version update submitted (CS: {})",
                &cs_id[..8.min(cs_id.len())]
            );
            app.log(LogLevel::Info, msg.clone());
            app.set_status(msg, false);

            app.pending_changesets.push(PendingChangeset {
                change_set_id: cs_id,
                change_set_name: format!("Update version: {}", &delivery_option_id[..8.min(delivery_option_id.len())]),
                operation: "UpdateDeliveryOptions".to_string(),
                product_entity_id: entity_id.clone(),
                submitted_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                status: ChangesetStatus::Preparing,
                failure_description: None,
            });

            app.version_metadata_editor = None;
            app.state = AppState::VersionList;
            app.pending_load_detail = Some(entity_id);
        }
        Err(e) => {
            log_error(app, "Failed to update version", &e);
            if let Some(editor) = &mut app.version_metadata_editor {
                editor.validation_error = Some(e.to_string());
            }
        }
    }

    app.is_loading = false;
}

// ─── Save Whitelist ───────────────────────────────────────────────────────────

async fn submit_save_whitelist(app: &mut App, aws: &AwsManager, accounts: Vec<String>) {
    let entity_id = match app.current_product_detail.as_ref().map(|d| d.entity_id.clone()) {
        Some(id) => id,
        None => return,
    };

    app.is_loading = true;
    app.log(
        LogLevel::Info,
        format!("Updating allowlist for {} ({} accounts)…", entity_id, accounts.len()),
    );

    match aws.update_allowed_accounts(&entity_id, &accounts).await {
        Ok(cs_id) => {
            let msg = format!("Allowlist update submitted (CS: {})", &cs_id[..8.min(cs_id.len())]);
            app.log(LogLevel::Info, msg.clone());
            app.set_status(msg, false);

            app.pending_changesets.push(PendingChangeset {
                change_set_id: cs_id,
                change_set_name: "Update allowlist".to_string(),
                operation: "UpdateTargeting".to_string(),
                product_entity_id: entity_id,
                submitted_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                status: ChangesetStatus::Preparing,
                failure_description: None,
            });
        }
        Err(e) => {
            log_error(app, "Failed to update allowlist", &e);
        }
    }

    app.is_loading = false;
}

// ─── Save Pricing ─────────────────────────────────────────────────────────────

async fn submit_save_pricing(app: &mut App, aws: &AwsManager) {
    let entity_id = match app.current_product_detail.as_ref().map(|d| d.entity_id.clone()) {
        Some(id) => id,
        None => return,
    };

    let price = app.pricing_price.clone();
    let currency = app.pricing_currency.clone();
    let dimension = app.pricing_dimension.clone();

    app.is_loading = true;
    app.log(
        LogLevel::Info,
        format!("Updating pricing for {}: ${} {}/{}…", entity_id, price, currency, dimension),
    );

    match aws.update_pricing(&entity_id, &price, &currency, &dimension).await {
        Ok(cs_id) => {
            let msg = format!("Pricing update submitted (CS: {})", &cs_id[..8.min(cs_id.len())]);
            app.log(LogLevel::Info, msg.clone());
            app.set_status(msg, false);

            app.pending_changesets.push(PendingChangeset {
                change_set_id: cs_id,
                change_set_name: "Update pricing".to_string(),
                operation: "UpdatePricingTerms".to_string(),
                product_entity_id: entity_id.clone(),
                submitted_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                status: ChangesetStatus::Preparing,
                failure_description: None,
            });

            app.state = AppState::ProductDetail;
            app.pending_load_detail = Some(entity_id);
        }
        Err(e) => {
            log_error(app, "Failed to update pricing", &e);
        }
    }

    app.is_loading = false;
}

// ─── Restrict Version ─────────────────────────────────────────────────────────

async fn submit_restrict_version(app: &mut App, aws: &AwsManager, version_id: &str) {
    let entity_id = match app.current_product_detail.as_ref().map(|d| d.entity_id.clone()) {
        Some(id) => id,
        None => return,
    };

    app.is_loading = true;
    app.log(
        LogLevel::Info,
        format!("Restricting version {} on {}…", version_id, entity_id),
    );

    match aws.restrict_version(&entity_id, version_id).await {
        Ok(cs_id) => {
            let msg = format!("Version restriction submitted (CS: {})", &cs_id[..8.min(cs_id.len())]);
            app.log(LogLevel::Info, msg.clone());
            app.set_status(msg, false);

            app.pending_changesets.push(PendingChangeset {
                change_set_id: cs_id,
                change_set_name: format!("Restrict version: {}", &version_id[..8.min(version_id.len())]),
                operation: "RestrictDeliveryOptions".to_string(),
                product_entity_id: entity_id.clone(),
                submitted_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                status: ChangesetStatus::Preparing,
                failure_description: None,
            });

            app.state = AppState::VersionList;
            app.pending_load_detail = Some(entity_id);
        }
        Err(e) => {
            log_error(app, "Failed to restrict version", &e);
            app.state = AppState::VersionList;
        }
    }

    app.is_loading = false;
}

// ─── Poll Changeset Statuses ─────────────────────────────────────────────────

async fn poll_changesets(app: &mut App, aws: &AwsManager) {
    let ids: Vec<(usize, String)> = app
        .pending_changesets
        .iter()
        .enumerate()
        .filter(|(_, cs)| !cs.status.is_terminal())
        .map(|(i, cs)| (i, cs.change_set_id.clone()))
        .collect();

    for (idx, cs_id) in ids {
        match aws.describe_changeset(&cs_id).await {
            Ok((status, failure)) => {
                let old_status = app.pending_changesets[idx].status.clone();
                app.pending_changesets[idx].status = status.clone();
                app.pending_changesets[idx].failure_description = failure;

                // Log status changes
                if old_status != status {
                    let msg = format!(
                        "Change set '{}': {} → {}",
                        app.pending_changesets[idx].change_set_name,
                        old_status,
                        status
                    );
                    let level = if status == ChangesetStatus::Failed {
                        LogLevel::Error
                    } else if status == ChangesetStatus::Succeeded {
                        LogLevel::Info
                    } else {
                        LogLevel::Info
                    };
                    app.log(level, msg.clone());

                    if status.is_terminal() {
                        app.set_status(msg, status == ChangesetStatus::Failed);
                    }
                }
            }
            Err(e) => {
                app.log(
                    LogLevel::Warning,
                    format!("Failed to poll changeset {}: {}", &cs_id[..8.min(cs_id.len())], e),
                );
            }
        }
    }
}
