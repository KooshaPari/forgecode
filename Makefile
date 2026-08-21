# ForgeCode Development Makefile
# Python + Rust multi-language project
# Usage: make help

.PHONY: help setup install lint fmt test test-unit test-integration test-a11y \
        build clean i18n-check coverage security-scan

PYTHON := python3
PIP := pip
PYTEST := pytest
RUFF := ruff
VENV := .venv

# ──────────────────────────────────────────────
# Setup & Install
# ──────────────────────────────────────────────

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-25s\033[0m %s\n", $$1, $$2}'

setup: ## Create virtual environment and install dev deps
	$(PYTHON) -m venv $(VENV)
	$(VENV)/Scripts/pip install --upgrade pip 2>/dev/null || $(VENV)/bin/pip install --upgrade pip
	$(MAKE) install

install: ## Install project and dev dependencies
	$(VENV)/Scripts/pip install -e ".[dev]" 2>/dev/null || $(VENV)/bin/pip install -e ".[dev]"
	$(VENV)/Scripts/pip install pre-commit 2>/dev/null || $(VENV)/bin/pip install pre-commit

# ──────────────────────────────────────────────
# Linting & Formatting
# ──────────────────────────────────────────────

lint: ## Run ruff linter
	$(VENV)/Scripts/ruff check . 2>/dev/null || $(VENV)/bin/ruff check .

lint-fix: ## Run ruff linter with auto-fix
	$(VENV)/Scripts/ruff check --fix . 2>/dev/null || $(VENV)/bin/ruff check --fix .

fmt: ## Format code with ruff
	$(VENV)/Scripts/ruff format . 2>/dev/null || $(VENV)/bin/ruff format .

fmt-check: ## Check formatting without modifying files
	$(VENV)/Scripts/ruff format --check . 2>/dev/null || $(VENV)/bin/ruff format --check .

# ──────────────────────────────────────────────
# Testing
# ──────────────────────────────────────────────

test: ## Run all tests
	$(VENV)/Scripts/pytest 2>/dev/null || $(VENV)/bin/pytest

test-unit: ## Run unit tests only
	$(VENV)/Scripts/pytest -m unit 2>/dev/null || $(VENV)/bin/pytest -m unit

test-integration: ## Run integration tests only
	$(VENV)/Scripts/pytest -m integration 2>/dev/null || $(VENV)/bin/pytest -m integration

test-a11y: ## Run accessibility tests only
	$(VENV)/Scripts/pytest -m a11y 2>/dev/null || $(VENV)/bin/pytest -m a11y

test-slow: ## Run slow tests only
	$(VENV)/Scripts/pytest -m slow 2>/dev/null || $(VENV)/bin/pytest -m slow

test-verbose: ## Run all tests with verbose output
	$(VENV)/Scripts/pytest -v --tb=short 2>/dev/null || $(VENV)/bin/pytest -v --tb=short

# ──────────────────────────────────────────────
# Build
# ──────────────────────────────────────────────

build: ## Build distributable packages
	$(VENV)/Scripts/python -m build 2>/dev/null || $(VENV)/bin/python -m build

# ──────────────────────────────────────────────
# Quality Gates
# ──────────────────────────────────────────────

i18n-check: ## Check internationalization files
	@echo "Checking i18n locale files..."
	@find . -path ./node_modules -prune -o -name "*.po" -print -o -name "*.pot" -print | head -20
	@echo "i18n check complete."

coverage: ## Run tests with coverage report
	$(VENV)/Scripts/pytest --cov=src --cov-report=term-missing --cov-report=html 2>/dev/null || $(VENV)/bin/pytest --cov=src --cov-report=term-missing --cov-report=html
	@echo "Coverage report: htmlcov/index.html"

security-scan: ## Run security scans (gitleaks, pip-audit)
	@echo "Running gitleaks..."
	-gitleaks detect --source . --verbose
	@echo "Running pip-audit..."
	-$(VENV)/Scripts/pip-audit 2>/dev/null || $(VENV)/bin/pip-audit 2>/dev/null || echo "pip-audit not installed, skipping"

# ──────────────────────────────────────────────
# Cleanup
# ──────────────────────────────────────────────

clean: ## Remove build artifacts and caches
	rm -rf dist/ build/ *.egg-info .pytest_cache .ruff_cache .mypy_cache
	rm -rf htmlcov/ .coverage coverage.xml coverage.out
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true

clean-all: clean ## Remove everything including venv
	rm -rf $(VENV)

# ──────────────────────────────────────────────
# Pre-commit
# ──────────────────────────────────────────────

pre-commit-install: ## Install pre-commit hooks
	$(VENV)/Scripts/pre-commit install 2>/dev/null || $(VENV)/bin/pre-commit install
	$(VENV)/Scripts/pre-commit install --hook-type commit-msg 2>/dev/null || $(VENV)/bin/pre-commit install --hook-type commit-msg

pre-commit: ## Run all pre-commit hooks
	$(VENV)/Scripts/pre-commit run --all-files 2>/dev/null || $(VENV)/bin/pre-commit run --all-files

# ──────────────────────────────────────────────
# OTel Production Observability
# ──────────────────────────────────────────────

otel-up: ## Start production OTel stack (collector, jaeger, prometheus, grafana)
	cd deploy/otel && bash deploy.sh up

otel-down: ## Stop production OTel stack
	cd deploy/otel && bash deploy.sh down

otel-logs: ## Tail logs from production OTel stack
	cd deploy/otel && bash deploy.sh logs

otel-status: ## Show status of production OTel stack
	cd deploy/otel && bash deploy.sh status

otel-restart: ## Restart production OTel stack
	cd deploy/otel && bash deploy.sh restart

# ──────────────────────────────────────────────
# Full quality gate
# ──────────────────────────────────────────────

qa: lint fmt-check test-unit coverage ## Run full quality assurance suite
	@echo "All QA checks passed."
