# MAGRAY CLI Configuration Guide

## Overview

MAGRAY CLI uses a flexible configuration system that supports:
- TOML and JSON configuration files
- Environment variables
- Default values
- Multiple configuration locations

## Configuration Priority

Configuration values are loaded in the following order (highest priority first):
1. Environment variables
2. Configuration file (first found)
3. Default values

## Configuration File Locations

MAGRAY looks for configuration files in the following locations (in order):
1. Current directory: `.magrayrc`, `.magrayrc.toml`, `.magrayrc.json`, `magray.toml`, `magray.json`
2. User home directory: `~/.magrayrc`, `~/.magrayrc.toml`, `~/.magrayrc.json`
3. User config directory: `~/.config/magray/config.toml`, `~/.config/magray/config.json`
4. System config directory: `/etc/magray/config.toml` (Linux/Mac) or `%PROGRAMDATA%\magray\config.toml` (Windows)

## Quick Start

### Generate Configuration

```bash
# Generate example configuration
magray config generate

# Generate JSON configuration
magray config generate -f json -o config.json

# Initialize configuration in current directory
magray config init
```

### Validate Configuration

```bash
# Validate current configuration
magray config validate

# Validate specific configuration file
magray config validate -c /path/to/config.toml
```

### Show Configuration

```bash
# Show current configuration
magray config show

# Show resolved configuration (with environment variables)
magray config show --resolved
```

## Configuration Parameters

### AI Configuration

```toml
[ai]
# Default AI provider to use
default_provider = "openai"

# Maximum tokens for generation
max_tokens = 4096

# Temperature for generation (0.0-2.0)
temperature = 0.7

# Fallback chain of providers
fallback_chain = ["openai", "anthropic", "local"]

# Provider configurations
[ai.providers.openai]
provider_type = "openai"
api_key = "your-api-key-here"
api_base = "https://api.openai.com/v1"
model = "gpt-4"

[ai.providers.anthropic]
provider_type = "anthropic"
api_key = "your-api-key-here"
api_base = "https://api.anthropic.com"
model = "claude-3-opus-20240229"

[ai.providers.local]
provider_type = "local"
model = "qwen3-0.6b"
model_path = "./models/qwen3-0.6b.onnx"

# Retry configuration
[ai.retry_config]
max_retries = 3
initial_delay_ms = 100
max_delay_ms = 10000
exponential_base = 2.0
```

### Memory Configuration

```toml
[memory]
# Backend type: sqlite, inmemory, hybrid
backend = "sqlite"

# Cache size in MB
cache_size_mb = 256

# Flush interval in seconds
flush_interval_sec = 60

# HNSW index configuration
[memory.hnsw]
m = 16                    # Number of connections
ef_construction = 200     # Size of dynamic candidate list
ef_search = 50           # Size of dynamic candidate list for search
max_elements = 1000000   # Maximum number of elements

# Embedding configuration
[memory.embedding]
model = "qwen3-0.6b"     # Embedding model
dimension = 384          # Embedding dimension
use_gpu = false          # Use GPU for embeddings
batch_size = 32          # Batch size for processing

# Persistence configuration
[memory.persistence]
enabled = true
path = "./data/memory.db"
auto_save_interval_sec = 300
```

### MCP (Model Context Protocol) Configuration

```toml
[mcp]
enabled = true
timeout_sec = 30
auto_discovery = true

# MCP server configurations
[[mcp.servers]]
name = "example-server"
url = "ws://localhost:3000"
auth_token = "optional-token"
capabilities = ["tools", "resources"]
```

### Plugin Configuration

```toml
[plugins]
enabled = true
plugin_dir = "./plugins"
auto_load = ["git", "web", "file"]
sandbox_enabled = true
```

### Logging Configuration

```toml
[logging]
# Log level: trace, debug, info, warn, error, off
level = "info"

# File logging
file_enabled = true
file_path = "./logs/magray.log"

# Structured logging (JSON format)
structured = false

# Maximum log file size in MB
max_size_mb = 100
```

### Paths Configuration

```toml
[paths]
data_dir = "./data"
cache_dir = "./cache"
models_dir = "./models"
logs_dir = "./logs"
```

### Performance Configuration

```toml
[performance]
worker_threads = 4
max_concurrent_requests = 10
enable_gpu = false
memory_limit_mb = 1024
```

## Environment Variables

All configuration parameters can be overridden using environment variables with the `MAGRAY_` prefix:

### AI Configuration
- `MAGRAY_AI_PROVIDER` - Default AI provider
- `MAGRAY_OPENAI_API_KEY` - OpenAI API key
- `MAGRAY_ANTHROPIC_API_KEY` - Anthropic API key
- `MAGRAY_GOOGLE_API_KEY` - Google AI API key

### Memory Configuration
- `MAGRAY_MEMORY_BACKEND` - Memory backend (sqlite, inmemory, hybrid)
- `MAGRAY_CACHE_SIZE_MB` - Cache size in MB

### Logging Configuration
- `MAGRAY_LOG_LEVEL` - Log level
- `MAGRAY_LOG_FILE` - Log file path

### Performance Configuration
- `MAGRAY_WORKER_THREADS` - Number of worker threads
- `MAGRAY_ENABLE_GPU` - Enable GPU acceleration (true/false)

### Path Configuration
- `MAGRAY_DATA_DIR` - Data directory
- `MAGRAY_CACHE_DIR` - Cache directory
- `MAGRAY_MODELS_DIR` - Models directory
- `MAGRAY_LOGS_DIR` - Logs directory

## Example Configurations

### Minimal Configuration

```toml
[ai]
default_provider = "openai"

[ai.providers.openai]
api_key = "sk-..."
```

### Development Configuration

```toml
[ai]
default_provider = "local"

[memory]
backend = "inmemory"

[logging]
level = "debug"
file_enabled = true

[performance]
worker_threads = 2
```

### Production Configuration

```toml
[ai]
default_provider = "openai"
fallback_chain = ["openai", "anthropic", "azure"]

[memory]
backend = "hybrid"
cache_size_mb = 1024

[memory.hnsw]
ef_search = 100
max_elements = 10000000

[memory.persistence]
enabled = true
auto_save_interval_sec = 60

[logging]
level = "info"
structured = true
file_enabled = true
max_size_mb = 500

[performance]
worker_threads = 8
max_concurrent_requests = 50
enable_gpu = true
memory_limit_mb = 4096
```

## Troubleshooting

### Configuration Not Loading

1. Check file permissions
2. Verify file format (TOML/JSON syntax)
3. Run `magray config validate` to check for errors
4. Use `magray config show --resolved` to see what's loaded

### Environment Variables Not Working

1. Ensure variables are exported: `export MAGRAY_AI_PROVIDER=openai`
2. Check variable names (must start with `MAGRAY_`)
3. Verify value format (some values need specific formats)

### API Keys Not Found

1. Check environment variables first
2. Verify configuration file contains the keys
3. Ensure keys are in the correct provider section
4. Use `magray config show` to see loaded configuration (keys are masked)

## Security Best Practices

1. **Never commit API keys** to version control
2. Use environment variables for sensitive data
3. Set appropriate file permissions on config files (e.g., `chmod 600 .magrayrc`)
4. Use different configurations for development and production
5. Rotate API keys regularly
6. Consider using secret management tools for production

## Migration Guide

### From Environment Variables Only

1. Generate a configuration file: `magray config generate`
2. Move environment variables to the config file
3. Keep sensitive data in environment variables

### From Other Tools

1. Create a new configuration: `magray config init`
2. Map your existing settings to MAGRAY parameters
3. Validate the configuration: `magray config validate`
4. Test with a simple command: `magray chat "Hello"`