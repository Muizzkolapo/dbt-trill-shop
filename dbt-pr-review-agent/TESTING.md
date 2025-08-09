# Testing Guide for dbt PR Review Agent

## Quick Start

### 1. Build the Project
```bash
cd dbt-pr-review-agent
cargo build --release
```

### 2. Run Tests
```bash
# Run all unit tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test agents::impact_agent::tests
```

### 3. Test with Your dbt Project

#### Prerequisites
- Your dbt project must have generated artifacts:
```bash
# In your dbt project directory
dbt compile  # Generates manifest.json
dbt docs generate  # Generates catalog.json (optional)
dbt run  # Generates run_results.json (optional)
```

#### Basic Testing Commands

1. **Validate Project Structure**
```bash
./target/release/dbt-pr-agent validate-project --project-path /path/to/your/dbt/project
```

2. **Test Local Analysis (No GitHub)**
```bash
# Analyze specific changed files
./target/release/dbt-pr-agent analyze-local \
  --project-path /path/to/your/dbt/project \
  --files models/staging/stg_customers.sql models/marts/customers.sql \
  --output json
```

3. **Health Check**
```bash
./target/release/dbt-pr-agent health-check --project-path /path/to/your/dbt/project
```

## Testing with LLM Integration

### 1. Set up Environment Variables
```bash
# For OpenAI
export OPENAI_API_KEY="your-api-key"

# For Anthropic
export ANTHROPIC_API_KEY="your-api-key"

# For local Ollama
# Make sure Ollama is running: ollama serve
```

### 2. Create Test Configuration
Create `.dbt-pr-agent.toml` in your project:
```toml
[project]
name = "my-dbt-project"
target_path = "target"

[llm]
provider = "openai"  # or "anthropic" or "ollama"
default_model = "gpt-4-turbo-preview"
temperature = 0.3
max_retries = 3

[agents]
[agents.impact]
enabled = true
use_llm = true

[agents.quality]
enabled = true
use_llm = true

[agents.performance]
enabled = true
use_llm = true
```

### 3. Test with Sample PR
Create a test script `test_pr_review.sh`:
```bash
#!/bin/bash

# Simulate PR changes
./target/release/dbt-pr-agent analyze-local \
  --project-path . \
  --files models/staging/stg_customers.sql \
  --author "test-user" \
  --title "Update customer staging model" \
  --base-branch "main" \
  --output markdown > pr_review_report.md

echo "Review report saved to pr_review_report.md"
```

## Integration Testing

### 1. Mock PR Context Test
Create `test/mock_pr.json`:
```json
{
  "repo_name": "my-org/my-dbt-project",
  "pr_number": 123,
  "title": "Add new customer segmentation model",
  "base_branch": "main",
  "head_branch": "feature/customer-segments",
  "changed_files": [
    {
      "filename": "models/marts/customer_segments.sql",
      "additions": 150,
      "deletions": 0,
      "changes": 150,
      "patch": "+ SELECT * FROM {{ ref('stg_customers') }}"
    }
  ],
  "author": "data-engineer",
  "created_at": "2024-01-15T10:00:00Z"
}
```

### 2. Test Individual Agents
```rust
// test/test_agents.rs
use dbt_pr_review_agent::*;
use std::sync::Arc;

#[tokio::test]
async fn test_impact_agent_with_llm() {
    let config = LLMConfig {
        provider: "openai".to_string(),
        api_key: Some(std::env::var("OPENAI_API_KEY").unwrap()),
        default_model: Model::GPT4Turbo,
        ..Default::default()
    };
    
    let llm_provider = LLMProviderFactory::create(config).unwrap();
    let artifact_parser = ArtifactParser::new(".");
    let comm_bus = Arc::new(AgentCommunicationBus::new());
    
    let agent = ImpactAnalysisAgent::with_llm(
        artifact_parser,
        comm_bus,
        llm_provider
    ).unwrap();
    
    // Test with mock PR context
    let pr_context = create_mock_pr_context();
    let report = agent.analyze(&pr_context).await.unwrap();
    
    assert!(!report.recommendations.is_empty());
    println!("Impact Report: {:?}", report);
}
```

## Testing Scenarios

### 1. Breaking Change Detection
```sql
-- models/staging/stg_orders.sql
-- Change a column name that downstream models depend on
SELECT 
    order_id,
    customer_id as user_id,  -- Changed from customer_id
    order_date
FROM {{ source('raw', 'orders') }}
```

Expected: High risk level, multiple downstream impacts

### 2. Performance Regression
```sql
-- models/marts/large_aggregation.sql
-- Remove incremental materialization
{{ config(
    materialized='table'  -- Changed from 'incremental'
) }}

SELECT * FROM {{ ref('fct_events') }}
WHERE event_date >= '2020-01-01'
```

Expected: Cost increase warning, performance recommendations

### 3. Missing Tests
```sql
-- models/marts/new_model.sql
-- New model without tests
SELECT * FROM {{ ref('stg_customers') }}
-- No schema.yml with tests defined
```

Expected: Quality score reduction, test recommendations

## Debugging

### 1. Enable Debug Logging
```bash
RUST_LOG=debug ./target/release/dbt-pr-agent analyze-local \
  --project-path . \
  --files models/staging/stg_customers.sql
```

### 2. Test Artifact Parsing
```rust
// Quick test to verify artifacts load correctly
use dbt_pr_review_agent::artifacts::ArtifactParser;

let mut parser = ArtifactParser::new(".");
let manifest = parser.load_manifest()?;
println!("Loaded {} nodes", manifest["nodes"].as_object().unwrap().len());
```

### 3. Test LLM Connection
```bash
# Test OpenAI connection
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# Test Ollama connection
curl http://localhost:11434/api/tags
```

## GitHub Integration Testing

### 1. Test with Real PR
```bash
./target/release/dbt-pr-agent analyze-pr \
  --repo "your-org/your-dbt-repo" \
  --pr-number 456 \
  --token $GITHUB_TOKEN \
  --output markdown
```

### 2. Webhook Testing
Set up a test webhook endpoint:
```python
# test_webhook.py
from flask import Flask, request
app = Flask(__name__)

@app.route('/webhook', methods=['POST'])
def webhook():
    # Trigger dbt-pr-agent
    pr_data = request.json
    # Call agent with PR data
    return {'status': 'ok'}

app.run(port=5000)
```

## Performance Testing

### 1. Large Project Test
```bash
# Time analysis for large project
time ./target/release/dbt-pr-agent analyze-local \
  --project-path /path/to/large/project \
  --files $(find models -name "*.sql" | head -20 | tr '\n' ' ')
```

### 2. Memory Usage
```bash
# Monitor memory usage
/usr/bin/time -v ./target/release/dbt-pr-agent validate-project
```

## Continuous Integration

### GitHub Actions Example
```yaml
# .github/workflows/pr-review.yml
name: dbt PR Review

on:
  pull_request:
    paths:
      - 'models/**'
      - 'macros/**'
      - 'tests/**'

jobs:
  review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install dbt
        run: pip install dbt-core dbt-bigquery
      
      - name: Generate artifacts
        run: |
          dbt deps
          dbt compile
          
      - name: Install PR Review Agent
        run: |
          curl -L https://github.com/your-org/dbt-pr-agent/releases/latest/download/dbt-pr-agent-linux-amd64 -o dbt-pr-agent
          chmod +x dbt-pr-agent
          
      - name: Run PR Review
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          ./dbt-pr-agent analyze-pr \
            --repo ${{ github.repository }} \
            --pr-number ${{ github.event.pull_request.number }} \
            --token ${{ secrets.GITHUB_TOKEN }} \
            --output markdown > review.md
            
      - name: Comment PR
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const review = fs.readFileSync('review.md', 'utf8');
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: review
            });
```

## Troubleshooting

### Common Issues

1. **"Failed to load manifest.json"**
   - Run `dbt compile` first
   - Check `target_path` in configuration

2. **"LLM provider error"**
   - Verify API keys are set
   - Check network connectivity
   - Ensure model name is correct

3. **"No changed models detected"**
   - Verify file paths match manifest
   - Check `original_file_path` in manifest.json

4. **Performance issues**
   - Reduce number of files analyzed
   - Use local LLM (Ollama) for testing
   - Enable incremental analysis mode