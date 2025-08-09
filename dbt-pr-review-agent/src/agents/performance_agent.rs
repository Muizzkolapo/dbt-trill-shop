use crate::agents::communication::{AgentCommunicationBus, AgentCommunication};
use crate::artifacts::ArtifactParser;
use crate::types::*;
use crate::llm::{LLMProvider, LLMRequest, Message, MessageRole, Model, AnalysisContext, AgentPrompts, ResponseFormat, AgentType};
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
    llm_provider: Option<Box<dyn LLMProvider>>,
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
            llm_provider: None,
        })
    }
    
    /// Create a new instance with LLM provider
    pub fn with_llm(
        artifact_parser: ArtifactParser,
        communication_bus: Arc<AgentCommunicationBus>,
        llm_provider: Box<dyn LLMProvider>,
    ) -> Result<Self> {
        Ok(Self {
            artifact_parser,
            communication_bus,
            agent_name: "performance_cost".to_string(),
            llm_provider: Some(llm_provider),
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

        // Use LLM if available for advanced recommendations
        if let Some(llm) = &self.llm_provider {
            let llm_recommendations = self.generate_llm_recommendations(pr_context, performance_changes).await?;
            recommendations.extend(llm_recommendations);
            return Ok(recommendations);
        }

        // Fallback to rule-based recommendations
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
    
    /// Generate recommendations using LLM
    async fn generate_llm_recommendations(
        &self,
        pr_context: &PRContext,
        performance_changes: &PerformanceAnalysis,
    ) -> Result<Vec<OptimizationRecommendation>> {
        let llm = self.llm_provider.as_ref().unwrap();
        
        // Prepare context for LLM
        let pr_diff = pr_context.changed_files.iter()
            .filter(|c| c.filename.ends_with(".sql"))
            .map(|c| format!("File: {}\n+++ {} additions\n--- {} deletions", 
                c.filename, c.additions, c.deletions))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        // Get model definitions for changed models
        let mut model_definitions = HashMap::new();
        let mut artifact_parser = self.artifact_parser.clone();
        let changed_models = artifact_parser.discover_changed_models(
            &pr_context.changed_files.iter()
                .map(|c| c.filename.clone())
                .collect::<Vec<_>>()
        )?;
        
        for model_id in &changed_models {
            if let Ok(def) = artifact_parser.get_model_definition(model_id) {
                model_definitions.insert(model_id.clone(), def);
            }
        }
        
        // Prepare historical metrics
        let historical_metrics = if let Ok(baseline) = self.load_baseline_metrics().await {
            let avg_query_time = baseline.values().sum::<f64>() / baseline.len() as f64;
            Some(crate::llm::interfaces::HistoricalMetrics {
                avg_query_time,
                avg_bytes_scanned: 0, // Would need actual data
                failure_rate: 0.0,
                last_30_days_cost: 0.0, // Would need actual data
            })
        } else {
            None
        };
        
        let context = AnalysisContext {
            pr_diff,
            model_definitions,
            lineage_graph: String::new(), // Not needed for performance analysis
            test_results: None,
            historical_metrics,
            warehouse_type: artifact_parser.get_warehouse_type()?.unwrap_or("unknown".to_string()),
        };
        
        // Get the prompt template for performance analysis
        let prompt_template = AgentPrompts::get_template(&AgentType::Performance);
        let prompt = AgentPrompts::build_prompt(&prompt_template, &context);
        
        // Create LLM request
        let request = LLMRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: prompt,
                tool_calls: None,
            }],
            model: Model::GPT4Turbo,
            temperature: Some(0.3),
            max_tokens: Some(4096),
            system_prompt: Some(prompt_template.system_prompt),
            tools: None,
            response_format: Some(ResponseFormat::JsonObject),
        };
        
        // Get LLM response
        let response = llm.complete(request).await?;
        
        // Parse the structured response
        let analysis: serde_json::Value = serde_json::from_str(&response.content)?;
        
        // Extract optimization recommendations from LLM response
        let mut recommendations = Vec::new();
        
        if let Some(llm_recs) = analysis["recommendations"].as_array() {
            for rec in llm_recs {
                if let Some(action) = rec["action"].as_str() {
                    let priority_str = rec["priority"].as_str().unwrap_or("Medium");
                    let priority = match priority_str {
                        "Urgent" | "Critical" => Priority::Critical,
                        "High" => Priority::High,
                        "Low" => Priority::Low,
                        _ => Priority::Medium,
                    };
                    
                    let effort = if action.contains("incremental") || action.contains("partition") {
                        ImplementationEffort::Medium
                    } else if action.contains("index") || action.contains("cluster") {
                        ImplementationEffort::Low
                    } else {
                        ImplementationEffort::High
                    };
                    
                    let estimated_improvement = analysis["metrics"]["performance_improvement_factor"]
                        .as_f64()
                        .unwrap_or(1.0) * 10.0; // Convert to percentage
                    
                    recommendations.push(OptimizationRecommendation {
                        recommendation_type: "llm_suggested".to_string(),
                        target_model: rec["affected_resources"][0].as_str().unwrap_or("").to_string(),
                        description: action.to_string(),
                        estimated_improvement,
                        implementation_effort: effort,
                        priority,
                    });
                }
            }
        }
        
        // Add cost-based recommendations if significant cost changes detected
        if let Some(cost_change) = analysis["metrics"]["estimated_monthly_cost_change"].as_f64() {
            if cost_change.abs() > 100.0 {
                recommendations.push(OptimizationRecommendation {
                    recommendation_type: "cost_optimization".to_string(),
                    target_model: "Multiple models".to_string(),
                    description: format!("Significant cost impact detected: ${:.2} monthly change. Review materialization strategies.", cost_change),
                    estimated_improvement: (cost_change.abs() / 100.0).min(50.0),
                    implementation_effort: ImplementationEffort::Medium,
                    priority: if cost_change > 500.0 { Priority::Critical } else { Priority::High },
                });
            }
        }
        
        Ok(recommendations)
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