# CTL Sync Python Makefile

.PHONY: help install test demo clean lint format check dev-install

help: ## Show this help message
	@echo "CTL Sync Python - Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'

install: ## Install package and dependencies
	pip install -e .

dev-install: ## Install package in development mode with test dependencies
	pip install -e ".[dev]"
	pip install pytest pytest-cov

test: ## Run unit tests
	python -m pytest tests/ -v

test-cov: ## Run unit tests with coverage
	python -m pytest tests/ --cov=ctl_sync --cov-report=term-missing

demo: ## Run demonstration script
	python test_demo.py

lint: ## Run linting checks
	python -m flake8 ctl_sync/ tests/

format: ## Format code with black
	python -m black ctl_sync/ tests/

check: lint test ## Run all checks (lint + tests)

clean: ## Clean build artifacts
	rm -rf build/
	rm -rf dist/
	rm -rf *.egg-info/
	find . -type d -name __pycache__ -exec rm -rf {} +
	find . -type f -name "*.pyc" -delete
	rm -f cache.json

build: ## Build distribution packages
	python -m build

# Quick sync commands (when installed)
sync: ## Run one-time sync (requires installation)
	ctl-sync once

watch: ## Run in watch mode (requires installation)  
	ctl-sync watch

stats: ## Show sync statistics (requires installation)
	ctl-sync stats

# Development workflow
dev: dev-install test demo ## Full development setup and test

# CI/CD targets
ci-test: ## Run tests for CI
	python -m pytest tests/ --cov=ctl_sync --cov-report=xml

ci-lint: ## Run linting for CI
	python -m flake8 ctl_sync/ tests/ --max-line-length=100