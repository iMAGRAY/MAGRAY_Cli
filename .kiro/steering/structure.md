# MAGRAY CLI Project Structure

## Workspace Organization

```
magray-cli/
├── crates/
│   ├── cli/           # Main binary and user interface
│   ├── llm/           # LLM client abstraction
│   └── tools/         # Tool system and AI router
├── .env.example       # Environment configuration template
├── Cargo.toml         # Workspace configuration
└── README.md          # Project documentation
```

## Crate Responsibilities

### `crates/cli/` - Main Binary
- **Purpose**: User interface, animations, command handling
- **Binary**: `magray` (main executable)
- **Key modules**:
  - `main.rs`: CLI parsing, welcome animations, command routing
  - `agent.rs`: UnifiedAgent that decides between chat and tools
- **Dependencies**: llm, tools, clap, console, indicatif

### `crates/llm/` - LLM Client
- **Purpose**: Abstract LLM provider interactions
- **Key features**:
  - Multi-provider support (OpenAI, Anthropic, Local)
  - Environment-based configuration
  - Consistent API across providers
- **Main file**: `lib.rs` with LlmClient struct
- **Dependencies**: reqwest, serde, dotenv

### `crates/tools/` - Tool System
- **Purpose**: File operations, git, web search, shell execution
- **Key modules**:
  - `lib.rs`: Tool trait, ToolRegistry, base types
  - `ai_router.rs`: SmartRouter for AI-driven tool selection
  - `file_ops.rs`: File read/write/list operations
  - `git_ops.rs`: Git status and commit operations
  - `shell_ops.rs`: Cross-platform shell command execution
  - `web_ops.rs`: Web search functionality
- **Dependencies**: llm, walkdir, syntect, ureq

## Architecture Patterns

### Unified Agent Pattern
- Single entry point for all user interactions
- AI-driven decision making between chat and tools
- Eliminates hard-coded heuristics

### Tool Registry Pattern
- Dynamic tool registration and discovery
- Consistent Tool trait interface
- Natural language parsing for each tool

### Cross-Platform Shell Execution
- Windows: `cmd.exe /C <command>`
- Unix: `/bin/sh -c <command>`
- Proper error handling and output capture

## File Naming Conventions
- Snake_case for Rust files and modules
- Kebab-case for binary names and directories
- Clear, descriptive names that indicate purpose

## Dependency Management
- Workspace-level dependency definitions
- Minimal external dependencies for fast builds
- Feature flags used appropriately (e.g., tokio "full" features)

## Configuration Files
- `.env`: Runtime configuration (API keys, models)
- `Cargo.toml`: Build configuration and dependencies
- No complex config files - simplicity first