#![allow(dead_code)]
use crate::models::*;
use ratatui::widgets::{ListState, TableState};
use std::sync::{Arc, Mutex};

pub type AppLog = Arc<Mutex<Vec<LogEntry>>>;

/// Top-level application state (which view/overlay is active)
#[derive(Clone, Debug, PartialEq)]
pub enum AppState {
    ProductList,
    ProductDetail,
    VersionList,
    CreateProductWizard,
    CreateVersionWizard,
    WhitelistManager,
    PricingEditor,
    MetadataEditor,
    VersionMetadataEditor,
    RestrictVersionConfirm,
    ChangeSetStatusView,
    CommandPalette,
    Help,
    Logs,
}

/// Tabs in the product detail view
#[derive(Clone, Debug, PartialEq)]
pub enum DetailTab {
    Overview,
    Versions,
    Pricing,
    Whitelist,
}

impl DetailTab {
    pub fn index(&self) -> usize {
        match self {
            DetailTab::Overview => 0,
            DetailTab::Versions => 1,
            DetailTab::Pricing => 2,
            DetailTab::Whitelist => 3,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => DetailTab::Overview,
            1 => DetailTab::Versions,
            2 => DetailTab::Pricing,
            3 => DetailTab::Whitelist,
            _ => DetailTab::Overview,
        }
    }
}

// ─── Create Product Wizard ──────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum CreateProductStep {
    BasicInfo,
    Descriptions,
    Support,
    Review,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CreateProductField {
    Title,
    ShortDescription,
    LongDescription,
    SupportDescription,
    RefundPolicy,
    Categories,
    Keywords,
    LogoUrl,
}

#[derive(Clone, Debug)]
pub struct CreateProductWizardState {
    pub step: CreateProductStep,
    pub focused_field: CreateProductField,
    pub title: String,
    pub short_description: String,
    pub long_description: String,
    pub support_description: String,
    pub refund_policy: String,
    pub categories: String,
    pub keywords: String,
    pub logo_url: String,
    pub validation_error: Option<String>,
}

impl CreateProductWizardState {
    pub fn new() -> Self {
        Self {
            step: CreateProductStep::BasicInfo,
            focused_field: CreateProductField::Title,
            title: String::new(),
            short_description: String::new(),
            long_description: String::new(),
            support_description: String::new(),
            refund_policy: "N/A".to_string(),
            categories: "Machine Learning".to_string(),
            keywords: String::new(),
            logo_url: String::new(),
            validation_error: None,
        }
    }

    pub fn active_field_mut(&mut self) -> &mut String {
        match self.focused_field {
            CreateProductField::Title => &mut self.title,
            CreateProductField::ShortDescription => &mut self.short_description,
            CreateProductField::LongDescription => &mut self.long_description,
            CreateProductField::SupportDescription => &mut self.support_description,
            CreateProductField::RefundPolicy => &mut self.refund_policy,
            CreateProductField::Categories => &mut self.categories,
            CreateProductField::Keywords => &mut self.keywords,
            CreateProductField::LogoUrl => &mut self.logo_url,
        }
    }

    pub fn validate_step(&self) -> Result<(), String> {
        match self.step {
            CreateProductStep::BasicInfo => {
                if self.title.trim().is_empty() {
                    return Err("Product title is required".to_string());
                }
                if self.title.len() > 255 {
                    return Err("Product title must be 255 characters or fewer".to_string());
                }
                if self.short_description.trim().is_empty() {
                    return Err("Short description is required".to_string());
                }
                if self.short_description.len() > 500 {
                    return Err("Short description must be 500 characters or fewer".to_string());
                }
            }
            CreateProductStep::Descriptions => {
                if self.long_description.trim().is_empty() {
                    return Err("Long description is required".to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn advance_field(&mut self) {
        use CreateProductField::*;
        self.focused_field = match (&self.step, &self.focused_field) {
            (CreateProductStep::BasicInfo, Title) => ShortDescription,
            (CreateProductStep::BasicInfo, ShortDescription) => Title,
            (CreateProductStep::Descriptions, LongDescription) => Categories,
            (CreateProductStep::Descriptions, Categories) => Keywords,
            (CreateProductStep::Descriptions, Keywords) => LogoUrl,
            (CreateProductStep::Descriptions, LogoUrl) => LongDescription,
            (CreateProductStep::Support, SupportDescription) => RefundPolicy,
            (CreateProductStep::Support, RefundPolicy) => SupportDescription,
            _ => self.focused_field.clone(),
        };
    }

    pub fn to_config(&self) -> CreateProductConfig {
        CreateProductConfig {
            title: self.title.trim().to_string(),
            short_description: self.short_description.trim().to_string(),
            long_description: self.long_description.trim().to_string(),
            support_description: self.support_description.trim().to_string(),
            refund_policy: self.refund_policy.trim().to_string(),
            categories: self
                .categories
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            search_keywords: self
                .keywords
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            logo_url: self.logo_url.trim().to_string(),
        }
    }
}

// ─── Create Version Wizard ──────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum CreateVersionStep {
    VersionInfo,   // title, release notes, ARNs
    Instructions,  // usage instructions, URLs, instance types
    IOProperties,  // input / output descriptions
    Review,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CreateVersionField {
    Title,
    ReleaseNotes,
    ModelPackageArn,
    AccessRoleArn,
    UsageInstructions,
    RealtimeInstanceType,
    BatchInstanceType,
    RepositoryUrl,
    SampleNotebookUrl,
    InputDescription,
    SampleInputUrl,
    OutputDescription,
    SampleOutput,
}

#[derive(Clone, Debug)]
pub struct CreateVersionWizardState {
    pub step: CreateVersionStep,
    pub focused_field: CreateVersionField,
    pub product_entity_id: String,
    pub title: String,
    pub release_notes: String,
    pub sagemaker_model_package_arn: String,
    pub access_role_arn: String,
    pub usage_instructions: String,
    pub realtime_instance_type: String,
    pub batch_instance_type: String,
    pub repository_url: String,
    pub sample_notebook_url: String,
    pub input_description: String,
    pub sample_input_url: String,
    pub output_description: String,
    pub sample_output: String,
    pub validation_error: Option<String>,
}

impl CreateVersionWizardState {
    pub fn new(product_entity_id: String) -> Self {
        Self {
            step: CreateVersionStep::VersionInfo,
            focused_field: CreateVersionField::Title,
            product_entity_id,
            title: String::new(),
            release_notes: String::new(),
            sagemaker_model_package_arn: String::new(),
            access_role_arn: String::new(),
            usage_instructions: String::new(),
            realtime_instance_type: String::new(),
            batch_instance_type: String::new(),
            repository_url: String::new(),
            sample_notebook_url: String::new(),
            input_description: String::new(),
            sample_input_url: String::new(),
            output_description: String::new(),
            sample_output: String::new(),
            validation_error: None,
        }
    }

    pub fn active_field_mut(&mut self) -> &mut String {
        match self.focused_field {
            CreateVersionField::Title => &mut self.title,
            CreateVersionField::ReleaseNotes => &mut self.release_notes,
            CreateVersionField::ModelPackageArn => &mut self.sagemaker_model_package_arn,
            CreateVersionField::AccessRoleArn => &mut self.access_role_arn,
            CreateVersionField::UsageInstructions => &mut self.usage_instructions,
            CreateVersionField::RealtimeInstanceType => &mut self.realtime_instance_type,
            CreateVersionField::BatchInstanceType => &mut self.batch_instance_type,
            CreateVersionField::RepositoryUrl => &mut self.repository_url,
            CreateVersionField::SampleNotebookUrl => &mut self.sample_notebook_url,
            CreateVersionField::InputDescription => &mut self.input_description,
            CreateVersionField::SampleInputUrl => &mut self.sample_input_url,
            CreateVersionField::OutputDescription => &mut self.output_description,
            CreateVersionField::SampleOutput => &mut self.sample_output,
        }
    }

    pub fn advance_field(&mut self) {
        use CreateVersionField::*;
        self.focused_field = match (&self.step, &self.focused_field) {
            (CreateVersionStep::VersionInfo, Title) => ReleaseNotes,
            (CreateVersionStep::VersionInfo, ReleaseNotes) => ModelPackageArn,
            (CreateVersionStep::VersionInfo, ModelPackageArn) => AccessRoleArn,
            (CreateVersionStep::VersionInfo, AccessRoleArn) => Title,
            (CreateVersionStep::Instructions, UsageInstructions) => RealtimeInstanceType,
            (CreateVersionStep::Instructions, RealtimeInstanceType) => BatchInstanceType,
            (CreateVersionStep::Instructions, BatchInstanceType) => RepositoryUrl,
            (CreateVersionStep::Instructions, RepositoryUrl) => SampleNotebookUrl,
            (CreateVersionStep::Instructions, SampleNotebookUrl) => UsageInstructions,
            (CreateVersionStep::IOProperties, InputDescription) => SampleInputUrl,
            (CreateVersionStep::IOProperties, SampleInputUrl) => OutputDescription,
            (CreateVersionStep::IOProperties, OutputDescription) => SampleOutput,
            (CreateVersionStep::IOProperties, SampleOutput) => InputDescription,
            _ => self.focused_field.clone(),
        };
    }

    pub fn validate_step(&self) -> Result<(), String> {
        match self.step {
            CreateVersionStep::VersionInfo => {
                if self.title.trim().is_empty() {
                    return Err("Version title is required".to_string());
                }
                if self.release_notes.trim().is_empty() {
                    return Err("Release notes are required".to_string());
                }
                if self.sagemaker_model_package_arn.trim().is_empty() {
                    return Err("SageMaker Model Package ARN is required".to_string());
                }
                if self.access_role_arn.trim().is_empty() {
                    return Err("Access Role ARN is required".to_string());
                }
            }
            CreateVersionStep::Instructions => {
                if self.usage_instructions.trim().is_empty() {
                    return Err("Usage instructions are required".to_string());
                }
                if self.realtime_instance_type.trim().is_empty() {
                    return Err("Real-time inference instance type is required".to_string());
                }
                if self.batch_instance_type.trim().is_empty() {
                    return Err("Batch transform instance type is required".to_string());
                }
            }
            CreateVersionStep::IOProperties => {
                if self.input_description.trim().is_empty() {
                    return Err("Input properties description is required".to_string());
                }
                if self.sample_input_url.trim().is_empty() {
                    return Err("Sample input URL is required".to_string());
                }
                if self.output_description.trim().is_empty() {
                    return Err("Output properties description is required".to_string());
                }
                if self.sample_output.trim().is_empty() {
                    return Err("Sample output is required".to_string());
                }
            }
            CreateVersionStep::Review => {}
        }
        Ok(())
    }

    pub fn to_config(&self) -> CreateVersionConfig {
        CreateVersionConfig {
            product_entity_id: self.product_entity_id.clone(),
            title: self.title.trim().to_string(),
            release_notes: self.release_notes.trim().to_string(),
            sagemaker_model_package_arn: self.sagemaker_model_package_arn.trim().to_string(),
            access_role_arn: self.access_role_arn.trim().to_string(),
            usage_instructions: self.usage_instructions.trim().to_string(),
            realtime_instance_type: self.realtime_instance_type.trim().to_string(),
            batch_instance_type: self.batch_instance_type.trim().to_string(),
            repository_url: self.repository_url.trim().to_string(),
            sample_notebook_url: self.sample_notebook_url.trim().to_string(),
            input_description: self.input_description.trim().to_string(),
            sample_input_url: self.sample_input_url.trim().to_string(),
            output_description: self.output_description.trim().to_string(),
            sample_output: self.sample_output.trim().to_string(),
        }
    }
}

// ─── Metadata Editor ────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum MetadataField {
    Title,
    ShortDescription,
    LongDescription,
    SupportDescription,
    LogoUrl,
    Categories,
    Keywords,
}

impl MetadataField {
    pub fn label(&self) -> &'static str {
        match self {
            MetadataField::Title => "Title",
            MetadataField::ShortDescription => "Short Description",
            MetadataField::LongDescription => "Long Description",
            MetadataField::SupportDescription => "Support Description",
            MetadataField::LogoUrl => "Logo URL",
            MetadataField::Categories => "Categories (comma-separated)",
            MetadataField::Keywords => "Keywords (comma-separated)",
        }
    }

    pub fn all() -> Vec<MetadataField> {
        vec![
            MetadataField::Title,
            MetadataField::ShortDescription,
            MetadataField::LongDescription,
            MetadataField::SupportDescription,
            MetadataField::LogoUrl,
            MetadataField::Categories,
            MetadataField::Keywords,
        ]
    }
}

#[derive(Clone, Debug)]
pub struct MetadataEditorState {
    pub product_entity_id: String,
    pub fields: Vec<(MetadataField, String)>,
    pub selected_field: usize,
    pub editing: bool,
    pub input_buffer: String,
    pub fields_state: ListState,
    pub validation_error: Option<String>,
}

impl MetadataEditorState {
    pub fn from_detail(detail: &ProductDetail) -> Self {
        let mut fields_state = ListState::default();
        fields_state.select(Some(0));
        Self {
            product_entity_id: detail.entity_id.clone(),
            fields: vec![
                (MetadataField::Title, detail.title.clone()),
                (MetadataField::ShortDescription, detail.short_description.clone()),
                (MetadataField::LongDescription, detail.long_description.clone()),
                (MetadataField::SupportDescription, detail.support_description.clone()),
                (MetadataField::LogoUrl, detail.logo_url.clone()),
                (
                    MetadataField::Categories,
                    detail.categories.join(", "),
                ),
                (
                    MetadataField::Keywords,
                    detail.search_keywords.join(", "),
                ),
            ],
            selected_field: 0,
            editing: false,
            input_buffer: String::new(),
            fields_state,
            validation_error: None,
        }
    }
}

// ─── Version Metadata Editor ─────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum VersionMetadataField {
    Title,
    ReleaseNotes,
}

impl VersionMetadataField {
    pub fn label(&self) -> &'static str {
        match self {
            VersionMetadataField::Title => "Version Title",
            VersionMetadataField::ReleaseNotes => "Release Notes",
        }
    }
}

#[derive(Clone, Debug)]
pub struct VersionMetadataEditorState {
    pub product_entity_id: String,
    /// Delivery-option ID used in the API call (preferred over version ID)
    pub delivery_option_id: String,
    pub fields: Vec<(VersionMetadataField, String)>,
    pub selected_field: usize,
    pub editing: bool,
    pub input_buffer: String,
    pub fields_state: ListState,
    pub validation_error: Option<String>,
}

impl VersionMetadataEditorState {
    pub fn from_version(
        product_entity_id: &str,
        version: &crate::models::DeliveryOption,
    ) -> Self {
        // Prefer the nested delivery-option ID; fall back to the version ID
        let delivery_option_id = version
            .delivery_option_id
            .clone()
            .unwrap_or_else(|| version.id.clone());

        let mut fields_state = ListState::default();
        fields_state.select(Some(0));

        Self {
            product_entity_id: product_entity_id.to_string(),
            delivery_option_id,
            fields: vec![
                (VersionMetadataField::Title, version.title.clone()),
                (VersionMetadataField::ReleaseNotes, version.release_notes.clone()),
            ],
            selected_field: 0,
            editing: false,
            input_buffer: String::new(),
            fields_state,
            validation_error: None,
        }
    }
}

// ─── Command Palette ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum CommandAction {
    CreateProduct,
    CreateVersion,
    Refresh,
    EditMetadata,
    ManageWhitelist,
    EditPricing,
    RestrictVersion,
    ViewChangeSets,
    ShowHelp,
    ShowLogs,
    CopyEntityId,
    CopyAllProductIds,
    Quit,
}

#[derive(Clone, Debug)]
pub struct CommandPaletteItem {
    pub name: String,
    pub description: String,
    pub shortcut: Option<String>,
    pub action: CommandAction,
}

fn build_command_palette_items() -> Vec<CommandPaletteItem> {
    vec![
        CommandPaletteItem {
            name: "Create Product".to_string(),
            description: "Create a new ML product listing".to_string(),
            shortcut: Some("c".to_string()),
            action: CommandAction::CreateProduct,
        },
        CommandPaletteItem {
            name: "Create Version".to_string(),
            description: "Add a new version to the selected product".to_string(),
            shortcut: Some("v".to_string()),
            action: CommandAction::CreateVersion,
        },
        CommandPaletteItem {
            name: "Refresh".to_string(),
            description: "Refresh the product list from AWS".to_string(),
            shortcut: Some("R".to_string()),
            action: CommandAction::Refresh,
        },
        CommandPaletteItem {
            name: "Edit Metadata".to_string(),
            description: "Edit product metadata fields".to_string(),
            shortcut: Some("e".to_string()),
            action: CommandAction::EditMetadata,
        },
        CommandPaletteItem {
            name: "Manage Allowlist".to_string(),
            description: "Manage allowed AWS account IDs for this product".to_string(),
            shortcut: Some("w".to_string()),
            action: CommandAction::ManageWhitelist,
        },
        CommandPaletteItem {
            name: "Edit Pricing".to_string(),
            description: "View and update product pricing terms".to_string(),
            shortcut: Some("p".to_string()),
            action: CommandAction::EditPricing,
        },
        CommandPaletteItem {
            name: "Restrict Version".to_string(),
            description: "Restrict (deprecate) the selected version".to_string(),
            shortcut: Some("r".to_string()),
            action: CommandAction::RestrictVersion,
        },
        CommandPaletteItem {
            name: "Change Set Status".to_string(),
            description: "View background change set operations".to_string(),
            shortcut: Some("s".to_string()),
            action: CommandAction::ViewChangeSets,
        },
        CommandPaletteItem {
            name: "Help".to_string(),
            description: "Show keyboard shortcuts and mouse controls".to_string(),
            shortcut: Some("?".to_string()),
            action: CommandAction::ShowHelp,
        },
        CommandPaletteItem {
            name: "Logs".to_string(),
            description: "Show the application log".to_string(),
            shortcut: Some("l".to_string()),
            action: CommandAction::ShowLogs,
        },
        CommandPaletteItem {
            name: "Copy Entity ID".to_string(),
            description: "Copy selected product entity ID to clipboard".to_string(),
            shortcut: Some("y".to_string()),
            action: CommandAction::CopyEntityId,
        },
        CommandPaletteItem {
            name: "Copy All Product IDs".to_string(),
            description: "Copy all product IDs and visibility status to clipboard".to_string(),
            shortcut: None,
            action: CommandAction::CopyAllProductIds,
        },
        CommandPaletteItem {
            name: "Quit".to_string(),
            description: "Exit the application".to_string(),
            shortcut: Some("q".to_string()),
            action: CommandAction::Quit,
        },
    ]
}

// ─── Pricing Editor ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum PricingField {
    Price,
    Currency,
    Dimension,
}

impl PricingField {
    pub fn label(&self) -> &'static str {
        match self {
            PricingField::Price => "Price per Unit (USD)",
            PricingField::Currency => "Currency Code",
            PricingField::Dimension => "Dimension Key",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            PricingField::Price => PricingField::Currency,
            PricingField::Currency => PricingField::Dimension,
            PricingField::Dimension => PricingField::Price,
        }
    }
}

// ─── Main App Struct ─────────────────────────────────────────────────────────

pub struct App {
    // State
    pub state: AppState,
    pub previous_state: AppState,

    // Product list
    pub products: Vec<ProductSummary>,
    pub products_table_state: TableState,
    pub selected_product_index: Option<usize>,
    pub product_filter: String,
    pub filtered_product_indices: Vec<usize>,

    // Product detail (lazy loaded on selection)
    pub current_product_detail: Option<ProductDetail>,
    pub detail_tab: DetailTab,

    // Version list
    pub versions_list_state: ListState,
    pub selected_version_index: Option<usize>,

    // Metadata editors
    pub metadata_editor: Option<MetadataEditorState>,
    pub version_metadata_editor: Option<VersionMetadataEditorState>,

    // Whitelist manager
    pub whitelist_input: String,
    pub whitelist_list_state: ListState,
    pub whitelist_selected: Option<usize>,
    pub whitelist_editing: bool,
    pub whitelist_input_valid: Option<bool>, // None = untouched, Some(true/false) = validated

    // Pricing editor
    pub pricing_price: String,
    pub pricing_currency: String,
    pub pricing_dimension: String,
    pub pricing_focused_field: PricingField,
    pub pricing_input_valid: Option<bool>,

    // Create product wizard
    pub create_product: CreateProductWizardState,

    // Create version wizard
    pub create_version: CreateVersionWizardState,

    // Background changesets
    pub pending_changesets: Vec<PendingChangeset>,
    pub changesets_list_state: ListState,

    // Command palette
    pub command_palette_input: String,
    pub command_palette_items: Vec<CommandPaletteItem>,
    pub filtered_commands: Vec<usize>,
    pub command_palette_state: ListState,

    // Log / help viewer scroll offsets
    pub log_scroll: u16,
    pub help_scroll: u16,

    // UI
    pub is_loading: bool,
    pub status_message: String,
    pub status_is_error: bool,
    pub ticker: u64, // increments each main loop tick for animations
    pub current_region: String,
    /// When true, all typed characters go into the product filter (entered with `/`)
    pub filter_input_mode: bool,
    pub log: AppLog,

    // Async operation flags
    pub pending_refresh: bool,
    pub pending_load_detail: Option<String>,
    pub pending_restrict_version_id: Option<String>,
    pub pending_create_product: bool,
    pub pending_create_version: bool,
    pub pending_save_metadata: bool,
    pub pending_save_version_metadata: bool,
    pub pending_save_whitelist: Option<Vec<String>>,
    pub pending_save_pricing: bool,
    pub pending_poll_changesets: bool,
}

impl App {
    pub fn new(log: AppLog) -> Self {
        let command_palette_items = build_command_palette_items();
        let filtered_commands = (0..command_palette_items.len()).collect();

        Self {
            state: AppState::ProductList,
            previous_state: AppState::ProductList,

            products: Vec::new(),
            products_table_state: TableState::default(),
            selected_product_index: None,
            product_filter: String::new(),
            filtered_product_indices: Vec::new(),

            current_product_detail: None,
            detail_tab: DetailTab::Overview,

            versions_list_state: ListState::default(),
            selected_version_index: None,

            metadata_editor: None,
            version_metadata_editor: None,

            whitelist_input: String::new(),
            whitelist_list_state: ListState::default(),
            whitelist_selected: None,
            whitelist_editing: false,
            whitelist_input_valid: None,

            pricing_price: "0.01".to_string(),
            pricing_currency: "USD".to_string(),
            pricing_dimension: "unit".to_string(),
            pricing_focused_field: PricingField::Price,
            pricing_input_valid: None,

            create_product: CreateProductWizardState::new(),
            create_version: CreateVersionWizardState::new(String::new()),

            pending_changesets: Vec::new(),
            changesets_list_state: ListState::default(),

            command_palette_input: String::new(),
            command_palette_items,
            filtered_commands,
            command_palette_state: ListState::default(),

            log_scroll: 0,
            help_scroll: 0,

            is_loading: false,
            status_message: String::new(),
            status_is_error: false,
            ticker: 0,
            current_region: "us-east-1".to_string(),
            filter_input_mode: false,
            log,

            pending_refresh: false,
            pending_load_detail: None,
            pending_restrict_version_id: None,
            pending_create_product: false,
            pending_create_version: false,
            pending_save_metadata: false,
            pending_save_version_metadata: false,
            pending_save_whitelist: None,
            pending_save_pricing: false,
            pending_poll_changesets: false,
        }
    }

    pub fn log(&self, level: LogLevel, message: String) {
        if let Ok(mut log) = self.log.lock() {
            log.push(LogEntry::new(level, message));
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status_message = msg.into();
        self.status_is_error = is_error;
    }

    pub fn tick(&mut self) {
        self.ticker = self.ticker.wrapping_add(1);
    }

    /// Returns the currently selected product summary (respects filter)
    pub fn selected_product(&self) -> Option<&ProductSummary> {
        self.selected_product_index
            .and_then(|i| self.filtered_product_indices.get(i))
            .and_then(|&real| self.products.get(real))
    }

    /// Returns the selected version from the loaded product detail
    pub fn selected_version(&self) -> Option<&DeliveryOption> {
        let idx = self.selected_version_index?;
        self.current_product_detail.as_ref()?.versions.get(idx)
    }

    /// Re-apply the current product name filter
    pub fn apply_product_filter(&mut self) {
        let filter = self.product_filter.to_lowercase();
        self.filtered_product_indices = self
            .products
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                filter.is_empty()
                    || p.name.to_lowercase().contains(&filter)
                    || p.entity_id.to_lowercase().contains(&filter)
                    || p.product_code
                        .as_deref()
                        .map(|c| c.to_lowercase().contains(&filter))
                        .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect();

        let count = self.filtered_product_indices.len();
        if count == 0 {
            self.selected_product_index = None;
            self.products_table_state.select(None);
        } else {
            let new_idx = self
                .selected_product_index
                .map(|i| i.min(count - 1))
                .unwrap_or(0);
            self.selected_product_index = Some(new_idx);
            self.products_table_state.select(Some(new_idx));
        }
    }

    /// Apply command palette filter
    pub fn apply_command_filter(&mut self) {
        let filter = self.command_palette_input.to_lowercase();
        self.filtered_commands = self
            .command_palette_items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                filter.is_empty()
                    || item.name.to_lowercase().contains(&filter)
                    || item.description.to_lowercase().contains(&filter)
            })
            .map(|(i, _)| i)
            .collect();
        let sel = if self.filtered_commands.is_empty() { None } else { Some(0) };
        self.command_palette_state.select(sel);
    }

    /// Navigate the product list up
    pub fn product_list_up(&mut self) {
        let n = self.filtered_product_indices.len();
        if n == 0 { return; }
        let cur = self.selected_product_index.unwrap_or(0);
        let next = if cur == 0 { n - 1 } else { cur - 1 };
        self.selected_product_index = Some(next);
        self.products_table_state.select(Some(next));
    }

    /// Navigate the product list down
    pub fn product_list_down(&mut self) {
        let n = self.filtered_product_indices.len();
        if n == 0 { return; }
        let cur = self.selected_product_index.unwrap_or(0);
        let next = if cur >= n - 1 { 0 } else { cur + 1 };
        self.selected_product_index = Some(next);
        self.products_table_state.select(Some(next));
    }

    /// Page through product list
    pub fn product_list_page_up(&mut self, page_size: usize) {
        let n = self.filtered_product_indices.len();
        if n == 0 { return; }
        let cur = self.selected_product_index.unwrap_or(0);
        let next = cur.saturating_sub(page_size);
        self.selected_product_index = Some(next);
        self.products_table_state.select(Some(next));
    }

    pub fn product_list_page_down(&mut self, page_size: usize) {
        let n = self.filtered_product_indices.len();
        if n == 0 { return; }
        let cur = self.selected_product_index.unwrap_or(0);
        let next = (cur + page_size).min(n - 1);
        self.selected_product_index = Some(next);
        self.products_table_state.select(Some(next));
    }

    /// Navigate version list
    pub fn version_list_up(&mut self) {
        let n = self
            .current_product_detail
            .as_ref()
            .map(|d| d.versions.len())
            .unwrap_or(0);
        if n == 0 { return; }
        let cur = self.selected_version_index.unwrap_or(0);
        let next = if cur == 0 { n - 1 } else { cur - 1 };
        self.selected_version_index = Some(next);
        self.versions_list_state.select(Some(next));
    }

    pub fn version_list_down(&mut self) {
        let n = self
            .current_product_detail
            .as_ref()
            .map(|d| d.versions.len())
            .unwrap_or(0);
        if n == 0 { return; }
        let cur = self.selected_version_index.unwrap_or(0);
        let next = if cur >= n - 1 { 0 } else { cur + 1 };
        self.selected_version_index = Some(next);
        self.versions_list_state.select(Some(next));
    }

    /// Enter product detail view and trigger lazy load
    pub fn open_product_detail(&mut self) {
        if let Some(entity_id) = self.selected_product().map(|p| p.entity_id.clone()) {
            self.current_product_detail = None;
            self.detail_tab = DetailTab::Overview;
            self.pending_load_detail = Some(entity_id);
            self.state = AppState::ProductDetail;
        }
    }

    /// Open the metadata editor pre-populated with current detail
    pub fn open_metadata_editor(&mut self) {
        if let Some(detail) = &self.current_product_detail {
            self.metadata_editor = Some(MetadataEditorState::from_detail(detail));
            self.state = AppState::MetadataEditor;
        }
    }

    /// Enter version list view
    pub fn open_version_list(&mut self) {
        let n = self
            .current_product_detail
            .as_ref()
            .map(|d| d.versions.len())
            .unwrap_or(0);
        self.selected_version_index = if n > 0 { Some(0) } else { None };
        self.versions_list_state.select(self.selected_version_index);
        self.state = AppState::VersionList;
    }

    /// Enter whitelist manager
    pub fn open_whitelist_manager(&mut self) {
        self.whitelist_input.clear();
        self.whitelist_editing = false;
        self.whitelist_input_valid = None;
        let n = self
            .current_product_detail
            .as_ref()
            .map(|d| d.allowed_accounts.len())
            .unwrap_or(0);
        self.whitelist_selected = if n > 0 { Some(0) } else { None };
        self.whitelist_list_state.select(self.whitelist_selected);
        self.state = AppState::WhitelistManager;
    }

    /// Enter pricing editor, pre-populated from current product detail
    pub fn open_pricing_editor(&mut self) {
        if let Some(detail) = &self.current_product_detail {
            for term in &detail.pricing_terms {
                if term.term_type == "UsageBasedPricingTerm" {
                    if let Some(p) = &term.price_per_unit {
                        self.pricing_price = p.clone();
                    }
                    if let Some(c) = &term.currency_code {
                        self.pricing_currency = c.clone();
                    }
                    if let Some(d) = &term.dimension_key {
                        self.pricing_dimension = d.clone();
                    }
                    break;
                }
            }
        }
        self.pricing_focused_field = PricingField::Price;
        self.pricing_input_valid = None;
        self.state = AppState::PricingEditor;
    }

    /// Open command palette from any state
    pub fn open_command_palette(&mut self) {
        self.previous_state = self.state.clone();
        self.command_palette_input.clear();
        self.filtered_commands = (0..self.command_palette_items.len()).collect();
        self.command_palette_state.select(Some(0));
        self.state = AppState::CommandPalette;
    }

    /// Close overlay and return to previous state
    pub fn close_overlay(&mut self) {
        self.state = self.previous_state.clone();
    }

    /// Open the version metadata editor for the currently selected version
    pub fn open_version_metadata_editor(&mut self) {
        if let (Some(detail), Some(version)) = (
            self.current_product_detail.as_ref(),
            self.selected_version(),
        ) {
            let entity_id = detail.entity_id.clone();
            let editor = VersionMetadataEditorState::from_version(&entity_id, version);
            self.version_metadata_editor = Some(editor);
            self.state = AppState::VersionMetadataEditor;
        }
    }

    /// Open help overlay (saves return state)
    pub fn open_help(&mut self) {
        self.previous_state = self.state.clone();
        self.help_scroll = 0;
        self.state = AppState::Help;
    }

    /// Open logs overlay
    pub fn open_logs(&mut self) {
        self.previous_state = self.state.clone();
        self.log_scroll = 0;
        self.state = AppState::Logs;
    }

    pub fn open_changeset_view(&mut self) {
        self.previous_state = self.state.clone();
        self.state = AppState::ChangeSetStatusView;
    }

    /// Validate whitelist input in real time
    pub fn validate_whitelist_input(&mut self) {
        if self.whitelist_input.is_empty() {
            self.whitelist_input_valid = None;
        } else {
            let valid = regex::Regex::new(r"^\d{12}$")
                .map(|re| re.is_match(&self.whitelist_input))
                .unwrap_or(false);
            self.whitelist_input_valid = Some(valid);
        }
    }

    /// Validate pricing price field in real time
    pub fn validate_pricing_input(&mut self) {
        let valid = self.pricing_price.parse::<f64>().map(|v| v >= 0.0).unwrap_or(false);
        self.pricing_input_valid = Some(valid);
    }

    /// Returns true if there are any non-terminal pending changesets
    pub fn has_active_changesets(&self) -> bool {
        self.pending_changesets
            .iter()
            .any(|cs| !cs.status.is_terminal())
    }

    pub fn spinner_char(&self) -> char {
        let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        frames[(self.ticker / 3) as usize % frames.len()]
    }
}
