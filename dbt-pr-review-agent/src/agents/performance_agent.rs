use crate::agents::communication::{AgentCommunicationBus, AgentCommunication};
use crate::artifacts::ArtifactParser;
use crate::types::*;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Performance & Cost Agent - analyzes query performance and cost implications
pub struct PerformanceCostAgent {
    artifact_parser: ArtifactParser,
    communication_bus: Arc<AgentCommunicationBus>,
    agent_name: String,
}

impl PerformanceCostAgent {
    pub fn new(
        artifact_parser: ArtifactParser,
        communication_bus: Arc<AgentCommunicationBus>,
    ) -> Result<Self> {
        Ok(Self {
            artifact_parser,
            communication_bus,
            agent_name: "performance_cost".to_string(),
        })
    }

    /// Assess performance and cost implications of PR changes
    #[instrument(skip(self, pr_context), fields(pr_number = pr_context.pr_number))]
    pub async fn assess(&self, pr_context: &PRContext) -> Result<PerformanceReport> {
        info!("Starting performance assessment for PR #{}", pr_context.pr_number);

        // Publish assessment start event
        self.publish_event(
            &self.communication_bus,
            "performance_assessment_started".to_string(),
            serde_json::json!({
                "pr_number": pr_context.pr_number,
                "changed_files_count": pr_context.changed_files.len()
            }),
        ).await?;

        // Load baseline performance metrics
        let baseline_metrics = self.load_baseline_metrics().await?;

        // Analyze cost impact
        let cost_impact = self.analyze_cost_impact(pr_context, &baseline_metrics).await?;

        // Analyze performance changes
        let performance_changes = self.analyze_performance_changes(pr_context, &baseline_metrics).await?;

        // Generate optimization recommendations
        let optimization_recommendations = self.generate_optimization_recommendations(
            pr_context,
            &performance_changes,
        ).await?;

        // Assess overall risk
        let risk_assessment = self.assess_performance_risk(&performance_changes, &cost_impact);

        let report = PerformanceReport {
            id: Uuid::new_v4(),
            generated_at: chrono::Utc::now(),
            cost_impact,
            performance_changes,
            optimization_recommendations,
            risk_assessment,
        };

        // Publish assessment complete event
        self.publish_event(
            &self.communication_bus,
            "performance_assessment_completed".to_string(),
            serde_json::json!({
                "pr_number": pr_context.pr_number,
                "cost_change_percentage": report.cost_impact.cost_change_percentage,
                "risk_level": format!("{:?}", report.risk_assessment),
                "optimization_count": report.optimization_recommendations.len()
            }),
        ).await?;

        info!(
            "Performance assessment completed: {:.1}% cost impact, {} optimizations suggested",
            report.cost_impact.cost_change_percentage,
            report.optimization_recommendations.len()
        );

        Ok(report)
    }

    /// Load baseline performance metrics from historical data
    async fn load_baseline_metrics(&self) -> Result<HashMap<String, f64>> {
        // TODO: Implement actual baseline loading from run_results.json or external storage
        // This is a stub implementation
        let mut baseline = HashMap::new();
        
        // Load performance metrics from artifact parser if available
        if let Some(metrics) = self.artifact_parser.get_performance_metrics()? {
            baseline.extend(metrics);
        }

        Ok(baseline)
    }

    /// Analyze cost impact of the changes
    async fn analyze_cost_impact(
        &self,
        pr_context: &PRContext,
        baseline_metrics: &HashMap<String, f64>,
    ) -> Result<CostAnalysis> {
        // TODO: Implement actual cost analysis based on warehouse type and query patterns
        // This is a stub implementation

        let estimated_cost_change = 0.0; // No change detected
        let cost_change_percentage = 0.0;

        let mut cost_breakdown = HashMap::new();
        cost_breakdown.insert("compute".to_string(), 0.0);
        cost_breakdown.insert("storage".to_string(), 0.0);
        cost_breakdown.insert("network".to_string(), 0.0);

        let cost_drivers = vec![];

        Ok(CostAnalysis {
            estimated_cost_change,
            cost_change_percentage,
            cost_breakdown,
            cost_drivers,
        })
    }

    /// Analyze performance changes
    async fn analyze_performance_changes(
        &self,
        pr_context: &PRContext,
        baseline_metrics: &HashMap<String, f64>,
    ) -> Result<PerformanceAnalysis> {
        // TODO: Implement actual performance change analysis
        // This is a stub implementation

        let execution_time_change = 0.0;
        let execution_time_percentage = 0.0;

        let resource_usage_change = ResourceUsageChange {
            cpu_change: 0.0,
            memory_change: 0.0,
            io_change: 0.0,
            network_change: 0.0,
        };

        let performance_regressions = vec![];

        Ok(PerformanceAnalysis {
            execution_time_change,
            execution_time_percentage,
            resource_usage_change,
            performance_regressions,
        })
    }

    /// Generate optimization recommendations
    async fn generate_optimization_recommendations(
        &self,
        pr_context: &PRContext,
        performance_changes: &PerformanceAnalysis,
    ) -> Result<Vec<OptimizationRecommendation>> {
        let mut recommendations = Vec::new();

        // Check for models that could benefit from materialization changes
        for change in &pr_context.changed_files {
            if change.filename.ends_with(".sql") && change.filename.contains("models/") {
                // TODO: Analyze actual SQL content and model configuration
                // This is a stub implementation
                
                if change.additions > 100 {
                    recommendations.push(OptimizationRecommendation {
                        recommendation_type: "materialization".to_string(),
                        target_model: change.filename.clone(),
                        description: "Consider materializing as table for better performance with large additions".to_string(),
                        estimated_improvement: 25.0,
                        implementation_effort: ImplementationEffort::Low,
                        priority: Priority::Medium,
                    });
                }
            }
        }

        // Add performance regression recommendations
        for regression in &performance_changes.performance_regressions {
            recommendations.push(OptimizationRecommendation {
                recommendation_type: "performance_fix".to_string(),
                target_model: regression.model_name.clone(),
                description: format!(
                    "Address {} regression: {:.1}% increase in {}",
                    regression.model_name, regression.change_percentage, regression.metric_name
                ),
                estimated_improvement: regression.change_percentage.abs(),
                implementation_effort: match regression.severity {
                    Severity::Critical => ImplementationEffort::High,
                    Severity::High => ImplementationEffort::Medium,
                    _ => ImplementationEffort::Low,
                },
                priority: match regression.severity {
                    Severity::Critical => Priority::Critical,
                    Severity::High => Priority::High,
                    _ => Priority::Medium,
                },
            });
        }

        Ok(recommendations)
    }

    /// Assess overall performance risk
    fn assess_performance_risk(
        &self,
        performance_changes: &PerformanceAnalysis,
        cost_impact: &CostAnalysis,
    ) -> RiskLevel {
        // Check for critical conditions
        if cost_impact.cost_change_percentage > 50.0 {
            return RiskLevel::Critical;
        }

        if performance_changes.execution_time_percentage > 100.0 {
            return RiskLevel::Critical;
        }

        // Check for high risk conditions
        if cost_impact.cost_change_percentage > 25.0 || performance_changes.execution_time_percentage > 50.0 {
            return RiskLevel::High;
        }

        // Check for medium risk conditions
        if cost_impact.cost_change_percentage > 10.0 || performance_changes.execution_time_percentage > 20.0 {
            return RiskLevel::Medium;
        }

        // Check for performance regressions
        let has_critical_regressions = performance_changes.performance_regressions
            .iter()
            .any(|r| matches!(r.severity, Severity::Critical));

        if has_critical_regressions {
            return RiskLevel::High;
        }

        let has_high_regressions = performance_changes.performance_regressions
            .iter()
            .any(|r| matches!(r.severity, Severity::High));

        if has_high_regressions {
            return RiskLevel::Medium;
        }

        RiskLevel::Low
    }

    /// Health check for the agent
    pub async fn health_check(&self) -> Result<()> {
        // Verify artifact parser is working
        let mut artifact_parser = self.artifact_parser.clone();
        artifact_parser.load_manifest()?;
        
        // Try to load performance metrics
        let _metrics = self.load_baseline_metrics().await?;
        
        Ok(())
    }
}

#[async_trait]
impl AgentCommunication for PerformanceCostAgent {
    fn agent_name(&self) -> &str {
        &self.agent_name
    }

    async fn handle_event(&self, event: &AgentEvent) -> Result<()> {
        // Handle events from other agents if needed
        match event.event_type.as_str() {
            "pr_analysis_started" => {
                info!("Received PR analysis start notification from {}", event.agent_name);
            }
            "impact_analysis_completed" => {
                // Could use impact analysis results to inform performance assessment
                info!("Impact analysis completed, incorporating results into performance assessment");
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
            "nodes": {},
            "sources": {}
        })
    }

    #[tokio::test]
    async fn test_performance_agent_creation() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        
        let manifest = create_test_manifest();
        let manifest_path = target_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let communication_bus = Arc::new(AgentCommunicationBus::new());
        
        let agent = PerformanceCostAgent::new(artifact_parser, communication_bus);
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
        let agent = PerformanceCostAgent::new(artifact_parser, communication_bus).unwrap();
        
        assert!(agent.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_risk_assessment() {
        let temp_dir = TempDir::new().unwrap();
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let communication_bus = Arc::new(AgentCommunicationBus::new());
        let agent = PerformanceCostAgent::new(artifact_parser, communication_bus).unwrap();

        // Test low risk scenario
        let performance_changes = PerformanceAnalysis {
            execution_time_change: 1.0,
            execution_time_percentage: 5.0,
            resource_usage_change: ResourceUsageChange {
                cpu_change: 0.0,
                memory_change: 0.0,
                io_change: 0.0,
                network_change: 0.0,
            },
            performance_regressions: vec![],
        };

        let cost_impact = CostAnalysis {
            estimated_cost_change: 1.0,
            cost_change_percentage: 5.0,
            cost_breakdown: HashMap::new(),
            cost_drivers: vec![],
        };

        let risk = agent.assess_performance_risk(&performance_changes, &cost_impact);
        assert!(matches!(risk, RiskLevel::Low));

        // Test high risk scenario
        let high_cost_impact = CostAnalysis {
            estimated_cost_change: 100.0,
            cost_change_percentage: 75.0,
            cost_breakdown: HashMap::new(),
            cost_drivers: vec![],
        };

        let high_risk = agent.assess_performance_risk(&performance_changes, &high_cost_impact);
        assert!(matches!(high_risk, RiskLevel::Critical));
    }
}