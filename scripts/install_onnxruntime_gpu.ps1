# ONNX Runtime GPU 1.19.2 Installation Script for Windows
# Downloads ONNX Runtime with CUDA 12 support

param(
    [string]$InstallPath = ".\onnxruntime",
    [switch]$AddToPath = $false,
    [switch]$Force = $false
)

$ErrorActionPreference = "Stop"

# Configuration for GPU version with CUDA 12
$ONNX_VERSION = "1.19.2"
$DOWNLOAD_URL = "https://github.com/microsoft/onnxruntime/releases/download/v$ONNX_VERSION/onnxruntime-win-x64-gpu-$ONNX_VERSION.zip"
$TEMP_ZIP = "$env:TEMP\onnxruntime-gpu-$ONNX_VERSION.zip"

Write-Host "ONNX Runtime GPU $ONNX_VERSION Installation Script" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "This version includes CUDA 12 support" -ForegroundColor Yellow

# Check CUDA installation
Write-Host "`nChecking CUDA installation..." -ForegroundColor Gray
$nvccPath = Get-Command nvcc -ErrorAction SilentlyContinue
if ($nvccPath) {
    $cudaVersion = & nvcc --version | Select-String "release" | ForEach-Object { $_ -match "release (\d+\.\d+)" | Out-Null; $matches[1] }
    Write-Host "✓ CUDA $cudaVersion detected" -ForegroundColor Green
} else {
    Write-Host "⚠ CUDA compiler (nvcc) not found in PATH" -ForegroundColor Yellow
    Write-Host "  Make sure CUDA is installed and added to PATH" -ForegroundColor Yellow
}

# Check if already installed
if (Test-Path $InstallPath) {
    if ($Force) {
        Write-Host "`nBacking up existing installation..." -ForegroundColor Yellow
        $backupPath = "$InstallPath.backup.$(Get-Date -Format 'yyyyMMdd_HHmmss')"
        Move-Item -Path $InstallPath -Destination $backupPath -Force
        Write-Host "Backup created at: $backupPath" -ForegroundColor Gray
    } else {
        Write-Host "`nONNX Runtime already installed at $InstallPath" -ForegroundColor Green
        Write-Host "Use -Force to reinstall" -ForegroundColor Yellow
        exit 0
    }
}

# Create installation directory
Write-Host "`nCreating installation directory: $InstallPath" -ForegroundColor Gray
New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null

# Download ONNX Runtime GPU
Write-Host "`nDownloading ONNX Runtime GPU $ONNX_VERSION..." -ForegroundColor Gray
Write-Host "URL: $DOWNLOAD_URL" -ForegroundColor DarkGray
Write-Host "This may take a while (GPU version is ~500MB)..." -ForegroundColor Yellow

try {
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $TEMP_ZIP -UseBasicParsing
    $ProgressPreference = 'Continue'
    
    $fileSize = (Get-Item $TEMP_ZIP).Length / 1MB
    Write-Host "Downloaded: $([math]::Round($fileSize, 2)) MB" -ForegroundColor Gray
} catch {
    Write-Host "Failed to download ONNX Runtime GPU: $_" -ForegroundColor Red
    Write-Host "`nTrying alternative download method..." -ForegroundColor Yellow
    
    # Alternative download using System.Net.WebClient
    try {
        $webClient = New-Object System.Net.WebClient
        $webClient.DownloadFile($DOWNLOAD_URL, $TEMP_ZIP)
    } catch {
        Write-Host "Alternative download also failed: $_" -ForegroundColor Red
        exit 1
    }
}

# Extract archive
Write-Host "`nExtracting ONNX Runtime GPU..." -ForegroundColor Gray
try {
    # Use .NET for extraction (more reliable for large files)
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory($TEMP_ZIP, $InstallPath)
    
    # Move files from nested directory
    $extractedDir = Get-ChildItem -Path $InstallPath -Directory | Where-Object { $_.Name -like "onnxruntime-*" } | Select-Object -First 1
    if ($extractedDir) {
        Write-Host "Moving files from $($extractedDir.Name)..." -ForegroundColor Gray
        Get-ChildItem -Path $extractedDir.FullName -Recurse | Move-Item -Destination $InstallPath -Force
        Remove-Item -Path $extractedDir.FullName -Recurse -Force
    }
} catch {
    Write-Host "Failed to extract ONNX Runtime GPU: $_" -ForegroundColor Red
    exit 1
} finally {
    # Clean up
    if (Test-Path $TEMP_ZIP) {
        Remove-Item -Path $TEMP_ZIP -Force
    }
}

# Verify installation
$requiredFiles = @(
    "lib\onnxruntime.dll",
    "lib\onnxruntime_providers_cuda.dll",
    "lib\onnxruntime_providers_shared.dll"
)

$allFilesFound = $true
foreach ($file in $requiredFiles) {
    $filePath = Join-Path $InstallPath $file
    if (Test-Path $filePath) {
        Write-Host "✓ Found: $file" -ForegroundColor Green
    } else {
        Write-Host "✗ Missing: $file" -ForegroundColor Red
        $allFilesFound = $false
    }
}

if (-not $allFilesFound) {
    Write-Host "`nError: Some required files are missing" -ForegroundColor Red
    exit 1
}

Write-Host "`nONNX Runtime GPU installed successfully!" -ForegroundColor Green

# Set environment variables
Write-Host "`nSetting environment variables..." -ForegroundColor Gray

# For current session
$dllPath = Join-Path $InstallPath "lib\onnxruntime.dll"
$env:ORT_DYLIB_PATH = $dllPath
$libPath = Join-Path $InstallPath "lib"
$env:PATH = "$libPath;$env:PATH"

Write-Host "ORT_DYLIB_PATH set to: $env:ORT_DYLIB_PATH" -ForegroundColor Gray

# For persistent environment variables
if ($AddToPath) {
    Write-Host "`nAdding to system PATH..." -ForegroundColor Gray
    
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$libPath*") {
        [Environment]::SetEnvironmentVariable("Path", "$libPath;$currentPath", "User")
        Write-Host "Added $libPath to user PATH" -ForegroundColor Green
    }
    
    [Environment]::SetEnvironmentVariable("ORT_DYLIB_PATH", $dllPath, "User")
    Write-Host "Set ORT_DYLIB_PATH in user environment" -ForegroundColor Green
}

# Create a batch file for easy environment setup
$batchFile = Join-Path (Get-Location) "setup_ort_gpu_env.bat"
@"
@echo off
echo Setting ONNX Runtime GPU environment variables...
set ORT_DYLIB_PATH=$dllPath
set PATH=$libPath;%PATH%
echo Environment configured for ONNX Runtime GPU $ONNX_VERSION
echo.
echo CUDA providers available:
echo - CUDA Execution Provider
echo - TensorRT Execution Provider (if TensorRT is installed)
"@ | Out-File -FilePath $batchFile -Encoding ASCII

Write-Host "`nCreated setup_ort_gpu_env.bat for easy environment configuration" -ForegroundColor Gray

# GPU info
Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host "GPU Support Information" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

# Check NVIDIA GPU
$gpuInfo = & nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader 2>$null
if ($gpuInfo) {
    Write-Host "`nDetected GPU:" -ForegroundColor Green
    $gpuInfo | ForEach-Object {
        $parts = $_ -split ', '
        Write-Host "  Name: $($parts[0])" -ForegroundColor Gray
        Write-Host "  Driver: $($parts[1])" -ForegroundColor Gray
        Write-Host "  Memory: $($parts[2])" -ForegroundColor Gray
    }
} else {
    Write-Host "`n⚠ No NVIDIA GPU detected or nvidia-smi not available" -ForegroundColor Yellow
}

# Instructions
Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host "Installation Complete!" -ForegroundColor Green
Write-Host "`nTo use ONNX Runtime GPU in your current session:" -ForegroundColor Yellow
Write-Host "  1. Run: .\setup_ort_gpu_env.bat" -ForegroundColor White
Write-Host "  2. Rebuild your Rust project with GPU feature:" -ForegroundColor White
Write-Host "     cargo build --features gpu --release" -ForegroundColor DarkGray

if (-not $AddToPath) {
    Write-Host "`nTo make changes permanent, run with -AddToPath flag" -ForegroundColor Yellow
}

Write-Host "`nIMPORTANT:" -ForegroundColor Red
Write-Host "- Make sure CUDA 12.x is installed" -ForegroundColor Yellow
Write-Host "- For best performance, install cuDNN 9.x" -ForegroundColor Yellow
Write-Host "- TensorRT support requires separate TensorRT installation" -ForegroundColor Yellow

Write-Host "`nYou can now use GPU acceleration in MAGRAY CLI!" -ForegroundColor Green