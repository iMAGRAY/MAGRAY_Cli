# Download ONNX Runtime GPU 1.22.0 from PyPI

$ErrorActionPreference = "Stop"

# File already downloaded via pip download
$outputFile = "onnxruntime_gpu-1.22.0-cp312-cp312-win_amd64.whl"
$extractDir = "onnxruntime_gpu_extracted"

Write-Host "Downloading ONNX Runtime GPU 1.22.0..." -ForegroundColor Cyan
Write-Host "This is a 214MB file, please wait..." -ForegroundColor Yellow

# Check if file exists
if (-not (Test-Path $outputFile)) {
    Write-Host "ERROR: File not found: $outputFile" -ForegroundColor Red
    Write-Host "Please run: py -m pip download onnxruntime-gpu==1.22.0" -ForegroundColor Yellow
    exit 1
} else {
    Write-Host "Using existing file: $outputFile" -ForegroundColor Green
}

# Extract wheel file (it's a zip)
Write-Host ""
Write-Host "Extracting wheel file..." -ForegroundColor Cyan

if (Test-Path $extractDir) {
    Remove-Item -Path $extractDir -Recurse -Force
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::ExtractToDirectory($outputFile, $extractDir)

Write-Host "Extracted to: $extractDir" -ForegroundColor Green

# Find DLL files
Write-Host ""
Write-Host "Looking for DLL files..." -ForegroundColor Cyan

$dllPath = Join-Path $extractDir "onnxruntime/capi"
if (Test-Path $dllPath) {
    $dlls = Get-ChildItem -Path $dllPath -Filter "*.dll"
    Write-Host "Found $($dlls.Count) DLL files:" -ForegroundColor Green
    $dlls | ForEach-Object { Write-Host "  - $($_.Name)" -ForegroundColor Gray }
    
    # Copy to onnxruntime/lib
    $targetDir = ".\onnxruntime\lib"
    if (-not (Test-Path $targetDir)) {
        New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
    }
    
    Write-Host ""
    Write-Host "Copying DLLs to $targetDir..." -ForegroundColor Cyan
    
    # Backup existing files
    $backupDir = ".\onnxruntime\lib_backup_$(Get-Date -Format 'yyyyMMdd_HHmmss')"
    if (Test-Path "$targetDir\*.dll") {
        Write-Host "Backing up existing DLLs to $backupDir" -ForegroundColor Yellow
        New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
        Copy-Item -Path "$targetDir\*.dll" -Destination $backupDir -Force
    }
    
    # Copy new files
    $dlls | Copy-Item -Destination $targetDir -Force
    Write-Host "DLL files copied successfully!" -ForegroundColor Green
    
    # Check for CUDA providers
    $cudaDll = Join-Path $targetDir "onnxruntime_providers_cuda.dll"
    if (Test-Path $cudaDll) {
        Write-Host ""
        Write-Host "CUDA provider found! GPU acceleration should work." -ForegroundColor Green
    } else {
        Write-Host ""
        Write-Host "WARNING: CUDA provider DLL not found!" -ForegroundColor Yellow
    }
} else {
    Write-Host "ERROR: Path $dllPath not found" -ForegroundColor Red
    Write-Host "Checking alternative locations..." -ForegroundColor Yellow
    
    # List all directories in extracted folder
    Get-ChildItem -Path $extractDir -Directory -Recurse | ForEach-Object {
        Write-Host "  - $($_.FullName)" -ForegroundColor Gray
    }
}

Write-Host ""
Write-Host "Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Run: .\setup_ort_env.bat" -ForegroundColor White
Write-Host "2. Rebuild with: cargo build --features gpu --release" -ForegroundColor White
Write-Host "3. Test GPU acceleration" -ForegroundColor White