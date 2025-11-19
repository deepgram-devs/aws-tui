use crate::models::*;
use ratatui::widgets::{ListState, TableState};
use std::sync::{Arc, Mutex};

pub type AppLog = Arc<Mutex<Vec<LogEntry>>>;

#[derive(Clone, Debug, PartialEq)]
pub enum AppState {
    RegularTablesView,
    TableItemsView,
    RegionSelector,
    TableCreationWizard,
    ItemEditor,
    DeleteConfirmation(DeleteTarget),
    Help,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeleteTarget {
    Table(String),
    Item,
}

pub struct App {
    // State
    pub state: AppState,

    // Tables view
    pub table_names: Vec<String>,
    pub tables: Vec<Option<DynamoTable>>, // Lazy loaded
    pub table_state: TableState,
    pub selected_table_index: Option<usize>,

    // Items view
    pub current_items: Vec<DynamoItem>,
    pub items_table_state: TableState,
    pub selected_item_index: Option<usize>,
    pub pagination: PaginationState,

    // Region management
    pub current_region: String,
    pub available_regions: Vec<String>,
    pub region_filter: String,
    pub filtered_regions: Vec<String>,
    pub region_selector_state: ListState,

    // Table creation wizard
    pub wizard_state: WizardState,

    // Item editor
    pub item_editor: Option<ItemEditorState>,

    // UI state
    pub is_loading: bool,
    pub status_message: String,
    pub log: AppLog,

    // Action flags
    pub pending_save_item: bool,
    pub pending_delete_item: bool,
    pub pending_delete_table: Option<String>,
    pub pending_refresh: bool,
}

#[derive(Clone, Debug)]
pub struct WizardState {
    pub step: WizardStep,
    pub table_name: String,
    pub hash_key_name: String,
    pub hash_key_type: AttributeType,
    pub has_range_key: bool,
    pub range_key_name: String,
    pub range_key_type: AttributeType,
    pub billing_mode: BillingMode,
    pub read_capacity: String,
    pub write_capacity: String,
    pub current_field: WizardField,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WizardStep {
    BasicConfig,
    BillingConfig,
    Review,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WizardField {
    TableName,
    HashKeyName,
    HashKeyType,
    HasRangeKey,
    RangeKeyName,
    RangeKeyType,
    BillingMode,
    ReadCapacity,
    WriteCapacity,
}

impl WizardState {
    pub fn new() -> Self {
        Self {
            step: WizardStep::BasicConfig,
            table_name: String::new(),
            hash_key_name: String::new(),
            hash_key_type: AttributeType::String,
            has_range_key: false,
            range_key_name: String::new(),
            range_key_type: AttributeType::String,
            billing_mode: BillingMode::OnDemand,
            read_capacity: "5".to_string(),
            write_capacity: "5".to_string(),
            current_field: WizardField::TableName,
        }
    }

    pub fn validate_basic_config(&self) -> Result<(), String> {
        if self.table_name.is_empty() {
            return Err("Table name is required".to_string());
        }
        if self.table_name.len() < 3 || self.table_name.len() > 255 {
            return Err("Table name must be 3-255 characters".to_string());
        }
        if self.hash_key_name.is_empty() {
            return Err("Hash key name is required".to_string());
        }
        if self.has_range_key && self.range_key_name.is_empty() {
            return Err("Range key name is required when enabled".to_string());
        }
        Ok(())
    }

    pub fn validate_billing_config(&self) -> Result<(), String> {
        if self.billing_mode == BillingMode::Provisioned {
            let read: Result<i64, _> = self.read_capacity.parse();
            let write: Result<i64, _> = self.write_capacity.parse();

            if read.is_err() || read.unwrap() < 1 {
                return Err("Read capacity must be a positive integer".to_string());
            }
            if write.is_err() || write.unwrap() < 1 {
                return Err("Write capacity must be a positive integer".to_string());
            }
        }
        Ok(())
    }

    pub fn to_table_config(&self) -> TableConfig {
        let range_key = if self.has_range_key {
            Some(KeySchema {
                name: self.range_key_name.clone(),
                key_type: self.range_key_type.clone(),
            })
        } else {
            None
        };

        let (read_capacity, write_capacity) = if self.billing_mode == BillingMode::Provisioned {
            (
                self.read_capacity.parse().ok(),
                self.write_capacity.parse().ok(),
            )
        } else {
            (None, None)
        };

        TableConfig {
            name: self.table_name.clone(),
            hash_key: KeySchema {
                name: self.hash_key_name.clone(),
                key_type: self.hash_key_type.clone(),
            },
            range_key,
            billing_mode: self.billing_mode.clone(),
            read_capacity,
            write_capacity,
        }
    }
}

impl App {
    pub fn new(log: AppLog) -> Self {
        let regions = vec![
            "us-east-1".to_string(),
            "us-east-2".to_string(),
            "us-west-1".to_string(),
            "us-west-2".to_string(),
            "af-south-1".to_string(),
            "ap-east-1".to_string(),
            "ap-south-1".to_string(),
            "ap-south-2".to_string(),
            "ap-southeast-1".to_string(),
            "ap-southeast-2".to_string(),
            "ap-southeast-3".to_string(),
            "ap-southeast-4".to_string(),
            "ap-northeast-1".to_string(),
            "ap-northeast-2".to_string(),
            "ap-northeast-3".to_string(),
            "ca-central-1".to_string(),
            "ca-west-1".to_string(),
            "eu-central-1".to_string(),
            "eu-central-2".to_string(),
            "eu-west-1".to_string(),
            "eu-west-2".to_string(),
            "eu-west-3".to_string(),
            "eu-south-1".to_string(),
            "eu-south-2".to_string(),
            "eu-north-1".to_string(),
            "me-south-1".to_string(),
            "me-central-1".to_string(),
            "sa-east-1".to_string(),
        ];

        let filtered_regions = regions.clone();

        Self {
            state: AppState::RegularTablesView,
            table_names: Vec::new(),
            tables: Vec::new(),
            table_state: TableState::default(),
            selected_table_index: None,
            current_items: Vec::new(),
            items_table_state: TableState::default(),
            selected_item_index: None,
            pagination: PaginationState::new(),
            current_region: "us-east-1".to_string(),
            available_regions: regions,
            region_filter: String::new(),
            filtered_regions,
            region_selector_state: ListState::default(),
            wizard_state: WizardState::new(),
            item_editor: None,
            is_loading: false,
            status_message: String::new(),
            log,
            pending_save_item: false,
            pending_delete_item: false,
            pending_delete_table: None,
            pending_refresh: false,
        }
    }

    pub fn log(&self, level: LogLevel, message: String) {
        if let Ok(mut log) = self.log.lock() {
            log.push(LogEntry::new(level, message));
        }
    }

    pub fn selected_table(&self) -> Option<&DynamoTable> {
        self.selected_table_index
            .and_then(|i| self.tables.get(i))
            .and_then(|opt| opt.as_ref())
    }

    pub fn selected_item(&self) -> Option<&DynamoItem> {
        self.selected_item_index
            .and_then(|i| self.current_items.get(i))
    }

    pub fn update_region_filter(&mut self, filter: String) {
        self.region_filter = filter;
        self.filtered_regions = self
            .available_regions
            .iter()
            .filter(|r| {
                if self.region_filter.is_empty() {
                    true
                } else {
                    r.contains(&self.region_filter)
                }
            })
            .cloned()
            .collect();

        // Reset selection to first item
        if !self.filtered_regions.is_empty() {
            self.region_selector_state.select(Some(0));
        }
    }

    pub fn enter_items_view(&mut self) {
        self.state = AppState::TableItemsView;
        self.current_items.clear();
        self.items_table_state.select(Some(0));
        self.selected_item_index = Some(0);
        self.pagination = PaginationState::new();
    }

    pub fn exit_items_view(&mut self) {
        self.state = AppState::RegularTablesView;
        self.current_items.clear();
        self.pagination = PaginationState::new();
    }

    pub fn start_create_item(&mut self, table: &DynamoTable) {
        let mut attributes = Vec::new();

        // Add hash key
        attributes.push((
            table.hash_key.name.clone(),
            AttributeValue::String(String::new()),
        ));

        // Add range key if present
        if let Some(range_key) = &table.range_key {
            attributes.push((
                range_key.name.clone(),
                AttributeValue::String(String::new()),
            ));
        }

        self.item_editor = Some(ItemEditorState {
            mode: EditorMode::Create,
            table_name: table.name.clone(),
            hash_key_schema: table.hash_key.clone(),
            range_key_schema: table.range_key.clone(),
            attributes,
            selected_attribute_index: 0,
            editing_field: None,
            input_buffer: String::new(),
        });
        self.state = AppState::ItemEditor;
    }

    pub fn start_edit_item(&mut self, table: &DynamoTable, item: &DynamoItem) {
        let mut attributes: Vec<(String, AttributeValue)> =
            item.attributes.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // Sort to put keys first
        attributes.sort_by(|a, b| {
            let a_is_key = a.0 == table.hash_key.name
                || table.range_key.as_ref().map(|k| &k.name) == Some(&a.0);
            let b_is_key = b.0 == table.hash_key.name
                || table.range_key.as_ref().map(|k| &k.name) == Some(&b.0);

            match (a_is_key, b_is_key) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.cmp(&b.0),
            }
        });

        self.item_editor = Some(ItemEditorState {
            mode: EditorMode::Edit,
            table_name: table.name.clone(),
            hash_key_schema: table.hash_key.clone(),
            range_key_schema: table.range_key.clone(),
            attributes,
            selected_attribute_index: 0,
            editing_field: None,
            input_buffer: String::new(),
        });
        self.state = AppState::ItemEditor;
    }

    pub fn exit_item_editor(&mut self) {
        self.item_editor = None;
        self.state = AppState::TableItemsView;
    }
}
