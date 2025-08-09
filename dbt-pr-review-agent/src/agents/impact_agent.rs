use crate::agents::communication::{AgentCommunicationBus, AgentCommunication};
use crate::artifacts::ArtifactParser;
use crate::lineage::LineageAnalyzer;
use crate::types::*;
use crate::llm::{LLMProvider, LLMRequest, Message, MessageRole, Model, AnalysisContext, AgentPrompts, ResponseFormat};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, instrument};
use uuid::Uuid;

/// Impact Analysis Agent - analyzes downstream effects of PR changes
pub struct ImpactAnalysisAgent {
    artifact_parser: ArtifactParser,
    lineage_analyzer: LineageAnalyzer,
    communication_bus: Arc<AgentCommunicationBus>,
    agent_name: String,
    llm_provider: Option<Box<dyn LLMProvider>>,
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
            llm_provider: None,
        })
    }
    
    /// Create a new instance with LLM provider
    pub fn with_llm(
        artifact_parser: ArtifactParser,
        communication_bus: Arc<AgentCommunicationBus>,
        llm_provider: Box<dyn LLMProvider>,
    ) -> Result<Self> {
        let lineage_analyzer = LineageAnalyzer::new(artifact_parser.clone());
        
        Ok(Self {
            artifact_parser,
            lineage_analyzer,
            communication_bus,
            agent_name: "impact_analysis".to_string(),
            llm_provider: Some(llm_provider),
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

        // Generate recommendations - use LLM if available
        let recommendations = if let Some(llm) = &self.llm_provider {
            self.generate_llm_recommendations(pr_context, &downstream_impact, &risk_level).await?
        } else {
            self.generate_recommendations(&downstream_impact, &risk_level)
        };

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
    
    /// Generate recommendations using LLM
    async fn generate_llm_recommendations(
        &self,
        pr_context: &PRContext,
        downstream_impact: &DownstreamImpact,
        risk_level: &RiskLevel,
    ) -> Result<Vec<String>> {
        let llm = self.llm_provider.as_ref().unwrap();
        
        // Prepare context for LLM
        let pr_diff = pr_context.changed_files.iter()
            .map(|c| format!("File: {}\n+++ {} additions\n--- {} deletions", 
                c.filename, c.additions, c.deletions))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        // Get model definitions for affected models
        let mut model_definitions = HashMap::new();
        let mut artifact_parser = self.artifact_parser.clone();
        for model in &downstream_impact.models {
            if let Ok(def) = artifact_parser.get_model_definition(model) {
                model_definitions.insert(model.clone(), def);
            }
        }
        
        // Convert lineage graph to DOT format for LLM
        let lineage_graph_dot = self.lineage_analyzer.export_to_dot(&affected_models)?;
        
        let context = AnalysisContext {
            pr_diff,
            model_definitions,
            lineage_graph: lineage_graph_dot,
            test_results: None, // Could be populated if test results are available
            historical_metrics: None, // Could be populated from historical data
            warehouse_type: artifact_parser.get_warehouse_type()?.unwrap_or("unknown".to_string()),
        };
        
        // Get the prompt template for impact analysis
        let prompt_template = AgentPrompts::impact_analysis();
        let prompt = AgentPrompts::build_prompt(&prompt_template, &context);
        
        // Create LLM request
        let request = LLMRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: prompt,
                tool_calls: None,
            }],
            model: Model::GPT4Turbo, // Could be configurable
            temperature: Some(0.3), // Lower temperature for more consistent analysis
            max_tokens: Some(4096),
            system_prompt: Some(prompt_template.system_prompt),
            tools: None,
            response_format: Some(ResponseFormat::JsonObject),
        };
        
        // Get LLM response
        let response = llm.complete(request).await?;
        
        // Parse the structured response
        let analysis: serde_json::Value = serde_json::from_str(&response.content)?;
        
        // Extract recommendations from the LLM response
        let mut recommendations = Vec::new();
        
        // Add basic risk level recommendation
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
        
        // Add LLM-generated recommendations
        if let Some(llm_recs) = analysis["recommendations"].as_array() {
            for rec in llm_recs {
                if let Some(action) = rec["action"].as_str() {
                    let priority = rec["priority"].as_str().unwrap_or("Medium");
                    let rationale = rec["rationale"].as_str().unwrap_or("");
                    recommendations.push(format!("[{}] {} - {}", priority, action, rationale));
                }
            }
        }
        
        // Add findings as recommendations if they're critical
        if let Some(findings) = analysis["findings"].as_array() {
            for finding in findings {
                if let Some(severity) = finding["severity"].as_str() {
                    if severity == "Critical" || severity == "High" {
                        if let Some(desc) = finding["description"].as_str() {
                            recommendations.push(format!("⚠️ {}: {}", severity, desc));
                        }
                    }
                }
            }
        }
        
        Ok(recommendations)
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