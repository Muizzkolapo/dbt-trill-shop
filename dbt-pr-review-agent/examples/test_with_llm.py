#!/usr/bin/env python3
"""
Test script for dbt PR Review Agent with LLM integration
This demonstrates how to test different scenarios with AI-powered analysis
"""

import subprocess
import json
import os
import sys
from pathlib import Path

# ANSI color codes
GREEN = '\033[0;32m'
YELLOW = '\033[1;33m'
RED = '\033[0;31m'
BLUE = '\033[0;34m'
NC = '\033[0m'  # No Color

def run_command(cmd, capture_output=True):
    """Run a shell command and return the result"""
    print(f"{BLUE}Running: {' '.join(cmd)}{NC}")
    result = subprocess.run(cmd, capture_output=capture_output, text=True)
    return result

def test_scenario(name, description, files, expected_risk="Medium"):
    """Test a specific scenario"""
    print(f"\n{GREEN}{'='*60}{NC}")
    print(f"{YELLOW}Scenario: {name}{NC}")
    print(f"Description: {description}")
    print(f"Files: {', '.join(files)}")
    print(f"Expected Risk: {expected_risk}")
    print(f"{GREEN}{'='*60}{NC}\n")
    
    # Run the analysis
    cmd = [
        "./target/release/dbt-pr-agent",
        "analyze-local",
        "--files"
    ] + files + [
        "--output", "json",
        "--author", "test-bot",
        "--title", f"Test: {name}"
    ]
    
    result = run_command(cmd)
    
    if result.returncode != 0:
        print(f"{RED}Error running analysis:{NC}")
        print(result.stderr)
        return False
    
    try:
        # Parse JSON output
        analysis = json.loads(result.stdout)
        
        # Extract key information
        risk_level = analysis.get("overall_risk_level", "Unknown")
        impact_score = analysis.get("impact_report", {}).get("impact_score", 0)
        quality_score = analysis.get("quality_report", {}).get("overall_score", 0)
        recommendations = analysis.get("recommendations", [])
        
        print(f"Risk Level: {risk_level}")
        print(f"Impact Score: {impact_score:.2f}")
        print(f"Quality Score: {quality_score:.1f}%")
        print(f"Recommendations: {len(recommendations)}")
        
        if recommendations:
            print(f"\n{YELLOW}Top Recommendations:{NC}")
            for i, rec in enumerate(recommendations[:3], 1):
                print(f"{i}. {rec}")
        
        # Check if risk level matches expected
        if risk_level == expected_risk:
            print(f"\n{GREEN}✓ Risk assessment matches expected: {risk_level}{NC}")
            return True
        else:
            print(f"\n{RED}✗ Risk mismatch - Expected: {expected_risk}, Got: {risk_level}{NC}")
            return False
            
    except json.JSONDecodeError as e:
        print(f"{RED}Error parsing JSON output: {e}{NC}")
        print("Raw output:", result.stdout[:500])
        return False

def create_test_sql_files():
    """Create test SQL files for different scenarios"""
    test_dir = Path("test_models")
    test_dir.mkdir(exist_ok=True)
    
    # Scenario 1: Breaking change
    (test_dir / "breaking_change.sql").write_text("""
-- This model changes a critical column name
SELECT 
    id as customer_id,  -- BREAKING: was 'id'
    email as contact_email,  -- BREAKING: was 'email'
    created_at
FROM {{ ref('stg_customers') }}
""")
    
    # Scenario 2: Performance issue
    (test_dir / "performance_issue.sql").write_text("""
{{ config(
    materialized='table'  -- ISSUE: Was incremental, now full refresh
) }}

SELECT 
    e.*,
    c.*
FROM {{ ref('fct_events') }} e
CROSS JOIN {{ ref('dim_customers') }} c  -- ISSUE: Cartesian join!
WHERE e.event_date >= '2020-01-01'
""")
    
    # Scenario 3: Missing documentation
    (test_dir / "undocumented_model.sql").write_text("""
-- No documentation, no tests
SELECT 
    user_id,
    sum(amount) as total_spent,
    count(*) as order_count
FROM {{ ref('fct_orders') }}
GROUP BY user_id
""")
    
    # Scenario 4: Good practice model
    (test_dir / "well_structured.sql").write_text("""
{{
    config(
        materialized='incremental',
        unique_key='order_id',
        on_schema_change='fail'
    )
}}

-- Well documented model with proper incremental logic
SELECT 
    order_id,
    customer_id,
    order_date,
    amount,
    _loaded_at
FROM {{ ref('stg_orders') }}

{% if is_incremental() %}
WHERE _loaded_at > (SELECT MAX(_loaded_at) FROM {{ this }})
{% endif %}
""")
    
    return test_dir

def main():
    """Main test runner"""
    print(f"{GREEN}=== dbt PR Review Agent - LLM Integration Test ==={NC}\n")
    
    # Check for LLM API keys
    has_openai = bool(os.environ.get("OPENAI_API_KEY"))
    has_anthropic = bool(os.environ.get("ANTHROPIC_API_KEY"))
    
    if not has_openai and not has_anthropic:
        print(f"{YELLOW}Warning: No LLM API keys found!{NC}")
        print("Set OPENAI_API_KEY or ANTHROPIC_API_KEY to enable AI analysis")
        print("The agent will run in deterministic mode only.\n")
    else:
        print(f"{GREEN}✓ LLM API key found!{NC}")
        print(f"Provider: {'OpenAI' if has_openai else 'Anthropic'}\n")
    
    # Check if binary exists
    if not Path("./target/release/dbt-pr-agent").exists():
        print(f"{RED}Error: dbt-pr-agent binary not found!{NC}")
        print("Please run: cargo build --release")
        sys.exit(1)
    
    # Create test files
    print(f"{YELLOW}Creating test SQL files...{NC}")
    test_dir = create_test_sql_files()
    
    # Run test scenarios
    scenarios = [
        {
            "name": "Breaking Schema Change",
            "description": "Model changes column names that downstream models depend on",
            "files": [str(test_dir / "breaking_change.sql")],
            "expected_risk": "High"
        },
        {
            "name": "Performance Regression",
            "description": "Model introduces cartesian join and removes incremental logic",
            "files": [str(test_dir / "performance_issue.sql")],
            "expected_risk": "Critical"
        },
        {
            "name": "Quality Issues",
            "description": "Model lacks documentation and tests",
            "files": [str(test_dir / "undocumented_model.sql")],
            "expected_risk": "Medium"
        },
        {
            "name": "Well Structured Model",
            "description": "Model follows best practices",
            "files": [str(test_dir / "well_structured.sql")],
            "expected_risk": "Low"
        },
        {
            "name": "Multiple Changes",
            "description": "Analyzing multiple files at once",
            "files": [
                str(test_dir / "breaking_change.sql"),
                str(test_dir / "performance_issue.sql")
            ],
            "expected_risk": "Critical"
        }
    ]
    
    # Run scenarios
    results = []
    for scenario in scenarios:
        success = test_scenario(**scenario)
        results.append((scenario["name"], success))
    
    # Summary
    print(f"\n{GREEN}{'='*60}{NC}")
    print(f"{YELLOW}Test Summary{NC}")
    print(f"{GREEN}{'='*60}{NC}\n")
    
    passed = sum(1 for _, success in results if success)
    total = len(results)
    
    for name, success in results:
        status = f"{GREEN}✓ PASS{NC}" if success else f"{RED}✗ FAIL{NC}"
        print(f"{status} - {name}")
    
    print(f"\n{YELLOW}Total: {passed}/{total} passed{NC}")
    
    # Cleanup
    print(f"\n{YELLOW}Cleaning up test files...{NC}")
    for file in test_dir.glob("*.sql"):
        file.unlink()
    test_dir.rmdir()
    
    # Exit with appropriate code
    sys.exit(0 if passed == total else 1)

if __name__ == "__main__":
    main()