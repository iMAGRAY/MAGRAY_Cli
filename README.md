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
- 🔌 **WASI Plugins** - Secure, sandboxed extensions in any language
- 📊 **Observable** - Built-in tracing, metrics, and event logging
- 🛡️ **Memory Safe** - 100% Rust with zero unsafe blocks in core

## 🚀 Quick Start

```bash
# Install from crates.io (when published)
cargo install ourcli

# Or build from source
git clone https://github.com/yourusername/MAGRAY_Cli
cd MAGRAY_Cli
cargo build --release
cargo install --path crates/cli

# First run - downloads models automatically
ourcli init

# Start using
ourcli ask "How do I implement a Redis cache?"
ourcli remember "Project uses PostgreSQL 15 with TimescaleDB"
ourcli search "database configuration"
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
./scripts/download_models.sh

# Build and install
cargo build --release
cargo install --path crates/cli

# Verify installation
ourcli --version
```

### With Features

```bash
# Enable GPU acceleration
cargo install --path crates/cli --features gpu

# Enable all features (GPU, TUI, remote LLMs)
cargo install --path crates/cli --all-features
```

## 🎯 Usage

### Basic Commands

```bash
# Ask questions with context-aware responses
ourcli ask "How do I optimize this SQL query?"

# Store information for later retrieval
ourcli remember "API rate limit is 1000 req/min"

# Search your knowledge base
ourcli search "rate limiting"

# Execute tools
ourcli run shell "ls -la"
ourcli run git "status"

# Interactive mode
ourcli chat
```

### Advanced Features

```bash
# Use specific LLM model
ourcli ask "Explain async Rust" --model llama-3.2

# Search with filters
ourcli search "error handling" --layer insights --limit 20

# Export memory
ourcli export memory --format json > backup.json

# Install plugin
ourcli plugin install ./my-plugin.wasm
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

## 🔧 Configuration

Configuration file at `~/.ourcli/config.toml`:

```toml
[ai]
embed_model = "bge-small-v1.5"
embed_batch_size = 32

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
plugin_dir = "~/.ourcli/plugins"
```

## 🔌 Plugin Development

Create WASI plugins in any language:

```rust
// Rust plugin example
use serde_json::{json, Value};

#[no_mangle]
pub extern "C" fn invoke(input: *const u8, len: usize) -> *mut u8 {
    // Parse input, process, return JSON result
}
```

```json
// manifest.json
{
  "name": "my-tool",
  "version": "1.0.0",
  "description": "Does something useful",
  "inputs": [
    {"name": "query", "type": "string", "required": true}
  ],
  "outputs": [
    {"name": "result", "type": "object"}
  ]
}
```

Build and install:
```bash
cargo build --target wasm32-wasi --release
ourcli plugin install target/wasm32-wasi/release/my_tool.wasm
```

## 🧪 Development

### Project Structure

```
MAGRAY_Cli/
├── crates/
│   ├── cli/        # Main binary
│   ├── core/       # Business logic
│   ├── memory/     # Vector store
│   ├── ai/         # ML models
│   ├── tools/      # Tool system
│   └── scheduler/  # Background jobs
├── plugins/        # Example plugins
├── models/         # ONNX models (git-ignored)
└── tests/          # Integration tests
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
```bash
# Re-download models
./scripts/download_models.sh

# Verify models
ls -la models/
```

**High memory usage**
```bash
# Reduce batch sizes in config
# Clear vector cache
rm -rf ~/.ourcli/cache/embeddings.db
```

**Plugin won't load**
```bash
# Verify WASI compatibility
wasmtime run my-plugin.wasm

# Check manifest
cat my-plugin.wasm.manifest.json | jq
```

## 📚 Documentation

- [Architecture Overview](docs/ARCHITECTURE.md)
- [API Reference](https://docs.rs/ourcli)
- [Plugin Development Guide](docs/PLUGINS.md)
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
- [Wasmtime](https://wasmtime.dev/) for WASI runtime
- The Rust community for amazing crates

---

Built with ❤️ in Rust | [Star us on GitHub!](https://github.com/yourusername/MAGRAY_Cli)