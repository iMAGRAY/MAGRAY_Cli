# Alternative cuDNN downloader for ONNX Runtime
# Downloads cuDNN from PyTorch wheel package which includes cuDNN

$ErrorActionPreference = "Stop"

Write-Host "Alternative cuDNN Downloader" -ForegroundColor Cyan
Write-Host "===========================" -ForegroundColor Cyan
Write-Host "This script downloads cuDNN from PyTorch packages" -ForegroundColor Yellow

# Check if pip is available
$pipPath = Get-Command pip -ErrorAction SilentlyContinue
if (-not $pipPath) {
    $pipPath = Get-Command py -ErrorAction SilentlyContinue
    if ($pipPath) {
        $pipCmd = "py -m pip"
    } else {
        Write-Host "ERROR: pip not found. Please install Python." -ForegroundColor Red
        exit 1
    }
} else {
    $pipCmd = "pip"
}

Write-Host "`nMethod 1: Download from PyTorch (includes cuDNN)" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan

# Download PyTorch with CUDA (includes cuDNN)
$torchUrl = "https://download.pytorch.org/whl/cu124/torch-2.5.1%2Bcu124-cp312-cp312-win_amd64.whl"
$torchFile = "torch-2.5.1+cu124-cp312-cp312-win_amd64.whl"

if (-not (Test-Path $torchFile)) {
    Write-Host "`nDownloading PyTorch with CUDA 12.4 (includes cuDNN)..." -ForegroundColor Yellow
    Write-Host "This is a large file (~2.4GB), please wait..." -ForegroundColor Yellow
    
    try {
        $webClient = New-Object System.Net.WebClient
        $webClient.DownloadFile($torchUrl, $torchFile)
        Write-Host "Downloaded: $torchFile" -ForegroundColor Green
    } catch {
        Write-Host "Download failed. Trying alternative method..." -ForegroundColor Yellow
        Invoke-WebRequest -Uri $torchUrl -OutFile $torchFile -UseBasicParsing
    }
} else {
    Write-Host "Using existing file: $torchFile" -ForegroundColor Green
}

# Extract wheel
Write-Host "`nExtracting PyTorch wheel..." -ForegroundColor Yellow
$extractDir = "torch_extracted"
if (Test-Path $extractDir) {
    Remove-Item -Path $extractDir -Recurse -Force
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::ExtractToDirectory($torchFile, $extractDir)

# Find cuDNN DLLs
Write-Host "`nSearching for cuDNN files..." -ForegroundColor Yellow
$cudnnFiles = Get-ChildItem -Path $extractDir -Recurse -Filter "cudnn*.dll" | Where-Object { $_.Name -like "cudnn*64*.dll" }

if ($cudnnFiles.Count -gt 0) {
    Write-Host "Found cuDNN files:" -ForegroundColor Green
    $cudnnFiles | ForEach-Object { Write-Host "  - $($_.Name)" -ForegroundColor Gray }
    
    # Copy to onnxruntime/lib
    $targetDir = "..\onnxruntime\lib"
    if (-not (Test-Path $targetDir)) {
        New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
    }
    
    Write-Host "`nCopying cuDNN files to $targetDir..." -ForegroundColor Yellow
    $cudnnFiles | Copy-Item -Destination $targetDir -Force
    
    # Also copy to CUDA bin if available
    $cudaPath = $env:CUDA_PATH
    if ($cudaPath) {
        $cudaBin = Join-Path $cudaPath "bin"
        if (Test-Path $cudaBin) {
            Write-Host "`nCopying cuDNN files to CUDA bin..." -ForegroundColor Yellow
            $cudnnFiles | Copy-Item -Destination $cudaBin -Force
        }
    }
    
    Write-Host "`ncuDNN files installed successfully!" -ForegroundColor Green
} else {
    Write-Host "No cuDNN files found in PyTorch package" -ForegroundColor Red
}

# Cleanup
Write-Host "`nCleaning up..." -ForegroundColor Yellow
Remove-Item -Path $extractDir -Recurse -Force

Write-Host "`nMethod 2: Direct cuDNN download URLs" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan
Write-Host "If Method 1 didn't work, you can download cuDNN directly:" -ForegroundColor Yellow
Write-Host "" -ForegroundColor White
Write-Host "1. cuDNN 9.5.1 for CUDA 12:" -ForegroundColor White
Write-Host "   https://developer.nvidia.com/cudnn-downloads" -ForegroundColor Gray
Write-Host "" -ForegroundColor White
Write-Host "2. Or use older cuDNN 8.9.7 (doesn't require login):" -ForegroundColor White
Write-Host "   https://github.com/phohenecker/install-cudnn-windows/releases/download/cudnn-12.2-8.9.7/cudnn-12.2-windows-8.9.7.29.zip" -ForegroundColor Gray

Write-Host "`nDone!" -ForegroundColor Green