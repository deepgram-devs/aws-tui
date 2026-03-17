mod common;
mod popups;
mod views;
mod wizard;

use crate::app::{App, AppState};
use ratatui::Frame;

/// Render the non-overlay content for a given state. Used both for normal
/// rendering and as the background when an overlay (help/logs/palette) is open.
fn draw_base(f: &mut Frame, app: &App, state: &AppState) {
    match state {
        AppState::ProductList => views::draw_product_list(f, app),
        AppState::ProductDetail => views::draw_product_detail(f, app),
        AppState::VersionList => views::draw_version_list(f, app),
        AppState::CreateProductWizard => wizard::draw_create_product(f, app),
        AppState::CreateVersionWizard => wizard::draw_create_version(f, app),
        AppState::WhitelistManager => views::draw_whitelist_manager(f, app),
        AppState::PricingEditor => views::draw_pricing_editor(f, app),
        AppState::MetadataEditor => views::draw_metadata_editor(f, app),
        AppState::VersionMetadataEditor => views::draw_version_metadata_editor(f, app),
        AppState::ChangeSetStatusView => popups::draw_changeset_status(f, app),
        AppState::RestrictVersionConfirm => {
            views::draw_version_list(f, app);
            popups::draw_restrict_confirm(f, app);
        }
        // Overlays: fall back to product list as a safe default
        _ => views::draw_product_list(f, app),
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    match &app.state {
        AppState::CommandPalette | AppState::Help | AppState::Logs => {
            // Draw the view that was active when the overlay opened, then overlay on top
            draw_base(f, app, &app.previous_state.clone());
            match &app.state {
                AppState::CommandPalette => popups::draw_command_palette(f, app),
                AppState::Help => popups::draw_help(f, app),
                AppState::Logs => popups::draw_logs(f, app),
                _ => {}
            }
        }
        state => draw_base(f, app, &state.clone()),
    }
}
