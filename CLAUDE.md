# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

MAGRAY CLI is an intelligent CLI agent built in Rust with a sophisticated 5-layer memory system and LLM integration. The project uses a workspace structure with multiple crates and local ONNX models for embeddings and reranking.

## Essential Commands

```bash
# Build the entire project
cargo build --release

# Build specific crates
cargo build -p cli
cargo build -p memory
cargo build -p llm
cargo build -p tools
cargo build -p router
cargo build -p todo

# Run the main CLI
cargo run --bin magray
./target/release/magray

# Run tests
cargo test                # All tests
cargo test -p memory      # Test specific crate
cargo test test_name      # Run specific test

# Check code (faster than full build)
cargo check
cargo check -p memory     # Check specific crate

# Format code
cargo fmt
cargo fmt --check         # Check without modifying

# Lint code
cargo clippy
cargo clippy -- -D warnings  # Treat warnings as errors
```

## Architecture Overview

### Workspace Structure
The project uses Rust workspaces with 6 main crates:

1. **cli** (`crates/cli/`) - User interface, animations, command handling
2. **llm** (`crates/llm/`) - LLM integration with multiple providers (OpenAI, Anthropic, local)
3. **memory** (`crates/memory/`) - 5-layer memory system with semantic search
4. **router** (`crates/router/`) - Request routing and gateway
5. **tools** (`crates/tools/`) - File operations, git, shell, web tools
6. **todo** (`crates/todo/`) - Task management with dependency tracking

### Memory System (5 Layers)
Located in `memory/` crate:

| Layer | Storage | Purpose | Indexed in M4 | TTL/Policies |
|-------|---------|---------|---------------|--------------|
| **M0 Ephemeral** | RAM | Temporary data for current session | Usually not | Until end of run |
| **M1 ShortTerm** | SQLite KV | Recent facts/responses | Yes (full text) | Hours/days, auto-cleanup |
| **M2 MediumTerm** | SQLite tables | Structured project knowledge | Yes (text fields) | Weeks/months |
| **M3 LongTerm** | File blobs | Artifacts, logs, large files | Yes (summary/chunks) | No limit |
| **M4 Semantic** | Vector index | HNSW index with references to M0-M3 | — | Cleaned by orphan refs |

All layers are indexed through M4 (semantic layer). Search always starts from M4 and returns references to other layers.

### Key Components

**Memory Coordinator** (`memory/src/coordinator.rs`)
- Routes between memory layers
- Handles promotion/demotion between layers
- Manages semantic indexing

**Semantic Router** (`memory/src/semantic.rs`, `memory/src/semantic_flexible.rs`, `memory/src/semantic_with_fallback.rs`)
- Uses Qwen3 ONNX models for embeddings and reranking
- Manages vector index with cosine similarity search
- Handles caching and batching
- Multiple implementations with fallback support

**LLM Agents** (`llm/src/agents/`)
- `intent_analyzer.rs` - Analyzes user intent
- `parameter_extractor.rs` - Extracts parameters from requests
- `tool_selector.rs` - Selects appropriate tools
- `action_planner.rs` - Plans action sequences

**Todo System** (`todo/src/`)
- `graph.rs`, `graph_v2.rs` - DAG-based task dependencies
- `service_v2.rs` - Task management service
- `store.rs`, `store_v2.rs` - Persistence layer
- `types.rs` - Core data types

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

The project uses local ONNX models for embeddings and reranking:
- **Qwen3-Embedding-0.6B-ONNX** - For text embeddings (1024 dimensions)
- **Qwen3-Reranker-0.6B-ONNX** - For cross-encoder reranking

Models are located in `models/Qwen3-*/` directories. The actual `.onnx` files are not tracked in git due to size.

Setup:
1. Download models from Hugging Face or provided sources
2. Place in `models/` directory maintaining the structure
3. Ensure `model_fp16.onnx` exists for embedding model
4. Ensure `model.onnx` exists for reranker model

## Development Workflow

### Initial Setup
```bash
# Clone and setup
git clone <repo>
cd MAGRAY_Cli

# Copy environment config
cp .env.example .env
# Edit .env with your API keys

# Build everything
cargo build --release

# Run tests to verify setup
cargo test
```

### Running the CLI
```bash
# Interactive mode with animations
./target/release/magray

# Single command
./target/release/magray chat "Hello"

# With debug logging
RUST_LOG=debug ./target/release/magray
```

### Working with LLM Providers
Configuration in `.env`:
```env
# OpenAI (default)
LLM_PROVIDER=openai
OPENAI_API_KEY=sk-your-key
OPENAI_MODEL=gpt-4o-mini

# Anthropic
LLM_PROVIDER=anthropic
ANTHROPIC_API_KEY=sk-ant-your-key
ANTHROPIC_MODEL=claude-3-haiku-20240307

# Local models
LLM_PROVIDER=local
LOCAL_LLM_URL=http://localhost:1234/v1
LOCAL_LLM_MODEL=llama-3.2-3b-instruct
```

## Common Development Tasks

### Adding a New Tool
1. Implement the tool in `tools/src/`
2. Add to the tool registry
3. Update semantic descriptions for tool selection
4. Add tests in `tools/src/` or `tools/tests/`

### Modifying Memory Layers
1. Update layer definitions in `memory/src/layers/`
2. Adjust coordinator logic in `memory/src/coordinator.rs`
3. Update semantic indexing if needed
4. Test with `cargo test -p memory`

### Working with the Todo System
1. Core types are in `todo/src/types.rs`
2. Task graph logic in `todo/src/graph_v2.rs`
3. Service interface in `todo/src/service_v2.rs`
4. Test with `cargo test -p todo`

## Important Files to Review

- `docs/ARCHITECTURE.md` - Detailed system architecture and design
- `docs/DIAGRAMS.md` - System diagrams and visualizations
- `docs/TODO_SPEC.md` - Todo system specification
- `memory/src/lib.rs` - Memory system public API
- `llm/src/lib.rs` - LLM integration interfaces
- `cli/src/main.rs` - Entry point and CLI commands
- `cli/src/agent.rs` - Core agent logic

## Testing Strategy

```bash
# Unit tests for specific functionality
cargo test unit_test_name

# Integration tests
cargo test --test integration_test

# Memory system tests (requires ONNX models)
cargo test -p memory

# Run all tests with output
cargo test -- --nocapture

# Run tests in single thread (for debugging)
cargo test -- --test-threads=1
```

## Performance Considerations

- Memory crate uses batching for embeddings to improve throughput
- Vector index uses HNSW for efficient similarity search
- SQLite with bundled feature for consistent performance
- Async/await throughout for concurrent operations
- Connection pooling for database access