use crate::agents::communication::{AgentCommunicationBus, AgentCommunication};
use crate::artifacts::ArtifactParser;
use crate::types::*;
use crate::llm::{LLMProvider, LLMRequest, Message, MessageRole, Model, AnalysisContext, AgentPrompts, ResponseFormat, AgentType};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, instrument};
use uuid::Uuid;

/// Quality Validation Agent - ensures code quality, documentation, and testing standards
pub struct QualityValidationAgent {
    artifact_parser: ArtifactParser,
    communication_bus: Arc<AgentCommunicationBus>,
    agent_name: String,
    llm_provider: Option<Box<dyn LLMProvider>>,
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
            agent_name: "quality_validation".to_string(),
            llm_provider: Some(llm_provider),
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

        // Use LLM if available for advanced analysis
        if let Some(llm) = &self.llm_provider {
            let llm_analysis = self.analyze_sql_quality_with_llm(pr_context).await?;
            
            // Extract issues from LLM analysis
            if let Some(findings) = llm_analysis["findings"].as_array() {
                for finding in findings {
                    let severity = finding["severity"].as_str().unwrap_or("Medium");
                    let category = finding["category"].as_str().unwrap_or("General");
                    let description = finding["description"].as_str().unwrap_or("");
                    let affected_files = finding["affected_resources"].as_array()
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>())
                        .unwrap_or_default();
                    
                    let issue = QualityIssue {
                        file_path: affected_files.first().cloned().unwrap_or_default(),
                        line_number: None,
                        column_number: None,
                        issue_type: category.to_string(),
                        severity: match severity {
                            "Critical" => Severity::Critical,
                            "High" => Severity::High,
                            "Medium" => Severity::Medium,
                            "Low" => Severity::Low,
                            _ => Severity::Medium,
                        },
                        message: description.to_string(),
                        suggestion: finding["recommendation"].as_str().map(|s| s.to_string()),
                    };
                    
                    match category {
                        "Syntax Error" => syntax_errors.push(issue),
                        "Complexity" => complexity_issues.push(issue),
                        _ => best_practice_violations.push(issue),
                    }
                }
            }
            
            let score = llm_analysis["metrics"]["code_quality_score"].as_f64().unwrap_or(85.0) * 100.0;
            
            return Ok(SqlQualityResult {
                syntax_errors,
                complexity_issues,
                best_practice_violations,
                score,
            });
        }

        // Fallback to basic validation
        for file in sql_files {
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
    
    /// Analyze SQL quality using LLM
    async fn analyze_sql_quality_with_llm(&self, pr_context: &PRContext) -> Result<serde_json::Value> {
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
        
        // Get test results if available
        let test_results = if let Ok(run_results) = artifact_parser.load_run_results() {
            if let Some(results) = run_results {
                let mut test_map = HashMap::new();
                if let Some(results_array) = results["results"].as_array() {
                    for result in results_array {
                        if let (Some(unique_id), Some(status)) = 
                            (result["unique_id"].as_str(), result["status"].as_str()) {
                            test_map.insert(
                                unique_id.to_string(),
                                crate::llm::interfaces::TestResult {
                                    status: status.to_string(),
                                    message: result["message"].as_str().map(|s| s.to_string()),
                                    severity: "info".to_string(),
                                }
                            );
                        }
                    }
                }
                Some(test_map)
            } else {
                None
            }
        } else {
            None
        };
        
        let context = AnalysisContext {
            pr_diff,
            model_definitions,
            lineage_graph: String::new(), // Not needed for quality analysis
            test_results,
            historical_metrics: None,
            warehouse_type: artifact_parser.get_warehouse_type()?.unwrap_or("unknown".to_string()),
        };
        
        // Get the prompt template for quality validation
        let prompt_template = AgentPrompts::get_template(&AgentType::Quality);
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
        
        Ok(analysis)
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