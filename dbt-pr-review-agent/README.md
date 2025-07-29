# dbt PR Review Agent

A universal dbt PR Review Agent built in Rust with a multi-agent architecture for comprehensive automated pull request analysis.

## Features

- 🔍 **Universal dbt Support**: Works with any dbt project structure (no hardcoded folder assumptions)
- 🏢 **Multi-Warehouse Compatible**: BigQuery, Snowflake, Databricks, Redshift, etc.
- 📊 **Artifact-Based Discovery**: Uses `manifest.json`, `catalog.json`, and `run_results.json`
- 🤖 **Multi-Agent Architecture**: Specialized agents for different analysis domains
- 🔗 **GitHub Integration**: Automated PR comments and status checks
- ⚠️ **Risk Assessment**: Intelligent risk scoring and approval recommendations

## Multi-Agent System

- **Impact Analysis Agent**: Analyzes downstream effects using lineage graphs
- **Quality Validation Agent**: Ensures code quality, documentation, and testing standards  
- **Performance & Cost Agent**: Analyzes query performance and cost implications

## Installation

### Prerequisites
- Rust 1.70+
- A dbt project with generated artifacts (`dbt compile` or `dbt run`)
- GitHub token for PR analysis (optional)

### Build from Source
```bash
git clone <repository-url>
cd dbt-pr-review-agent
cargo build --release
```

## Usage

### Validate Your dbt Project
```bash
./target/release/dbt-pr-agent validate-project
```

### Test Local Analysis
```bash
./target/release/dbt-pr-agent analyze-local \
  --files models/staging/stg_customers.sql
```

### Analyze a GitHub PR
```bash
./target/release/dbt-pr-agent analyze-pr \
  --repo "owner/repo" \
  --pr-number 123 \
  --token $GITHUB_TOKEN \
  --output markdown
```

### Initialize Configuration
```bash
./target/release/dbt-pr-agent init --project-path .
```

### Health Check
```bash
./target/release/dbt-pr-agent health-check
```

## Configuration

The agent automatically discovers dbt projects and their artifacts. No configuration files are required for basic usage.

### Optional Configuration

Create a `.dbt-pr-agent.toml` file in your project root:

```toml
[project]
name = "my-dbt-project"
target_path = "target"
artifacts_path = "target"

[github]
token = "your-github-token"
repo = "owner/repo"

[agents]
[agents.impact]
enabled = true
max_depth = 5

[agents.quality]
enabled = true
min_test_coverage = 0.8

[agents.performance]
enabled = true
cost_threshold = 0.1
```

## Output Formats

The agent supports multiple output formats:

- **JSON**: Machine-readable structured output
- **Markdown**: GitHub-friendly formatted reports
- **Text**: Plain text summary for logs

Example JSON output:
```json
{
  "pr_context": {
    "repo_name": "my-org/my-dbt-project",
    "pr_number": 123,
    "title": "Add new customer segmentation model"
  },
  "overall_risk_level": "Medium",
  "approval_status": "RequiresReview",
  "impact_report": {
    "directly_affected": ["models/marts/customers.sql"],
    "downstream_models": ["models/marts/customer_orders.sql"],
    "impact_score": 0.65
  }
}
```

## Architecture

The agent uses a multi-agent architecture where specialized agents analyze different aspects of the PR:

```
┌─────────────────────┐
│  PR Review Agent    │
│     Orchestrator    │
└──────────┬──────────┘
           │
    ┌──────┴──────┐
    │             │
┌───▼───┐ ┌───────▼───┐ ┌─────────▼─────┐
│Impact │ │  Quality   │ │ Performance & │
│Agent  │ │   Agent    │ │  Cost Agent   │
└───────┘ └───────────┘ └───────────────┘
```

Each agent operates independently and communicates through a shared event bus, enabling parallel analysis and comprehensive reporting.

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with Rust for performance and reliability
- Uses [petgraph](https://github.com/petgraph/petgraph) for lineage analysis
- Inspired by the dbt community's need for automated PR review tools