# Quick Start Guide

## 1. Build the Agent

```bash
cd dbt-pr-review-agent
cargo build --release
```

## 2. Test with Your Current dbt Project

From the parent dbt project directory:

```bash
# Go to your dbt project root (where dbt_project.yml is)
cd ../

# Make sure you have dbt artifacts
dbt compile  # Creates manifest.json

# Test the agent
./dbt-pr-review-agent/target/release/dbt-pr-agent validate-project

# Analyze some models
./dbt-pr-review-agent/target/release/dbt-pr-agent analyze-local \
  --files models/staging/stg_customers.sql \
  --output markdown
```

## 3. Test with LLM (Optional)

```bash
# Set your API key
export OPENAI_API_KEY="sk-..."  # or
export ANTHROPIC_API_KEY="..."

# Run the same command - it will now use AI
./dbt-pr-review-agent/target/release/dbt-pr-agent analyze-local \
  --files models/staging/stg_customers.sql \
  --output markdown
```

## 4. Run Automated Tests

```bash
cd dbt-pr-review-agent

# Run unit tests
cargo test

# Run integration test script
./examples/test_local.sh ../

# Test with different scenarios (requires Python)
python3 examples/test_with_llm.py
```

## 5. Example Output

### Without LLM:
```markdown
# dbt PR Review Report

**Risk Level**: Medium
**Impact Score**: 15.5

## Impact Analysis
- Directly Affected Models: 1
- Downstream Models: 5
- Affected Tests: 12

## Recommendations
- Medium impact change. Review affected models and tests before merging.
- Consider running a full refresh of the 5 affected downstream models.
```

### With LLM:
```markdown
# dbt PR Review Report

**Risk Level**: High
**Impact Score**: 15.5

## Impact Analysis
- Directly Affected Models: 1
- Downstream Models: 5
- Affected Tests: 12

### AI-Powered Insights
**Critical Finding**: The change to `stg_customers` removes the `customer_lifetime_value` calculation which is used by downstream revenue forecasting models.

## Recommendations
- [High] Add data validation tests for new join key - Ensure data integrity during transition
- [Medium] Consider implementing incremental materialization - Reduce daily processing costs by 90%
- ⚠️ High: Schema change affects critical business metrics used in executive dashboards
```

## 6. Configuration File (Optional)

Create `.dbt-pr-agent.toml` in your project:

```toml
[llm]
provider = "openai"
default_model = "gpt-4-turbo-preview"
temperature = 0.3

[agents]
[agents.impact]
use_llm = true

[agents.quality]
use_llm = true

[agents.performance]
use_llm = true
```

## Common Commands

```bash
# Validate project structure
dbt-pr-agent validate-project

# Check agent health
dbt-pr-agent health-check

# Analyze specific files
dbt-pr-agent analyze-local --files model1.sql model2.sql

# Analyze with PR context
dbt-pr-agent analyze-local \
  --files models/marts/customers.sql \
  --author "jane-doe" \
  --title "Update customer segmentation logic" \
  --base-branch "main"

# Different output formats
dbt-pr-agent analyze-local --files model.sql --output json
dbt-pr-agent analyze-local --files model.sql --output markdown
dbt-pr-agent analyze-local --files model.sql --output text

# Analyze a GitHub PR (requires token)
dbt-pr-agent analyze-pr \
  --repo "org/repo" \
  --pr-number 123 \
  --token $GITHUB_TOKEN
```