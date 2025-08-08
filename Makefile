# MAGRAY CLI Build System
# Supports multiple feature configurations

.PHONY: help build-all build-cpu build-gpu build-minimal test test-all bench clean docker-build docker-test release

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
	@echo "  make bench         - Run performance benchmarks"
	@echo ""
	@echo "🐳 Docker Commands:"
	@echo "  make docker-build  - Build all Docker images"
	@echo "  make docker-test   - Test Docker containers"
	@echo ""
	@echo "📊 Analysis Commands:"
	@echo "  make size-analysis - Compare binary sizes"
	@echo "  make perf-test     - Quick performance test"
	@echo ""
	@echo "🔧 Utility Commands:"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make release       - Create release binaries"

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
	RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --features=cpu --no-fail-fast

test-full:
	@echo "🧪 Running full CPU tests (extended)..."
	RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --features="cpu,extended-tests" --no-fail-fast

test-persistence:
	@echo "🧪 Running persistence tests (CPU + persistence + extended)..."
	RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --features="cpu,persistence,extended-tests" --no-fail-fast

test-gpu-full:
	@echo "🎮 Running full GPU tests (extended)..."
	RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --features="gpu,extended-tests" --no-fail-fast

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
	RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo tarpaulin --engine llvm --features=cpu --timeout 120 --out Html
	@echo "✅ Coverage report: tarpaulin-report.html"

coverage-full:
	@echo "📈 Running full coverage (extended-tests) ..."
	@which cargo-tarpaulin >/dev/null 2>&1 || cargo install cargo-tarpaulin
	RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo tarpaulin --engine llvm --features="cpu,extended-tests" --timeout 600 --out Html
	@echo "✅ Coverage report: tarpaulin-report.html"

ci-local-fast:
	@echo "🏃 CI-local: fast cpu tests"
	RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --features=cpu --no-fail-fast --quiet

ci-local-extended:
	@echo "🏃 CI-local: extended cpu tests"
	RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --features="cpu,extended-tests" --no-fail-fast --quiet

ci-local-persistence:
	@echo "🏃 CI-local: persistence suite"
	RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --features="cpu,persistence,extended-tests" --no-fail-fast --quiet

ci-local-gpu:
	@echo "🏃 CI-local: gpu extended (if available)"
	RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --features="gpu,extended-tests" --no-fail-fast --quiet || true