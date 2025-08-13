# Test WASM Modules

This directory contains test WASM modules used for sandbox isolation testing.

## Files

- `safe_add.wasm` - Simple addition function for basic testing
- `malicious_fs.wasm` - WASM module that attempts filesystem escape
- `malicious_net.wasm` - WASM module that attempts network escape
- `resource_bomb.wasm` - WASM module that attempts resource exhaustion

## Usage

These modules are used by the sandbox isolation tests to verify that:
1. Safe modules execute correctly within sandbox
2. Malicious modules are properly contained or rejected
3. Resource exhaustion attempts are prevented
4. WASI security boundaries are respected

## Security Note

The malicious modules in this directory are designed for testing purposes only.
They contain harmless bytecode that simulates attack vectors without actually
performing any harmful operations.