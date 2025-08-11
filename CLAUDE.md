# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**MAGRAY CLI** - локальный интеллектуальный CLI агент для разработки с использованием LLM моделей (как API, так и локальных). Проект находится в ALPHA стадии с множеством нерешённых проблем.

## 🛠️ Development Commands

### Building
```bash
# Build all variants (minimal, cpu, gpu)
powershell scripts/build_all.ps1 -Variant all

# Build specific variant
cargo build --release --no-default-features --features="cpu" --target-dir="target/cpu"
cargo build --release --no-default-features --features="gpu" --target-dir="target/gpu"  
cargo build --release --no-default-features --features="minimal" --target-dir="target/minimal"

# Quick development build
cargo build --features cpu
```

### Testing
```bash
# Run all tests
cargo test --workspace

# Test specific variant
cargo test --features=cpu --workspace
cargo test --features=gpu --workspace  
cargo test --features=minimal --workspace

# Test specific crate
cargo test -p memory --features=cpu
cargo test -p ai --features=gpu

# Run single test
cargo test test_name -- --exact

# Run tests with output
cargo test -- --nocapture
```

### Linting & Format
```bash
# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all

# Check format without applying
cargo fmt --all -- --check
```

### Coverage
```bash
# Generate coverage report
powershell scripts/check_coverage.ps1

# Or using tarpaulin directly
cargo tarpaulin --out Html --output-dir coverage_report
```

## 🏗️ Architecture

### Crate Structure
```
crates/
├── ai/          # ONNX models, embeddings, GPU support
├── application/ # Application layer with CQRS, adapters
├── cli/         # Main CLI binary, agents, handlers
├── common/      # Shared utilities, service traits
├── domain/      # Domain models and business logic
├── llm/         # Multi-provider LLM integration
├── memory/      # 3-layer HNSW vector memory system
├── router/      # Smart task routing
├── todo/        # Task DAG system
└── tools/       # External tools registry
```

### Key Design Patterns
- **DI Container**: Extensive dependency injection in memory crate
- **Service Traits**: Common service interface across all modules
- **Agent System**: Specialized agents for different tasks (in cli/agents/)
- **Feature Flags**: Conditional compilation for cpu/gpu/minimal builds
- **SIMD Optimizations**: Vector operations in memory crate

### Memory System Architecture
- **3-Layer HNSW**: Hierarchical Navigable Small World index
- **Vector Store**: Embeddings storage with SIMD acceleration
- **Promotion System**: ML-based memory promotion between layers
- **GPU Acceleration**: Optional CUDA/TensorRT support

## ⚠️ Critical Issues (ALPHA Status)

### Statistics (Auto-updated: 2025-08-08)
- **Critical issues**: 118
- **High priority issues**: 319
- **Code duplications**: 999 cases
- **Test coverage**: 25.1% (77/307 modules)
- **Technical debt**: 7908 hours
- **High complexity files**: 182

### Major Architectural Issues
- Excessive complexity in DI container (cyclomatic complexity up to 97)
- Missing error handling in many modules (extensive unwrap() usage)
- Incomplete GPU feature implementation
- Memory leaks in vector operations
- Race conditions in async handlers

### Top Priority Fixes Needed
1. Remove all unwrap() calls and add proper error handling
2. Reduce code duplication (141 serious cases with >4 copies)
3. Fix SIMD implementations causing segfaults
4. Complete test coverage for critical paths
5. Simplify DI container architecture
6. Fix memory promotion system bugs
7. Resolve async/sync boundary issues

## 📦 Dependencies & Setup

### Prerequisites
- Rust toolchain (rustup)
- CUDA Toolkit 12.x (for GPU builds)
- ONNX Runtime libraries

### Setup Steps
```bash
# Download ONNX Runtime
powershell scripts/download_onnxruntime.ps1      # CPU version
powershell scripts/download_onnxruntime_gpu.ps1  # GPU version

# Install models
python scripts/install_qwen3_minimal.py

# Setup environment
cp .env.example .env
# Edit .env with your LLM provider keys
```

## 🔧 Common Development Tasks

### Running the CLI
```bash
# Basic commands
magray health
magray chat "Your message"
magray smart "analyze src/ and suggest refactoring"

# Memory operations
magray memory add "Important fact" --layer insights
magray memory search "query"

# Tool execution
magray tool "create file hello.rs"
magray tool "git status"
```

### Debugging
```bash
# Enable debug logging
set RUST_LOG=debug
magray [command]

# Run with backtrace
set RUST_BACKTRACE=1
magray [command]
```

## 📝 Important Notes

- **Python**: Use `py` command instead of `python` on Windows
- **Code Comments**: Avoid Russian comments and emojis in code (causes formatting issues)
- **Error Handling**: Always use proper error handling, avoid unwrap()
- **Testing**: Write tests for all new functionality
- **Performance**: Run benchmarks before optimizing

## Scripts & Utilities

### CTL (Claude Tensor Language) Tool
```bash
# Task management system
python scripts/ctl.py add --kind T --id "task-1" --title "Fix memory leak"
python scripts/ctl.py query --priority 1
```

### Architecture Analysis
```bash
# Run architecture daemon for continuous analysis
powershell scripts/run_architecture_daemon.ps1

# One-time analysis  
powershell scripts/archilens-auto-analysis.ps1
```

### Model Management
```bash
# Download and install models
python scripts/download_models.ps1
python scripts/install_qwen3_onnx.py
```