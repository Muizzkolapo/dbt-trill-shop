use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Representation of a dbt catalog node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogNode {
    pub unique_id: String,
    pub metadata: NodeMetadata,
    pub columns: HashMap<String, CatalogColumn>,
    pub stats: HashMap<String, StatValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    #[serde(rename = "type")]
    pub node_type: String,
    pub schema: String,
    pub name: String,
    pub database: String,
    pub comment: Option<String>,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogColumn {
    #[serde(rename = "type")]
    pub data_type: String,
    pub index: u32,
    pub name: String,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatValue {
    pub id: String,
    pub label: String,
    pub value: serde_json::Value,
    pub include: bool,
    pub description: String,
}

impl CatalogNode {
    pub fn get_row_count(&self) -> Option<u64> {
        self.stats
            .get("num_rows")
            .and_then(|stat| stat.value.as_u64())
    }

    pub fn get_size_bytes(&self) -> Option<u64> {
        self.stats
            .get("num_bytes")
            .and_then(|stat| stat.value.as_u64())
    }

    pub fn has_stats(&self) -> bool {
        self.stats
            .get("has_stats")
            .and_then(|stat| stat.value.as_bool())
            .unwrap_or(false)
    }

    pub fn is_partitioned(&self) -> bool {
        self.stats.contains_key("partitioning_type")
    }

    pub fn get_partitioning_type(&self) -> Option<String> {
        self.stats
            .get("partitioning_type")
            .and_then(|stat| stat.value.as_str())
            .map(|s| s.to_string())
    }
}