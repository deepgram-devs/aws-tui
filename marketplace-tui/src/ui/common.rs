use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub const BRAND_ORANGE: Color = Color::Rgb(255, 153, 0); // AWS orange
pub const BRAND_DARK: Color = Color::Rgb(35, 47, 62); // AWS dark navy
pub const SELECTED_BG: Color = Color::Rgb(50, 70, 90);
pub const SUBTLE: Color = Color::Rgb(120, 130, 140);
pub const ERROR_RED: Color = Color::Red;
pub const SUCCESS_GREEN: Color = Color::Green;
pub const WARNING_YELLOW: Color = Color::Yellow;

/// Split the full frame into header, content, and status bar regions.
pub fn base_layout(f: &Frame) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // content
            Constraint::Length(2), // status bar
        ])
        .split(f.area());

    (chunks[0], chunks[1], chunks[2])
}

/// Render the application header bar.
pub fn draw_header(f: &mut Frame, area: Rect, app: &App, subtitle: &str) {
    let title_style = Style::default()
        .fg(BRAND_ORANGE)
        .add_modifier(Modifier::BOLD);

    let subtitle_style = Style::default().fg(Color::White);

    let spinner = if app.is_loading || app.has_active_changesets() {
        format!(" {} ", app.spinner_char())
    } else {
        String::new()
    };

    let cs_indicator = if app.has_active_changesets() {
        Span::styled(
            format!("{}[CS pending] ", spinner),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )
    } else if app.is_loading {
        Span::styled(
            spinner,
            Style::default().fg(BRAND_ORANGE),
        )
    } else {
        Span::raw("")
    };

    let title = Line::from(vec![
        Span::styled(" AWS Marketplace TUI", title_style),
        Span::raw("  "),
        Span::styled(subtitle, subtitle_style),
        Span::raw("  "),
        cs_indicator,
    ]);

    let region_line = Line::from(vec![
        Span::raw(" Region: "),
        Span::styled(
            &app.current_region,
            Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  [:] Command Palette"),
    ])
    .alignment(Alignment::Right);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BRAND_DARK))
        .style(Style::default().bg(BRAND_DARK));

    let inner = header_block.inner(area);
    f.render_widget(header_block, area);

    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(inner);

    f.render_widget(Paragraph::new(title), header_layout[0]);
    f.render_widget(Paragraph::new(region_line), header_layout[1]);
}

/// Render the status bar at the bottom.
pub fn draw_status_bar(f: &mut Frame, area: Rect, app: &App, hint: &str) {
    let msg_style = if app.status_is_error {
        Style::default().fg(ERROR_RED).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let status_line = if !app.status_message.is_empty() {
        Line::from(vec![
            Span::raw(" "),
            Span::styled(&app.status_message, msg_style),
        ])
    } else {
        Line::from(vec![
            Span::raw(" "),
            Span::styled(hint, Style::default().fg(SUBTLE)),
        ])
    };

    let shortcut_line = Line::from(vec![
        Span::styled(
            " [?] Help  [l] Logs  [q] Quit",
            Style::default().fg(SUBTLE),
        ),
    ])
    .alignment(Alignment::Right);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(32)])
        .split(area);

    f.render_widget(Paragraph::new(status_line), layout[0]);
    f.render_widget(Paragraph::new(shortcut_line), layout[1]);
}

/// Center a rectangle of given width/height within a parent rect.
pub fn centered_rect(width: u16, height: u16, parent: Rect) -> Rect {
    let x = parent.x + parent.width.saturating_sub(width) / 2;
    let y = parent.y + parent.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(parent.width),
        height: height.min(parent.height),
    }
}

/// Returns the color for a visibility value.
pub fn visibility_color(vis: &crate::models::Visibility) -> Color {
    match vis.color_hint() {
        "green" => SUCCESS_GREEN,
        "yellow" => WARNING_YELLOW,
        "red" => ERROR_RED,
        "cyan" => Color::Cyan,
        _ => SUBTLE,
    }
}

/// Truncate a string to fit within max_len, appending "…" if needed.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}
