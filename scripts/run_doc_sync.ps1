# Run doc sync from project root
Write-Host "Running documentation sync from project root..." -ForegroundColor Blue

# Save current location
$originalLocation = Get-Location

try {
    # Always run from project root
    Set-Location $PSScriptRoot
    
    # Run the doc sync daemon
    Set-Location docs-daemon
    cargo run --bin doc_sync_daemon -- once
}
finally {
    # Return to original location
    Set-Location $originalLocation
}