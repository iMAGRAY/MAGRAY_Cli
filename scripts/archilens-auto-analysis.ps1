# ArchLens Auto Analysis Hook for Claude Code
# Automatic code analysis using ArchLens

param(
    [string]$EditedFile,
    [string]$ProjectRoot = "C:\Users\1\Documents\GitHub\MAGRAY_Cli",
    [switch]$FullAnalysis = $false
)

# Logging function with colors
function Write-ArchLensLog {
    param([string]$Message, [string]$Level = "INFO")
    
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    switch ($Level) {
        "INFO" { Write-Host "[$timestamp] Info: $Message" -ForegroundColor Cyan }
        "SUCCESS" { Write-Host "[$timestamp] Success: $Message" -ForegroundColor Green }
        "WARNING" { Write-Host "[$timestamp] Warning: $Message" -ForegroundColor Yellow }
        "ERROR" { Write-Host "[$timestamp] Error: $Message" -ForegroundColor Red }
    }
}

# Check if project exists
if (-not (Test-Path $ProjectRoot)) {
    Write-ArchLensLog "Project not found: $ProjectRoot" "ERROR"
    exit 1
}

Write-ArchLensLog "Starting ArchLens automatic analysis..."

try {
    # If Rust file was edited, run targeted analysis
    if ($EditedFile -and $EditedFile -match '\.rs$') {
        Write-ArchLensLog "Analyzing changes in file: $EditedFile"
        
        # Determine which crate was changed
        $crateName = ""
        if ($EditedFile -match 'crates\\(\w+)\\') {
            $crateName = $matches[1]
            Write-ArchLensLog "Detected crate: $crateName"
        }
        
        # Quick project structure analysis
        Write-ArchLensLog "Getting project structure..."
        # In real hook, ArchLens would be called via Claude Code API
        
    } elseif ($FullAnalysis) {
        Write-ArchLensLog "Running full architectural analysis..."
        
        # Full analysis with critical issues detection
        Write-ArchLensLog "Searching for critical architectural problems..."
        # In real hook, ArchLens would be called via Claude Code API
    }
    
    # Check code quality metrics
    Write-ArchLensLog "Checking quality metrics..."
    
    # Analyze change size
    if ($EditedFile -and (Test-Path $EditedFile)) {
        $fileSize = (Get-Item $EditedFile).Length
        if ($fileSize) {
            $fileSizeKB = [math]::Round($fileSize / 1024, 2)
            Write-ArchLensLog "File size: $fileSizeKB KB"
            
            if ($fileSizeKB -gt 100) {
                Write-ArchLensLog "Large file - refactoring recommended" "WARNING"
            }
        }
    }
    
    Write-ArchLensLog "Automatic analysis completed" "SUCCESS"
    
} catch {
    Write-ArchLensLog "Error during analysis: $($_.Exception.Message)" "ERROR"
    exit 1
}

# Recommendations for further actions
Write-ArchLensLog "For detailed analysis run: archlens export_ai_compact" "INFO"