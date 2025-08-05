#!/usr/bin/env pwsh
# Fix merge conflicts script

param(
    [string]$Path = ".",
    [switch]$DryRun = $false
)

$ErrorActionPreference = "Stop"

Write-Host "Searching and fixing merge conflicts..." -ForegroundColor Cyan

# Find all files with conflicts
$conflictFiles = Get-ChildItem -Path $Path -Include "*.rs","*.md","*.toml","*.yml","*.yaml" -Recurse | 
    Where-Object { 
        $content = Get-Content $_ -Raw -ErrorAction SilentlyContinue
        $content -match '<<<<<<< HEAD|=======|>>>>>>> '
    }

if ($conflictFiles.Count -eq 0) {
    Write-Host "No merge conflicts found!" -ForegroundColor Green
    exit 0
}

Write-Host "Found files with conflicts: $($conflictFiles.Count)" -ForegroundColor Yellow

foreach ($file in $conflictFiles) {
    Write-Host "`nProcessing: $($file.FullName)" -ForegroundColor Cyan
    
    $content = Get-Content $file -Raw
    $originalContent = $content
    
    # Pattern to find merge conflicts
    $conflictPattern = '(?s)<<<<<<< HEAD\r?\n(.*?)\r?\n=======\r?\n(.*?)\r?\n>>>>>>> [^\r\n]+'
    
    # Count conflicts
    $conflicts = [regex]::Matches($content, $conflictPattern)
    Write-Host "   Conflicts in file: $($conflicts.Count)" -ForegroundColor Yellow
    
    if (-not $DryRun) {
        # Resolution strategy: choose HEAD version (current branch)
        $resolvedContent = $content -replace $conflictPattern, '$1'
        
        # Save backup
        $backupPath = "$($file.FullName).backup"
        $originalContent | Set-Content -Path $backupPath -Encoding UTF8
        
        # Write fixed file
        $resolvedContent | Set-Content -Path $file.FullName -Encoding UTF8 -NoNewline
        
        Write-Host "   Conflicts resolved (HEAD version selected)" -ForegroundColor Green
        Write-Host "   Backup saved: $backupPath" -ForegroundColor Gray
    } else {
        Write-Host "   DRY RUN: file would be modified" -ForegroundColor Magenta
    }
}

if ($DryRun) {
    Write-Host "`nDRY RUN completed. Use without -DryRun to apply changes" -ForegroundColor Yellow
} else {
    Write-Host "`nAll conflicts resolved!" -ForegroundColor Green
    Write-Host "Check changes with: git diff" -ForegroundColor Cyan
}