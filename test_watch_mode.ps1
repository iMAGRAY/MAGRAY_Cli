#!/usr/bin/env pwsh
# Test script for watch mode

Write-Host "Testing CTL sync daemon watch mode..." -ForegroundColor Cyan

# Start daemon in background
Push-Location docs-daemon
$daemonJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    ./target/release/ctl-sync.exe watch
}

Start-Sleep 2

# Make a small change to trigger update
Write-Host "Making test change..." -ForegroundColor Yellow
$testFile = "../crates/cli/src/agent.rs"
$content = Get-Content $testFile -Raw
$content = $content -replace "Main agent orchestrator", "Main agent orchestrator (test)"
Set-Content $testFile $content

Start-Sleep 3

# Revert change
Write-Host "Reverting test change..." -ForegroundColor Yellow  
$content = $content -replace "Main agent orchestrator \(test\)", "Main agent orchestrator"
Set-Content $testFile $content

Start-Sleep 2

# Stop daemon
Write-Host "Stopping daemon..." -ForegroundColor Yellow
Stop-Job $daemonJob
Remove-Job $daemonJob

Pop-Location

Write-Host "Test complete! Check CLAUDE.md for updates." -ForegroundColor Green