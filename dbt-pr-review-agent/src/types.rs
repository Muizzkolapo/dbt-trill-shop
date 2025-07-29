use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Core types for the dbt PR Review Agent system

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRContext {
    pub repo_name: String,
    pub pr_number: u64,
    pub base_branch: String,
    pub head_branch: String,
    pub changed_files: Vec<ChangeDetail>,
    pub author: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDetail {
    pub filename: String,
    pub status: ChangeStatus,
    pub additions: u32,
    pub deletions: u32,
    pub patch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    pub model_patterns: ModelPatterns,
    pub folder_structure: HashMap<String, Vec<String>>,
    pub naming_conventions: NamingConventions,
    pub warehouse_info: WarehouseInfo,
    pub total_models: usize,
    pub total_tests: usize,
    pub total_sources: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPatterns {
    pub folder_groups: HashMap<String, Vec<String>>,
    pub materialization_patterns: HashMap<String, Vec<String>>,
    pub total_folders: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingConventions {
    pub model_prefix_patterns: Vec<String>,
    pub staging_patterns: Vec<String>,
    pub mart_patterns: Vec<String>,
    pub test_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseInfo {
    pub adapter_type: String,
    pub database: String,
    pub schema: String,
    pub supports_partitioning: bool,
    pub supports_clustering: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub unique_id: String,
    pub name: String,
    pub file_path: String,
    pub folder: String,
    pub materialization: String,
    pub dependencies: Vec<String>,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub description: String,
    pub columns: HashMap<String, ColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub description: String,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDependencies {
    pub upstream_models: Vec<String>,
    pub upstream_sources: Vec<String>,
    pub macros_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownstreamImpact {
    pub models: Vec<String>,
    pub tests: Vec<String>,
    pub sources: Vec<String>,
    pub total_affected: usize,
    pub warehouse_impact: Vec<WarehouseImpact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseImpact {
    pub warehouse_type: String,
    pub impact_type: String,
    pub description: String,
    pub severity: Severity,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub directly_affected: Vec<String>,
    pub downstream_models: Vec<String>,
    pub affected_tests: Vec<String>,
    pub affected_sources: Vec<String>,
    pub affected_documentation: Vec<String>,
    pub impact_score: f64,
    pub risk_level: RiskLevel,
    pub visualization: Option<String>,
    pub warehouse_specific_impact: Vec<WarehouseImpact>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub sql_quality: SqlQualityResult,
    pub documentation_quality: DocumentationQualityResult,
    pub test_coverage: TestCoverageResult,
    pub standards_compliance: StandardsComplianceResult,
    pub schema_validation: SchemaValidationResult,
    pub overall_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlQualityResult {
    pub syntax_errors: Vec<QualityIssue>,
    pub complexity_issues: Vec<QualityIssue>,
    pub best_practice_violations: Vec<QualityIssue>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationQualityResult {
    pub missing_descriptions: Vec<String>,
    pub incomplete_column_docs: Vec<String>,
    pub doc_block_issues: Vec<QualityIssue>,
    pub completeness_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCoverageResult {
    pub models_without_tests: Vec<String>,
    pub insufficient_test_coverage: Vec<String>,
    pub test_recommendations: Vec<String>,
    pub coverage_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardsComplianceResult {
    pub naming_violations: Vec<QualityIssue>,
    pub formatting_issues: Vec<QualityIssue>,
    pub structural_issues: Vec<QualityIssue>,
    pub compliance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaValidationResult {
    pub breaking_changes: Vec<SchemaChange>,
    pub backward_compatible_changes: Vec<SchemaChange>,
    pub risk_assessment: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub file_path: String,
    pub line_number: Option<u32>,
    pub column_number: Option<u32>,
    pub issue_type: String,
    pub severity: Severity,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChange {
    pub model_name: String,
    pub change_type: SchemaChangeType,
    pub field_name: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub impact_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaChangeType {
    ColumnAdded,
    ColumnRemoved,
    ColumnRenamed,
    TypeChanged,
    ConstraintAdded,
    ConstraintRemoved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub cost_impact: CostAnalysis,
    pub performance_changes: PerformanceAnalysis,
    pub optimization_recommendations: Vec<OptimizationRecommendation>,
    pub risk_assessment: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalysis {
    pub estimated_cost_change: f64,
    pub cost_change_percentage: f64,
    pub cost_breakdown: HashMap<String, f64>,
    pub cost_drivers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysis {
    pub execution_time_change: f64,
    pub execution_time_percentage: f64,
    pub resource_usage_change: ResourceUsageChange,
    pub performance_regressions: Vec<PerformanceRegression>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageChange {
    pub cpu_change: f64,
    pub memory_change: f64,
    pub io_change: f64,
    pub network_change: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRegression {
    pub model_name: String,
    pub metric_name: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub change_percentage: f64,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub recommendation_type: String,
    pub target_model: String,
    pub description: String,
    pub estimated_improvement: f64,
    pub implementation_effort: ImplementationEffort,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementationEffort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveReport {
    pub id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub pr_context: PRContext,
    pub executive_summary: ExecutiveSummary,
    pub overall_risk_level: RiskLevel,
    pub impact_report: ImpactReport,
    pub quality_report: QualityReport,
    pub performance_report: PerformanceReport,
    pub recommendations: Vec<String>,
    pub approval_status: ApprovalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    pub summary: String,
    pub key_findings: Vec<String>,
    pub critical_issues: Vec<String>,
    pub recommendation_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Approved,
    ApprovedWithConditions,
    ChangesRequested,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub agent_name: String,
    pub event_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub enabled: bool,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub analysis_duration_seconds: f64,
    pub models_analyzed: usize,
    pub tests_executed: usize,
    pub errors_encountered: usize,
    pub warnings_generated: usize,
    pub recommendations_created: usize,
}

impl Default for RiskLevel {
    fn default() -> Self {
        RiskLevel::Low
    }
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Low
    }
}

impl Default for ApprovalStatus {
    fn default() -> Self {
        ApprovalStatus::Approved
    }
}

impl PRContext {
    pub fn get_changed_sql_files(&self) -> Vec<&ChangeDetail> {
        self.changed_files
            .iter()
            .filter(|change| change.filename.ends_with(".sql"))
            .collect()
    }

    pub fn get_changed_yaml_files(&self) -> Vec<&ChangeDetail> {
        self.changed_files
            .iter()
            .filter(|change| {
                change.filename.ends_with(".yml") || change.filename.ends_with(".yaml")
            })
            .collect()
    }

    pub fn has_breaking_changes(&self) -> bool {
        self.changed_files
            .iter()
            .any(|change| matches!(change.status, ChangeStatus::Deleted))
    }
}

impl ImpactReport {
    pub fn is_high_impact(&self) -> bool {
        matches!(self.risk_level, RiskLevel::High | RiskLevel::Critical)
    }

    pub fn total_affected_resources(&self) -> usize {
        self.downstream_models.len()
            + self.affected_tests.len()
            + self.affected_sources.len()
            + self.affected_documentation.len()
    }
}

impl QualityReport {
    pub fn has_critical_issues(&self) -> bool {
        self.sql_quality.syntax_errors.iter().any(|issue| matches!(issue.severity, Severity::Critical))
            || self.schema_validation.breaking_changes.len() > 0
    }

    pub fn total_issues(&self) -> usize {
        self.sql_quality.syntax_errors.len()
            + self.sql_quality.complexity_issues.len()
            + self.sql_quality.best_practice_violations.len()
            + self.documentation_quality.missing_descriptions.len()
            + self.documentation_quality.incomplete_column_docs.len()
            + self.standards_compliance.naming_violations.len()
            + self.standards_compliance.formatting_issues.len()
    }
}

impl PerformanceReport {
    pub fn has_performance_regressions(&self) -> bool {
        !self.performance_changes.performance_regressions.is_empty()
    }

    pub fn estimated_cost_impact(&self) -> f64 {
        self.cost_impact.estimated_cost_change
    }
}

impl ComprehensiveReport {
    pub fn should_block_merge(&self) -> bool {
        matches!(self.approval_status, ApprovalStatus::Blocked | ApprovalStatus::ChangesRequested)
            || matches!(self.overall_risk_level, RiskLevel::Critical)
    }

    pub fn total_recommendations(&self) -> usize {
        self.recommendations.len()
            + self.impact_report.recommendations.len()
            + self.performance_report.optimization_recommendations.len()
    }
}