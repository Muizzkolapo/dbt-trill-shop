#!/bin/bash

# Test script for local dbt PR review analysis
# This script demonstrates how to test the agent without GitHub

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== dbt PR Review Agent - Local Test ===${NC}"

# Check if binary exists
if [ ! -f "./target/release/dbt-pr-agent" ]; then
    echo -e "${RED}Error: dbt-pr-agent binary not found!${NC}"
    echo "Please run: cargo build --release"
    exit 1
fi

# Set project path (use current directory or provide as argument)
PROJECT_PATH="${1:-.}"
echo -e "${YELLOW}Using project path: $PROJECT_PATH${NC}"

# Check for required dbt artifacts
if [ ! -f "$PROJECT_PATH/target/manifest.json" ]; then
    echo -e "${RED}Error: manifest.json not found!${NC}"
    echo "Please run: dbt compile"
    exit 1
fi

# Create test output directory
mkdir -p test_output

# Test 1: Validate Project
echo -e "\n${GREEN}Test 1: Validating dbt project structure...${NC}"
./target/release/dbt-pr-agent validate-project \
    --project-path "$PROJECT_PATH" \
    > test_output/validation.txt

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Project validation passed${NC}"
else
    echo -e "${RED}✗ Project validation failed${NC}"
    cat test_output/validation.txt
    exit 1
fi

# Test 2: Health Check
echo -e "\n${GREEN}Test 2: Running health check...${NC}"
./target/release/dbt-pr-agent health-check \
    --project-path "$PROJECT_PATH" \
    > test_output/health_check.txt

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Health check passed${NC}"
else
    echo -e "${RED}✗ Health check failed${NC}"
    cat test_output/health_check.txt
fi

# Test 3: Analyze Local Changes (find some SQL files to analyze)
echo -e "\n${GREEN}Test 3: Analyzing local changes...${NC}"

# Find up to 3 SQL model files to analyze
SQL_FILES=$(find "$PROJECT_PATH/models" -name "*.sql" -type f | head -3 | tr '\n' ' ')

if [ -z "$SQL_FILES" ]; then
    echo -e "${YELLOW}No SQL files found in models/ directory${NC}"
else
    echo -e "${YELLOW}Analyzing files: $SQL_FILES${NC}"
    
    # Test JSON output
    echo -e "\n${GREEN}Testing JSON output...${NC}"
    ./target/release/dbt-pr-agent analyze-local \
        --project-path "$PROJECT_PATH" \
        --files $SQL_FILES \
        --output json \
        > test_output/analysis.json
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ JSON analysis completed${NC}"
        echo "Output saved to: test_output/analysis.json"
        
        # Pretty print first few lines
        if command -v jq &> /dev/null; then
            echo -e "\n${YELLOW}Preview:${NC}"
            head -20 test_output/analysis.json | jq '.'
        fi
    fi
    
    # Test Markdown output
    echo -e "\n${GREEN}Testing Markdown output...${NC}"
    ./target/release/dbt-pr-agent analyze-local \
        --project-path "$PROJECT_PATH" \
        --files $SQL_FILES \
        --output markdown \
        > test_output/analysis.md
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Markdown analysis completed${NC}"
        echo "Output saved to: test_output/analysis.md"
        
        # Show first few lines
        echo -e "\n${YELLOW}Preview:${NC}"
        head -20 test_output/analysis.md
    fi
    
    # Test Text output
    echo -e "\n${GREEN}Testing Text output...${NC}"
    ./target/release/dbt-pr-agent analyze-local \
        --project-path "$PROJECT_PATH" \
        --files $SQL_FILES \
        --output text \
        > test_output/analysis.txt
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Text analysis completed${NC}"
        echo "Output saved to: test_output/analysis.txt"
    fi
fi

# Test 4: Test with mock PR metadata
echo -e "\n${GREEN}Test 4: Testing with PR metadata...${NC}"
if [ -n "$SQL_FILES" ]; then
    ./target/release/dbt-pr-agent analyze-local \
        --project-path "$PROJECT_PATH" \
        --files $SQL_FILES \
        --author "test-engineer" \
        --title "Test PR: Update customer models" \
        --base-branch "main" \
        --head-branch "feature/update-customers" \
        --output markdown \
        > test_output/pr_analysis.md
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ PR analysis completed${NC}"
        echo "Output saved to: test_output/pr_analysis.md"
    fi
fi

# Summary
echo -e "\n${GREEN}=== Test Summary ===${NC}"
echo -e "${YELLOW}Test outputs saved in: ./test_output/${NC}"
echo ""
echo "Files generated:"
ls -la test_output/

echo -e "\n${GREEN}To test with LLM integration:${NC}"
echo "1. Set your API key: export OPENAI_API_KEY='your-key'"
echo "2. Re-run this script"

# Check if LLM is configured
if [ -n "$OPENAI_API_KEY" ] || [ -n "$ANTHROPIC_API_KEY" ]; then
    echo -e "\n${GREEN}LLM API key detected! The analysis will include AI-powered insights.${NC}"
else
    echo -e "\n${YELLOW}No LLM API key detected. Running in deterministic mode only.${NC}"
fi