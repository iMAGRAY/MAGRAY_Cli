# MAGRAY CLI

**An intelligent CLI agent built in Rust with a sophisticated multi-layer memory system and LLM integration**

```
  ███╗   ███╗ █████╗  ██████╗ ██████╗  █████╗ ██╗   ██╗
  ████╗ ████║██╔══██╗██╔════╝ ██╔══██╗██╔══██╗╚██╗ ██╔╝
  ██╔████╔██║███████║██║  ███╗██████╔╝███████║ ╚████╔╝ 
  ██║╚██╔╝██║██╔══██║██║   ██║██╔══██╗██╔══██║  ╚██╔╝  
  ██║ ╚═╝ ██║██║  ██║╚██████╔╝██║  ██║██║  ██║   ██║   
  ╚═╝     ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   

       Intelligent CLI Agent v0.1.0
       Memory-Enhanced AI Assistant
```

## Features

- **5-Layer Memory System** - Ephemeral, Short-term, Medium-term, Long-term, Semantic
- **Multi-LLM Support** - OpenAI, Anthropic, local models
- **Intelligent Task Management** - Todo system with dependency tracking
- **Tool Integration** - File operations, git, shell, web
- **Local ONNX Models** - Qwen3 embeddings and reranking
- **Professional CLI** - Animations, progress indicators, colored output  

## Quick Start

### 1. Build

```bash
cargo build --release
```

### 2. Configuration

Copy the example configuration and set up your API key:

```bash
cp .env.example .env
# Отредактируйте .env файл
```

Example `.env` for OpenAI:
```env
LLM_PROVIDER=openai
OPENAI_API_KEY=sk-your-api-key-here
OPENAI_MODEL=gpt-4o-mini
MAX_TOKENS=1000
TEMPERATURE=0.7
```

### 3. Usage

**Interactive chat (with welcome animation):**
```bash
./target/release/magray
```

**Single message:**
```bash
./target/release/magray chat "Hello, how are you?"
```

## Architecture

### Workspace Structure
```
crates/
├── cli/        # User interface, animations, command handling
├── llm/        # LLM integration (OpenAI, Anthropic, local)
├── memory/     # 5-layer memory system with semantic search
├── router/     # Request routing and gateway
├── tools/      # File operations, git, shell, web tools
└── todo/       # Task management with dependency tracking
```

### Memory System (5 Layers)
- **M0 Ephemeral** - RAM-based temporary storage
- **M1 ShortTerm** - SQLite KV store for recent facts
- **M2 MediumTerm** - SQLite tables for structured data
- **M3 LongTerm** - File blobs for archives
- **M4 Semantic** - Vector index using ONNX models (Qwen3)

All layers are indexed through M4 (semantic layer). Search always starts from M4 and returns references to other layers.

## Supported Providers

<details>
<summary><strong>OpenAI (recommended)</strong></summary>

```env
LLM_PROVIDER=openai
OPENAI_API_KEY=sk-your-key
OPENAI_MODEL=gpt-4o-mini  # or gpt-4, gpt-3.5-turbo
```
</details>

<details>
<summary><strong>Anthropic (Claude)</strong></summary>

```env
LLM_PROVIDER=anthropic
ANTHROPIC_API_KEY=sk-ant-your-key
ANTHROPIC_MODEL=claude-3-haiku-20240307  # or claude-3-sonnet-20240229
```
</details>

<details>
<summary><strong>Local Models</strong></summary>

```env
LLM_PROVIDER=local
LOCAL_LLM_URL=http://localhost:1234/v1
LOCAL_LLM_MODEL=llama-3.2-3b-instruct
```

Compatible with LM Studio, Ollama with OpenAI API, and others.
</details>

## Commands

```bash
magray                      # Interactive chat with animations
magray chat "message"       # Quick response
magray --help               # Help
```

In interactive mode, type `exit` or `quit` to exit gracefully.

## Settings

In the `.env` file you can configure:

| Parameter | Description | Default |
| `MAX_TOKENS` | Maximum tokens in response | 1000 |
| `TEMPERATURE` | Creativity (0.0-2.0) | 0.7 |

## Debugging

For detailed logging:
```bash
RUST_LOG=debug ./target/release/magray chat "test"
```

## ONNX Models

The memory crate uses local ONNX models for embeddings and reranking:
- **Qwen3-Embedding-0.6B-ONNX** - For text embeddings (1024 dimensions)
- **Qwen3-Reranker-0.6B-ONNX** - For cross-encoder reranking

Models should be placed in the `models/` directory. The actual `.onnx` files are not tracked in git due to size.

## Project Structure

```
MAGRAY_Cli/
├── Cargo.toml          # Workspace configuration
├── crates/             # All Rust crates
│   ├── cli/            # Main CLI application
│   ├── llm/            # LLM providers and agents
│   ├── memory/         # 5-layer memory system
│   ├── router/         # Request routing
│   ├── tools/          # External tools
│   └── todo/           # Task management
├── models/             # ONNX model files
├── docs/               # Documentation
└── config/             # Configuration examples
```  

## License

MIT License - see LICENSE file for details.

---

**Version:** 0.1.0  
**License:** MIT