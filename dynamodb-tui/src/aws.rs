use crate::models::*;
use anyhow::{anyhow, Result};
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, BillingMode as AwsBillingMode, KeySchemaElement, KeyType,
    ProvisionedThroughput, ScalarAttributeType, AttributeValue as AwsAttributeValue,
};
use aws_sdk_dynamodb::Client as DynamoDbClient;
use std::collections::HashMap;

pub struct AwsManager {
    client: DynamoDbClient,
    region: String,
}

impl AwsManager {
    pub async fn new(region: &str) -> Result<Self> {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = DynamoDbClient::new(&config);

        Ok(Self {
            client,
            region: region.to_string(),
        })
    }

    pub async fn switch_region(&mut self, region: &str) -> Result<()> {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        self.client = DynamoDbClient::new(&config);
        self.region = region.to_string();
        Ok(())
    }

    pub fn current_region(&self) -> &str {
        &self.region
    }

    // List table names only (fast operation)
    pub async fn list_table_names(&self) -> Result<Vec<String>> {
        let resp = self.client.list_tables().send().await?;
        Ok(resp.table_names().to_vec())
    }

    // Describe a specific table (lazy loading)
    pub async fn describe_table(&self, table_name: &str) -> Result<DynamoTable> {
        let resp = self
            .client
            .describe_table()
            .table_name(table_name)
            .send()
            .await?;

        let table_desc = resp
            .table()
            .ok_or_else(|| anyhow!("No table description returned"))?;

        // Parse key schema
        let key_schema = table_desc.key_schema();
        let mut hash_key: Option<KeySchema> = None;
        let mut range_key: Option<KeySchema> = None;

        // Get attribute definitions for key types
        let attr_defs: HashMap<String, ScalarAttributeType> = table_desc
            .attribute_definitions()
            .iter()
            .map(|ad| {
                (
                    ad.attribute_name().to_string(),
                    ad.attribute_type().clone(),
                )
            })
            .collect();

        for key in key_schema {
            let key_name = key.attribute_name().to_string();
            let key_type = attr_defs
                .get(&key_name)
                .map(|t| match t {
                    ScalarAttributeType::S => AttributeType::String,
                    ScalarAttributeType::N => AttributeType::Number,
                    ScalarAttributeType::B => AttributeType::Binary,
                    _ => AttributeType::String,
                })
                .unwrap_or(AttributeType::String);

            let schema = KeySchema {
                name: key_name,
                key_type,
            };

            match key.key_type() {
                KeyType::Hash => hash_key = Some(schema),
                KeyType::Range => range_key = Some(schema),
                _ => {}
            }
        }

        let hash_key =
            hash_key.ok_or_else(|| anyhow!("Table {} has no hash key", table_name))?;

        // Parse billing mode
        let billing_mode = match table_desc.billing_mode_summary() {
            Some(summary) => match summary.billing_mode() {
                Some(AwsBillingMode::PayPerRequest) => BillingMode::OnDemand,
                Some(AwsBillingMode::Provisioned) => BillingMode::Provisioned,
                _ => BillingMode::OnDemand,
            },
            None => BillingMode::Provisioned, // Default for older tables
        };

        // Parse provisioned throughput
        let (read_capacity, write_capacity) = match table_desc.provisioned_throughput() {
            Some(pt) => (
                Some(pt.read_capacity_units().unwrap_or(0)),
                Some(pt.write_capacity_units().unwrap_or(0)),
            ),
            None => (None, None),
        };

        Ok(DynamoTable {
            name: table_name.to_string(),
            hash_key,
            range_key,
            billing_mode,
            read_capacity,
            write_capacity,
            table_status: table_desc
                .table_status()
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "Unknown".to_string()),
            item_count: table_desc.item_count(),
            table_size_bytes: table_desc.table_size_bytes(),
            region: self.region.clone(),
        })
    }

    // Create a new table
    pub async fn create_table(&self, config: TableConfig) -> Result<()> {
        let mut attribute_defs = vec![AttributeDefinition::builder()
            .attribute_name(&config.hash_key.name)
            .attribute_type(match config.hash_key.key_type {
                AttributeType::String => ScalarAttributeType::S,
                AttributeType::Number => ScalarAttributeType::N,
                AttributeType::Binary => ScalarAttributeType::B,
            })
            .build()?];

        let mut key_schema = vec![KeySchemaElement::builder()
            .attribute_name(&config.hash_key.name)
            .key_type(KeyType::Hash)
            .build()?];

        if let Some(range_key) = &config.range_key {
            attribute_defs.push(
                AttributeDefinition::builder()
                    .attribute_name(&range_key.name)
                    .attribute_type(match range_key.key_type {
                        AttributeType::String => ScalarAttributeType::S,
                        AttributeType::Number => ScalarAttributeType::N,
                        AttributeType::Binary => ScalarAttributeType::B,
                    })
                    .build()?,
            );

            key_schema.push(
                KeySchemaElement::builder()
                    .attribute_name(&range_key.name)
                    .key_type(KeyType::Range)
                    .build()?,
            );
        }

        let mut request = self
            .client
            .create_table()
            .table_name(&config.name)
            .set_attribute_definitions(Some(attribute_defs))
            .set_key_schema(Some(key_schema));

        match config.billing_mode {
            BillingMode::OnDemand => {
                request = request.billing_mode(AwsBillingMode::PayPerRequest);
            }
            BillingMode::Provisioned => {
                let throughput = ProvisionedThroughput::builder()
                    .read_capacity_units(config.read_capacity.unwrap_or(5))
                    .write_capacity_units(config.write_capacity.unwrap_or(5))
                    .build()?;
                request = request
                    .provisioned_throughput(throughput)
                    .billing_mode(AwsBillingMode::Provisioned);
            }
        }

        request.send().await?;
        Ok(())
    }

    // Delete a table
    pub async fn delete_table(&self, table_name: &str) -> Result<()> {
        self.client
            .delete_table()
            .table_name(table_name)
            .send()
            .await?;
        Ok(())
    }

    // Scan table for items with pagination
    pub async fn scan_table(
        &self,
        table_name: &str,
        limit: i32,
        exclusive_start_key: Option<HashMap<String, AwsAttributeValue>>,
    ) -> Result<(Vec<DynamoItem>, Option<HashMap<String, AwsAttributeValue>>)> {
        let mut request = self
            .client
            .scan()
            .table_name(table_name)
            .limit(limit);

        if let Some(start_key) = exclusive_start_key {
            request = request.set_exclusive_start_key(Some(start_key));
        }

        let resp = request.send().await?;

        // Get table schema for key identification
        let table = self.describe_table(table_name).await?;
        let hash_key_name = &table.hash_key.name;
        let range_key_name = table.range_key.as_ref().map(|k| k.name.clone());

        let items: Vec<DynamoItem> = resp
            .items()
            .iter()
            .filter_map(|item| {
                let mut attributes = HashMap::new();
                for (k, v) in item.iter() {
                    attributes.insert(k.clone(), AttributeValue::from_aws(v));
                }

                let hash_key_value = item
                    .get(hash_key_name)
                    .map(|v| AttributeValue::from_aws(v))?;

                let range_key_value = range_key_name
                    .as_ref()
                    .and_then(|name| item.get(name))
                    .map(|v| AttributeValue::from_aws(v));

                Some(DynamoItem {
                    attributes,
                    hash_key_value,
                    range_key_value,
                })
            })
            .collect();

        let last_evaluated_key = resp.last_evaluated_key().map(|k| k.clone());

        Ok((items, last_evaluated_key))
    }

    // Get a specific item
    pub async fn get_item(
        &self,
        table_name: &str,
        key: HashMap<String, AttributeValue>,
    ) -> Result<Option<DynamoItem>> {
        let aws_key: HashMap<String, AwsAttributeValue> =
            key.iter().map(|(k, v)| (k.clone(), v.to_aws())).collect();

        let resp = self
            .client
            .get_item()
            .table_name(table_name)
            .set_key(Some(aws_key))
            .send()
            .await?;

        if let Some(item) = resp.item() {
            // Get table schema for key identification
            let table = self.describe_table(table_name).await?;
            let hash_key_name = &table.hash_key.name;
            let range_key_name = table.range_key.as_ref().map(|k| k.name.clone());

            let mut attributes = HashMap::new();
            for (k, v) in item.iter() {
                attributes.insert(k.clone(), AttributeValue::from_aws(v));
            }

            let hash_key_value = item
                .get(hash_key_name)
                .map(|v| AttributeValue::from_aws(v))
                .ok_or_else(|| anyhow!("Item missing hash key"))?;

            let range_key_value = range_key_name
                .as_ref()
                .and_then(|name| item.get(name))
                .map(|v| AttributeValue::from_aws(v));

            Ok(Some(DynamoItem {
                attributes,
                hash_key_value,
                range_key_value,
            }))
        } else {
            Ok(None)
        }
    }

    // Put (create or replace) an item
    pub async fn put_item(
        &self,
        table_name: &str,
        item: HashMap<String, AttributeValue>,
    ) -> Result<()> {
        let aws_item: HashMap<String, AwsAttributeValue> =
            item.iter().map(|(k, v)| (k.clone(), v.to_aws())).collect();

        self.client
            .put_item()
            .table_name(table_name)
            .set_item(Some(aws_item))
            .send()
            .await?;

        Ok(())
    }

    // Delete an item
    pub async fn delete_item(
        &self,
        table_name: &str,
        key: HashMap<String, AttributeValue>,
    ) -> Result<()> {
        let aws_key: HashMap<String, AwsAttributeValue> =
            key.iter().map(|(k, v)| (k.clone(), v.to_aws())).collect();

        self.client
            .delete_item()
            .table_name(table_name)
            .set_key(Some(aws_key))
            .send()
            .await?;

        Ok(())
    }
}
