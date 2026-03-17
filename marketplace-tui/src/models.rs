#![allow(dead_code)]
use std::collections::HashMap;

/// Visibility state of a product or delivery option
#[derive(Clone, Debug, PartialEq)]
pub enum Visibility {
    Public,
    Limited,
    Restricted,
    Draft,
    Unknown(String),
}

impl Visibility {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Public" => Visibility::Public,
            "Limited" => Visibility::Limited,
            "Restricted" => Visibility::Restricted,
            "Draft" => Visibility::Draft,
            other => Visibility::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Visibility::Public => "Public",
            Visibility::Limited => "Limited",
            Visibility::Restricted => "Restricted",
            Visibility::Draft => "Draft",
            Visibility::Unknown(s) => s.as_str(),
        }
    }

    pub fn color_hint(&self) -> &'static str {
        match self {
            Visibility::Public => "green",
            Visibility::Limited => "yellow",
            Visibility::Restricted => "red",
            Visibility::Draft => "cyan",
            Visibility::Unknown(_) => "gray",
        }
    }
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Summary of an ML product returned by ListEntities
#[derive(Clone, Debug)]
pub struct ProductSummary {
    pub entity_id: String,
    pub entity_arn: String,
    pub name: String,
    pub visibility: Visibility,
    pub last_modified_date: Option<String>,
    pub product_code: Option<String>,
}

/// Full product detail returned by DescribeEntity (lazy loaded)
#[derive(Clone, Debug)]
pub struct ProductDetail {
    pub entity_id: String,
    pub entity_arn: String,
    pub title: String,
    pub short_description: String,
    pub long_description: String,
    pub logo_url: String,
    pub support_description: String,
    pub support_resources: String,
    pub product_code: String,
    pub visibility: Visibility,
    pub categories: Vec<String>,
    pub search_keywords: Vec<String>,
    pub versions: Vec<DeliveryOption>,
    pub pricing_terms: Vec<PricingTerm>,
    pub dimensions: Vec<PricingDimension>,
    pub allowed_accounts: Vec<String>,
    pub refund_policy: String,
    pub tags: HashMap<String, String>,
    pub last_modified_date: Option<String>,
    /// Raw JSON string returned by DescribeEntity — logged on load for debugging
    pub raw_details: String,
}

/// A delivery option (version) of a product
#[derive(Clone, Debug)]
pub struct DeliveryOption {
    /// Version-level ID (Versions[n].Id)
    pub id: String,
    /// Delivery-option-level ID (Versions[n].DeliveryOptions[0].Id) — used for update API calls
    pub delivery_option_id: Option<String>,
    pub title: String,
    pub release_notes: String,
    pub visibility: Visibility,
    pub model_package_arn: Option<String>,
    pub created_date: Option<String>,
}

/// Pricing term from a `Terms` array (rate-card style)
#[derive(Clone, Debug)]
pub struct PricingTerm {
    pub term_type: String,
    pub price_per_unit: Option<String>,
    pub currency_code: Option<String>,
    pub dimension_key: Option<String>,
}

impl PricingTerm {
    pub fn display(&self) -> String {
        match self.term_type.as_str() {
            "UsageBasedPricingTerm" => {
                let price = self.price_per_unit.as_deref().unwrap_or("N/A");
                let currency = self.currency_code.as_deref().unwrap_or("USD");
                let dim = self.dimension_key.as_deref().unwrap_or("unit");
                format!("Usage: ${} {} per {}", price, currency, dim)
            }
            "FreeTrialPricingTerm" => "Free Trial".to_string(),
            "StandardContractForAWSMarketplace" => "Standard AWS Marketplace EULA".to_string(),
            other => other.to_string(),
        }
    }
}

/// A pricing dimension from the `Dimensions` array (metered/usage-based billing)
#[derive(Clone, Debug)]
pub struct PricingDimension {
    pub name: String,
    pub description: String,
    pub key: String,
    pub unit: String,
    pub pricing_types: Vec<String>,
}

impl PricingDimension {
    pub fn display(&self) -> String {
        let types = if self.pricing_types.is_empty() {
            String::new()
        } else {
            format!("[{}]  ", self.pricing_types.join(", "))
        };
        format!("{}{}  ({}/{})", types, self.name, self.unit, self.key)
    }
}

/// Tracks an in-flight changeset submitted to AWS Marketplace
#[derive(Clone, Debug)]
pub struct PendingChangeset {
    pub change_set_id: String,
    pub change_set_name: String,
    pub operation: String,
    pub product_entity_id: String,
    pub submitted_at: String,
    pub status: ChangesetStatus,
    pub failure_description: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChangesetStatus {
    Preparing,
    Applying,
    Succeeded,
    Cancelled,
    Failed,
    Unknown,
}

impl ChangesetStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "PREPARING" => ChangesetStatus::Preparing,
            "APPLYING" => ChangesetStatus::Applying,
            "SUCCEEDED" => ChangesetStatus::Succeeded,
            "CANCELLED" => ChangesetStatus::Cancelled,
            "FAILED" => ChangesetStatus::Failed,
            _ => ChangesetStatus::Unknown,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ChangesetStatus::Succeeded | ChangesetStatus::Cancelled | ChangesetStatus::Failed
        )
    }

    pub fn color_hint(&self) -> &'static str {
        match self {
            ChangesetStatus::Succeeded => "green",
            ChangesetStatus::Failed => "red",
            ChangesetStatus::Cancelled => "yellow",
            ChangesetStatus::Preparing | ChangesetStatus::Applying => "cyan",
            ChangesetStatus::Unknown => "gray",
        }
    }
}

impl std::fmt::Display for ChangesetStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangesetStatus::Preparing => write!(f, "Preparing"),
            ChangesetStatus::Applying => write!(f, "Applying..."),
            ChangesetStatus::Succeeded => write!(f, "Succeeded"),
            ChangesetStatus::Cancelled => write!(f, "Cancelled"),
            ChangesetStatus::Failed => write!(f, "Failed"),
            ChangesetStatus::Unknown => write!(f, "Unknown"),
        }
    }
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

/// Config for creating a new product (built by wizard)
#[derive(Clone, Debug)]
pub struct CreateProductConfig {
    pub title: String,
    pub short_description: String,
    pub long_description: String,
    pub support_description: String,
    pub refund_policy: String,
    pub categories: Vec<String>,
    pub search_keywords: Vec<String>,
    pub logo_url: String,
}

/// Config for adding a new version (built by wizard)
#[derive(Clone, Debug)]
pub struct CreateVersionConfig {
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
}
