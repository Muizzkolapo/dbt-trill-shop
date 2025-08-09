use crate::types::ComprehensiveReport;
use anyhow::Result;

/// Report generator for creating various output formats
pub struct ReportGenerator;

impl ReportGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate report in the specified format
    pub fn generate(&self, report: &ComprehensiveReport, format: &str) -> Result<String> {
        match format.to_lowercase().as_str() {
            "json" => self.generate_json(report),
            "markdown" => self.generate_markdown(report),
            "text" => self.generate_text(report),
            _ => Err(anyhow::anyhow!("Unsupported format: {}", format)),
        }
    }

    /// Generate JSON format report
    fn generate_json(&self, report: &ComprehensiveReport) -> Result<String> {
        Ok(serde_json::to_string_pretty(report)?)
    }

    /// Generate Markdown format report
    fn generate_markdown(&self, report: &ComprehensiveReport) -> Result<String> {
        Ok(format!(
            r#"# dbt PR Review Report

**PR**: #{} - {}
**Repository**: {}
**Risk Level**: {:?}
**Approval Status**: {:?}

## Executive Summary
{}

### Key Findings
{}

### Critical Issues
{}

## Impact Analysis
- **Directly Affected Models**: {}
- **Downstream Models**: {}
- **Affected Tests**: {}
- **Total Resources Affected**: {}
- **Impact Score**: {:.2}

## Quality Assessment
- **Overall Score**: {:.1}%
- **Total Issues**: {}
- **Test Coverage**: {:.1}%

## Performance Analysis
- **Estimated Cost Impact**: {:.1}%
- **Performance Regressions**: {}

## Recommendations
{}

---
*Generated at: {}*
"#,
            report.pr_context.pr_number,
            report.pr_context.title,
            report.pr_context.repo_name,
            report.overall_risk_level,
            report.approval_status,
            report.executive_summary.summary,
            report.executive_summary.key_findings.join("\n- "),
            if report.executive_summary.critical_issues.is_empty() {
                "None".to_string()
            } else {
                report.executive_summary.critical_issues.join("\n- ")
            },
            report.impact_report.directly_affected.len(),
            report.impact_report.downstream_models.len(),
            report.impact_report.affected_tests.len(),
            report.impact_report.total_affected_resources(),
            report.impact_report.impact_score,
            report.quality_report.overall_score,
            report.quality_report.total_issues(),
            report.quality_report.test_coverage.coverage_percentage,
            report.performance_report.cost_impact.cost_change_percentage,
            report.performance_report.performance_changes.performance_regressions.len(),
            if report.recommendations.is_empty() {
                "None".to_string()
            } else {
                report.recommendations.join("\n- ")
            },
            report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ))
    }

    /// Generate plain text format report
    fn generate_text(&self, report: &ComprehensiveReport) -> Result<String> {
        Ok(format!(
            r#"dbt PR Review Report
===================

PR: #{} - {}
Repository: {}
Risk Level: {:?}
Approval Status: {:?}

Executive Summary:
{}

Impact Analysis:
- Directly Affected Models: {}
- Downstream Models: {}
- Affected Tests: {}
- Impact Score: {:.2}

Quality Assessment:
- Overall Score: {:.1}%
- Total Issues: {}

Performance Analysis:
- Cost Impact: {:.1}%
- Performance Regressions: {}

Recommendations: {}

Generated at: {}
"#,
            report.pr_context.pr_number,
            report.pr_context.title,
            report.pr_context.repo_name,
            report.overall_risk_level,
            report.approval_status,
            report.executive_summary.summary,
            report.impact_report.directly_affected.len(),
            report.impact_report.downstream_models.len(),
            report.impact_report.affected_tests.len(),
            report.impact_report.impact_score,
            report.quality_report.overall_score,
            report.quality_report.total_issues(),
            report.performance_report.cost_impact.cost_change_percentage,
            report.performance_report.performance_changes.performance_regressions.len(),
            report.recommendations.len(),
            report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ))
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}