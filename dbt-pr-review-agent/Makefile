# Makefile for dbt PR Review Agent

# Variables
CARGO = cargo
BINARY_NAME = dbt-pr-agent
TARGET_DIR = target/release
BINARY = $(TARGET_DIR)/$(BINARY_NAME)

# Default target
.PHONY: all
all: build

# Build targets
.PHONY: build
build:
	@echo "Building release binary..."
	$(CARGO) build --release

.PHONY: build-debug
build-debug:
	@echo "Building debug binary..."
	$(CARGO) build

# Test targets
.PHONY: test
test:
	@echo "Running unit tests..."
	$(CARGO) test

.PHONY: test-verbose
test-verbose:
	@echo "Running tests with output..."
	$(CARGO) test -- --nocapture

.PHONY: test-integration
test-integration: build
	@echo "Running integration tests..."
	@if [ -f "$(BINARY)" ]; then \
		./examples/test_local.sh ..; \
	else \
		echo "Error: Binary not found. Run 'make build' first."; \
		exit 1; \
	fi

.PHONY: test-llm
test-llm: build
	@echo "Running LLM integration tests..."
	@if [ -f "$(BINARY)" ]; then \
		python3 examples/test_with_llm.py; \
	else \
		echo "Error: Binary not found. Run 'make build' first."; \
		exit 1; \
	fi

# Quick test commands
.PHONY: validate
validate: build
	@echo "Validating dbt project..."
	$(BINARY) validate-project --project-path ..

.PHONY: health
health: build
	@echo "Running health check..."
	$(BINARY) health-check --project-path ..

.PHONY: analyze
analyze: build
	@echo "Analyzing sample models..."
	@FILES=$$(find ../models -name "*.sql" -type f | head -3 | tr '\n' ' '); \
	if [ -n "$$FILES" ]; then \
		$(BINARY) analyze-local --project-path .. --files $$FILES --output markdown; \
	else \
		echo "No SQL files found in ../models/"; \
	fi

# Development targets
.PHONY: check
check:
	@echo "Running cargo check..."
	$(CARGO) check

.PHONY: clippy
clippy:
	@echo "Running clippy..."
	$(CARGO) clippy -- -D warnings

.PHONY: fmt
fmt:
	@echo "Formatting code..."
	$(CARGO) fmt

.PHONY: fmt-check
fmt-check:
	@echo "Checking code formatting..."
	$(CARGO) fmt -- --check

# Documentation
.PHONY: doc
doc:
	@echo "Building documentation..."
	$(CARGO) doc --no-deps --open

# Clean targets
.PHONY: clean
clean:
	@echo "Cleaning build artifacts..."
	$(CARGO) clean
	@rm -rf test_output/

# Installation
.PHONY: install
install: build
	@echo "Installing binary to /usr/local/bin..."
	@sudo cp $(BINARY) /usr/local/bin/

.PHONY: uninstall
uninstall:
	@echo "Removing binary from /usr/local/bin..."
	@sudo rm -f /usr/local/bin/$(BINARY_NAME)

# Release builds
.PHONY: release-linux
release-linux:
	@echo "Building for Linux..."
	cargo build --release --target x86_64-unknown-linux-gnu

.PHONY: release-macos
release-macos:
	@echo "Building for macOS..."
	cargo build --release --target x86_64-apple-darwin
	cargo build --release --target aarch64-apple-darwin

# Help target
.PHONY: help
help:
	@echo "dbt PR Review Agent - Makefile targets:"
	@echo ""
	@echo "Build targets:"
	@echo "  make build          - Build release binary"
	@echo "  make build-debug    - Build debug binary"
	@echo ""
	@echo "Test targets:"
	@echo "  make test           - Run unit tests"
	@echo "  make test-verbose   - Run tests with output"
	@echo "  make test-integration - Run integration tests"
	@echo "  make test-llm       - Run LLM integration tests"
	@echo ""
	@echo "Quick commands:"
	@echo "  make validate       - Validate dbt project"
	@echo "  make health         - Run health check"
	@echo "  make analyze        - Analyze sample models"
	@echo ""
	@echo "Development:"
	@echo "  make check          - Run cargo check"
	@echo "  make clippy         - Run clippy linter"
	@echo "  make fmt            - Format code"
	@echo "  make doc            - Build and open docs"
	@echo ""
	@echo "Other:"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make install        - Install binary to system"
	@echo "  make help           - Show this help"

# Set default goal
.DEFAULT_GOAL := help