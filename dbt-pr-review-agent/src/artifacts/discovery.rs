use crate::artifacts::ArtifactParser;
use crate::types::*;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

/// Universal project discovery system that analyzes dbt project structure from artifacts
pub struct ProjectDiscovery {
    artifact_parser: ArtifactParser,
}

impl ProjectDiscovery {
    pub fn new(artifact_parser: ArtifactParser) -> Self {
        Self { artifact_parser }
    }

    /// Discover project structure dynamically from artifacts
    pub async fn discover_structure(&mut self) -> Result<ProjectStructure> {
        info!("Starting universal project structure discovery");

        let manifest = self.artifact_parser.load_manifest()?;
        let catalog = self.artifact_parser.load_catalog()?;

        // Discover model organization patterns
        let model_patterns = self.analyze_model_patterns(manifest)?;

        // Discover folder hierarchy
        let folder_structure = self.discover_folder_hierarchy(manifest)?;

        // Discover naming conventions
        let naming_conventions = self.analyze_naming_conventions(manifest)?;

        // Discover warehouse and adapter type
        let warehouse_info = self.discover_warehouse_type(manifest)?;

        // Count resources
        let total_models = self.count_resource_type(manifest, "model")?;
        let total_tests = self.count_resource_type(manifest, "test")?;
        let total_sources = manifest
            .get("sources")
            .and_then(|s| s.as_object())
            .map(|o| o.len())
            .unwrap_or(0);

        let structure = ProjectStructure {
            model_patterns,
            folder_structure,
            naming_conventions,
            warehouse_info,
            total_models,
            total_tests,
            total_sources,
        };

        info!(
            "Project discovery completed: {} models, {} tests, {} sources across {} folders",
            total_models, total_tests, total_sources, structure.model_patterns.total_folders
        );

        Ok(structure)
    }

    /// Analyze model organization patterns without assumptions
    fn analyze_model_patterns(
        &self,
        manifest: &serde_json::Value,
    ) -> Result<ModelPatterns> {
        let nodes = manifest
            .get("nodes")
            .and_then(|n| n.as_object())
            .context("Invalid manifest structure: missing nodes")?;

        let models: Vec<_> = nodes
            .iter()
            .filter_map(|(node_id, node_data)| {
                if node_data.get("resource_type")?.as_str()? == "model" {
                    Some((node_id, node_data))
                } else {
                    None
                }
            })
            .collect();

        // Group by directory structure
        let mut folder_groups: HashMap<String, Vec<String>> = HashMap::new();
        for (model_id, model_data) in &models {
            if let Some(file_path) = model_data.get("original_file_path").and_then(|p| p.as_str()) {
                let folder_path = Path::new(file_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "root".to_string());
                
                folder_groups
                    .entry(folder_path)
                    .or_insert_with(Vec::new)
                    .push((*model_id).clone());
            }
        }

        // Analyze materialization patterns
        let mut materialization_patterns: HashMap<String, Vec<String>> = HashMap::new();
        for (model_id, model_data) in &models {
            let materialization = model_data
                .get("config")
                .and_then(|c| c.get("materialized"))
                .and_then(|m| m.as_str())
                .unwrap_or("view")
                .to_string();
            
            materialization_patterns
                .entry(materialization)
                .or_insert_with(Vec::new)
                .push((*model_id).clone());
        }

        debug!(
            "Discovered {} folder groups and {} materialization types",
            folder_groups.len(),
            materialization_patterns.len()
        );

        Ok(ModelPatterns {
            folder_groups,
            materialization_patterns,
            total_folders: folder_groups.len(),
        })
    }

    /// Discover folder hierarchy from manifest
    fn discover_folder_hierarchy(
        &self,
        manifest: &serde_json::Value,
    ) -> Result<HashMap<String, Vec<String>>> {
        let nodes = manifest
            .get("nodes")
            .and_then(|n| n.as_object())
            .context("Invalid manifest structure: missing nodes")?;

        let mut folder_structure: HashMap<String, Vec<String>> = HashMap::new();

        for (node_id, node_data) in nodes {
            if let Some(file_path) = node_data.get("original_file_path").and_then(|p| p.as_str()) {
                let path = Path::new(file_path);
                if let Some(parent) = path.parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    
                    // Build hierarchical structure
                    let parts: Vec<&str> = parent_str.split('/').collect();
                    for (i, _) in parts.iter().enumerate() {
                        let current_path = parts[..=i].join("/");
                        let child_path = if i + 1 < parts.len() {
                            Some(parts[..=i + 1].join("/"))
                        } else {
                            None
                        };

                        let entry = folder_structure.entry(current_path).or_insert_with(Vec::new);
                        if let Some(child) = child_path {
                            if !entry.contains(&child) {
                                entry.push(child);
                            }
                        }
                    }
                }
            }
        }

        Ok(folder_structure)
    }

    /// Analyze naming conventions used in the project
    fn analyze_naming_conventions(
        &self,
        manifest: &serde_json::Value,
    ) -> Result<NamingConventions> {
        let nodes = manifest
            .get("nodes")
            .and_then(|n| n.as_object())
            .context("Invalid manifest structure: missing nodes")?;

        let mut model_prefixes = Vec::new();
        let mut staging_patterns = Vec::new();
        let mut mart_patterns = Vec::new();
        let mut test_patterns = Vec::new();

        // Analyze model names for patterns
        for (node_id, node_data) in nodes {
            if let Some(resource_type) = node_data.get("resource_type").and_then(|r| r.as_str()) {
                if let Some(name) = node_data.get("name").and_then(|n| n.as_str()) {
                    match resource_type {
                        "model" => {
                            // Extract prefix patterns
                            if let Some(prefix) = self.extract_prefix(name) {
                                if !model_prefixes.contains(&prefix) {
                                    model_prefixes.push(prefix);
                                }
                            }

                            // Detect staging patterns
                            if name.contains("stg_") || name.contains("staging_") {
                                staging_patterns.push(name.to_string());
                            }

                            // Detect mart patterns
                            if name.contains("mart_") || name.contains("dim_") || name.contains("fact_") {
                                mart_patterns.push(name.to_string());
                            }
                        }
                        "test" => {
                            test_patterns.push(name.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(NamingConventions {
            model_prefix_patterns: model_prefixes,
            staging_patterns,
            mart_patterns,
            test_patterns,
        })
    }

    /// Extract prefix from model name
    fn extract_prefix(&self, name: &str) -> Option<String> {
        if let Some(underscore_pos) = name.find('_') {
            Some(name[..underscore_pos].to_string())
        } else {
            None
        }
    }

    /// Discover warehouse type from manifest metadata
    fn discover_warehouse_type(
        &self,
        manifest: &serde_json::Value,
    ) -> Result<WarehouseInfo> {
        // Try to detect warehouse from metadata
        let metadata = manifest.get("metadata").and_then(|m| m.as_object());
        
        let adapter_type = if let Some(metadata) = metadata {
            metadata
                .get("adapter_type")
                .and_then(|at| at.as_str())
                .unwrap_or("unknown")
                .to_string()
        } else {
            // Fallback: try to detect from first model's database
            let nodes = manifest.get("nodes").and_then(|n| n.as_object());
            if let Some(nodes) = nodes {
                for (_, node_data) in nodes {
                    if node_data.get("resource_type").and_then(|r| r.as_str()) == Some("model") {
                        if let Some(database) = node_data.get("database").and_then(|d| d.as_str()) {
                            return Ok(self.infer_warehouse_from_database(database));
                        }
                    }
                }
            }
            "unknown".to_string()
        };

        Ok(self.create_warehouse_info(&adapter_type))
    }

    /// Infer warehouse type from database name
    fn infer_warehouse_from_database(&self, database: &str) -> WarehouseInfo {
        let database_lower = database.to_lowercase();
        
        if database_lower.contains("bigquery") || database_lower.contains("bq") {
            self.create_warehouse_info("bigquery")
        } else if database_lower.contains("snowflake") {
            self.create_warehouse_info("snowflake")
        } else if database_lower.contains("databricks") {
            self.create_warehouse_info("databricks")
        } else if database_lower.contains("redshift") {
            self.create_warehouse_info("redshift")
        } else {
            self.create_warehouse_info("unknown")
        }
    }

    /// Create warehouse info based on adapter type
    fn create_warehouse_info(&self, adapter_type: &str) -> WarehouseInfo {
        match adapter_type.to_lowercase().as_str() {
            "bigquery" => WarehouseInfo {
                adapter_type: "bigquery".to_string(),
                database: "unknown".to_string(),
                schema: "unknown".to_string(),
                supports_partitioning: true,
                supports_clustering: true,
            },
            "snowflake" => WarehouseInfo {
                adapter_type: "snowflake".to_string(),
                database: "unknown".to_string(),
                schema: "unknown".to_string(),
                supports_partitioning: false,
                supports_clustering: true,
            },
            "databricks" => WarehouseInfo {
                adapter_type: "databricks".to_string(),
                database: "unknown".to_string(),
                schema: "unknown".to_string(),
                supports_partitioning: true,
                supports_clustering: false,
            },
            "redshift" => WarehouseInfo {
                adapter_type: "redshift".to_string(),
                database: "unknown".to_string(),
                schema: "unknown".to_string(),
                supports_partitioning: false,
                supports_clustering: false,
            },
            _ => WarehouseInfo {
                adapter_type: adapter_type.to_string(),
                database: "unknown".to_string(),
                schema: "unknown".to_string(),
                supports_partitioning: false,
                supports_clustering: false,
            },
        }
    }

    /// Count resources of a specific type
    fn count_resource_type(
        &self,
        manifest: &serde_json::Value,
        resource_type: &str,
    ) -> Result<usize> {
        let nodes = manifest
            .get("nodes")
            .and_then(|n| n.as_object())
            .context("Invalid manifest structure: missing nodes")?;

        let count = nodes
            .values()
            .filter(|node_data| {
                node_data
                    .get("resource_type")
                    .and_then(|r| r.as_str())
                    == Some(resource_type)
            })
            .count();

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::ArtifactParser;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_manifest() -> serde_json::Value {
        serde_json::json!({
            "metadata": {
                "adapter_type": "bigquery"
            },
            "nodes": {
                "model.test.stg_orders": {
                    "name": "stg_orders",
                    "resource_type": "model",
                    "original_file_path": "models/staging/stg_orders.sql",
                    "config": {
                        "materialized": "view"
                    },
                    "database": "test-project"
                },
                "model.test.mart_orders": {
                    "name": "mart_orders",
                    "resource_type": "model", 
                    "original_file_path": "models/marts/mart_orders.sql",
                    "config": {
                        "materialized": "table"
                    },
                    "database": "test-project"
                },
                "test.test.not_null_orders_id": {
                    "name": "not_null_orders_id",
                    "resource_type": "test",
                    "original_file_path": "models/staging/schema.yml"
                }
            },
            "sources": {
                "source.test.raw_orders": {
                    "name": "raw_orders",
                    "resource_type": "source"
                }
            }
        })
    }

    #[tokio::test]
    async fn test_discover_structure() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        
        let manifest = create_test_manifest();
        let manifest_path = target_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let mut discovery = ProjectDiscovery::new(artifact_parser);
        
        let structure = discovery.discover_structure().await.unwrap();
        
        assert_eq!(structure.total_models, 2);
        assert_eq!(structure.total_tests, 1);
        assert_eq!(structure.total_sources, 1);
        assert_eq!(structure.warehouse_info.adapter_type, "bigquery");
        assert!(structure.warehouse_info.supports_partitioning);
        assert!(structure.warehouse_info.supports_clustering);
    }

    #[test]
    fn test_analyze_model_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let discovery = ProjectDiscovery::new(artifact_parser);
        
        let manifest = create_test_manifest();
        let patterns = discovery.analyze_model_patterns(&manifest).unwrap();
        
        assert_eq!(patterns.total_folders, 2); // staging and marts folders
        assert!(patterns.folder_groups.contains_key("models/staging"));
        assert!(patterns.folder_groups.contains_key("models/marts"));
        assert!(patterns.materialization_patterns.contains_key("view"));
        assert!(patterns.materialization_patterns.contains_key("table"));
    }
}