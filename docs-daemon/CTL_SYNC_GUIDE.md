# CTL v2.0 Sync Daemon Guide

## Overview
High-performance Rust daemon that synchronizes CTL v2.0 annotations from code to CLAUDE.md.

## Key Features
- **Incremental updates** - Only processes changed files
- **SHA256 file hashing** - Reliable change detection  
- **Watch mode** - Real-time monitoring with debouncing
- **Non-destructive** - Preserves manual edits to CLAUDE.md
- **Cross-platform** - Works on Windows/Linux/macOS

## Quick Start

### Add CTL annotations to your code:
```rust
// @component: {"k":"C","id":"my_service","t":"Core service","m":{"cur":75,"tgt":100,"u":"%"}}
pub struct MyService {
    // ...
}
```

### Run sync:
```bash
# One-time sync
cd docs-daemon && cargo run --release

# Watch mode (continuous)
cd docs-daemon && cargo run --release -- watch

# Or use the PowerShell script
./run_ctl_sync.ps1
```

## How It Works

1. **Scans** all `*.rs` files in `crates/` directory
2. **Extracts** JSON annotations matching `// @component: {...}`
3. **Caches** file hashes to detect changes efficiently
4. **Updates** the AUTO-GENERATED ARCHITECTURE section in CLAUDE.md
5. **Preserves** all other content in CLAUDE.md

## CTL v2.0 Format

```json
{"k":"C","id":"unique_id","t":"Title","m":{"cur":50,"tgt":100,"u":"%"},"d":["dep1"],"f":["tag1"]}
```

- **k** - Kind (C=Component, T=Task, M=Metric, etc.)
- **id** - Unique identifier (snake_case, ≤32 chars)
- **t** - Title (≤40 chars)
- **m** - Metric object {cur, tgt, u}
- **d** - Dependencies array
- **f** - Flags/tags array
- **x_file** - Auto-added: source file location

## Performance

- Initial scan: ~100ms for 100 files
- Incremental updates: ~10ms per changed file
- Memory usage: <10MB
- Watch mode CPU: <1% when idle

## Troubleshooting

### No components found
- Check JSON syntax is valid
- Ensure annotation is on single line
- Verify regex pattern: `// @component: {...}`

### Changes not detected
- Delete `cache.json` to force full rescan
- Check file permissions
- Ensure not in `target/` directory

### Watch mode issues
- Increase debounce time if too sensitive
- Check filesystem events are supported
- Run with RUST_LOG=debug for more info