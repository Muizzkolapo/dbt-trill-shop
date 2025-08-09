use crate::agents::{ImpactAnalysisAgent, QualityValidationAgent, PerformanceCostAgent, AgentCommunicationBus};
use crate::artifacts::ArtifactParser;
use crate::types::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn, instrument};
use uuid::Uuid;

/// Main orchestrator for the dbt PR review agent system
pub struct PRReviewOrchestrator {
    impact_agent: Arc<ImpactAnalysisAgent>,
    quality_agent: Arc<QualityValidationAgent>, 
    performance_agent: Arc<PerformanceCostAgent>,
    communication_bus: Arc<AgentCommunicationBus>,
    config: OrchestratorConfig,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub agent_timeout_seconds: u64,
    pub max_retries: u32,
    pub parallel_execution: bool,
    pub fail_fast: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            agent_timeout_seconds: 300, // 5 minutes
            max_retries: 3,
            parallel_execution: true,
            fail_fast: false,
        }
    }
}

impl PRReviewOrchestrator {
    /// Create a new PR review orchestrator
    pub fn new(
        artifact_parser: ArtifactParser,
        config: OrchestratorConfig,
    ) -> Result<Self> {
        let communication_bus = Arc::new(AgentCommunicationBus::new());
        
        // Initialize specialized agents
        let impact_agent = Arc::new(ImpactAnalysisAgent::new(
            artifact_parser.clone(),
            communication_bus.clone(),
        )?);
        
        let quality_agent = Arc::new(QualityValidationAgent::new(
            artifact_parser.clone(),
            communication_bus.clone(),
        )?);
        
        let performance_agent = Arc::new(PerformanceCostAgent::new(
            artifact_parser,
            communication_bus.clone(),
        )?);

        Ok(Self {
            impact_agent,
            quality_agent,
            performance_agent,
            communication_bus,
            config,
        })
    }

    /// Main PR analysis workflow - orchestrates all specialized agents
    #[instrument(skip(self, pr_context), fields(pr_number = pr_context.pr_number))]
    pub async fn analyze_pr(&self, pr_context: PRContext) -> Result<ComprehensiveReport> {
        info!("Starting comprehensive PR analysis for PR #{}", pr_context.pr_number);
        
        let analysis_start = std::time::Instant::now();
        let report_id = Uuid::new_v4();

        // Publish analysis start event
        self.publish_analysis_event("analysis_started", &pr_context).await?;

        // Execute agent analysis (parallel or sequential based on config)
        let (impact_report, quality_report, performance_report) = if self.config.parallel_execution {
            self.execute_agents_parallel(&pr_context).await?
        } else {
            self.execute_agents_sequential(&pr_context).await?
        };

        // Synthesize comprehensive report
        let comprehensive_report = self.synthesize_reports(
            report_id,
            pr_context,
            impact_report,
            quality_report, 
            performance_report,
        ).await?;

        let analysis_duration = analysis_start.elapsed();
        info!("PR analysis completed in {:.2}s with risk level: {:?}", 
              analysis_duration.as_secs_f64(), comprehensive_report.overall_risk_level);

        // Publish analysis completion event
        self.publish_analysis_event("analysis_completed", &comprehensive_report.pr_context).await?;

        Ok(comprehensive_report)
    }

    /// Execute all agents in parallel for faster analysis
    async fn execute_agents_parallel(
        &self,
        pr_context: &PRContext,
    ) -> Result<(ImpactReport, QualityReport, PerformanceReport)> {
        info!("Executing agents in parallel");

        let timeout_duration = Duration::from_secs(self.config.agent_timeout_seconds);
        
        // Execute all agents concurrently
        let (impact_result, quality_result, performance_result) = tokio::try_join!(
            timeout(timeout_duration, self.execute_impact_agent_with_retry(pr_context)),
            timeout(timeout_duration, self.execute_quality_agent_with_retry(pr_context)),
            timeout(timeout_duration, self.execute_performance_agent_with_retry(pr_context))
        )?;

        Ok((impact_result?, quality_result?, performance_result?))
    }

    /// Execute all agents sequentially (useful for debugging or resource constraints)
    async fn execute_agents_sequential(
        &self,
        pr_context: &PRContext,
    ) -> Result<(ImpactReport, QualityReport, PerformanceReport)> {
        info!("Executing agents sequentially");

        let timeout_duration = Duration::from_secs(self.config.agent_timeout_seconds);

        let impact_report = timeout(
            timeout_duration,
            self.execute_impact_agent_with_retry(pr_context)
        ).await??;

        let quality_report = timeout(
            timeout_duration,
            self.execute_quality_agent_with_retry(pr_context)
        ).await??;

        let performance_report = timeout(
            timeout_duration,
            self.execute_performance_agent_with_retry(pr_context)
        ).await??;

        Ok((impact_report, quality_report, performance_report))
    }

    /// Execute impact analysis agent with retry logic
    async fn execute_impact_agent_with_retry(&self, pr_context: &PRContext) -> Result<ImpactReport> {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            match self.impact_agent.analyze(pr_context).await {
                Ok(report) => {
                    info!("Impact analysis completed successfully on attempt {}", attempt);
                    return Ok(report);
                },
                Err(e) => {
                    warn!("Impact analysis failed on attempt {}: {}", attempt, e);
                    last_error = Some(e);
                    
                    if self.config.fail_fast && attempt == 1 {
                        break;
                    }
                    
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(Duration::from_secs(attempt as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Impact analysis failed after {} attempts", self.config.max_retries)))
    }

    /// Execute quality validation agent with retry logic
    async fn execute_quality_agent_with_retry(&self, pr_context: &PRContext) -> Result<QualityReport> {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            match self.quality_agent.validate(pr_context).await {
                Ok(report) => {
                    info!("Quality validation completed successfully on attempt {}", attempt);
                    return Ok(report);
                },
                Err(e) => {
                    warn!("Quality validation failed on attempt {}: {}", attempt, e);
                    last_error = Some(e);
                    
                    if self.config.fail_fast && attempt == 1 {
                        break;
                    }
                    
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(Duration::from_secs(attempt as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Quality validation failed after {} attempts", self.config.max_retries)))
    }

    /// Execute performance analysis agent with retry logic
    async fn execute_performance_agent_with_retry(&self, pr_context: &PRContext) -> Result<PerformanceReport> {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            match self.performance_agent.assess(pr_context).await {
                Ok(report) => {
                    info!("Performance analysis completed successfully on attempt {}", attempt);
                    return Ok(report);
                },
                Err(e) => {
                    warn!("Performance analysis failed on attempt {}: {}", attempt, e);
                    last_error = Some(e);
                    
                    if self.config.fail_fast && attempt == 1 {
                        break;
                    }
                    
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(Duration::from_secs(attempt as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Performance analysis failed after {} attempts", self.config.max_retries)))
    }

    /// Synthesize individual agent reports into comprehensive analysis
    async fn synthesize_reports(
        &self,
        report_id: Uuid,
        pr_context: PRContext,
        impact_report: ImpactReport,
        quality_report: QualityReport,
        performance_report: PerformanceReport,
    ) -> Result<ComprehensiveReport> {
        info!("Synthesizing comprehensive report from agent outputs");

        // Calculate overall risk level based on individual reports
        let overall_risk_level = self.calculate_overall_risk_level(
            &impact_report.risk_level,
            &quality_report,
            &performance_report.risk_assessment,
        );

        // Generate executive summary
        let executive_summary = self.create_executive_summary(
            &impact_report,
            &quality_report,
            &performance_report,
        );

        // Generate cross-agent recommendations
        let recommendations = self.generate_cross_agent_recommendations(
            &impact_report,
            &quality_report,
            &performance_report,
        );

        // Determine approval status
        let approval_status = self.determine_approval_status(
            &overall_risk_level,
            &impact_report,
            &quality_report,
            &performance_report,
        );

        Ok(ComprehensiveReport {
            id: report_id,
            generated_at: Utc::now(),
            pr_context,
            executive_summary,
            overall_risk_level,
            impact_report,
            quality_report,
            performance_report,
            recommendations,
            approval_status,
        })
    }

    /// Calculate overall risk level from individual agent assessments
    fn calculate_overall_risk_level(
        &self,
        impact_risk: &RiskLevel,
        quality_report: &QualityReport,
        performance_risk: &RiskLevel,
    ) -> RiskLevel {
        // If any agent reports critical risk, overall is critical
        if matches!(impact_risk, RiskLevel::Critical) 
            || matches!(performance_risk, RiskLevel::Critical)
            || quality_report.has_critical_issues() {
            return RiskLevel::Critical;
        }

        // If any agent reports high risk, overall is high
        if matches!(impact_risk, RiskLevel::High) 
            || matches!(performance_risk, RiskLevel::High)
            || quality_report.overall_score < 50.0 {
            return RiskLevel::High;
        }

        // If any agent reports medium risk, overall is medium
        if matches!(impact_risk, RiskLevel::Medium) 
            || matches!(performance_risk, RiskLevel::Medium)
            || quality_report.overall_score < 75.0 {
            return RiskLevel::Medium;
        }

        RiskLevel::Low
    }

    /// Create executive summary from agent reports
    fn create_executive_summary(
        &self,
        impact_report: &ImpactReport,
        quality_report: &QualityReport,
        performance_report: &PerformanceReport,
    ) -> ExecutiveSummary {
        let mut key_findings = Vec::new();
        let mut critical_issues = Vec::new();

        // Impact findings
        if impact_report.is_high_impact() {
            key_findings.push(format!(
                "High impact changes affecting {} downstream resources",
                impact_report.total_affected_resources()
            ));
        }

        // Quality findings
        if quality_report.has_critical_issues() {
            critical_issues.push("Critical quality issues detected requiring immediate attention".to_string());
        }

        let total_quality_issues = quality_report.total_issues();
        if total_quality_issues > 0 {
            key_findings.push(format!("{} quality issues identified", total_quality_issues));
        }

        // Performance findings
        if performance_report.has_performance_regressions() {
            key_findings.push("Performance regressions detected".to_string());
        }

        let cost_impact = performance_report.estimated_cost_impact();
        if cost_impact.abs() > 0.1 {
            key_findings.push(format!(
                "Estimated cost impact: {:.1}%",
                performance_report.cost_impact.cost_change_percentage
            ));
        }

        let total_recommendations = impact_report.recommendations.len()
            + performance_report.optimization_recommendations.len();

        let summary = if critical_issues.is_empty() {
            format!(
                "PR analysis completed with {} key findings and {} recommendations. Overall risk level is appropriate for review.",
                key_findings.len(),
                total_recommendations
            )
        } else {
            format!(
                "PR analysis identified {} critical issues that must be addressed before merge. {} additional findings and {} recommendations provided.",
                critical_issues.len(),
                key_findings.len(),
                total_recommendations
            )
        };

        ExecutiveSummary {
            summary,
            key_findings,
            critical_issues,
            recommendation_count: total_recommendations,
        }
    }

    /// Generate recommendations that span multiple agent domains
    fn generate_cross_agent_recommendations(
        &self,
        impact_report: &ImpactReport,
        quality_report: &QualityReport,
        performance_report: &PerformanceReport,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // High impact + quality issues
        if impact_report.is_high_impact() && quality_report.has_critical_issues() {
            recommendations.push(
                "High-impact changes with quality issues detected. Consider splitting PR into smaller changes."
                .to_string()
            );
        }

        // Performance regression + high impact
        if performance_report.has_performance_regressions() && impact_report.is_high_impact() {
            recommendations.push(
                "Performance regressions in high-impact changes may compound downstream effects. Review optimization opportunities."
                .to_string()
            );
        }

        // Missing tests on changed models
        let models_without_tests = &quality_report.test_coverage.models_without_tests;
        let changed_models = &impact_report.directly_affected;
        
        let untested_changed_models: Vec<_> = changed_models
            .iter()
            .filter(|model| models_without_tests.iter().any(|untested| untested.contains(&model.split('.').last().unwrap_or(""))))
            .collect();

        if !untested_changed_models.is_empty() {
            recommendations.push(format!(
                "Changed models lack adequate testing: {}. Add tests before merge to prevent downstream issues.",
                untested_changed_models.join(", ")
            ));
        }

        recommendations
    }

    /// Determine approval status based on all agent reports
    fn determine_approval_status(
        &self,
        overall_risk: &RiskLevel,
        impact_report: &ImpactReport,
        quality_report: &QualityReport,
        performance_report: &PerformanceReport,
    ) -> ApprovalStatus {
        // Block for critical risk or critical issues
        if matches!(overall_risk, RiskLevel::Critical) || quality_report.has_critical_issues() {
            return ApprovalStatus::Blocked;
        }

        // Request changes for high risk
        if matches!(overall_risk, RiskLevel::High) {
            return ApprovalStatus::ChangesRequested;
        }

        // Approve with conditions for medium risk
        if matches!(overall_risk, RiskLevel::Medium) {
            return ApprovalStatus::ApprovedWithConditions;
        }

        // Approve for low risk
        ApprovalStatus::Approved
    }

    /// Publish analysis events to the communication bus
    async fn publish_analysis_event(&self, event_type: &str, pr_context: &PRContext) -> Result<()> {
        let event = AgentEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            agent_name: "orchestrator".to_string(),
            event_type: event_type.to_string(),
            data: serde_json::to_value(pr_context)?,
        };

        self.communication_bus.publish(event).await?;
        Ok(())
    }

    /// Get orchestrator health status
    pub async fn health_check(&self) -> Result<HealthStatus> {
        // Check agent availability
        let impact_healthy = self.impact_agent.health_check().await.is_ok();
        let quality_healthy = self.quality_agent.health_check().await.is_ok();
        let performance_healthy = self.performance_agent.health_check().await.is_ok();

        let all_healthy = impact_healthy && quality_healthy && performance_healthy;

        Ok(HealthStatus {
            healthy: all_healthy,
            components: vec![
                ComponentHealth { name: "impact_agent".to_string(), healthy: impact_healthy },
                ComponentHealth { name: "quality_agent".to_string(), healthy: quality_healthy },
                ComponentHealth { name: "performance_agent".to_string(), healthy: performance_healthy },
            ],
            timestamp: Utc::now(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub components: Vec<ComponentHealth>,
    pub timestamp: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub name: String,
    pub healthy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::ArtifactParser;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let config = OrchestratorConfig::default();
        
        let orchestrator = PRReviewOrchestrator::new(artifact_parser, config);
        assert!(orchestrator.is_ok());
    }

    #[tokio::test]
    async fn test_risk_level_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let config = OrchestratorConfig::default();
        let orchestrator = PRReviewOrchestrator::new(artifact_parser, config).unwrap();

        // Test critical risk calculation
        let impact_risk = RiskLevel::Critical;
        let quality_report = QualityReport {
            id: Uuid::new_v4(),
            generated_at: Utc::now(),
            sql_quality: crate::types::SqlQualityResult {
                syntax_errors: vec![],
                complexity_issues: vec![],
                best_practice_violations: vec![],
                score: 100.0,
            },
            documentation_quality: crate::types::DocumentationQualityResult {
                missing_descriptions: vec![],
                incomplete_column_docs: vec![],
                doc_block_issues: vec![],
                completeness_score: 100.0,
            },
            test_coverage: crate::types::TestCoverageResult {
                models_without_tests: vec![],
                insufficient_test_coverage: vec![],
                test_recommendations: vec![],
                coverage_percentage: 100.0,
            },
            standards_compliance: crate::types::StandardsComplianceResult {
                naming_violations: vec![],
                formatting_issues: vec![],
                structural_issues: vec![],
                compliance_score: 100.0,
            },
            schema_validation: crate::types::SchemaValidationResult {
                breaking_changes: vec![],
                backward_compatible_changes: vec![],
                risk_assessment: RiskLevel::Low,
            },
            overall_score: 100.0,
        };
        let performance_risk = RiskLevel::Low;

        let overall_risk = orchestrator.calculate_overall_risk_level(
            &impact_risk,
            &quality_report,
            &performance_risk,
        );

        assert!(matches!(overall_risk, RiskLevel::Critical));
    }
}