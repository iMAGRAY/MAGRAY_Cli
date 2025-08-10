# MAGRAY CLI Build System
# Supports multiple feature configurations

# Minimum coverage threshold for coverage-based CI runs (percentage)
MIN_COVERAGE ?= 40

.PHONY: help build-all build-cpu build-gpu build-minimal test test-all bench clean docker-build docker-test release rag-report rag-report-fast rag-report-rerank

# Default target
help:
	@echo "🚀 MAGRAY CLI Build System"
	@echo ""
	@echo "📦 Build Commands:"
	@echo "  make build-cpu      - Build CPU-only version (production)"
	@echo "  make build-gpu      - Build GPU-enabled version (workstation)"
	@echo "  make build-minimal  - Build minimal version (containers)"
	@echo "  make build-all      - Build all feature combinations"
	@echo ""
	@echo "🧪 Test Commands:"
	@echo "  make test          - Run tests for default features"
	@echo "  make test-all      - Run tests for all feature combinations"
	@echo "  make test-fast     - Run fast CPU tests"
	@echo "  make test-full     - Run CPU tests with extended-tests"
	@echo "  make test-persistence - Run CPU+persistence+extended-tests"
	@echo "  make test-gpu-full - Run GPU tests with extended-tests"
	@echo ""
	@echo "🐳 Docker Commands:"
	@echo "  make docker-build  - Build all Docker images"
	@echo "  make docker-test   - Test Docker containers"
	@echo ""
	@echo "📊 Analysis Commands:"
	@echo "  make size-analysis - Compare binary sizes"
	@echo "  make perf-test     - Quick performance test"
	@echo "  make coverage      - Coverage for CPU core"
	@echo "  make coverage-full - Coverage for extended tests"
	@echo "  make ci-local-extended-cov - Extended tests with coverage gate ($(MIN_COVERAGE)%)"
	@echo "  make ci-local-cov-core    - Core coverage gate for 'common' crate ($(MIN_COVERAGE)%)"
	@echo ""
	@echo "🔧 Utility Commands:"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make release       - Create release binaries"
	@echo "  make rag-report      - Run RAG golden suite and save metrics"
	@echo "  make rag-report-fast - Run only RAG test (fast path)"
	@echo "  make rag-report-rerank - Run RAG golden suite and save rerank metrics"

# Build commands
build-cpu:
	@echo "🔨 Building CPU-only version..."
	cargo build --release --features=cpu
	@echo "✅ CPU build complete: $(shell du -h target/release/magray* | cut -f1)"

build-gpu:
	@echo "🎮 Building GPU-enabled version..."
	cargo build --release --features=gpu
	@echo "✅ GPU build complete: $(shell du -h target/release/magray* | cut -f1)"

build-minimal:
	@echo "📦 Building minimal version..."
	cargo build --release --features=minimal
	@echo "✅ Minimal build complete: $(shell du -h target/release/magray* | cut -f1)"

build-all: build-cpu build-gpu build-minimal
	@echo "🎯 All builds complete!"

# Test commands
test:
	@echo "🧪 Running default tests..."
	cargo test

test-cpu:
	@echo "🧪 Testing CPU features..."
	cargo test --features=cpu

test-gpu:
	@echo "🧪 Testing GPU features..."
	cargo test --features=gpu

test-minimal:
	@echo "🧪 Testing minimal features..."
	cargo test --features=minimal

# New test matrix targets
test-fast:
	@echo "⚡ Running fast CPU tests..."
	cargo test --features=cpu --no-fail-fast

test-full:
	@echo "🧪 Running full CPU tests (extended)..."
	cargo test --features="cpu,extended-tests" --no-fail-fast

test-persistence:
	@echo "🧪 Running persistence tests (CPU + persistence + extended)..."
	cargo test --features="cpu,persistence,extended-tests" --no-fail-fast

test-gpu-full:
	@echo "🎮 Running full GPU tests (extended)..."
	cargo test --features="gpu,extended-tests" --no-fail-fast

test-all: test-cpu test-gpu test-minimal
	@echo "✅ All feature tests passed!"

# Benchmark commands
bench:
	@echo "⚡ Running performance benchmarks..."
	cargo bench --features=cpu

bench-gpu:
	@echo "⚡ Running GPU benchmarks..."
	cargo bench --features=gpu

# Docker commands
docker-build:
	@echo "🐳 Building Docker images..."
	docker-compose -f docker/docker-compose.yml --profile all build
	@echo "✅ Docker images built"

docker-test:
	@echo "🧪 Testing Docker containers..."
	docker-compose -f docker/docker-compose.yml --profile benchmark up --abort-on-container-exit
	@echo "✅ Docker tests complete"

# Analysis commands
size-analysis: build-all
	@echo "📊 Binary Size Analysis:"
	@echo "Feature     | Size    | Status"
	@echo "------------|---------|--------"
	@if [ -f target/release/magray ]; then \
		SIZE=$$(du -h target/release/magray | cut -f1); \
		echo "Latest      | $$SIZE   | ✅ Built"; \
	fi
	@echo ""
	@echo "💡 Recommendations:"
	@echo "  - CPU: Best for production servers"
	@echo "  - GPU: Best for workstations with CUDA"
	@echo "  - Minimal: Best for containers/edge"

perf-test: build-cpu
	@echo "⚡ Quick Performance Test:"
	@echo "Startup time (5 runs):"
	@for i in {1..5}; do \
		/usr/bin/time -f "Run $$i: %E" ./target/release/magray --version >/dev/null; \
	done
	@echo ""
	@echo "Status command:"
	@/usr/bin/time -f "Status: %E" ./target/release/magray status >/dev/null

# Utility commands
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	docker system prune -f --filter "label=org.opencontainers.image.title=MAGRAY*" 2>/dev/null || true
	@echo "✅ Clean complete"

# Release preparation
release: clean
	@echo "📦 Creating release binaries..."
	@mkdir -p dist
	
	@echo "Building CPU release..."
	cargo build --release --features=cpu
	@cp target/release/magray* dist/magray-cpu 2>/dev/null || cp target/release/magray.exe dist/magray-cpu.exe 2>/dev/null || true
	
	@echo "Building GPU release..."
	cargo build --release --features=gpu  
	@cp target/release/magray* dist/magray-gpu 2>/dev/null || cp target/release/magray.exe dist/magray-gpu.exe 2>/dev/null || true
	
	@echo "Building minimal release..."
	cargo build --release --features=minimal
	@cp target/release/magray* dist/magray-minimal 2>/dev/null || cp target/release/magray.exe dist/magray-minimal.exe 2>/dev/null || true
	
	@echo "✅ Release binaries created in dist/"
	@ls -lah dist/

# Development helpers
dev-cpu:
	@echo "🔧 Development build (CPU)..."
	cargo build --features=cpu

dev-gpu:
	@echo "🔧 Development build (GPU)..."
	cargo build --features=gpu

watch:
	@echo "👀 Watching for changes..."
	cargo watch -x "build --features=cpu"

# Linting and formatting
lint:
	@echo "📋 Running linter..."
	cargo clippy --all-features -- -D warnings

fmt:
	@echo "✨ Formatting code..."
	cargo fmt

check: lint fmt test
	@echo "✅ All checks passed!"

# Feature verification
verify-features:
	@echo "🔍 Verifying feature combinations..."
	@echo "Testing CPU features..."
	@cargo check --features=cpu --quiet && echo "✅ CPU features OK" || echo "❌ CPU features FAIL"
	@echo "Testing GPU features..."
	@cargo check --features=gpu --quiet && echo "✅ GPU features OK" || echo "❌ GPU features FAIL"
	@echo "Testing minimal features..."
	@cargo check --features=minimal --quiet && echo "✅ Minimal features OK" || echo "❌ Minimal features FAIL"

# Information
info:
	@echo "📋 Project Information:"
	@echo "  Name: MAGRAY CLI"
	@echo "  Version: $(shell cargo pkgid | cut -d'#' -f2)"
	@echo "  Features: cpu, gpu, minimal"
	@echo "  Workspace crates: $(shell ls crates/)"
	@echo ""
	@echo "🏗️ Build Targets:"
	@echo "  - x86_64-unknown-linux-gnu (Linux)"
	@echo "  - x86_64-pc-windows-msvc (Windows)"
	@echo "  - x86_64-apple-darwin (macOS)"
	@echo "  - x86_64-unknown-linux-musl (Alpine)"

coverage:
	@echo "📈 Running test coverage (tarpaulin) ..."
	@which cargo-tarpaulin >/dev/null 2>&1 || cargo install cargo-tarpaulin
	cargo tarpaulin --config Tarpaulin.toml --engine llvm --features=cpu --timeout 120 --out Html
	@echo "✅ Coverage report: tarpaulin-report.html"

coverage-full:
	@echo "📈 Running full coverage (extended-tests) ..."
	@which cargo-tarpaulin >/dev/null 2>&1 || cargo install cargo-tarpaulin
	cargo tarpaulin --config Tarpaulin.toml --engine llvm --features="cpu,extended-tests" --timeout 600 --out Html
	@echo "✅ Coverage report: tarpaulin-report.html"

ci-local-fast:
	@echo "🏃 CI-local: fast cpu tests"
	cargo test --features=cpu --no-fail-fast --quiet

ci-local-extended:
	@echo "🏃 CI-local: extended cpu tests"
	cargo test --features="cpu,extended-tests" --no-fail-fast --quiet

# Coverage-enforced extended suite (fails if coverage below MIN_COVERAGE)
ci-local-extended-cov:
	@echo "🏃 CI-local: extended cpu tests with coverage threshold >= $(MIN_COVERAGE)%"
	@which cargo-tarpaulin >/dev/null 2>&1 || cargo install cargo-tarpaulin
	cargo tarpaulin --engine llvm --workspace -p common -p tools -p ai -p memory -p cli --features="cpu,extended-tests" --timeout 600 \
		--include-files crates/common/src/** \
		--include-files crates/tools/src/file_ops.rs \
		--include-files crates/tools/src/git_ops.rs \
		--include-files crates/tools/src/web_ops.rs \
		--include-files crates/tools/src/shell_ops.rs \
		--include-files crates/tools/src/registry/** \
		--include-files crates/tools/src/execution/resource_manager.rs \
		--include-files crates/tools/src/execution/security_enforcer.rs \
		--include-files crates/tools/src/execution/pipeline.rs \
		--include-files crates/ai/src/config.rs \
		--include-files crates/ai/src/reranker_qwen3.rs \
		--include-files crates/ai/src/embeddings_cpu.rs \
		--include-files crates/ai/src/memory_pool.rs \
		--include-files crates/ai/src/errors.rs \
		--include-files crates/ai/src/auto_device_selector.rs \
		--include-files crates/ai/src/model_registry.rs \
		--include-files crates/cli/src/commands/gpu.rs \
		--include-files crates/cli/src/commands/models.rs \
		--include-files crates/cli/src/commands/tools.rs \
		--include-files crates/cli/src/commands/memory.rs \
		--include-files crates/cli/src/progress.rs \
		--include-files crates/memory/src/api.rs \
		--include-files crates/memory/src/fallback.rs \
		--include-files crates/memory/src/metrics.rs \
		--include-files crates/memory/src/di/container_metrics_impl.rs \
		--include-files crates/memory/src/di/dependency_graph_validator.rs \
		--fail-under $(MIN_COVERAGE) --out Html
	@echo "✅ Coverage (>= $(MIN_COVERAGE)%) OK: tarpaulin-report.html"

# Core coverage gate for common crate only (fast signal)
ci-local-cov-core:
	@echo "🏃 CI-local: core coverage (crate=common) threshold >= $(MIN_COVERAGE)%"
	@which cargo-tarpaulin >/dev/null 2>&1 || cargo install cargo-tarpaulin
	cargo tarpaulin -p common --config Tarpaulin.toml --engine llvm --timeout 300 --fail-under $(MIN_COVERAGE) --out Html
	@echo "✅ Core coverage (>= $(MIN_COVERAGE)%) OK: tarpaulin-report.html"

ci-local-persistence:
	@echo "🏃 CI-local: persistence suite"
	cargo test --features="cpu,persistence,extended-tests" --no-fail-fast --quiet

ci-local-gpu:
	@echo "🏃 CI-local: gpu extended (if available)"
	cargo test --features="gpu,extended-tests" --no-fail-fast --quiet || true

rag-report:
	@echo "🧪 Running RAG golden suite (extended-tests) ..."
	cargo test --features="cpu,extended-tests" --tests -q -- test rag_golden_suite_metrics --exact --nocapture
	@echo "📄 Report: reports/rag_metrics_summary.json"

rag-report-fast:
	@echo "🧪 Running RAG golden suite test only (extended-tests) ..."
	cargo test --features="cpu,extended-tests" -q -- tests::rag_golden_suite_metrics --exact --nocapture || true
	@echo "📄 Report (if generated): reports/rag_metrics_summary.json"

rag-report-rerank:
	@echo "🧪 Running RAG golden suite with rerank variant (extended-tests) ..."
	cargo test --features="cpu,extended-tests" --tests -q -- test rag_golden_suite_metrics --exact --nocapture
	@echo "📄 Baseline: reports/rag_metrics_summary.json"
	@echo "📄 Rerank:   reports/rag_metrics_rerank.json"

orchestrated-check:
	@echo "🔎 Checking orchestrated build..."
	cargo check --features="cpu,extended-tests,orchestrated-search"

orchestrated-tests:
	@echo "🧪 Running orchestrated CLI tests (extended)..."
	cargo test --features="cpu,extended-tests,orchestrated-search" -q --test test_memory_orchestrated_cli -- --nocapture | cat

ci-local:
	@echo "🧪 Running fast CI (cpu + extended tests, without legacy) with 12m timeout..."
	CI=1 MAGRAY_NO_ANIM=1 MAGRAY_SKIP_AUTO_INSTALL=1 MAGRAY_FORCE_NO_ORT=1 timeout 720s \
	cargo test -q --features="cpu,extended-tests" --tests -- --nocapture | cat

ci-local-all:
	@echo "🧪 Running full CI matrix locally with 20m timeout (non-interactive)..."
	CI=1 MAGRAY_NO_ANIM=1 MAGRAY_SKIP_AUTO_INSTALL=1 MAGRAY_FORCE_NO_ORT=1 timeout 1200s \
	cargo test -q --features="cpu,extended-tests,orchestrated-search,keyword-search,hnsw-index" --tests -- --nocapture | cat

# MAGRAY CLI - Container workflows
# Короткие цели для сборки/запуска контейнеров и smoke-тестов

ENGINE ?= docker
COMPOSE ?= $(ENGINE) compose
COMPOSE_FILE ?= scripts/docker/docker-compose.yml

# -------- CPU profile via docker compose --------
.PHONY: docker-build-cpu
docker-build-cpu:
	$(COMPOSE) -f $(COMPOSE_FILE) --profile cpu build

.PHONY: docker-up-cpu
docker-up-cpu:
	$(COMPOSE) -f $(COMPOSE_FILE) --profile cpu up -d

.PHONY: docker-ps
docker-ps:
	$(COMPOSE) -f $(COMPOSE_FILE) ps

.PHONY: docker-logs-cpu
docker-logs-cpu:
	$(COMPOSE) -f $(COMPOSE_FILE) logs -f magray-cpu

.PHONY: docker-smoke-cpu
docker-smoke-cpu:
	$(COMPOSE) -f $(COMPOSE_FILE) --profile cpu exec -T magray-cpu /usr/local/bin/magray --version
	$(COMPOSE) -f $(COMPOSE_FILE) --profile cpu exec -T magray-cpu /usr/local/bin/magray status || true

.PHONY: docker-down
docker-down:
	$(COMPOSE) -f $(COMPOSE_FILE) down -v --remove-orphans

# -------- Direct docker build/run fallback (без compose) --------
IMAGE_CPU ?= magray:cpu
DOCKERFILE_CPU ?= scripts/docker/Dockerfile.cpu

.PHONY: docker-build-cpu-direct
docker-build-cpu-direct:
	$(ENGINE) build -t $(IMAGE_CPU) -f $(DOCKERFILE_CPU) .

.PHONY: docker-run-cpu-direct
docker-run-cpu-direct:
	$(ENGINE) run --rm $(IMAGE_CPU) /usr/local/bin/magray --version
	$(ENGINE) run --rm -e RUST_LOG=info $(IMAGE_CPU) /usr/local/bin/magray --help | head -n 20

# -------- All-in-one helpers --------
.PHONY: up-cpu
up-cpu: docker-build-cpu docker-up-cpu docker-ps

.PHONY: smoke-cpu
smoke-cpu: docker-smoke-cpu

.PHONY: down
down: docker-down

.PHONY: agent-setup
agent-setup:
	bash scripts/agents/bootstrap.sh --non-interactive

.PHONY: agent-analyze
agent-analyze:
	bash scripts/agents/analyze_project.sh