use crate::artifacts::ArtifactParser;
use crate::lineage::graph::LineageGraph;
use crate::types::*;
use anyhow::{Context, Result};
use petgraph::{Graph, Direction};
use petgraph::graph::{DiGraph, NodeIndex};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

/// Universal lineage analyzer that works with any dbt project structure
#[derive(Debug)]
pub struct LineageAnalyzer {
    artifact_parser: ArtifactParser,
}

impl LineageAnalyzer {
    pub fn new(artifact_parser: ArtifactParser) -> Self {
        Self { artifact_parser }
    }

    /// Build complete dependency graph from dbt manifest
    pub fn build_dependency_graph(&mut self) -> Result<LineageGraph> {
        let manifest = self.artifact_parser.load_manifest()?;
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // First pass: Add all nodes
        info!("Building dependency graph - adding nodes");
        self.add_nodes_to_graph(&mut graph, &mut node_map, manifest)?;

        // Second pass: Add edges based on dependencies
        info!("Building dependency graph - adding edges");
        self.add_edges_to_graph(&mut graph, &node_map, manifest)?;

        let lineage_graph = LineageGraph::new(graph, node_map);
        
        info!("Dependency graph built successfully with {} nodes and {} edges", 
              lineage_graph.node_count(), lineage_graph.edge_count());

        Ok(lineage_graph)
    }

    /// Add all nodes (models, sources, tests) to the graph
    fn add_nodes_to_graph(
        &self,
        graph: &mut DiGraph<String, ()>,
        node_map: &mut HashMap<String, NodeIndex>,
        manifest: &Value,
    ) -> Result<()> {
        // Add model nodes
        if let Some(nodes) = manifest.get("nodes").and_then(|n| n.as_object()) {
            for (node_id, _node_data) in nodes {
                let node_index = graph.add_node(node_id.clone());
                node_map.insert(node_id.clone(), node_index);
            }
        }

        // Add source nodes
        if let Some(sources) = manifest.get("sources").and_then(|s| s.as_object()) {
            for (source_id, _source_data) in sources {
                let node_index = graph.add_node(source_id.clone());
                node_map.insert(source_id.clone(), node_index);
            }
        }

        debug!("Added {} nodes to dependency graph", node_map.len());
        Ok(())
    }

    /// Add dependency edges to the graph
    fn add_edges_to_graph(
        &self,
        graph: &mut DiGraph<String, ()>,
        node_map: &HashMap<String, NodeIndex>,
        manifest: &Value,
    ) -> Result<()> {
        let mut edge_count = 0;

        if let Some(nodes) = manifest.get("nodes").and_then(|n| n.as_object()) {
            for (node_id, node_data) in nodes {
                if let Some(depends_on) = node_data.get("depends_on").and_then(|d| d.as_object()) {
                    // Add dependencies from nodes (models and sources)
                    if let Some(deps) = depends_on.get("nodes").and_then(|n| n.as_array()) {
                        for dep in deps {
                            if let Some(dep_id) = dep.as_str() {
                                if let (Some(&from_idx), Some(&to_idx)) = 
                                    (node_map.get(dep_id), node_map.get(node_id)) {
                                    graph.add_edge(from_idx, to_idx, ());
                                    edge_count += 1;
                                } else {
                                    warn!("Could not find node indices for edge: {} -> {}", dep_id, node_id);
                                }
                            }
                        }
                    }
                }
            }
        }

        debug!("Added {} edges to dependency graph", edge_count);
        Ok(())
    }

    /// Analyze downstream impact of changed models
    pub fn analyze_downstream_impact(
        &mut self,
        changed_models: &[String],
        lineage_graph: &LineageGraph,
    ) -> Result<DownstreamImpact> {
        let manifest = self.artifact_parser.load_manifest()?;
        
        let mut downstream_models = HashSet::new();
        let mut downstream_tests = HashSet::new();
        let mut downstream_sources = HashSet::new();
        let mut warehouse_impacts = Vec::new();

        for model_id in changed_models {
            debug!("Analyzing downstream impact for model: {}", model_id);
            
            // Find all downstream nodes using graph traversal
            let descendants = lineage_graph.get_descendants(model_id)?;
            
            for descendant_id in descendants {
                self.categorize_downstream_node(
                    &descendant_id,
                    manifest,
                    &mut downstream_models,
                    &mut downstream_tests,
                    &mut downstream_sources,
                    &mut warehouse_impacts,
                )?;
            }
        }

        let total_affected = downstream_models.len() + downstream_tests.len() + downstream_sources.len();
        
        info!("Downstream impact analysis complete: {} models, {} tests, {} sources affected",
              downstream_models.len(), downstream_tests.len(), downstream_sources.len());

        Ok(DownstreamImpact {
            models: downstream_models.into_iter().collect(),
            tests: downstream_tests.into_iter().collect(),
            sources: downstream_sources.into_iter().collect(),
            total_affected,
            warehouse_impact: warehouse_impacts,
        })
    }

    /// Categorize downstream nodes by type and analyze warehouse-specific impacts
    fn categorize_downstream_node(
        &self,
        node_id: &str,
        manifest: &Value,
        downstream_models: &mut HashSet<String>,
        downstream_tests: &mut HashSet<String>,
        downstream_sources: &mut HashSet<String>,
        warehouse_impacts: &mut Vec<WarehouseImpact>,
    ) -> Result<()> {
        // Check in nodes first
        if let Some(nodes) = manifest.get("nodes").and_then(|n| n.as_object()) {
            if let Some(node_data) = nodes.get(node_id) {
                if let Some(resource_type) = node_data.get("resource_type").and_then(|r| r.as_str()) {
                    match resource_type {
                        "model" => {
                            downstream_models.insert(node_id.to_string());
                            
                            // Analyze warehouse-specific impact for models
                            if let Some(warehouse_impact) = self.analyze_warehouse_impact(node_data)? {
                                warehouse_impacts.push(warehouse_impact);
                            }
                        },
                        "test" => {
                            downstream_tests.insert(node_id.to_string());
                        },
                        _ => {
                            debug!("Unhandled resource type in downstream analysis: {}", resource_type);
                        }
                    }
                }
                return Ok(());
            }
        }

        // Check in sources
        if let Some(sources) = manifest.get("sources").and_then(|s| s.as_object()) {
            if sources.contains_key(node_id) {
                downstream_sources.insert(node_id.to_string());
                return Ok(());
            }
        }

        warn!("Could not categorize downstream node: {}", node_id);
        Ok(())
    }

    /// Analyze warehouse-specific impacts from model configuration
    fn analyze_warehouse_impact(&self, model_node: &Value) -> Result<Option<WarehouseImpact>> {
        let config = model_node.get("config").and_then(|c| c.as_object());
        
        if config.is_none() {
            return Ok(None);
        }
        
        let config = config.unwrap();

        // Extract warehouse-agnostic configuration
        let materialization = config
            .get("materialized")
            .and_then(|m| m.as_str())
            .unwrap_or("view");

        let database = model_node
            .get("database")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_lowercase();

        // Detect warehouse type and analyze specific impacts
        if database.contains("bigquery") || database.contains("bq") {
            self.analyze_bigquery_impact(config, materialization)
        } else if database.contains("snowflake") {
            self.analyze_snowflake_impact(config, materialization)
        } else if database.contains("databricks") {
            self.analyze_databricks_impact(config, materialization)
        } else if database.contains("redshift") {
            self.analyze_redshift_impact(config, materialization)
        } else {
            Ok(None)
        }
    }

    /// Analyze BigQuery-specific impact
    fn analyze_bigquery_impact(
        &self,
        config: &serde_json::Map<String, Value>,
        materialization: &str,
    ) -> Result<Option<WarehouseImpact>> {
        let mut recommendations = Vec::new();
        let mut severity = Severity::Low;

        // Check partitioning configuration
        if let Some(partition_by) = config.get("partition_by") {
            if materialization == "table" {
                recommendations.push("Partitioned table detected - changes may affect partition pruning".to_string());
                severity = Severity::Medium;
            }
        }

        // Check clustering configuration
        if let Some(cluster_by) = config.get("cluster_by") {
            recommendations.push("Clustered table detected - changes may affect query performance".to_string());
            severity = Severity::Medium;
        }

        // Check materialization impact
        match materialization {
            "table" => {
                recommendations.push("Table materialization - changes will trigger full rebuild".to_string());
                severity = Severity::Medium;
            },
            "incremental" => {
                recommendations.push("Incremental model - verify incremental logic still works".to_string());
                severity = Severity::High;
            },
            _ => {}
        }

        if recommendations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(WarehouseImpact {
                warehouse_type: "BigQuery".to_string(),
                impact_type: "Configuration Change".to_string(),
                description: format!("Model uses {} materialization with specific BigQuery configurations", materialization),
                severity,
                recommendations,
            }))
        }
    }

    /// Analyze Snowflake-specific impact
    fn analyze_snowflake_impact(
        &self,
        config: &serde_json::Map<String, Value>,
        materialization: &str,
    ) -> Result<Option<WarehouseImpact>> {
        let mut recommendations = Vec::new();
        let mut severity = Severity::Low;

        // Check for Snowflake-specific configurations
        if config.contains_key("cluster_by") {
            recommendations.push("Clustered table in Snowflake - changes may affect auto-clustering".to_string());
            severity = Severity::Medium;
        }

        if materialization == "incremental" {
            recommendations.push("Incremental model - verify merge strategy and clustering keys".to_string());
            severity = Severity::High;
        }

        if recommendations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(WarehouseImpact {
                warehouse_type: "Snowflake".to_string(),
                impact_type: "Performance Impact".to_string(),
                description: format!("Snowflake-specific configurations may be affected by changes"),
                severity,
                recommendations,
            }))
        }
    }

    /// Analyze Databricks-specific impact
    fn analyze_databricks_impact(
        &self,
        config: &serde_json::Map<String, Value>,
        materialization: &str,
    ) -> Result<Option<WarehouseImpact>> {
        let mut recommendations = Vec::new();
        let mut severity = Severity::Low;

        // Check for Delta Lake specific configurations
        if materialization == "table" {
            recommendations.push("Delta table - changes will create new version with ACID properties".to_string());
            severity = Severity::Medium;
        }

        if materialization == "incremental" {
            recommendations.push("Incremental Delta table - verify merge conditions and optimize commands".to_string());
            severity = Severity::High;
        }

        if recommendations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(WarehouseImpact {
                warehouse_type: "Databricks".to_string(),
                impact_type: "Delta Lake Impact".to_string(),
                description: format!("Delta Lake table configurations may be affected"),
                severity,
                recommendations,
            }))
        }
    }

    /// Analyze Redshift-specific impact
    fn analyze_redshift_impact(
        &self,
        config: &serde_json::Map<String, Value>,
        materialization: &str,
    ) -> Result<Option<WarehouseImpact>> {
        let mut recommendations = Vec::new();
        let mut severity = Severity::Low;

        // Check for Redshift-specific configurations
        if config.contains_key("dist_key") || config.contains_key("sort_key") {
            recommendations.push("Table has distribution/sort keys - changes may affect query performance".to_string());
            severity = Severity::Medium;
        }

        if materialization == "table" {
            recommendations.push("Table materialization - consider VACUUM and ANALYZE after changes".to_string());
            severity = Severity::Medium;
        }

        if recommendations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(WarehouseImpact {
                warehouse_type: "Redshift".to_string(),
                impact_type: "Performance Impact".to_string(),
                description: format!("Redshift-specific configurations may need optimization"),
                severity,
                recommendations,
            }))
        }
    }

    /// Calculate impact score based on downstream effects
    pub fn calculate_impact_score(
        &self,
        downstream_impact: &DownstreamImpact,
        project_structure: Option<&ProjectStructure>,
    ) -> f64 {
        let base_score = downstream_impact.total_affected as f64;
        
        // Weight different types of impacts
        let model_weight = 3.0;
        let test_weight = 1.0;
        let source_weight = 2.0;
        
        let weighted_score = (downstream_impact.models.len() as f64 * model_weight)
            + (downstream_impact.tests.len() as f64 * test_weight)
            + (downstream_impact.sources.len() as f64 * source_weight);

        // Normalize based on project size if available
        if let Some(structure) = project_structure {
            let total_project_resources = structure.total_models + structure.total_tests + structure.total_sources;
            if total_project_resources > 0 {
                return (weighted_score / total_project_resources as f64) * 100.0;
            }
        }

        // Add warehouse-specific impact weight
        let warehouse_impact_score: f64 = downstream_impact.warehouse_impact
            .iter()
            .map(|impact| match impact.severity {
                Severity::Low => 1.0,
                Severity::Medium => 2.0,
                Severity::High => 4.0,
                Severity::Critical => 8.0,
            })
            .sum();

        weighted_score + warehouse_impact_score
    }

    /// Assess risk level based on impact score and downstream effects
    pub fn assess_risk_level(
        &self,
        impact_score: f64,
        downstream_impact: &DownstreamImpact,
    ) -> RiskLevel {
        // Check for critical conditions first
        let has_critical_warehouse_impact = downstream_impact.warehouse_impact
            .iter()
            .any(|impact| matches!(impact.severity, Severity::Critical));

        if has_critical_warehouse_impact {
            return RiskLevel::Critical;
        }

        // Check for high-impact conditions
        let high_model_impact = downstream_impact.models.len() > 10;
        let high_test_impact = downstream_impact.tests.len() > 20;
        
        if high_model_impact || high_test_impact {
            return RiskLevel::High;
        }

        // Use impact score for classification
        match impact_score {
            score if score >= 50.0 => RiskLevel::Critical,
            score if score >= 25.0 => RiskLevel::High,
            score if score >= 10.0 => RiskLevel::Medium,
            _ => RiskLevel::Low,
        }
    }

    /// Find upstream dependencies for a model
    pub fn find_upstream_dependencies(
        &self,
        model_id: &str,
        lineage_graph: &LineageGraph,
    ) -> Result<Vec<String>> {
        lineage_graph.get_ancestors(model_id)
    }

    /// Find models that have no downstream dependencies (leaf nodes)
    pub fn find_leaf_models(&self, lineage_graph: &LineageGraph) -> Result<Vec<String>> {
        lineage_graph.get_leaf_nodes()
    }

    /// Find models that have no upstream dependencies (root nodes)
    pub fn find_root_models(&self, lineage_graph: &LineageGraph) -> Result<Vec<String>> {
        lineage_graph.get_root_nodes()
    }

    /// Calculate the depth of impact (how many levels downstream are affected)
    pub fn calculate_impact_depth(
        &self,
        changed_models: &[String],
        lineage_graph: &LineageGraph,
    ) -> Result<HashMap<String, usize>> {
        let mut depth_map = HashMap::new();

        for model_id in changed_models {
            let depth = lineage_graph.calculate_max_depth(model_id)?;
            depth_map.insert(model_id.clone(), depth);
        }

        Ok(depth_map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::ArtifactParser;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_manifest_with_dependencies() -> serde_json::Value {
        serde_json::json!({
            "nodes": {
                "model.test.staging_orders": {
                    "name": "staging_orders",
                    "resource_type": "model",
                    "depends_on": {
                        "nodes": ["source.test.raw_orders"]
                    },
                    "config": {
                        "materialized": "view"
                    }
                },
                "model.test.mart_orders": {
                    "name": "mart_orders", 
                    "resource_type": "model",
                    "depends_on": {
                        "nodes": ["model.test.staging_orders"]
                    },
                    "config": {
                        "materialized": "table"
                    }
                },
                "test.test.not_null_mart_orders_id": {
                    "name": "not_null_mart_orders_id",
                    "resource_type": "test",
                    "depends_on": {
                        "nodes": ["model.test.mart_orders"]
                    }
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
    async fn test_build_dependency_graph() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        
        let manifest = create_test_manifest_with_dependencies();
        let manifest_path = target_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let mut analyzer = LineageAnalyzer::new(artifact_parser);
        
        let lineage_graph = analyzer.build_dependency_graph().unwrap();
        
        assert_eq!(lineage_graph.node_count(), 4); // 2 models + 1 test + 1 source
        assert!(lineage_graph.edge_count() > 0);
    }

    #[tokio::test]
    async fn test_analyze_downstream_impact() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        
        let manifest = create_test_manifest_with_dependencies();
        let manifest_path = target_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let mut analyzer = LineageAnalyzer::new(artifact_parser);
        
        let lineage_graph = analyzer.build_dependency_graph().unwrap();
        let changed_models = vec!["model.test.staging_orders".to_string()];
        
        let impact = analyzer.analyze_downstream_impact(&changed_models, &lineage_graph).unwrap();
        
        assert!(!impact.models.is_empty());
        assert!(impact.models.contains(&"model.test.mart_orders".to_string()));
    }
}