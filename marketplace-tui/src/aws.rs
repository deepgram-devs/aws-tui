#![allow(dead_code)]
use crate::models::*;
use anyhow::{anyhow, Result};
use aws_config::BehaviorVersion;
use aws_sdk_marketplacecatalog::types::{Change, Entity};
use aws_sdk_marketplacecatalog::Client;
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;

const CATALOG: &str = "AWSMarketplace";
const ENTITY_TYPE: &str = "MachineLearningProduct";
const ENTITY_TYPE_VERSIONED: &str = "MachineLearningProduct@1.0";

pub struct AwsManager {
    client: Client,
    region: String,
}

impl AwsManager {
    pub async fn new(region: &str) -> Result<Self> {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = Client::new(&config);
        Ok(Self { client, region: region.to_string() })
    }

    pub fn current_region(&self) -> &str {
        &self.region
    }

    // ─── List / Describe ────────────────────────────────────────────────────

    /// List all ML product summaries (handles pagination)
    pub async fn list_products(&self) -> Result<Vec<ProductSummary>> {
        let mut products = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = self
                .client
                .list_entities()
                .catalog(CATALOG)
                .entity_type(ENTITY_TYPE);

            if let Some(token) = next_token {
                req = req.next_token(token);
            }

            let resp = req.send().await?;
            let summaries = resp.entity_summary_list();

            for s in summaries {
                let entity_id = s.entity_id().unwrap_or_default().to_string();
                let name = s.name().unwrap_or_default().to_string();
                let arn = s.entity_arn().unwrap_or_default().to_string();
                let visibility = Visibility::from_str(s.visibility().unwrap_or_default());
                let last_modified = s.last_modified_date().map(|d| d.to_string());

                products.push(ProductSummary {
                    entity_id,
                    entity_arn: arn,
                    name,
                    visibility,
                    last_modified_date: last_modified,
                    product_code: None,
                });
            }

            next_token = resp.next_token().map(|t| t.to_string());
            if next_token.is_none() {
                break;
            }
        }

        Ok(products)
    }

    /// Get full product detail by entity ID.
    ///
    /// The AWS Marketplace Catalog API nests ML product fields under sections
    /// such as `Description`, `SupportInformation`, `Targeting`, etc.  We try
    /// the nested path first, then fall back to a flat root-level key so the
    /// code handles variation across API versions gracefully.
    pub async fn describe_product(&self, entity_id: &str) -> Result<ProductDetail> {
        let resp = self
            .client
            .describe_entity()
            .catalog(CATALOG)
            .entity_id(entity_id)
            .send()
            .await?;

        let details_str = resp.details().unwrap_or("{}").to_string();
        let detail: Value =
            serde_json::from_str(&details_str).unwrap_or_else(|_| json!({}));

        let arn = resp.entity_arn().unwrap_or_default().to_string();
        let last_modified = resp.last_modified_date().map(|d| d.to_string());

        // ── Core product fields ──────────────────────────────────────────────
        // The API places these under a `Description` section; fall back to flat.
        let title = multi_str(&detail, &[
            "Description.ProductTitle", "ProductTitle",
        ]);
        let short_desc = multi_str(&detail, &[
            "Description.ShortDescription", "ShortDescription",
        ]);
        let long_desc = multi_str(&detail, &[
            "Description.LongDescription", "LongDescription",
        ]);
        let product_code = multi_str(&detail, &[
            "Description.ProductCode", "ProductCode",
        ]);
        let visibility = Visibility::from_str(&multi_str(&detail, &[
            "Description.Visibility", "Description.ProductState", "Visibility",
        ]));
        let categories = multi_arr(&detail, &[
            "Description.Categories", "Categories",
        ]);
        let search_keywords = multi_arr(&detail, &[
            "Description.SearchKeywords", "SearchKeywords",
        ]);

        // ── Logo / promotional ───────────────────────────────────────────────
        let logo_url = multi_str(&detail, &[
            "PromotionalResources.LogoUrl", "Description.LogoUrl", "LogoUrl",
        ]);

        // ── Support ──────────────────────────────────────────────────────────
        // The inner key varies: "SupportDescription", "Description", or "Text"
        let support_desc = multi_str(&detail, &[
            "SupportInformation.SupportDescription",
            "SupportInformation.Description",
            "SupportInformation.Text",
            "SupportDescription",
        ]);
        let support_resources = multi_str(&detail, &[
            "SupportInformation.SupportResources",
            "SupportInformation.Resources",
            "SupportResources",
        ]);

        // ── Refund policy ────────────────────────────────────────────────────
        let refund_policy = multi_str(&detail, &[
            "RefundPolicy",
            "Description.RefundPolicy",
            "SupportInformation.RefundPolicy",
        ]);

        // ── Versions / delivery options ──────────────────────────────────────
        let versions = parse_delivery_options(&detail);

        // ── Pricing terms (rate-card style) ──────────────────────────────────
        let pricing_terms = parse_pricing_terms(&detail);

        // ── Pricing dimensions (metered billing) ─────────────────────────────
        let dimensions = parse_dimensions(&detail);

        // ── Allowlist ────────────────────────────────────────────────────────
        let allowed_accounts = parse_allowed_accounts(&detail);

        // ── Tags ─────────────────────────────────────────────────────────────
        let tags = parse_tags(&detail);

        Ok(ProductDetail {
            entity_id: entity_id.to_string(),
            entity_arn: arn,
            title,
            short_description: short_desc,
            long_description: long_desc,
            logo_url,
            support_description: support_desc,
            support_resources,
            product_code,
            visibility,
            categories,
            search_keywords,
            versions,
            pricing_terms,
            dimensions,
            allowed_accounts,
            refund_policy,
            tags,
            last_modified_date: last_modified,
            raw_details: details_str,
        })
    }

    // ─── Mutations via StartChangeSet ────────────────────────────────────────

    /// Create a new ML product; returns the submitted changeset ID
    pub async fn create_product(&self, config: &CreateProductConfig) -> Result<String> {
        let created_date = Utc::now().format("%Y-%m-%d").to_string();

        let details = json!({
            "ProductTitle": config.title,
            "ShortDescription": config.short_description,
            "LongDescription": config.long_description,
            "SupportDescription": config.support_description,
            "RefundPolicy": config.refund_policy,
            "Categories": config.categories,
            "SearchKeywords": config.search_keywords,
            "LogoUrl": config.logo_url,
            "Tags": { "CreatedDate": created_date }
        });

        let change = build_change(
            "CreateProduct",
            ENTITY_TYPE_VERSIONED,
            None,
            details.to_string(),
        )?;

        let resp = self
            .client
            .start_change_set()
            .catalog(CATALOG)
            .change_set_name(format!("Create product: {}", &config.title))
            .change_set(change)
            .send()
            .await?;

        resp.change_set_id()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No changeset ID returned"))
    }

    /// Update product information metadata
    pub async fn update_product_info(
        &self,
        entity_id: &str,
        fields: &HashMap<String, String>,
    ) -> Result<String> {
        let mut details = json!({});

        // Map from our internal field labels to API field names
        let field_map: &[(&str, &str)] = &[
            ("Title", "ProductTitle"),
            ("Short Description", "ShortDescription"),
            ("Long Description", "LongDescription"),
            ("Support Description", "SupportDescription"),
            ("Logo URL", "LogoUrl"),
            ("Categories (comma-separated)", "Categories"),
            ("Keywords (comma-separated)", "SearchKeywords"),
        ];

        for (app_label, api_key) in field_map {
            if let Some(value) = fields.get(*app_label) {
                if *api_key == "Categories" || *api_key == "SearchKeywords" {
                    let items: Vec<&str> =
                        value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                    details[api_key] = json!(items);
                } else {
                    details[api_key] = json!(value);
                }
            }
        }

        let change = build_change(
            "UpdateInformation",
            ENTITY_TYPE_VERSIONED,
            Some(entity_id),
            details.to_string(),
        )?;

        let resp = self
            .client
            .start_change_set()
            .catalog(CATALOG)
            .change_set_name(format!("Update info: {}", entity_id))
            .change_set(change)
            .send()
            .await?;

        resp.change_set_id()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No changeset ID returned"))
    }

    /// Add a new delivery option (version) to a product.
    ///
    /// Required structure per the API:
    ///   { "Version": { "VersionTitle": "...", "ReleaseNotes": "..." },
    ///     "DeliveryOptions": [{ "Details": {
    ///       "SageMakerModelPackageDeliveryOptionDetails": { "ModelPackageArn": "..." }
    ///     }}] }
    pub async fn add_version(&self, config: &CreateVersionConfig) -> Result<String> {
        let details = json!({
            "Version": {
                "VersionTitle": config.title,
                "ReleaseNotes": config.release_notes
            },
            "DeliveryOptions": [{
                "Details": {
                    "SageMakerModelPackageDeliveryOptionDetails": {
                        "SageMakerModelPackageArn": config.sagemaker_model_package_arn,
                        "AccessRoleArn": config.access_role_arn,
                        "UsageInstructions": config.usage_instructions,
                        "RecommendedInstanceTypes": {
                            "RealtimeInference": config.realtime_instance_type,
                            "BatchTransform": config.batch_instance_type
                        },
                        "RepositoryUrl": config.repository_url,
                        "SampleNotebookUrl": config.sample_notebook_url,
                        "InputProperties": {
                            "Description": config.input_description,
                            "SampleInput": {
                                "RealtimeInferenceUrl": config.sample_input_url,
                                "BatchTransformUrl": config.sample_input_url
                            }
                        },
                        "OutputProperties": {
                            "Description": config.output_description,
                            "SampleOutput": {
                                "RealtimeInferenceText": config.sample_output,
                                "BatchTransformText": config.sample_output
                            }
                        }
                    }
                }
            }]
        });

        let change = build_change(
            "AddDeliveryOptions",
            ENTITY_TYPE_VERSIONED,
            Some(&config.product_entity_id),
            details.to_string(),
        )?;

        let resp = self
            .client
            .start_change_set()
            .catalog(CATALOG)
            .change_set_name(format!("Add version: {}", &config.title))
            .change_set(change)
            .send()
            .await?;

        resp.change_set_id()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No changeset ID returned"))
    }

    /// Restrict (deprecate) a delivery option by ID
    pub async fn restrict_version(
        &self,
        entity_id: &str,
        delivery_option_id: &str,
    ) -> Result<String> {
        let details = json!({ "DeliveryOptionIds": [delivery_option_id] });

        let change = build_change(
            "RestrictDeliveryOptions",
            ENTITY_TYPE_VERSIONED,
            Some(entity_id),
            details.to_string(),
        )?;

        let resp = self
            .client
            .start_change_set()
            .catalog(CATALOG)
            .change_set_name(format!("Restrict version: {}", delivery_option_id))
            .change_set(change)
            .send()
            .await?;

        resp.change_set_id()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No changeset ID returned"))
    }

    /// Update allowed accounts (allowlist) for a product
    pub async fn update_allowed_accounts(
        &self,
        entity_id: &str,
        account_ids: &[String],
    ) -> Result<String> {
        let details = json!({
            "PositiveTargeting": {
                "BuyerAccounts": account_ids
            }
        });

        let change = build_change(
            "UpdateTargeting",
            ENTITY_TYPE_VERSIONED,
            Some(entity_id),
            details.to_string(),
        )?;

        let resp = self
            .client
            .start_change_set()
            .catalog(CATALOG)
            .change_set_name(format!("Update allowlist: {}", entity_id))
            .change_set(change)
            .send()
            .await?;

        resp.change_set_id()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No changeset ID returned"))
    }

    /// Update usage-based pricing for a product
    pub async fn update_pricing(
        &self,
        entity_id: &str,
        price: &str,
        currency: &str,
        dimension_key: &str,
    ) -> Result<String> {
        let details = json!({
            "Terms": [
                {
                    "Type": "UsageBasedPricingTerm",
                    "CurrencyCode": currency,
                    "RateCards": [{
                        "Selector": { "Type": "BuyerAccountType", "Value": "ALL" },
                        "RateCard": [{
                            "DimensionKey": dimension_key,
                            "Price": price
                        }]
                    }]
                },
                {
                    "Type": "LegalTerm",
                    "Documents": [{
                        "Type": "StandardEula",
                        "Url": "https://s3.amazonaws.com/EULA/standardEULAmazon.txt"
                    }]
                }
            ]
        });

        let change = build_change(
            "UpdatePricingTerms",
            ENTITY_TYPE_VERSIONED,
            Some(entity_id),
            details.to_string(),
        )?;

        let resp = self
            .client
            .start_change_set()
            .catalog(CATALOG)
            .change_set_name(format!("Update pricing: {}", entity_id))
            .change_set(change)
            .send()
            .await?;

        resp.change_set_id()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No changeset ID returned"))
    }

    /// Update editable fields (title, release notes) on an existing version.
    /// Uses the delivery-option ID if available, falling back to the version ID.
    pub async fn update_version_metadata(
        &self,
        entity_id: &str,
        delivery_option_id: &str,
        title: &str,
        release_notes: &str,
    ) -> Result<String> {
        let details = json!({
            "DeliveryOptions": [{
                "Id": delivery_option_id,
                "Title": title,
                "ReleaseNotes": release_notes
            }]
        });

        let change = build_change(
            "UpdateDeliveryOptions",
            ENTITY_TYPE_VERSIONED,
            Some(entity_id),
            details.to_string(),
        )?;

        let resp = self
            .client
            .start_change_set()
            .catalog(CATALOG)
            .change_set_name(format!(
                "Update version: {}",
                &delivery_option_id[..8.min(delivery_option_id.len())]
            ))
            .change_set(change)
            .send()
            .await?;

        resp.change_set_id()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No changeset ID returned"))
    }

    // ─── Changeset Status Polling ─────────────────────────────────────────────

    /// Poll the status of a changeset
    pub async fn describe_changeset(
        &self,
        change_set_id: &str,
    ) -> Result<(ChangesetStatus, Option<String>)> {
        let resp = self
            .client
            .describe_change_set()
            .catalog(CATALOG)
            .change_set_id(change_set_id)
            .send()
            .await?;

        let status_str = resp
            .status()
            .map(|s| s.as_str().to_uppercase())
            .unwrap_or_default();
        let status = ChangesetStatus::from_str(&status_str);
        let failure = resp.failure_description().map(|s| s.to_string());

        Ok((status, failure))
    }
}

// ─── Builder helpers ──────────────────────────────────────────────────────────

/// Build a Change with an optional entity identifier.
fn build_change(
    change_type: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    details_json: String,
) -> Result<Change> {
    let mut entity_builder =
        Entity::builder().r#type(entity_type);

    if let Some(id) = entity_id {
        entity_builder = entity_builder.identifier(id);
    }

    let entity = entity_builder.build()?;

    let change = Change::builder()
        .change_type(change_type)
        .entity(entity)
        .details(details_json)
        .build()?;

    Ok(change)
}

// ─── JSON navigation helpers ──────────────────────────────────────────────────

/// Navigate a dot-separated path through a JSON value.
/// Returns `&Value::Null` when any segment is missing.
fn get_path<'a>(v: &'a Value, path: &str) -> &'a Value {
    let mut cur = v;
    for key in path.split('.') {
        match cur.get(key) {
            Some(next) => cur = next,
            None => return &Value::Null,
        }
    }
    cur
}

/// Extract a string, trying each dot-path in order until a non-empty value is found.
fn multi_str(v: &Value, paths: &[&str]) -> String {
    for path in paths {
        if let Some(s) = get_path(v, path).as_str() {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

/// Extract a string array, trying each dot-path in order until a non-empty array is found.
fn multi_arr(v: &Value, paths: &[&str]) -> Vec<String> {
    for path in paths {
        if let Some(arr) = get_path(v, path).as_array() {
            let result: Vec<String> = arr
                .iter()
                .filter_map(|x| x.as_str())
                .map(|s| s.to_string())
                .collect();
            if !result.is_empty() {
                return result;
            }
        }
    }
    Vec::new()
}

// ─── Domain-specific parsers ──────────────────────────────────────────────────

fn parse_delivery_options(v: &Value) -> Vec<DeliveryOption> {
    // Try every key the API has been known to use for versions
    let arr = ["Versions", "DeliveryOptions", "Description.Versions"]
        .iter()
        .find_map(|path| {
            let node = get_path(v, path);
            node.as_array().filter(|a| !a.is_empty()).cloned()
        })
        .unwrap_or_default();

    arr.iter()
        .map(|ver| {
            let title = multi_str(ver, &["VersionTitle", "Title"]);
            let id = multi_str(ver, &["Id", "VersionId"]);
            let release_notes = multi_str(ver, &["ReleaseNotes", "UpgradeInstructions"]);

            // Visibility lives on the first DeliveryOption entry inside the version,
            // not on the version object itself
            let visibility_str = multi_str(ver, &["Visibility", "Status"])
                .is_empty()
                .then(|| {
                    ver.get("DeliveryOptions")
                        .and_then(|d| d.as_array())
                        .and_then(|a| a.first())
                        .and_then(|opt| opt.get("Visibility"))
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string()
                })
                .unwrap_or_else(|| multi_str(ver, &["Visibility", "Status"]));
            let visibility = Visibility::from_str(&visibility_str);

            let created_date = ["CreationDate", "CreatedDate", "CreationTime"]
                .iter()
                .find_map(|k| ver.get(*k).and_then(|x| x.as_str()).map(|s| s.to_string()));

            // Model package ARN is in Sources[0].ModelPackageArn
            let model_package_arn = ver
                .get("Sources")
                .and_then(|s| s.as_array())
                .and_then(|a| a.first())
                .and_then(|s| s.get("ModelPackageArn"))
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());

            // Capture the nested delivery-option ID for use in update API calls
            let delivery_option_id = ver
                .get("DeliveryOptions")
                .and_then(|d| d.as_array())
                .and_then(|a| a.first())
                .and_then(|opt| opt.get("Id"))
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());

            DeliveryOption {
                id,
                delivery_option_id,
                title,
                release_notes,
                visibility,
                model_package_arn,
                created_date,
            }
        })
        .collect()
}

fn parse_pricing_terms(v: &Value) -> Vec<PricingTerm> {
    let arr = ["Terms", "PricingTerms"]
        .iter()
        .find_map(|key| get_path(v, key).as_array().cloned())
        .unwrap_or_default();

    arr.iter()
        .map(|term| {
            let term_type = multi_str(term, &["Type", "TermType"]);
            let currency_code = term
                .get("CurrencyCode")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());

            // Dig into RateCards → first entry → RateCard → first entry
            let (price_per_unit, dimension_key) = term
                .get("RateCards")
                .and_then(|rc| rc.as_array())
                .and_then(|rc| rc.first())
                .and_then(|rc| rc.get("RateCard"))
                .and_then(|rr| rr.as_array())
                .and_then(|rr| rr.first())
                .map(|entry| {
                    let price = entry.get("Price").and_then(|x| x.as_str()).map(|s| s.to_string());
                    let dim = entry.get("DimensionKey").and_then(|x| x.as_str()).map(|s| s.to_string());
                    (price, dim)
                })
                .unwrap_or((None, None));

            PricingTerm { term_type, price_per_unit, currency_code, dimension_key }
        })
        .collect()
}

fn parse_dimensions(v: &Value) -> Vec<PricingDimension> {
    let arr = get_path(v, "Dimensions")
        .as_array()
        .cloned()
        .unwrap_or_default();

    arr.iter()
        .map(|d| {
            let name = multi_str(d, &["Name"]);
            let description = multi_str(d, &["Description"]);
            let key = multi_str(d, &["Key"]);
            let unit = multi_str(d, &["Unit"]);
            let pricing_types = d
                .get("Types")
                .and_then(|t| t.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            PricingDimension { name, description, key, unit, pricing_types }
        })
        .collect()
}

fn parse_allowed_accounts(v: &Value) -> Vec<String> {
    // Actual API field is BuyerAccounts (not BuyerAccountIds)
    let paths: &[&str] = &[
        "Targeting.PositiveTargeting.BuyerAccounts",
        "Targeting.PositiveTargeting.BuyerAccountIds",
        "Targeting.BuyerAccounts",
        "Targeting.BuyerAccountIds",
        "BuyerAccounts",
        "BuyerAccountIds",
    ];
    for path in paths {
        if let Some(arr) = get_path(v, path).as_array() {
            let ids: Vec<String> = arr
                .iter()
                .filter_map(|x| x.as_str())
                .map(|s| s.to_string())
                .collect();
            if !ids.is_empty() {
                return ids;
            }
        }
    }
    Vec::new()
}

fn parse_tags(v: &Value) -> HashMap<String, String> {
    v.get("Tags")
        .and_then(|t| t.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}
