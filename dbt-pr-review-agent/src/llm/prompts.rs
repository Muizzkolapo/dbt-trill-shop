use super::interfaces::{AgentType, AnalysisContext};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prompt template for different agents
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub system_prompt: String,
    pub user_prompt_template: String,
    pub examples: Vec<Example>,
    pub output_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct Example {
    pub input: String,
    pub output: String,
}

/// Collection of prompts for all agents
pub struct AgentPrompts;

impl AgentPrompts {
    /// Get impact analysis agent prompt
    pub fn impact_analysis() -> PromptTemplate {
        PromptTemplate {
            system_prompt: r#"You are an expert dbt Impact Analysis Agent specializing in analyzing the downstream effects of changes in dbt projects. Your role is to:

1. Analyze data lineage and dependency graphs
2. Identify all affected downstream models, tests, and exposures
3. Assess the business impact of changes
4. Evaluate risk levels based on criticality and scope
5. Provide clear, actionable insights

You have deep knowledge of:
- dbt project structures and best practices
- Data warehouse optimization patterns
- SQL and data modeling
- Business intelligence workflows
- Data quality implications

Always provide structured analysis with clear severity levels and evidence."#.to_string(),
            
            user_prompt_template: r#"Analyze the impact of the following dbt pull request:

## PR Changes
```diff
{pr_diff}
```

## Current Model Definitions
{model_definitions}

## Lineage Graph (Graphviz DOT format)
```
{lineage_graph}
```

## Warehouse Type: {warehouse_type}

Please analyze:
1. Direct impact on modified models
2. Downstream dependencies affected
3. Potential data quality issues
4. Business impact assessment
5. Risk level determination

Provide your analysis in a structured format with findings, recommendations, and metrics."#.to_string(),
            
            examples: vec![
                Example {
                    input: "Changed join condition in stg_customers from customer_id to user_id".to_string(),
                    output: r#"{
  "findings": [
    {
      "severity": "High",
      "category": "Schema Change",
      "description": "Join key modification affects all downstream models",
      "evidence": ["15 downstream models depend on stg_customers", "3 dashboards directly query affected marts"],
      "affected_resources": ["mart_customer_360", "mart_revenue_analysis", "rpt_daily_metrics"]
    }
  ],
  "recommendations": [
    {
      "priority": "Urgent",
      "action": "Add data validation tests for new join key",
      "rationale": "Ensure data integrity during transition",
      "implementation_hints": ["Test for NULL values", "Verify uniqueness", "Check referential integrity"]
    }
  ],
  "metrics": {
    "downstream_model_count": 15,
    "affected_test_count": 23,
    "impact_radius": 3,
    "risk_score": 0.85
  },
  "confidence": 0.92
}"#.to_string(),
                }
            ],
            
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "findings": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "severity": {"type": "string", "enum": ["Critical", "High", "Medium", "Low", "Info"]},
                                "category": {"type": "string"},
                                "description": {"type": "string"},
                                "evidence": {"type": "array", "items": {"type": "string"}},
                                "affected_resources": {"type": "array", "items": {"type": "string"}}
                            }
                        }
                    },
                    "recommendations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "priority": {"type": "string", "enum": ["Urgent", "High", "Medium", "Low"]},
                                "action": {"type": "string"},
                                "rationale": {"type": "string"},
                                "implementation_hints": {"type": "array", "items": {"type": "string"}}
                            }
                        }
                    },
                    "metrics": {
                        "type": "object",
                        "properties": {
                            "downstream_model_count": {"type": "number"},
                            "affected_test_count": {"type": "number"},
                            "impact_radius": {"type": "number"},
                            "risk_score": {"type": "number"}
                        }
                    },
                    "confidence": {"type": "number"}
                }
            })),
        }
    }
    
    /// Get quality validation agent prompt
    pub fn quality_validation() -> PromptTemplate {
        PromptTemplate {
            system_prompt: r#"You are an expert dbt Quality Validation Agent responsible for ensuring code quality, documentation standards, and testing coverage in dbt projects. Your role is to:

1. Review SQL code quality and style
2. Verify documentation completeness
3. Assess test coverage and quality
4. Check for anti-patterns and best practices
5. Validate naming conventions and standards

You have expertise in:
- SQL optimization and best practices
- dbt testing frameworks and strategies
- Data documentation standards
- Code maintainability patterns
- Data quality dimensions

Focus on actionable feedback that improves code quality and maintainability."#.to_string(),
            
            user_prompt_template: r#"Review the quality of the following dbt pull request:

## PR Changes
```diff
{pr_diff}
```

## Model Definitions and Current State
{model_definitions}

## Test Results (if available)
{test_results}

## Warehouse Type: {warehouse_type}

Please evaluate:
1. SQL code quality and optimization
2. Documentation completeness and clarity
3. Test coverage adequacy
4. Naming convention compliance
5. Best practices adherence
6. Potential quality issues

Provide structured feedback with specific issues and improvement suggestions."#.to_string(),
            
            examples: vec![
                Example {
                    input: "New model added without tests or documentation".to_string(),
                    output: r#"{
  "findings": [
    {
      "severity": "High",
      "category": "Missing Tests",
      "description": "Model lacks basic data quality tests",
      "evidence": ["No schema tests defined", "No data tests present", "Complex business logic untested"],
      "affected_resources": ["models/marts/new_revenue_model.sql"]
    },
    {
      "severity": "Medium",
      "category": "Documentation",
      "description": "Model documentation is incomplete",
      "evidence": ["Missing model description", "Column descriptions absent", "No business context provided"],
      "affected_resources": ["models/marts/new_revenue_model.sql"]
    }
  ],
  "recommendations": [
    {
      "priority": "High",
      "action": "Add essential data quality tests",
      "rationale": "Ensure data reliability and catch issues early",
      "implementation_hints": [
        "Add not_null tests for key columns",
        "Add unique test for primary key",
        "Add relationships test for foreign keys",
        "Consider custom data tests for business rules"
      ]
    }
  ],
  "metrics": {
    "test_coverage": 0.0,
    "documentation_score": 0.3,
    "code_quality_score": 0.75,
    "total_issues": 5
  },
  "confidence": 0.95
}"#.to_string(),
                }
            ],
            
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "findings": {"type": "array"},
                    "recommendations": {"type": "array"},
                    "metrics": {
                        "type": "object",
                        "properties": {
                            "test_coverage": {"type": "number"},
                            "documentation_score": {"type": "number"},
                            "code_quality_score": {"type": "number"},
                            "total_issues": {"type": "number"}
                        }
                    },
                    "confidence": {"type": "number"}
                }
            })),
        }
    }
    
    /// Get performance and cost agent prompt
    pub fn performance_cost() -> PromptTemplate {
        PromptTemplate {
            system_prompt: r#"You are an expert dbt Performance & Cost Analysis Agent specializing in query optimization and cloud data warehouse cost management. Your role is to:

1. Analyze query patterns for performance issues
2. Estimate cost impact of changes
3. Identify optimization opportunities
4. Assess resource utilization patterns
5. Provide cost-saving recommendations

You have deep expertise in:
- Query optimization for {warehouse_type}
- Cloud data warehouse pricing models
- Partitioning and clustering strategies
- Materialization best practices
- Cost-performance trade-offs

Always quantify performance impacts and provide specific optimization strategies."#.to_string(),
            
            user_prompt_template: r#"Analyze the performance and cost implications of the following dbt pull request:

## PR Changes
```diff
{pr_diff}
```

## Model Definitions
{model_definitions}

## Historical Metrics (if available)
{historical_metrics}

## Warehouse Type: {warehouse_type}

Please analyze:
1. Query performance implications
2. Estimated cost changes
3. Resource utilization impact
4. Optimization opportunities
5. Materialization strategy assessment

Focus on specific performance bottlenecks and quantifiable cost impacts."#.to_string(),
            
            examples: vec![
                Example {
                    input: "Changed materialization from view to table for large fact table".to_string(),
                    output: r#"{
  "findings": [
    {
      "severity": "High",
      "category": "Cost Increase",
      "description": "Materialization change will increase storage costs",
      "evidence": [
        "Table size estimated at 2.5TB",
        "Daily refresh will scan 500GB",
        "Monthly storage cost increase: $1,250"
      ],
      "affected_resources": ["models/marts/fct_transactions.sql"]
    },
    {
      "severity": "Medium",
      "category": "Performance Improvement",
      "description": "Query performance will improve for downstream models",
      "evidence": [
        "Downstream queries will be 10x faster",
        "Reduced compute costs for frequent queries",
        "Better cache utilization"
      ],
      "affected_resources": ["models/reports/daily_revenue.sql", "models/reports/customer_ltv.sql"]
    }
  ],
  "recommendations": [
    {
      "priority": "High",
      "action": "Implement incremental materialization",
      "rationale": "Reduce daily processing costs by 90%",
      "implementation_hints": [
        "Use timestamp column for incremental key",
        "Set appropriate lookback window",
        "Add unique key for merge operations"
      ]
    },
    {
      "priority": "Medium",
      "action": "Add table partitioning",
      "rationale": "Improve query performance and reduce scan costs",
      "implementation_hints": [
        "Partition by date column",
        "Consider clustering on frequently filtered columns"
      ]
    }
  ],
  "metrics": {
    "estimated_monthly_cost_change": 1250.00,
    "performance_improvement_factor": 10.0,
    "data_scanned_reduction": 0.0,
    "storage_increase_gb": 2500
  },
  "confidence": 0.88
}"#.to_string(),
                }
            ],
            
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "findings": {"type": "array"},
                    "recommendations": {"type": "array"},
                    "metrics": {
                        "type": "object",
                        "properties": {
                            "estimated_monthly_cost_change": {"type": "number"},
                            "performance_improvement_factor": {"type": "number"},
                            "data_scanned_reduction": {"type": "number"},
                            "storage_increase_gb": {"type": "number"}
                        }
                    },
                    "confidence": {"type": "number"}
                }
            })),
        }
    }
    
    /// Build a prompt with context
    pub fn build_prompt(template: &PromptTemplate, context: &AnalysisContext) -> String {
        let mut prompt = template.user_prompt_template.clone();
        
        // Replace placeholders
        prompt = prompt.replace("{pr_diff}", &context.pr_diff);
        prompt = prompt.replace("{warehouse_type}", &context.warehouse_type);
        prompt = prompt.replace("{lineage_graph}", &context.lineage_graph);
        
        // Format model definitions
        let model_defs = context.model_definitions.iter()
            .map(|(name, def)| format!("### {}\n```sql\n{}\n```", name, def))
            .collect::<Vec<_>>()
            .join("\n\n");
        prompt = prompt.replace("{model_definitions}", &model_defs);
        
        // Format test results if available
        if let Some(test_results) = &context.test_results {
            let test_summary = test_results.iter()
                .map(|(name, result)| {
                    format!("- {} [{}]: {}", 
                        name, 
                        result.status,
                        result.message.as_ref().unwrap_or(&"OK".to_string())
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            prompt = prompt.replace("{test_results}", &test_summary);
        } else {
            prompt = prompt.replace("{test_results}", "No test results available");
        }
        
        // Format historical metrics if available
        if let Some(metrics) = &context.historical_metrics {
            let metrics_summary = format!(
                r#"- Average Query Time: {:.2}s
- Average Bytes Scanned: {:.2}GB
- Failure Rate: {:.1}%
- Last 30 Days Cost: ${:.2}"#,
                metrics.avg_query_time,
                metrics.avg_bytes_scanned as f64 / 1_073_741_824.0,
                metrics.failure_rate * 100.0,
                metrics.last_30_days_cost
            );
            prompt = prompt.replace("{historical_metrics}", &metrics_summary);
        } else {
            prompt = prompt.replace("{historical_metrics}", "No historical metrics available");
        }
        
        // Add examples if needed
        if !template.examples.is_empty() {
            prompt.push_str("\n\n## Examples\n");
            for example in &template.examples {
                prompt.push_str(&format!(
                    "\nInput: {}\nExpected Output:\n```json\n{}\n```\n",
                    example.input,
                    example.output
                ));
            }
        }
        
        prompt
    }
    
    /// Get the appropriate prompt template for an agent type
    pub fn get_template(agent_type: &AgentType) -> PromptTemplate {
        match agent_type {
            AgentType::Impact => Self::impact_analysis(),
            AgentType::Quality => Self::quality_validation(),
            AgentType::Performance => Self::performance_cost(),
        }
    }
}