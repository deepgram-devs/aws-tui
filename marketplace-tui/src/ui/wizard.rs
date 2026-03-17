use super::common::*;
use crate::app::{App, CreateProductField, CreateProductStep, CreateVersionField, CreateVersionStep};
use crate::app::CreateVersionStep::{VersionInfo, Instructions, IOProperties, Review as VersionReview};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
    Frame,
};

// ─── Create Product Wizard ────────────────────────────────────────────────────

pub fn draw_create_product(f: &mut Frame, app: &App) {
    let (header_area, content_area, status_area) = base_layout(f);
    draw_header(f, header_area, app, "Create ML Product");

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(content_area);

    // Progress indicator
    draw_product_wizard_progress(f, layout[0], app);

    // Step content
    match &app.create_product.step {
        CreateProductStep::BasicInfo => draw_product_step_basic(f, layout[1], app),
        CreateProductStep::Descriptions => draw_product_step_descriptions(f, layout[1], app),
        CreateProductStep::Support => draw_product_step_support(f, layout[1], app),
        CreateProductStep::Review => draw_product_step_review(f, layout[1], app),
    }

    // Validation error / navigation hint
    let hint = if let Some(err) = &app.create_product.validation_error {
        err.as_str()
    } else {
        "[Tab] Next field  [Enter] Next step  [Shift+Tab] Back  [Esc] Cancel"
    };
    let is_error = app.create_product.validation_error.is_some();
    draw_status_bar(f, status_area, app, hint);

    // Override status bar style with error if needed
    if is_error {
        let err_para = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" ✗ {}", app.create_product.validation_error.as_deref().unwrap_or("")),
                Style::default().fg(ERROR_RED).add_modifier(Modifier::BOLD),
            ),
        ]));
        f.render_widget(err_para, layout[2]);
    }
}

fn draw_product_wizard_progress(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let (step_num, total) = match app.create_product.step {
        CreateProductStep::BasicInfo => (1u16, 4u16),
        CreateProductStep::Descriptions => (2, 4),
        CreateProductStep::Support => (3, 4),
        CreateProductStep::Review => (4, 4),
    };

    let label = match app.create_product.step {
        CreateProductStep::BasicInfo => "Step 1/4 — Basic Info",
        CreateProductStep::Descriptions => "Step 2/4 — Descriptions",
        CreateProductStep::Support => "Step 3/4 — Support & Compliance",
        CreateProductStep::Review => "Step 4/4 — Review & Create",
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BRAND_DARK)))
        .gauge_style(Style::default().fg(BRAND_ORANGE).bg(BRAND_DARK))
        .percent((step_num * 100 / total) as u16)
        .label(Span::styled(label, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));

    f.render_widget(gauge, area);
}

fn draw_product_step_basic(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let w = &app.create_product;

    let lines: Vec<Line> = vec![
        Line::raw(""),
        wizard_field_line(
            "Product Title *",
            &w.title,
            w.focused_field == CreateProductField::Title,
            255,
        ),
        Line::raw(""),
        wizard_field_line(
            "Short Description *  (max 500 chars)",
            &w.short_description,
            w.focused_field == CreateProductField::ShortDescription,
            500,
        ),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  * Required fields",
            Style::default().fg(SUBTLE),
        )]),
    ];

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK))
                .title(Span::styled(
                    " Basic Information ",
                    Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn draw_product_step_descriptions(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let w = &app.create_product;

    let lines: Vec<Line> = vec![
        Line::raw(""),
        wizard_field_line(
            "Long Description *",
            &w.long_description,
            w.focused_field == CreateProductField::LongDescription,
            2000,
        ),
        Line::raw(""),
        wizard_field_line(
            "Categories (comma-separated)",
            &w.categories,
            w.focused_field == CreateProductField::Categories,
            200,
        ),
        Line::raw(""),
        wizard_field_line(
            "Search Keywords (comma-separated)",
            &w.keywords,
            w.focused_field == CreateProductField::Keywords,
            500,
        ),
        Line::raw(""),
        wizard_field_line(
            "Logo URL",
            &w.logo_url,
            w.focused_field == CreateProductField::LogoUrl,
            500,
        ),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  * Required fields",
            Style::default().fg(SUBTLE),
        )]),
    ];

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK))
                .title(Span::styled(
                    " Descriptions & Discovery ",
                    Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn draw_product_step_support(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let w = &app.create_product;

    let lines: Vec<Line> = vec![
        Line::raw(""),
        wizard_field_line(
            "Support Description",
            &w.support_description,
            w.focused_field == CreateProductField::SupportDescription,
            500,
        ),
        Line::raw(""),
        wizard_field_line(
            "Refund Policy",
            &w.refund_policy,
            w.focused_field == CreateProductField::RefundPolicy,
            100,
        ),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  EULA: ", Style::default().fg(SUBTLE)),
            Span::styled(
                "Standard AWS Marketplace Contract (applied automatically)",
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Initial Pricing: ", Style::default().fg(SUBTLE)),
            Span::styled("$0.01 USD (can be updated after creation)", Style::default().fg(Color::Cyan)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Creation Date Tag: ", Style::default().fg(SUBTLE)),
            Span::styled(
                chrono::Utc::now().format("%Y-%m-%d").to_string(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_DARK))
                .title(Span::styled(
                    " Support & Compliance ",
                    Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn draw_product_step_review(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let w = &app.create_product;

    let lines: Vec<Line> = vec![
        Line::raw(""),
        review_field("Title", &w.title),
        review_field("Short Description", &truncate(&w.short_description, 80)),
        review_field("Long Description", &truncate(&w.long_description, 60)),
        review_field("Support Description", &truncate(&w.support_description, 60)),
        review_field("Refund Policy", &w.refund_policy),
        review_field("Categories", &w.categories),
        review_field("Keywords", &w.keywords),
        review_field("Logo URL", &truncate(&w.logo_url, 60)),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  EULA:  Standard AWS Marketplace Contract",
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(vec![Span::styled(
            "  Initial pricing: $0.01 USD",
            Style::default().fg(Color::Cyan),
        )]),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  Press Enter to submit this product for creation.",
            Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
        )]),
    ];

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BRAND_ORANGE))
                .title(Span::styled(
                    " Review & Confirm ",
                    Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

// ─── Create Version Wizard ────────────────────────────────────────────────────

pub fn draw_create_version(f: &mut Frame, app: &App) {
    let (header_area, content_area, status_area) = base_layout(f);
    draw_header(f, header_area, app, "Add Product Version");

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(content_area);

    let (step_pct, step_label) = match app.create_version.step {
        VersionInfo    => (25u16, "Step 1/4 — Version Info & ARNs"),
        Instructions   => (50,    "Step 2/4 — Usage Instructions"),
        IOProperties   => (75,    "Step 3/4 — I/O Properties"),
        VersionReview  => (100,   "Step 4/4 — Review & Submit"),
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BRAND_DARK)))
        .gauge_style(Style::default().fg(BRAND_ORANGE).bg(BRAND_DARK))
        .percent(step_pct)
        .label(Span::styled(step_label, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
    f.render_widget(gauge, layout[0]);

    match app.create_version.step {
        VersionInfo   => draw_version_step_info(f, layout[1], app),
        Instructions  => draw_version_step_instructions(f, layout[1], app),
        IOProperties  => draw_version_step_io(f, layout[1], app),
        VersionReview => draw_version_step_review(f, layout[1], app),
    }

    let hint = if let Some(err) = &app.create_version.validation_error {
        err.as_str()
    } else {
        "[Tab] Next field  [Enter] Next step  [Shift+Tab] Back  [Esc] Cancel"
    };
    draw_status_bar(f, status_area, app, hint);
}

fn draw_version_step_info(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let w = &app.create_version;

    let lines: Vec<Line> = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Product: ", Style::default().fg(SUBTLE)),
            Span::styled(&w.product_entity_id, Style::default().fg(Color::Cyan)),
        ]),
        Line::raw(""),
        wizard_field_line("Version Title *", &w.title,
            w.focused_field == CreateVersionField::Title, 255),
        Line::raw(""),
        wizard_field_line("Release Notes *", &w.release_notes,
            w.focused_field == CreateVersionField::ReleaseNotes, 2000),
        Line::raw(""),
        wizard_field_line("SageMaker Model Package ARN *", &w.sagemaker_model_package_arn,
            w.focused_field == CreateVersionField::ModelPackageArn, 500),
        Line::raw(""),
        wizard_field_line("Access Role ARN *", &w.access_role_arn,
            w.focused_field == CreateVersionField::AccessRoleArn, 500),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  The Access Role ARN must be an IAM role with a trust relationship to",
            Style::default().fg(SUBTLE),
        )]),
        Line::from(vec![Span::styled(
            "  the SageMaker service, granting Marketplace access to the model package.",
            Style::default().fg(SUBTLE),
        )]),
        Line::raw(""),
        Line::from(vec![Span::styled("  * Required fields", Style::default().fg(SUBTLE))]),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(BRAND_DARK))
            .title(Span::styled(" Version Info & ARNs ",
                Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD))))
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn draw_version_step_instructions(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let w = &app.create_version;

    let lines: Vec<Line> = vec![
        Line::raw(""),
        wizard_field_line("Usage Instructions *", &w.usage_instructions,
            w.focused_field == CreateVersionField::UsageInstructions, 2000),
        Line::raw(""),
        wizard_field_line("Real-Time Inference Instance Type *", &w.realtime_instance_type,
            w.focused_field == CreateVersionField::RealtimeInstanceType, 50),
        Line::from(vec![Span::styled(
            "    e.g.  ml.g5.2xlarge",
            Style::default().fg(SUBTLE),
        )]),
        Line::raw(""),
        wizard_field_line("Batch Transform Instance Type *", &w.batch_instance_type,
            w.focused_field == CreateVersionField::BatchInstanceType, 50),
        Line::from(vec![Span::styled(
            "    e.g.  ml.m5.xlarge",
            Style::default().fg(SUBTLE),
        )]),
        Line::raw(""),
        wizard_field_line("Repository URL", &w.repository_url,
            w.focused_field == CreateVersionField::RepositoryUrl, 500),
        Line::raw(""),
        wizard_field_line("Sample Notebook URL", &w.sample_notebook_url,
            w.focused_field == CreateVersionField::SampleNotebookUrl, 500),
        Line::raw(""),
        Line::from(vec![Span::styled("  * Required fields", Style::default().fg(SUBTLE))]),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(BRAND_DARK))
            .title(Span::styled(" Usage Instructions ",
                Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD))))
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn draw_version_step_io(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let w = &app.create_version;

    let lines: Vec<Line> = vec![
        Line::raw(""),
        wizard_field_line("Input Properties Description *", &w.input_description,
            w.focused_field == CreateVersionField::InputDescription, 1000),
        Line::from(vec![Span::styled(
            "    Describe the input format the model expects (e.g. audio payload)",
            Style::default().fg(SUBTLE),
        )]),
        Line::raw(""),
        wizard_field_line("Sample Input URL *", &w.sample_input_url,
            w.focused_field == CreateVersionField::SampleInputUrl, 500),
        Line::from(vec![Span::styled(
            "    URL to a sample input file (used for both real-time and batch inference)",
            Style::default().fg(SUBTLE),
        )]),
        Line::raw(""),
        wizard_field_line("Output Properties Description *", &w.output_description,
            w.focused_field == CreateVersionField::OutputDescription, 1000),
        Line::from(vec![Span::styled(
            "    Describe the output format the model returns (e.g. JSON transcription)",
            Style::default().fg(SUBTLE),
        )]),
        Line::raw(""),
        wizard_field_line("Sample Output *", &w.sample_output,
            w.focused_field == CreateVersionField::SampleOutput, 500),
        Line::from(vec![Span::styled(
            "    A brief example of the model output (e.g. '{\"transcript\": \"hello world\"}')",
            Style::default().fg(SUBTLE),
        )]),
        Line::raw(""),
        Line::from(vec![Span::styled("  * Required fields", Style::default().fg(SUBTLE))]),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(BRAND_DARK))
            .title(Span::styled(" Input / Output Properties ",
                Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD))))
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn draw_version_step_review(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let w = &app.create_version;

    let lines: Vec<Line> = vec![
        Line::raw(""),
        review_field("Version Title", &w.title),
        review_field("Release Notes", &truncate(&w.release_notes, 70)),
        review_field("SageMaker Model Package ARN", &truncate(&w.sagemaker_model_package_arn, 70)),
        review_field("Access Role ARN", &truncate(&w.access_role_arn, 70)),
        review_field("Usage Instructions", &truncate(&w.usage_instructions, 70)),
        review_field("Real-Time Instance Type", &w.realtime_instance_type),
        review_field("Batch Instance Type", &w.batch_instance_type),
        review_field("Repository URL", &truncate(&w.repository_url, 70)),
        review_field("Sample Notebook URL", &truncate(&w.sample_notebook_url, 70)),
        review_field("Input Description", &truncate(&w.input_description, 70)),
        review_field("Sample Input URL", &truncate(&w.sample_input_url, 70)),
        review_field("Output Description", &truncate(&w.output_description, 70)),
        review_field("Sample Output", &truncate(&w.sample_output, 70)),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "  Press Enter to submit this version.",
            Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD),
        )]),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(BRAND_ORANGE))
            .title(Span::styled(" Review & Confirm ",
                Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD))))
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

// ─── Shared wizard helpers ───────────────────────────────────────────────────

fn wizard_field_line<'a>(label: &str, value: &str, focused: bool, _max: usize) -> Line<'a> {
    let label_style = if focused {
        Style::default().fg(BRAND_ORANGE).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(SUBTLE)
    };
    let value_style = if focused {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let cursor = if focused { "_" } else { "" };
    let prefix = if focused { "▶ " } else { "  " };

    Line::from(vec![
        Span::styled(format!("{}{}: ", prefix, label), label_style),
        Span::styled(format!("{}{}", value, cursor), value_style),
    ])
}

fn review_field<'a>(label: &str, value: &str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:.<30} ", label),
            Style::default().fg(SUBTLE),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}
