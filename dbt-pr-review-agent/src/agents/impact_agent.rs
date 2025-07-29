use crate::agents::communication::{AgentCommunicationBus, AgentCommunication};
use crate::artifacts::ArtifactParser;
use crate::lineage::LineageAnalyzer;
use crate::types::*;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Impact Analysis Agent - analyzes downstream effects of PR changes
pub struct ImpactAnalysisAgent {
    artifact_parser: ArtifactParser,
    lineage_analyzer: LineageAnalyzer,
    communication_bus: Arc<AgentCommunicationBus>,
    agent_name: String,
}

impl ImpactAnalysisAgent {
    pub fn new(
        artifact_parser: ArtifactParser,
        communication_bus: Arc<AgentCommunicationBus>,
    ) -> Result<Self> {
        let lineage_analyzer = LineageAnalyzer::new(artifact_parser.clone());
        
        Ok(Self {
            artifact_parser,
            lineage_analyzer,
            communication_bus,
            agent_name: "impact_analysis".to_string(),
        })
    }

    /// Analyze impact of PR changes
    #[instrument(skip(self, pr_context), fields(pr_number = pr_context.pr_number))]
    pub async fn analyze(&self, pr_context: &PRContext) -> Result<ImpactReport> {
        info!("Starting impact analysis for PR #{}", pr_context.pr_number);

        // Publish analysis start event
        self.publish_event(
            &self.communication_bus,
            "impact_analysis_started".to_string(),
            serde_json::json!({
                "pr_number": pr_context.pr_number,
                "changed_files_count": pr_context.changed_files.len()
            }),
        ).await?;

        // Build dependency graph
        let mut lineage_graph = self.lineage_analyzer.build_dependency_graph()?;

        // Map changed files to model unique IDs
        let changed_files: Vec<String> = pr_context.changed_files.iter()
            .map(|change| change.filename.clone())
            .collect();
        
        let mut artifact_parser = self.artifact_parser.clone();
        let affected_models = artifact_parser.discover_changed_models(&changed_files)?;

        // Analyze downstream impact
        let downstream_impact = self.lineage_analyzer.analyze_downstream_impact(
            &affected_models,
            &lineage_graph,
        )?;

        // Calculate impact score
        let impact_score = self.lineage_analyzer.calculate_impact_score(
            &downstream_impact,
            None, // Project structure would be passed here if available
        );

        // Assess risk level
        let risk_level = self.lineage_analyzer.assess_risk_level(
            impact_score,
            &downstream_impact,
        );

        // Generate recommendations
        let recommendations = self.generate_recommendations(&downstream_impact, &risk_level);

        let report = ImpactReport {
            id: Uuid::new_v4(),
            generated_at: chrono::Utc::now(),
            directly_affected: affected_models,
            downstream_models: downstream_impact.models,
            affected_tests: downstream_impact.tests,
            affected_sources: downstream_impact.sources,
            affected_documentation: Vec::new(), // TODO: Implement doc impact analysis
            impact_score,
            risk_level,
            visualization: None, // TODO: Implement visualization generation
            warehouse_specific_impact: downstream_impact.warehouse_impact,
            recommendations,
        };

        // Publish analysis complete event
        self.publish_event(
            &self.communication_bus,
            "impact_analysis_completed".to_string(),
            serde_json::json!({
                "pr_number": pr_context.pr_number,
                "impact_score": impact_score,
                "risk_level": format!("{:?}", risk_level),
                "total_affected": report.total_affected_resources()
            }),
        ).await?;

        info!(
            "Impact analysis completed: {} total affected resources, risk level: {:?}",
            report.total_affected_resources(),
            risk_level
        );

        Ok(report)
    }

    /// Generate recommendations based on impact analysis
    fn generate_recommendations(&self, downstream_impact: &DownstreamImpact, risk_level: &RiskLevel) -> Vec<String> {
        let mut recommendations = Vec::new();

        match risk_level {
            RiskLevel::Critical => {
                recommendations.push("🚨 Critical impact detected! This change affects a large number of downstream resources. Consider breaking this into smaller, incremental changes.".to_string());
            }
            RiskLevel::High => {
                recommendations.push("⚠️ High impact change. Ensure thorough testing of all affected downstream models.".to_string());
            }
            RiskLevel::Medium => {
                recommendations.push("📊 Medium impact change. Review affected models and tests before merging.".to_string());
            }
            RiskLevel::Low => {
                recommendations.push("✅ Low impact change. Standard review process is sufficient.".to_string());
            }
        }

        // Model-specific recommendations
        if downstream_impact.models.len() > 10 {
            recommendations.push(format!(
                "Consider running a full refresh of the {} affected downstream models after deployment.",
                downstream_impact.models.len()
            ));
        }

        // Test-specific recommendations
        if downstream_impact.tests.len() > 20 {
            recommendations.push(format!(
                "Large number of tests affected ({}). Ensure CI/CD pipeline has sufficient time for test execution.",
                downstream_impact.tests.len()
            ));
        }

        // Warehouse-specific recommendations
        for impact in &downstream_impact.warehouse_impact {
            recommendations.extend(impact.recommendations.clone());
        }

        recommendations
    }

    /// Health check for the agent
    pub async fn health_check(&self) -> Result<()> {
        // Try to load manifest to verify artifact parser is working
        let mut artifact_parser = self.artifact_parser.clone();
        artifact_parser.load_manifest()?;
        
        // Try to build a basic dependency graph
        let _graph = self.lineage_analyzer.build_dependency_graph()?;
        
        Ok(())
    }
}

#[async_trait]
impl AgentCommunication for ImpactAnalysisAgent {
    fn agent_name(&self) -> &str {
        &self.agent_name
    }

    async fn handle_event(&self, event: &AgentEvent) -> Result<()> {
        // Handle events from other agents if needed
        match event.event_type.as_str() {
            "pr_analysis_started" => {
                info!("Received PR analysis start notification from {}", event.agent_name);
            }
            _ => {
                // Ignore other event types
            }
        }
        Ok(())
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
            "nodes": {
                "model.test.test_model": {
                    "name": "test_model",
                    "resource_type": "model",
                    "original_file_path": "models/test_model.sql",
                    "depends_on": {
                        "nodes": [],
                        "macros": []
                    }
                }
            },
            "sources": {}
        })
    }

    #[tokio::test]
    async fn test_impact_agent_creation() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        
        let manifest = create_test_manifest();
        let manifest_path = target_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let communication_bus = Arc::new(AgentCommunicationBus::new());
        
        let agent = ImpactAnalysisAgent::new(artifact_parser, communication_bus);
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        
        let manifest = create_test_manifest();
        let manifest_path = target_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let communication_bus = Arc::new(AgentCommunicationBus::new());
        let agent = ImpactAnalysisAgent::new(artifact_parser, communication_bus).unwrap();
        
        assert!(agent.health_check().await.is_ok());
    }
}