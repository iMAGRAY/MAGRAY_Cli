#!/usr/bin/env pwsh
# Full CTL System Integration Script

Write-Host "🚀 MAGRAY CLI - Full CTL System Sync" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan

$ErrorActionPreference = "Continue"

# 1. Build sync daemon
Write-Host "`n1. Building CTL sync daemon..." -ForegroundColor Yellow
Push-Location docs-daemon
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Daemon build failed" -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location

# 2. Generate automatic metrics  
Write-Host "`n2. Generating project metrics..." -ForegroundColor Yellow
if (Get-Command python3 -ErrorAction SilentlyContinue) {
    python3 scripts/generate_ctl_metrics.py
} elseif (Get-Command python -ErrorAction SilentlyContinue) {
    python scripts/generate_ctl_metrics.py
} else {
    Write-Host "⚠️ Python not found, skipping metrics generation" -ForegroundColor Orange
}

# 3. Sync CTL annotations
Write-Host "`n3. Syncing CTL annotations..." -ForegroundColor Yellow
Push-Location docs-daemon
Remove-Item cache.json -ErrorAction SilentlyContinue
./target/release/ctl-sync.exe
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ CTL sync failed" -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location

# 4. Show results
Write-Host "`n4. CTL System Status:" -ForegroundColor Green

# Count components
$claudeContent = Get-Content CLAUDE.md -Raw
$componentCount = ([regex]::Matches($claudeContent, '"k":"C"')).Count
$metricCount = ([regex]::Matches($claudeContent, '"k":"M"')).Count  
$taskCount = ([regex]::Matches($claudeContent, '"k":"T"')).Count

Write-Host "   📊 Components: $componentCount" -ForegroundColor White
Write-Host "   📈 Metrics: $metricCount" -ForegroundColor White
Write-Host "   📋 Tasks: $taskCount" -ForegroundColor White

# Show latest update time
$timestamp = $claudeContent | Select-String "Last updated: (.+)" | ForEach-Object { $_.Matches.Groups[1].Value }
if ($timestamp) {
    Write-Host "   🕐 Last update: $timestamp" -ForegroundColor White
}

Write-Host "`n✅ CTL system fully synchronized!" -ForegroundColor Green
Write-Host "`nNext steps:" -ForegroundColor Cyan
Write-Host "  - Run './docs-daemon/target/release/ctl-sync.exe watch' for continuous sync" -ForegroundColor Gray
Write-Host "  - Add more @component annotations to expand coverage" -ForegroundColor Gray
Write-Host "  - Review generated metrics in .ctl/auto_generated.jsonl" -ForegroundColor Gray