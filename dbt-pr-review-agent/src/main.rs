use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dbt_pr_review_agent::{
    agents::PRReviewOrchestrator,
    artifacts::ArtifactParser,
    github::GitHubClient,
    types::*,
};
use std::path::PathBuf;
use tokio;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "dbt-pr-agent")]
#[command(about = "Universal dbt PR Review Agent with multi-agent architecture")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Project path (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    project_path: PathBuf,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a Pull Request
    AnalyzePR {
        /// GitHub repository name (owner/repo)
        #[arg(short, long)]
        repo: String,

        /// Pull request number
        #[arg(short, long)]
        pr_number: u64,

        /// GitHub token for API access
        #[arg(short, long, env = "GITHUB_TOKEN")]
        token: String,

        /// Output format (json, markdown, text)
        #[arg(short, long, default_value = "markdown")]
        output: String,

        /// Output file path (defaults to stdout)
        #[arg(short = 'f', long)]
        output_file: Option<PathBuf>,
    },

    /// Analyze local changes
    AnalyzeLocal {
        /// Changed files to analyze
        #[arg(short, long)]
        files: Vec<String>,

        /// Output format (json, markdown, text)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Health check of the system
    HealthCheck,

    /// Initialize configuration file
    Init {
        /// Configuration file path
        #[arg(short, long, default_value = "dbt-pr-agent.yml")]
        config_file: PathBuf,
    },

    /// Validate dbt project structure
    ValidateProject,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    init_tracing(&cli.log_level)?;

    info!("Starting dbt PR Review Agent");

    // Load configuration
    let config = load_config(cli.config.as_ref()).await?;

    match cli.command {
        Commands::AnalyzePR {
            repo,
            pr_number,
            token,
            output,
            output_file,
        } => {
            analyze_pr(
                cli.project_path,
                repo,
                pr_number,
                token,
                output,
                output_file,
                config,
            )
            .await?;
        }

        Commands::AnalyzeLocal { files, output } => {
            analyze_local_changes(cli.project_path, files, output, config).await?;
        }

        Commands::HealthCheck => {
            health_check(cli.project_path, config).await?;
        }

        Commands::Init { config_file } => {
            init_config(config_file).await?;
        }

        Commands::ValidateProject => {
            validate_project(cli.project_path).await?;
        }
    }

    Ok(())
}

/// Initialize tracing with the specified log level
fn init_tracing(log_level: &str) -> Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(log_level))
        .context("Failed to create env filter")?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_level(true),
        )
        .with(env_filter)
        .init();

    Ok(())
}

/// Load configuration from file or use defaults
async fn load_config(config_path: Option<&PathBuf>) -> Result<AgentConfig> {
    if let Some(path) = config_path {
        if path.exists() {
            info!("Loading configuration from: {:?}", path);
            let content = tokio::fs::read_to_string(path).await
                .with_context(|| format!("Failed to read config file: {:?}", path))?;

            let config: AgentConfig = serde_yaml::from_str(&content)
                .with_context(|| "Failed to parse configuration file")?;

            return Ok(config);
        } else {
            warn!("Configuration file not found: {:?}. Using defaults.", path);
        }
    }

    // Return default configuration
    Ok(AgentConfig {
        name: "dbt-pr-agent".to_string(),
        enabled: true,
        timeout_seconds: 300,
        retry_attempts: 3,
        config: std::collections::HashMap::new(),
    })
}

/// Analyze a GitHub Pull Request
async fn analyze_pr(
    project_path: PathBuf,
    repo: String,
    pr_number: u64,
    token: String,
    output_format: String,
    output_file: Option<PathBuf>,
    _config: AgentConfig,
) -> Result<()> {
    info!("Analyzing PR #{} in repository {}", pr_number, repo);

    // Initialize GitHub client
    let github_client = GitHubClient::new(token)?;

    // Fetch PR context
    let pr_context = github_client
        .get_pr_context(&repo, pr_number)
        .await
        .context("Failed to fetch PR context from GitHub")?;

    info!("Fetched PR context: {} changed files", pr_context.changed_files.len());

    // Initialize artifact parser and orchestrator
    let artifact_parser = ArtifactParser::new(&project_path);
    let orchestrator_config = crate::agents::orchestrator::OrchestratorConfig::default();
    let orchestrator = PRReviewOrchestrator::new(artifact_parser, orchestrator_config)
        .context("Failed to create PR review orchestrator")?;

    // Perform comprehensive analysis
    let comprehensive_report = orchestrator
        .analyze_pr(pr_context)
        .await
        .context("Failed to analyze PR")?;

    // Output results
    output_report(&comprehensive_report, &output_format, output_file.as_ref()).await?;

    info!("PR analysis completed successfully");
    Ok(())
}

/// Analyze local changes without GitHub integration
async fn analyze_local_changes(
    project_path: PathBuf,
    files: Vec<String>,
    output_format: String,
    _config: AgentConfig,
) -> Result<()> {
    info!("Analyzing {} local files", files.len());

    if files.is_empty() {
        warn!("No files specified for analysis");
        return Ok(());
    }

    // Create mock PR context for local analysis
    let pr_context = PRContext {
        repo_name: "local".to_string(),
        pr_number: 0,
        base_branch: "main".to_string(),
        head_branch: "local".to_string(),
        changed_files: files
            .into_iter()
            .map(|filename| ChangeDetail {
                filename,
                status: ChangeStatus::Modified,
                additions: 0,
                deletions: 0,
                patch: None,
            })
            .collect(),
        author: "local".to_string(),
        title: "Local Analysis".to_string(),
        description: Some("Analyzing local file changes".to_string()),
        created_at: chrono::Utc::now(),
    };

    // Initialize artifact parser and orchestrator
    let artifact_parser = ArtifactParser::new(&project_path);
    let orchestrator_config = crate::agents::orchestrator::OrchestratorConfig::default();
    let orchestrator = PRReviewOrchestrator::new(artifact_parser, orchestrator_config)
        .context("Failed to create PR review orchestrator")?;

    // Perform analysis
    let comprehensive_report = orchestrator
        .analyze_pr(pr_context)
        .await
        .context("Failed to analyze local changes")?;

    // Output results
    output_report(&comprehensive_report, &output_format, None).await?;

    info!("Local analysis completed successfully");
    Ok(())
}

/// Perform health check of the system
async fn health_check(project_path: PathBuf, _config: AgentConfig) -> Result<()> {
    info!("Performing system health check");

    // Check if dbt artifacts exist
    let artifact_parser = ArtifactParser::new(&project_path);
    
    // Try to create orchestrator
    let orchestrator_config = crate::agents::orchestrator::OrchestratorConfig::default();
    match PRReviewOrchestrator::new(artifact_parser, orchestrator_config) {
        Ok(orchestrator) => {
            // Perform health check
            match orchestrator.health_check().await {
                Ok(health_status) => {
                    if health_status.healthy {
                        info!("✅ System health check passed");
                        println!("System Status: Healthy");
                        for component in health_status.components {
                            println!("  {}: {}", component.name, if component.healthy { "✅" } else { "❌" });
                        }
                    } else {
                        error!("❌ System health check failed");
                        println!("System Status: Unhealthy");
                        for component in health_status.components {
                            println!("  {}: {}", component.name, if component.healthy { "✅" } else { "❌" });
                        }
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    error!("Health check failed: {}", e);
                    println!("System Status: Error - {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            error!("Failed to initialize orchestrator: {}", e);
            println!("System Status: Error - Failed to initialize: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Initialize configuration file
async fn init_config(config_file: PathBuf) -> Result<()> {
    info!("Initializing configuration file: {:?}", config_file);

    if config_file.exists() {
        warn!("Configuration file already exists: {:?}", config_file);
        print!("Overwrite existing file? (y/N): ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if !input.trim().to_lowercase().starts_with('y') {
            info!("Configuration initialization cancelled");
            return Ok(());
        }
    }

    let default_config = r#"# dbt PR Review Agent Configuration

# Agent settings
agent:
  timeout_seconds: 300
  max_retries: 3
  parallel_execution: true
  fail_fast: false

# Quality gates
quality_gates:
  critical_issues:
    enabled: true
    block_merge: true
  
  performance_threshold:
    cost_increase_percent: 25
    execution_time_increase_percent: 50
  
  test_coverage:
    minimum_coverage_percent: 80
    require_tests_for_new_models: true

# Notification settings
notifications:
  slack:
    enabled: false
    webhook_url: ""
  
  email:
    enabled: false
    recipients: []

# Integration settings
integrations:
  github:
    auto_comment: true
    update_pr_status: true
  
  dbt_cloud:
    enabled: false
    api_key: ""
"#;

    tokio::fs::write(&config_file, default_config)
        .await
        .with_context(|| format!("Failed to write configuration file: {:?}", config_file))?;

    info!("Configuration file created successfully: {:?}", config_file);
    println!("Configuration file created: {:?}", config_file);
    println!("Edit this file to customize the agent behavior.");

    Ok(())
}

/// Validate dbt project structure
async fn validate_project(project_path: PathBuf) -> Result<()> {
    info!("Validating dbt project structure at: {:?}", project_path);

    let mut artifact_parser = ArtifactParser::new(&project_path);

    // Check for dbt_project.yml
    if !project_path.join("dbt_project.yml").exists() {
        error!("❌ dbt_project.yml not found. This doesn't appear to be a valid dbt project.");
        std::process::exit(1);
    }
    println!("✅ dbt_project.yml found");

    // Check for target directory
    let target_path = project_path.join("target");
    if !target_path.exists() {
        error!("❌ target/ directory not found. Run 'dbt compile' or 'dbt run' first.");
        std::process::exit(1);
    }
    println!("✅ target/ directory found");

    // Check for manifest.json
    match artifact_parser.load_manifest() {
        Ok(_) => println!("✅ manifest.json loaded successfully"),
        Err(e) => {
            error!("❌ Failed to load manifest.json: {}", e);
            println!("Run 'dbt compile' to generate manifest.json");
            std::process::exit(1);
        }
    }

    // Check for catalog.json (optional)
    match artifact_parser.load_catalog() {
        Ok(Some(_)) => println!("✅ catalog.json found"),
        Ok(None) => println!("⚠️  catalog.json not found. Run 'dbt docs generate' for enhanced analysis."),
        Err(e) => println!("⚠️  catalog.json issue: {}", e),
    }

    // Get project statistics
    match artifact_parser.get_all_models() {
        Ok(models) => {
            println!("📊 Project Statistics:");
            println!("  Models: {}", models.len());
            
            let materializations: std::collections::HashMap<String, usize> = models
                .iter()
                .fold(std::collections::HashMap::new(), |mut acc, model| {
                    *acc.entry(model.materialization.clone()).or_insert(0) += 1;
                    acc
                });
            
            for (mat_type, count) in materializations {
                println!("    {}: {}", mat_type, count);
            }
        }
        Err(e) => {
            error!("❌ Failed to analyze models: {}", e);
        }
    }

    match artifact_parser.get_all_sources() {
        Ok(sources) => println!("  Sources: {}", sources.len()),
        Err(e) => error!("❌ Failed to count sources: {}", e),
    }

    match artifact_parser.get_all_tests() {
        Ok(tests) => println!("  Tests: {}", tests.len()),
        Err(e) => error!("❌ Failed to count tests: {}", e),
    }

    info!("Project validation completed");
    println!("\n✅ dbt project validation completed successfully!");

    Ok(())
}

/// Output the comprehensive report in the specified format
async fn output_report(
    report: &ComprehensiveReport,
    format: &str,
    output_file: Option<&PathBuf>,
) -> Result<()> {
    let content = match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(report)?,
        "markdown" => generate_markdown_report(report),
        "text" => generate_text_report(report),
        _ => {
            warn!("Unknown output format '{}', using text", format);
            generate_text_report(report)
        }
    };

    if let Some(file_path) = output_file {
        tokio::fs::write(file_path, &content)
            .await
            .with_context(|| format!("Failed to write output to: {:?}", file_path))?;
        info!("Report written to: {:?}", file_path);
    } else {
        println!("{}", content);
    }

    Ok(())
}

/// Generate markdown format report
fn generate_markdown_report(report: &ComprehensiveReport) -> String {
    format!(
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
    )
}

/// Generate plain text format report
fn generate_text_report(report: &ComprehensiveReport) -> String {
    format!(
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
    )
}