use crate::agents::communication::{AgentCommunicationBus, AgentCommunication};
use crate::artifacts::ArtifactParser;
use crate::types::*;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Quality Validation Agent - ensures code quality, documentation, and testing standards
pub struct QualityValidationAgent {
    artifact_parser: ArtifactParser,
    communication_bus: Arc<AgentCommunicationBus>,
    agent_name: String,
}

impl QualityValidationAgent {
    pub fn new(
        artifact_parser: ArtifactParser,
        communication_bus: Arc<AgentCommunicationBus>,
    ) -> Result<Self> {
        Ok(Self {
            artifact_parser,
            communication_bus,
            agent_name: "quality_validation".to_string(),
        })
    }

    /// Validate quality aspects of PR changes
    #[instrument(skip(self, pr_context), fields(pr_number = pr_context.pr_number))]
    pub async fn validate(&self, pr_context: &PRContext) -> Result<QualityReport> {
        info!("Starting quality validation for PR #{}", pr_context.pr_number);

        // Publish validation start event
        self.publish_event(
            &self.communication_bus,
            "quality_validation_started".to_string(),
            serde_json::json!({
                "pr_number": pr_context.pr_number,
                "changed_files_count": pr_context.changed_files.len()
            }),
        ).await?;

        // Validate SQL quality
        let sql_quality = self.validate_sql_quality(pr_context).await?;

        // Validate documentation
        let documentation_quality = self.validate_documentation_quality(pr_context).await?;

        // Validate test coverage
        let test_coverage = self.validate_test_coverage(pr_context).await?;

        // Validate standards compliance
        let standards_compliance = self.validate_standards_compliance(pr_context).await?;

        // Validate schema changes
        let schema_validation = self.validate_schema_changes(pr_context).await?;

        // Calculate overall score
        let overall_score = self.calculate_overall_score(
            &sql_quality,
            &documentation_quality,
            &test_coverage,
            &standards_compliance,
        );

        let report = QualityReport {
            id: Uuid::new_v4(),
            generated_at: chrono::Utc::now(),
            sql_quality,
            documentation_quality,
            test_coverage,
            standards_compliance,
            schema_validation,
            overall_score,
        };

        // Publish validation complete event
        self.publish_event(
            &self.communication_bus,
            "quality_validation_completed".to_string(),
            serde_json::json!({
                "pr_number": pr_context.pr_number,
                "overall_score": overall_score,
                "total_issues": report.total_issues(),
                "has_critical_issues": report.has_critical_issues()
            }),
        ).await?;

        info!(
            "Quality validation completed: {:.1}% overall score, {} total issues",
            overall_score,
            report.total_issues()
        );

        Ok(report)
    }

    /// Validate SQL code quality
    async fn validate_sql_quality(&self, pr_context: &PRContext) -> Result<SqlQualityResult> {
        let sql_files = pr_context.get_changed_sql_files();
        
        let mut syntax_errors = Vec::new();
        let mut complexity_issues = Vec::new();
        let mut best_practice_violations = Vec::new();

        for file in sql_files {
            // TODO: Implement actual SQL parsing and validation
            // For now, create stub validation
            if file.filename.contains("deprecated") {
                best_practice_violations.push(QualityIssue {
                    file_path: file.filename.clone(),
                    line_number: None,
                    column_number: None,
                    issue_type: "deprecated_usage".to_string(),
                    severity: Severity::Medium,
                    message: "File contains deprecated patterns".to_string(),
                    suggestion: Some("Consider updating to current best practices".to_string()),
                });
            }
        }

        let score = if syntax_errors.is_empty() && complexity_issues.is_empty() && best_practice_violations.is_empty() {
            100.0
        } else {
            85.0 - (syntax_errors.len() * 20 + complexity_issues.len() * 10 + best_practice_violations.len() * 5) as f64
        };

        Ok(SqlQualityResult {
            syntax_errors,
            complexity_issues,
            best_practice_violations,
            score: score.max(0.0),
        })
    }

    /// Validate documentation quality
    async fn validate_documentation_quality(&self, pr_context: &PRContext) -> Result<DocumentationQualityResult> {
        let mut missing_descriptions = Vec::new();
        let mut incomplete_column_docs = Vec::new();
        let mut doc_block_issues = Vec::new();

        // TODO: Implement actual documentation validation using artifact parser
        // This is a stub implementation

        let completeness_score = if missing_descriptions.is_empty() && incomplete_column_docs.is_empty() {
            100.0
        } else {
            75.0
        };

        Ok(DocumentationQualityResult {
            missing_descriptions,
            incomplete_column_docs,
            doc_block_issues,
            completeness_score,
        })
    }

    /// Validate test coverage
    async fn validate_test_coverage(&self, pr_context: &PRContext) -> Result<TestCoverageResult> {
        let mut models_without_tests = Vec::new();
        let mut insufficient_test_coverage = Vec::new();
        let mut test_recommendations = Vec::new();

        // TODO: Implement actual test coverage analysis
        // This is a stub implementation

        let coverage_percentage = 85.0; // Stub value

        if coverage_percentage < 80.0 {
            test_recommendations.push("Consider adding more tests to improve coverage".to_string());
        }

        Ok(TestCoverageResult {
            models_without_tests,
            insufficient_test_coverage,
            test_recommendations,
            coverage_percentage,
        })
    }

    /// Validate standards compliance
    async fn validate_standards_compliance(&self, pr_context: &PRContext) -> Result<StandardsComplianceResult> {
        let mut naming_violations = Vec::new();
        let mut formatting_issues = Vec::new();
        let mut structural_issues = Vec::new();

        // TODO: Implement actual standards validation
        // This is a stub implementation

        let compliance_score = 90.0; // Stub value

        Ok(StandardsComplianceResult {
            naming_violations,
            formatting_issues,
            structural_issues,
            compliance_score,
        })
    }

    /// Validate schema changes
    async fn validate_schema_changes(&self, pr_context: &PRContext) -> Result<SchemaValidationResult> {
        let mut breaking_changes = Vec::new();
        let mut backward_compatible_changes = Vec::new();

        // TODO: Implement actual schema change detection
        // This is a stub implementation

        let risk_assessment = if breaking_changes.is_empty() {
            RiskLevel::Low
        } else {
            RiskLevel::High
        };

        Ok(SchemaValidationResult {
            breaking_changes,
            backward_compatible_changes,
            risk_assessment,
        })
    }

    /// Calculate overall quality score
    fn calculate_overall_score(
        &self,
        sql_quality: &SqlQualityResult,
        documentation_quality: &DocumentationQualityResult,
        test_coverage: &TestCoverageResult,
        standards_compliance: &StandardsComplianceResult,
    ) -> f64 {
        // Weighted average of different quality aspects
        let sql_weight = 0.3;
        let doc_weight = 0.2;
        let test_weight = 0.3;
        let standards_weight = 0.2;

        let weighted_score = sql_quality.score * sql_weight
            + documentation_quality.completeness_score * doc_weight
            + test_coverage.coverage_percentage * test_weight
            + standards_compliance.compliance_score * standards_weight;

        weighted_score
    }

    /// Health check for the agent
    pub async fn health_check(&self) -> Result<()> {
        // Verify artifact parser is working
        let mut artifact_parser = self.artifact_parser.clone();
        artifact_parser.load_manifest()?;
        
        Ok(())
    }
}

#[async_trait]
impl AgentCommunication for QualityValidationAgent {
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
            "nodes": {},
            "sources": {}
        })
    }

    #[tokio::test]
    async fn test_quality_agent_creation() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        
        let manifest = create_test_manifest();
        let manifest_path = target_dir.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let artifact_parser = ArtifactParser::new(temp_dir.path());
        let communication_bus = Arc::new(AgentCommunicationBus::new());
        
        let agent = QualityValidationAgent::new(artifact_parser, communication_bus);
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
        let agent = QualityValidationAgent::new(artifact_parser, communication_bus).unwrap();
        
        assert!(agent.health_check().await.is_ok());
    }
}