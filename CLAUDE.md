# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

MAGRAY CLI is an intelligent CLI agent built in Rust with a sophisticated multi-layer memory system and LLM integration. The project uses a workspace structure with multiple crates and local ONNX models for embeddings and reranking.

## Essential Commands

```bash
# Build the entire project
cargo build --release

# Build specific crates
cargo build -p cli
cargo build -p memory

# Run the main CLI
cargo run --bin magray
./target/release/magray

# Run tests
cargo test
cargo test -p memory  # Test specific crate

# Check code (faster than full build)
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Architecture Overview

### Workspace Structure
The project uses Rust workspaces with 5 main crates:

1. **cli** - User interface, animations, command handling
2. **llm** - LLM integration with multiple providers (OpenAI, Anthropic, local)
3. **memory** - 5-layer memory system with semantic search
4. **router** - Request routing and gateway
5. **tools** - File operations, git, shell, web tools

### Memory System (5 Layers)
Located in `memory/` crate:

- **M0 Ephemeral** - RAM-based temporary storage
- **M1 ShortTerm** - SQLite KV store for recent facts
- **M2 MediumTerm** - SQLite tables for structured data
- **M3 LongTerm** - File blobs for archives
- **M4 Semantic** - Vector index using ONNX models (Qwen3)

All layers are indexed through M4 (semantic layer). Search always starts from M4 and returns references to other layers.

### Key Components

**Memory Coordinator** (`memory/src/coordinator.rs`)
- Routes between memory layers
- Handles promotion/demotion between layers
- Manages semantic indexing

**Semantic Router** (`memory/src/semantic.rs`)
- Uses Qwen3 ONNX models for embeddings and reranking
- Manages vector index with cosine similarity search
- Handles caching and batching

**LLM Agents** (`llm/src/agents/`)
- Intent analyzer
- Parameter extractor
- Tool selector
- Action planner

### Data Storage
User data is stored outside the project directory:
```
~/.ourcli/projects/<project_id>/
  config.toml
  sqlite.db
  tasks.db
  vectors/
  blobs/
  embed_cache.db
  events.log
```

## Working with ONNX Models

The memory crate uses local ONNX models for embeddings and reranking:
- **Qwen3-Embedding-0.6B-ONNX** - For text embeddings (1024 dimensions)
- **Qwen3-Reranker-0.6B-ONNX** - For cross-encoder reranking

Models are located in `memory/src/Qwen3-*/` directories. The actual `.onnx` files are not tracked in git due to size.

## Development Tips

1. **Testing Memory Layer**: Use `cargo test -p memory` to test the memory system
2. **Model Loading**: Ensure ONNX models are present before running memory tests
3. **Logging**: Set `RUST_LOG=debug` for detailed logs
4. **Configuration**: Use `.env` file for LLM provider settings

## Common Tasks

### Adding a New Tool
1. Implement the tool in `tools/src/`
2. Add to the tool registry
3. Update semantic descriptions for tool selection

### Modifying Memory Layers
1. Update layer definitions in `memory/src/layers/`
2. Adjust coordinator logic in `memory/src/coordinator.rs`
3. Update semantic indexing if needed

### Working with LLM Providers
Configuration is in `.env`:
```env
LLM_PROVIDER=openai
OPENAI_API_KEY=sk-your-key
OPENAI_MODEL=gpt-4o-mini
```

## Important Files to Review

- `docs/ARCHITECTURE.md` - Detailed system architecture
- `memory/src/lib.rs` - Memory system public API
- `llm/src/lib.rs` - LLM integration interfaces
- `cli/src/main.rs` - Entry point and CLI commands