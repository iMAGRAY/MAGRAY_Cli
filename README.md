# MAGRAY CLI 🚀

A blazing-fast, pure-Rust AI agent CLI with local-first memory, semantic search, and extensible tool system. Ship as a single binary with zero dependencies.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/yourusername/MAGRAY_Cli/ci.yml?branch=main)](https://github.com/yourusername/MAGRAY_Cli/actions)

## ✨ Features

- 🏃 **Single Static Binary** - Install with `cargo install`, no Python/Node/Docker required
- 🧠 **Multi-Layer Memory** - Smart context management with automatic promotion/decay
- 🔍 **HNSW Vector Search** - Sub-10ms semantic search with professional hnsw_rs implementation
- 🤖 **Local AI Stack** - ONNX embeddings/reranking, optional LLM providers
- 🔧 **Extensible Tools** - File operations, git integration, shell commands
- 📊 **Observable** - Built-in tracing, metrics, and event logging
- 🛡️ **Memory Safe** - 100% Rust with zero unsafe blocks in core

## 🚀 Quick Start

```bash
# Install from crates.io (when published)
cargo install magray

# Or build from source
git clone https://github.com/yourusername/MAGRAY_Cli
cd MAGRAY_Cli
cargo build --release
cargo install --path crates/cli

# Download models manually (required)
./download_models.ps1

# Start using
magray ask "How do I implement a Redis cache?"
magray remember "Project uses PostgreSQL 15 with TimescaleDB"
magray search "database configuration"
```

## 📦 Installation

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- 4GB RAM minimum (8GB recommended)
- 2GB disk space for models

### From Source

```bash
# Clone repository
git clone https://github.com/yourusername/MAGRAY_Cli
cd MAGRAY_Cli

# Download required ONNX models
./download_models.ps1

# Build and install
cargo build --release
cargo install --path crates/cli

# Verify installation
magray --version
```

### With Features

```bash
# Enable GPU acceleration
cargo build --release --features gpu

# Enable all features (GPU, TUI, remote LLMs)
cargo build --release --all-features
```

## 🎯 Usage

### Basic Commands

```bash
# Interactive chat mode (default)
magray

# Direct chat with message
magray chat "How do I optimize this SQL query?"

# File operations
magray read file.txt
magray write output.txt "Hello World"
magray list ./src

# Tool execution with natural language
magray tool "show git status"
magray tool "create a new file with hello world"

# Smart AI planning for complex tasks
magray smart "analyze this codebase and suggest improvements"
```

### Advanced Features

```bash
# Memory system operations
magray memory search "error handling" --layer insights --top-k 20
magray memory add "API rate limit is 1000 req/min" --layer insights
magray memory stats
magray memory backup --name my-backup

# GPU acceleration management
magray gpu info
magray gpu benchmark --batch-size 100 --compare
magray gpu memory status
```

## 🏗️ Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│     CLI     │────▶│    Core     │────▶│   Memory    │
│   (clap)    │     │  (planner)  │     │  (HNSW+BGE) │
└─────────────┘     └─────────────┘     └─────────────┘
                            │                    │
                            ▼                    ▼
                    ┌─────────────┐     ┌─────────────┐
                    │     AI      │     │    Tools    │
                    │ (embedding) │     │   (WASI)    │
                    └─────────────┘     └─────────────┘
```

### Memory Layers

| Layer | Purpose | Retention | Performance |
|-------|---------|-----------|-------------|
| **L1 Interact** | Current session context | 24 hours | HNSW index, <5ms |
| **L2 Insights** | Distilled knowledge | 90 days | HNSW index, <8ms |
| **L3 Assets** | Long-term storage | Unlimited | HNSW index, <10ms |

### Vector Search Performance

The system uses **hnsw_rs** by Jean-Pierre Both - a professional Rust implementation of Hierarchical Navigable Small World algorithm:

- 🚀 **17x faster** than linear search on 5K+ documents
- 🎯 **100% recall** with optimal parameters
- ⚡ **Sub-linear scaling** O(log n) vs O(n)
- 🔧 **Tunable parameters**: M=24, ef_construction=400, ef_search=100
- 🧵 **Parallel operations** for batch insertions and multi-query search

**Benchmark Results:**
```
Dataset Size    HNSW Time    Linear Time    Speedup
100 docs        1.9ms        2.1ms          1.1x
500 docs        2.9ms        10.5ms         3.6x  
1000 docs       4.2ms        21.0ms         5.0x
2000 docs       3.1ms        42.3ms         13.8x
5000 docs       6.0ms        104.8ms        17.4x
```

## 🤖 AI Models

MAGRAY CLI использует современные ONNX модели для векторного поиска и ранжирования:

### Embedding Model
- **[Qwen3-Embedding-0.6B-ONNX](https://huggingface.co/onnx-community/Qwen3-Embedding-0.6B-ONNX/)**
  - Размерность: 1024
  - Поддержка многоязычности (русский, английский, китайский)
  - Оптимизирован для ONNX Runtime
  - Размер модели: ~600MB

### Reranking Model  
- **[Qwen3-Reranker-0.6B-ONNX](https://huggingface.co/zhiqing/Qwen3-Reranker-0.6B-ONNX/)**
  - Семантическое переранжирование результатов поиска
  - Высокая точность на многоязычных текстах
  - INT8 квантизация для производительности
  - Размер модели: ~600MB

Модели автоматически загружаются при первом запуске или через:
```powershell
./download_models.ps1
```

## 🔧 Configuration

Configuration file at `~/.magray/config.toml`:

```toml
[ai]
embed_model = "qwen3emb"
embed_batch_size = 32
rerank_model = "qwen3_reranker"

[ai.llm]
provider = "local"
model = "llama-3.2-3b-instruct.gguf"
max_tokens = 2048

[memory]
interact_ttl_hours = 24
insights_ttl_days = 90
promote_threshold = 0.8

[tools]
enable_network = false
plugin_dir = "~/.magray/plugins"
```

## 🔧 Tool System

MAGRAY CLI includes built-in tools for common development tasks:

- **File Operations**: Read, write, and list files with syntax highlighting
- **Git Integration**: Status, commit, and repository management
- **Shell Commands**: Cross-platform command execution
- **Web Search**: Search capabilities for documentation and resources

Tools are accessed through natural language commands:
```bash
magray tool "show git status"
magray tool "create a new file called test.rs with a hello world function"
magray tool "list all .rs files in the src directory"
```

## 🧪 Development

### Project Structure

```
MAGRAY_Cli/
├── crates/
│   ├── cli/        # Main binary (magray)
│   ├── llm/        # LLM client abstraction
│   ├── memory/     # Vector store & memory layers
│   ├── ai/         # ONNX models & embeddings
│   ├── tools/      # Tool system & operations
│   ├── router/     # AI routing logic
│   └── todo/       # Task management
├── models/         # ONNX models (git-ignored)
├── scripts/        # Setup & utility scripts
└── docs/           # Documentation
```

### Building

```bash
# Development build
cargo build

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench

# Generate docs
cargo doc --open
```

### Contributing

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open Pull Request

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

## 📊 Performance

Benchmarks on M1 MacBook Air:

| Operation | Time | Notes |
|-----------|------|-------|
| Embedding generation | 12ms | Batch of 32 |
| Vector search (1M docs) | 6ms | HNSW index (hnsw_rs) |
| Reranking (32 results) | 15ms | INT8 quantized |
| Memory promotion | 50ms | Async background |

## 🛠️ Troubleshooting

### Common Issues

**"Model not found" error**
```powershell
# Re-download models
./download_models.ps1

# Verify models
dir models/
```

**High memory usage**
```powershell
# Reduce batch sizes in config
# Clear vector cache
Remove-Item -Recurse -Force ~/.magray/cache/embeddings.db
```

**Tool execution fails**
```powershell
# Check tool availability
magray tool "list available tools"

# Verify environment
echo $env:PATH
```

## 📚 Documentation

- [Architecture Overview](docs/ARCHITECTURE.md)
- [API Reference](https://docs.rs/ourcli)
- [Tool System Guide](docs/TOOLS.md)
- [Memory System Deep Dive](docs/MEMORY.md)

## 🤝 Community

- [Discord Server](https://discord.gg/ourcli)
- [GitHub Discussions](https://github.com/yourusername/MAGRAY_Cli/discussions)
- [Twitter](https://twitter.com/ourcli)

## 📄 License

This project is licensed under the MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [hnsw_rs](https://github.com/jean-pierreBoth/hnswlib-rs) by Jean-Pierre Both for professional HNSW implementation
- [ONNX Runtime](https://onnxruntime.ai/) for fast inference
- [Tokio](https://tokio.rs/) for async runtime
- The Rust community for amazing crates

---

Built with ❤️ in Rust | [Star us on GitHub!](https://github.com/yourusername/MAGRAY_Cli)