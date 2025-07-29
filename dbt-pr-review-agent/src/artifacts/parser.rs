use crate::types::*;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Universal dbt artifact parser that works with any project structure
#[derive(Debug, Clone)]
pub struct ArtifactParser {
    project_path: PathBuf,
    target_path: PathBuf,
    manifest_cache: Option<Value>,
    catalog_cache: Option<Value>,
}

impl ArtifactParser {
    pub fn new<P: AsRef<Path>>(project_path: P) -> Self {
        let project_path = project_path.as_ref().to_path_buf();
        let target_path = project_path.join("target");
        
        Self {
            project_path,
            target_path,
            manifest_cache: None,
            catalog_cache: None,
        }
    }

    /// Load and cache the dbt manifest.json - the source of truth for project structure
    pub fn load_manifest(&mut self) -> Result<&Value> {
        if self.manifest_cache.is_none() {
            let manifest_path = self.target_path.join("manifest.json");
            debug!("Loading manifest from: {:?}", manifest_path);
            
            let content = fs::read_to_string(&manifest_path)
                .with_context(|| format!("Failed to read manifest.json from {:?}", manifest_path))?;
            
            let manifest: Value = serde_json::from_str(&content)
                .with_context(|| "Failed to parse manifest.json")?;
            
            info!("Successfully loaded manifest with {} nodes", 
                  manifest.get("nodes").and_then(|n| n.as_object()).map(|o| o.len()).unwrap_or(0));
            
            self.manifest_cache = Some(manifest);
        }
        
        Ok(self.manifest_cache.as_ref().unwrap())
    }

    /// Load and cache the dbt catalog.json - runtime schema and statistics
    pub fn load_catalog(&mut self) -> Result<Option<&Value>> {
        if self.catalog_cache.is_none() {
            let catalog_path = self.target_path.join("catalog.json");
            
            if catalog_path.exists() {
                debug!("Loading catalog from: {:?}", catalog_path);
                
                let content = fs::read_to_string(&catalog_path)
                    .with_context(|| format!("Failed to read catalog.json from {:?}", catalog_path))?;
                
                let catalog: Value = serde_json::from_str(&content)
                    .with_context(|| "Failed to parse catalog.json")?;
                
                info!("Successfully loaded catalog");
                self.catalog_cache = Some(catalog);
            } else {
                warn!("Catalog.json not found at {:?}. Run 'dbt docs generate' to create it.", catalog_path);
                return Ok(None);
            }
        }
        
        Ok(self.catalog_cache.as_ref())
    }

    /// Load run results for performance metrics
    pub fn load_run_results(&self) -> Result<Option<Value>> {
        let run_results_path = self.target_path.join("run_results.json");
        
        if run_results_path.exists() {
            debug!("Loading run results from: {:?}", run_results_path);
            
            let content = fs::read_to_string(&run_results_path)
                .with_context(|| format!("Failed to read run_results.json from {:?}", run_results_path))?;
            
            let run_results: Value = serde_json::from_str(&content)
                .with_context(|| "Failed to parse run_results.json")?;
            
            Ok(Some(run_results))
        } else {
            debug!("Run results not found at {:?}", run_results_path);
            Ok(None)
        }
    }

    /// Get all models from manifest regardless of folder structure
    pub fn get_all_models(&mut self) -> Result<Vec<ModelInfo>> {
        let manifest = self.load_manifest()?;
        let nodes = manifest
            .get("nodes")
            .and_then(|n| n.as_object())
            .context("Invalid manifest structure: missing nodes")?;

        let mut models = Vec::new();

        for (node_id, node_data) in nodes {
            if let Some(resource_type) = node_data.get("resource_type").and_then(|r| r.as_str()) {
                if resource_type == "model" {
                    let model_info = self.parse_model_info(node_id, node_data)
                        .with_context(|| format!("Failed to parse model info for {}", node_id))?;
                    models.push(model_info);
                }
            }
        }

        info!("Discovered {} models from manifest", models.len());
        Ok(models)
    }

    /// Parse individual model information from manifest node
    fn parse_model_info(&self, node_id: &str, node_data: &Value) -> Result<ModelInfo> {
        let name = node_data
            .get("name")
            .and_then(|n| n.as_str())
            .context("Model missing name")?
            .to_string();

        let file_path = node_data
            .get("original_file_path")
            .and_then(|p| p.as_str())
            .context("Model missing file path")?
            .to_string();

        let folder = Path::new(&file_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let materialization = node_data
            .get("config")
            .and_then(|c| c.get("materialized"))
            .and_then(|m| m.as_str())
            .unwrap_or("view")
            .to_string();

        let dependencies = node_data
            .get("depends_on")
            .and_then(|d| d.get("nodes"))
            .and_then(|n| n.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let database = node_data
            .get("database")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        let schema = node_data
            .get("schema")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        let description = node_data
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        // Parse columns information
        let columns = self.parse_columns_info(node_data)?;

        Ok(ModelInfo {
            unique_id: node_id.to_string(),
            name,
            file_path,
            folder,
            materialization,
            dependencies,
            database,
            schema,
            description,
            columns,
        })
    }

    /// Parse column information from model node
    fn parse_columns_info(&self, node_data: &Value) -> Result<HashMap<String, ColumnInfo>> {
        let mut columns = HashMap::new();

        if let Some(columns_data) = node_data.get("columns").and_then(|c| c.as_object()) {
            for (col_name, col_data) in columns_data {
                let column_info = ColumnInfo {
                    name: col_name.clone(),
                    data_type: col_data
                        .get("data_type")
                        .and_then(|dt| dt.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    description: col_data
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string(),
                    constraints: col_data
                        .get("constraints")
                        .and_then(|c| c.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect()
                        })
                        .unwrap_or_default(),
                };
                columns.insert(col_name.clone(), column_info);
            }
        }

        Ok(columns)
    }

    /// Get model dependencies for a specific model
    pub fn get_model_dependencies(&mut self, model_unique_id: &str) -> Result<ModelDependencies> {
        let manifest = self.load_manifest()?;
        let nodes = manifest
            .get("nodes")
            .and_then(|n| n.as_object())
            .context("Invalid manifest structure: missing nodes")?;

        let model_node = nodes
            .get(model_unique_id)
            .context("Model not found in manifest")?;

        let depends_on = model_node
            .get("depends_on")
            .and_then(|d| d.as_object())
            .unwrap_or(&serde_json::Map::new());

        let upstream_models = depends_on
            .get("nodes")
            .and_then(|n| n.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter(|s| s.starts_with("model."))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let upstream_sources = depends_on
            .get("nodes")
            .and_then(|n| n.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter(|s| s.starts_with("source."))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let macros_used = depends_on
            .get("macros")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(ModelDependencies {
            upstream_models,
            upstream_sources,
            macros_used,
        })
    }

    /// Map changed files to model unique IDs using manifest
    pub fn discover_changed_models(&mut self, changed_files: &[String]) -> Result<Vec<String>> {
        let manifest = self.load_manifest()?;
        let nodes = manifest
            .get("nodes")
            .and_then(|n| n.as_object())
            .context("Invalid manifest structure: missing nodes")?;

        let mut changed_models = Vec::new();

        for (node_id, node_data) in nodes {
            if let Some(file_path) = node_data.get("original_file_path").and_then(|p| p.as_str()) {
                if changed_files.iter().any(|changed| changed == file_path) {
                    changed_models.push(node_id.clone());
                }
            }
        }

        info!("Mapped {} changed files to {} models", changed_files.len(), changed_models.len());
        Ok(changed_models)
    }

    /// Get all sources from manifest
    pub fn get_all_sources(&mut self) -> Result<Vec<String>> {
        let manifest = self.load_manifest()?;
        let sources = manifest
            .get("sources")
            .and_then(|s| s.as_object())
            .context("Invalid manifest structure: missing sources")?;

        let source_ids: Vec<String> = sources.keys().cloned().collect();
        info!("Discovered {} sources from manifest", source_ids.len());
        Ok(source_ids)
    }

    /// Get all tests from manifest
    pub fn get_all_tests(&mut self) -> Result<Vec<String>> {
        let manifest = self.load_manifest()?;
        let nodes = manifest
            .get("nodes")
            .and_then(|n| n.as_object())
            .context("Invalid manifest structure: missing nodes")?;

        let test_ids: Vec<String> = nodes
            .iter()
            .filter_map(|(node_id, node_data)| {
                if let Some(resource_type) = node_data.get("resource_type").and_then(|r| r.as_str()) {
                    if resource_type == "test" {
                        Some(node_id.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        info!("Discovered {} tests from manifest", test_ids.len());
        Ok(test_ids)
    }

    /// Get performance metrics from run results
    pub fn get_performance_metrics(&self) -> Result<Option<HashMap<String, f64>>> {
        if let Some(run_results) = self.load_run_results()? {
            let mut metrics = HashMap::new();

            if let Some(results) = run_results.get("results").and_then(|r| r.as_array()) {
                for result in results {
                    if let (Some(unique_id), Some(execution_time)) = (
                        result.get("unique_id").and_then(|id| id.as_str()),
                        result.get("execution_time").and_then(|t| t.as_f64()),
                    ) {
                        metrics.insert(unique_id.to_string(), execution_time);
                    }
                }
            }

            Ok(Some(metrics))
        } else {
            Ok(None)
        }
    }

    /// Check if catalog has statistics for models
    pub fn has_catalog_stats(&mut self) -> Result<bool> {
        if let Some(catalog) = self.load_catalog()? {
            if let Some(nodes) = catalog.get("nodes").and_then(|n| n.as_object()) {
                // Check if any model has statistics
                for node_data in nodes.values() {
                    if let Some(stats) = node_data.get("stats").and_then(|s| s.as_object()) {
                        if let Some(has_stats) = stats.get("has_stats").and_then(|hs| hs.get("value")).and_then(|v| v.as_bool()) {
                            if has_stats {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    /// Get project information from dbt_project.yml
    pub fn get_project_info(&self) -> Result<HashMap<String, Value>> {
        let project_file = self.project_path.join("dbt_project.yml");
        
        if project_file.exists() {
            let content = fs::read_to_string(&project_file)
                .with_context(|| format!("Failed to read dbt_project.yml from {:?}", project_file))?;
            
            let project_config: HashMap<String, Value> = serde_yaml::from_str(&content)
                .with_context(|| "Failed to parse dbt_project.yml")?;
            
            Ok(project_config)
        } else {
            warn!("dbt_project.yml not found at {:?}", project_file);
            Ok(HashMap::new())
        }
    }
    
    /// Get model definition (SQL content) for a specific model
    pub fn get_model_definition(&mut self, model_unique_id: &str) -> Result<String> {
        let manifest = self.load_manifest()?;
        let nodes = manifest
            .get("nodes")
            .and_then(|n| n.as_object())
            .context("Invalid manifest structure: missing nodes")?;
        
        let model_node = nodes
            .get(model_unique_id)
            .context("Model not found in manifest")?;
        
        let file_path = model_node
            .get("original_file_path")
            .and_then(|p| p.as_str())
            .context("Model missing file path")?;
        
        let full_path = self.project_path.join(file_path);
        
        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read model file from {:?}", full_path))?;
        
        Ok(content)
    }
    
    /// Get warehouse type from project configuration
    pub fn get_warehouse_type(&mut self) -> Result<Option<String>> {
        let manifest = self.load_manifest()?;
        
        // Try to detect from adapter type in manifest metadata
        if let Some(metadata) = manifest.get("metadata") {
            if let Some(adapter_type) = metadata.get("adapter_type").and_then(|a| a.as_str()) {
                return Ok(Some(adapter_type.to_string()));
            }
        }
        
        // Try to detect from project config
        if let Ok(project_config) = self.get_project_info() {
            if let Some(profile) = project_config.get("profile").and_then(|p| p.as_str()) {
                // Could look up profile in profiles.yml if needed
                return Ok(Some(profile.to_string()));
            }
        }
        
        // Try to infer from database names in nodes
        if let Some(nodes) = manifest.get("nodes").and_then(|n| n.as_object()) {
            for node in nodes.values() {
                if let Some(database) = node.get("database").and_then(|d| d.as_str()) {
                    let db_lower = database.to_lowercase();
                    if db_lower.contains("bigquery") || db_lower.contains("bq") {
                        return Ok(Some("bigquery".to_string()));
                    } else if db_lower.contains("snowflake") {
                        return Ok(Some("snowflake".to_string()));
                    } else if db_lower.contains("databricks") {
                        return Ok(Some("databricks".to_string()));
                    } else if db_lower.contains("redshift") {
                        return Ok(Some("redshift".to_string()));
                    }
                }
            }
        }
        
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_manifest() -> Value {
        serde_json::json!({
            "nodes": {
                "model.test_project.test_model": {
                    "name": "test_model",
                    "resource_type": "model",
                    "original_file_path": "models/test_model.sql",
                    "config": {
                        "materialized": "table"
                    },
                    "depends_on": {
                        "nodes": ["source.test_project.raw_data"],
                        "macros": []
                    },
                    "database": "test_db",
                    "schema": "test_schema",
                    "description": "A test model"
                }
            },
            "sources": {
                "source.test_project.raw_data": {
                    "name": "raw_data",
                    "resource_type": "source"
                }
            }
        })
    }

    #[test]
    fn test_artifact_parser_creation() {
        let temp_dir = TempDir::new().unwrap();
        let parser = ArtifactParser::new(temp_dir.path());
        
        assert_eq!(parser.project_path, temp_dir.path());
        assert_eq!(parser.target_path, temp_dir.path().join("target"));
    }

    #[test]
    fn test_load_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        
        let manifest = create_test_manifest();
        let manifest_path = target_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let mut parser = ArtifactParser::new(temp_dir.path());
        let loaded_manifest = parser.load_manifest().unwrap();
        
        assert!(loaded_manifest.get("nodes").is_some());
        assert!(loaded_manifest.get("sources").is_some());
    }

    #[test]
    fn test_get_all_models() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        
        let manifest = create_test_manifest();
        let manifest_path = target_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let mut parser = ArtifactParser::new(temp_dir.path());
        let models = parser.get_all_models().unwrap();
        
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "test_model");
        assert_eq!(models[0].materialization, "table");
    }
}