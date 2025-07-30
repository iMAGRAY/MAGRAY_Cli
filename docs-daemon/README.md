# Documentation Sync Daemon

Automatically syncs CTL v2.0 component annotations from code to CLAUDE.md.

## Implementation

High-performance Rust implementation for efficient file watching and incremental updates.

## Files

- `src/main.rs` - Main Rust implementation
- `Cargo.toml` - Rust dependencies
- `cache.json` - File hash cache (auto-generated)
- `sync_daemon.ps1` - PowerShell alternative (backup)

## Usage

### Build
```bash
cd docs-daemon
cargo build --release
```

### One-time sync
```bash
./target/release/ctl-sync
# or
cargo run --release
```

### Watch mode (continuous sync)
```bash
./target/release/ctl-sync watch
# or
cargo run --release -- watch
```

## How it works

1. Scans `crates/` for `*.rs` files
2. Looks for CTL annotations: `// @component: {"k":"C","id":"..."}`
3. Caches file hashes to detect changes
4. Updates only changed components in CLAUDE.md
5. Preserves manual edits outside auto-generated sections

## Component Annotation Format

Add to your Rust code:
```rust
// @component: {"k":"C","id":"vector_store","t":"Vector storage","m":{"cur":65,"tgt":100,"u":"%"}}
pub struct VectorStore {
    // ...
}
```

## Features

- **High performance** - Rust implementation for speed
- **Incremental updates** - Only processes changed files
- **SHA256 hashing** - Reliable change detection
- **Watch mode** - Efficient file system monitoring
- **Debouncing** - Smart event batching
- **Non-destructive** - Preserves manual CLAUDE.md edits
- **Cross-platform** - Works on Windows/Linux/macOS