#!/usr/bin/env pwsh
# Run CTL sync daemon in watch mode

Write-Host "Starting CTL v2.0 Sync Daemon in watch mode..." -ForegroundColor Cyan

# Build if needed
if (-not (Test-Path "docs-daemon/target/release/ctl-sync.exe")) {
    Write-Host "Building sync daemon..." -ForegroundColor Yellow
    Push-Location docs-daemon
    cargo build --release
    Pop-Location
}

# Run in watch mode
Push-Location docs-daemon
./target/release/ctl-sync.exe watch
Pop-Location