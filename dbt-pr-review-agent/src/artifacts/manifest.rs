use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Representation of a dbt manifest node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestNode {
    pub unique_id: String,
    pub name: String,
    pub resource_type: String,
    pub original_file_path: String,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub description: Option<String>,
    pub depends_on: Option<DependsOn>,
    pub config: Option<NodeConfig>,
    pub columns: Option<HashMap<String, ColumnDef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependsOn {
    pub nodes: Vec<String>,
    pub macros: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub materialized: Option<String>,
    pub partition_by: Option<serde_json::Value>,
    pub cluster_by: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: Option<String>,
    pub description: Option<String>,
    pub constraints: Option<Vec<String>>,
}

impl ManifestNode {
    pub fn is_model(&self) -> bool {
        self.resource_type == "model"
    }

    pub fn is_test(&self) -> bool {
        self.resource_type == "test"
    }

    pub fn is_source(&self) -> bool {
        self.resource_type == "source"
    }

    pub fn get_materialization(&self) -> String {
        self.config
            .as_ref()
            .and_then(|c| c.materialized.as_ref())
            .cloned()
            .unwrap_or_else(|| "view".to_string())
    }

    pub fn get_dependencies(&self) -> Vec<String> {
        self.depends_on
            .as_ref()
            .map(|d| d.nodes.clone())
            .unwrap_or_default()
    }
}