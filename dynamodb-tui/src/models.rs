use aws_sdk_dynamodb::types::AttributeValue as AwsAttributeValue;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct DynamoTable {
    pub name: String,
    pub hash_key: KeySchema,
    pub range_key: Option<KeySchema>,
    pub billing_mode: BillingMode,
    pub read_capacity: Option<i64>,
    pub write_capacity: Option<i64>,
    pub table_status: String,
    pub item_count: Option<i64>,
    pub table_size_bytes: Option<i64>,
    pub region: String,
}

#[derive(Clone, Debug)]
pub struct KeySchema {
    pub name: String,
    pub key_type: AttributeType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BillingMode {
    OnDemand,
    Provisioned,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttributeType {
    String,
    Number,
    Binary,
}

impl AttributeType {
    pub fn to_aws_type(&self) -> String {
        match self {
            AttributeType::String => "S".to_string(),
            AttributeType::Number => "N".to_string(),
            AttributeType::Binary => "B".to_string(),
        }
    }

    pub fn from_aws_type(s: &str) -> Self {
        match s {
            "S" => AttributeType::String,
            "N" => AttributeType::Number,
            "B" => AttributeType::Binary,
            _ => AttributeType::String,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DynamoItem {
    pub attributes: HashMap<String, AttributeValue>,
    pub hash_key_value: AttributeValue,
    pub range_key_value: Option<AttributeValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttributeValue {
    String(String),
    Number(String),
    Binary(Vec<u8>),
    Boolean(bool),
    Null,
    StringSet(Vec<String>),
    NumberSet(Vec<String>),
    BinarySet(Vec<Vec<u8>>),
    List(Vec<AttributeValue>),
    Map(HashMap<String, AttributeValue>),
}

impl AttributeValue {
    pub fn to_aws(&self) -> AwsAttributeValue {
        match self {
            AttributeValue::String(s) => AwsAttributeValue::S(s.clone()),
            AttributeValue::Number(n) => AwsAttributeValue::N(n.clone()),
            AttributeValue::Binary(b) => AwsAttributeValue::B(b.clone().into()),
            AttributeValue::Boolean(b) => AwsAttributeValue::Bool(*b),
            AttributeValue::Null => AwsAttributeValue::Null(true),
            AttributeValue::StringSet(ss) => AwsAttributeValue::Ss(ss.clone()),
            AttributeValue::NumberSet(ns) => AwsAttributeValue::Ns(ns.clone()),
            AttributeValue::BinarySet(bs) => {
                AwsAttributeValue::Bs(bs.iter().map(|b| b.clone().into()).collect())
            }
            AttributeValue::List(l) => {
                AwsAttributeValue::L(l.iter().map(|v| v.to_aws()).collect())
            }
            AttributeValue::Map(m) => {
                let mut aws_map = HashMap::new();
                for (k, v) in m.iter() {
                    aws_map.insert(k.clone(), v.to_aws());
                }
                AwsAttributeValue::M(aws_map)
            }
        }
    }

    pub fn from_aws(aws: &AwsAttributeValue) -> Self {
        match aws {
            AwsAttributeValue::S(s) => AttributeValue::String(s.clone()),
            AwsAttributeValue::N(n) => AttributeValue::Number(n.clone()),
            AwsAttributeValue::B(b) => AttributeValue::Binary(b.as_ref().to_vec()),
            AwsAttributeValue::Bool(b) => AttributeValue::Boolean(*b),
            AwsAttributeValue::Null(_) => AttributeValue::Null,
            AwsAttributeValue::Ss(ss) => AttributeValue::StringSet(ss.clone()),
            AwsAttributeValue::Ns(ns) => AttributeValue::NumberSet(ns.clone()),
            AwsAttributeValue::Bs(bs) => {
                AttributeValue::BinarySet(bs.iter().map(|b| b.as_ref().to_vec()).collect())
            }
            AwsAttributeValue::L(l) => {
                AttributeValue::List(l.iter().map(|v| AttributeValue::from_aws(v)).collect())
            }
            AwsAttributeValue::M(m) => {
                let mut map = HashMap::new();
                for (k, v) in m.iter() {
                    map.insert(k.clone(), AttributeValue::from_aws(v));
                }
                AttributeValue::Map(map)
            }
            _ => AttributeValue::Null,
        }
    }

    pub fn display_short(&self, max_len: usize) -> String {
        let full = self.display_full();
        if full.len() > max_len {
            format!("{}...", &full[..max_len.min(full.len())])
        } else {
            full
        }
    }

    pub fn display_full(&self) -> String {
        match self {
            AttributeValue::String(s) => format!("\"{}\"", s),
            AttributeValue::Number(n) => n.clone(),
            AttributeValue::Binary(b) => {
                if b.is_empty() {
                    "<empty binary>".to_string()
                } else if b.len() <= 16 {
                    // Show hex for small binary
                    let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
                    format!("0x{}", hex)
                } else {
                    format!("<binary {} bytes>", b.len())
                }
            }
            AttributeValue::Boolean(b) => b.to_string(),
            AttributeValue::Null => "null".to_string(),
            AttributeValue::StringSet(ss) => format!("{:?}", ss),
            AttributeValue::NumberSet(ns) => format!("{:?}", ns),
            AttributeValue::BinarySet(bs) => format!("<binary set {} items>", bs.len()),
            AttributeValue::List(l) => format!("[{} items]", l.len()),
            AttributeValue::Map(m) => format!("{{{}  fields}}", m.len()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Region {
    pub name: String,
}

#[derive(Clone, Debug)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            level,
            message,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ItemEditorState {
    pub mode: EditorMode,
    pub table_name: String,
    pub hash_key_schema: KeySchema,
    pub range_key_schema: Option<KeySchema>,
    pub attributes: Vec<(String, AttributeValue)>,
    pub selected_attribute_index: usize,
    pub editing_field: Option<EditingField>,
    pub input_buffer: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditorMode {
    Create,
    Edit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditingField {
    Key,
    Value,
    Type,
}

#[derive(Clone, Debug)]
pub struct TableConfig {
    pub name: String,
    pub hash_key: KeySchema,
    pub range_key: Option<KeySchema>,
    pub billing_mode: BillingMode,
    pub read_capacity: Option<i64>,
    pub write_capacity: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct PaginationState {
    pub current_page: usize,
    pub last_evaluated_key: Option<HashMap<String, AwsAttributeValue>>,
    pub has_more_pages: bool,
    pub items_per_page: usize,
}

impl PaginationState {
    pub fn new() -> Self {
        Self {
            current_page: 1,
            last_evaluated_key: None,
            has_more_pages: false,
            items_per_page: 50,
        }
    }
}
