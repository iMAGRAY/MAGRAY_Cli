# MAGRAY CLI Technical Stack

## Build System
- **Cargo Workspace**: Multi-crate Rust project with workspace dependencies
- **Target**: Single binary executable (`magray`)
- **Build time**: ~30 seconds for release builds
- **Output size**: ~2MB executable

## Tech Stack

### Core Language & Runtime
- **Rust 2021 Edition**: All crates use workspace edition
- **Tokio**: Async runtime with full features
- **Anyhow**: Error handling throughout the project

### LLM Integration
- **reqwest**: HTTP client for API calls to OpenAI, Anthropic, local models
- **serde/serde_json**: JSON serialization for API requests/responses
- **dotenv**: Environment configuration management

### CLI & UX
- **clap**: Command-line argument parsing with derive features
- **console**: Cross-platform terminal styling and control
- **indicatif**: Progress bars and spinners for animations
- **tracing/tracing-subscriber**: Logging (usually set to error level for clean UX)

### Tools & Utilities
- **walkdir**: Directory traversal
- **syntect**: Syntax highlighting for file display
- **ignore**: Git-aware file filtering
- **tempfile**: Temporary file operations
- **ureq**: Alternative HTTP client for some tools

## Common Commands

### Development
```bash
# Build for development
cargo build

# Build optimized release
cargo build --release

# Run with debug logging
RUST_LOG=debug ./target/release/magray

# Run tests
cargo test

# Check all crates
cargo check --workspace
```

### Usage
```bash
# Interactive mode with animations
./target/release/magray

# Direct chat
./target/release/magray chat "your message"

# Tool operations (handled by unified agent)
./target/release/magray chat "read file main.rs"
./target/release/magray chat "create file test.txt with hello world"
```

## Environment Configuration
- Copy `.env.example` to `.env`
- Configure LLM provider (openai/anthropic/local)
- Set API keys and model preferences
- Adjust MAX_TOKENS and TEMPERATURE as needed

## Cross-Platform Support
- Windows: Uses `cmd.exe /C` for shell commands
- Unix/Linux/macOS: Uses `/bin/sh -c` for shell commands
- Path handling is platform-aware