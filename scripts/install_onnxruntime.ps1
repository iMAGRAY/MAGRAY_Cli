# ONNX Runtime 1.22.0 Installation Script for Windows
# This script downloads and installs ONNX Runtime 1.22.0 required by ort 2.0.0-rc.4

param(
    [string]$InstallPath = ".\onnxruntime",
    [switch]$AddToPath = $false,
    [switch]$Force = $false
)

$ErrorActionPreference = "Stop"

# Configuration
$ONNX_VERSION = "1.22.0"
$DOWNLOAD_URL = "https://github.com/microsoft/onnxruntime/releases/download/v$ONNX_VERSION/onnxruntime-win-x64-$ONNX_VERSION.zip"
$TEMP_ZIP = "$env:TEMP\onnxruntime-$ONNX_VERSION.zip"

Write-Host "ONNX Runtime $ONNX_VERSION Installation Script" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

# Check if already installed
if (Test-Path $InstallPath) {
    if ($Force) {
        Write-Host "Removing existing installation at $InstallPath" -ForegroundColor Yellow
        Remove-Item -Path $InstallPath -Recurse -Force
    } else {
        Write-Host "ONNX Runtime already installed at $InstallPath" -ForegroundColor Green
        Write-Host "Use -Force to reinstall" -ForegroundColor Yellow
        exit 0
    }
}

# Create installation directory
Write-Host "`nCreating installation directory: $InstallPath" -ForegroundColor Gray
New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null

# Download ONNX Runtime
Write-Host "`nDownloading ONNX Runtime $ONNX_VERSION..." -ForegroundColor Gray
Write-Host "URL: $DOWNLOAD_URL" -ForegroundColor DarkGray

try {
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $TEMP_ZIP -UseBasicParsing
    $ProgressPreference = 'Continue'
} catch {
    Write-Host "Failed to download ONNX Runtime: $_" -ForegroundColor Red
    exit 1
}

# Extract archive
Write-Host "`nExtracting ONNX Runtime..." -ForegroundColor Gray
try {
    Expand-Archive -Path $TEMP_ZIP -DestinationPath $InstallPath -Force
    
    # Move files from nested directory
    $extractedDir = Get-ChildItem -Path $InstallPath -Directory | Where-Object { $_.Name -like "onnxruntime-*" } | Select-Object -First 1
    if ($extractedDir) {
        Get-ChildItem -Path $extractedDir.FullName -Recurse | Move-Item -Destination $InstallPath -Force
        Remove-Item -Path $extractedDir.FullName -Recurse -Force
    }
} catch {
    Write-Host "Failed to extract ONNX Runtime: $_" -ForegroundColor Red
    exit 1
} finally {
    # Clean up
    if (Test-Path $TEMP_ZIP) {
        Remove-Item -Path $TEMP_ZIP -Force
    }
}

# Verify installation
$dllPath = Join-Path $InstallPath "lib\onnxruntime.dll"
if (Test-Path $dllPath) {
    Write-Host "`nONNX Runtime installed successfully!" -ForegroundColor Green
    Write-Host "DLL location: $dllPath" -ForegroundColor Gray
} else {
    Write-Host "`nError: onnxruntime.dll not found in expected location" -ForegroundColor Red
    exit 1
}

# Set environment variables
Write-Host "`nSetting environment variables..." -ForegroundColor Gray

# For current session
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
$batchFile = Join-Path (Get-Location) "setup_ort_env.bat"
@"
@echo off
echo Setting ONNX Runtime environment variables...
set ORT_DYLIB_PATH=$dllPath
set PATH=$libPath;%PATH%
echo Environment configured for ONNX Runtime $ONNX_VERSION
"@ | Out-File -FilePath $batchFile -Encoding ASCII

Write-Host "`nCreated setup_ort_env.bat for easy environment configuration" -ForegroundColor Gray

# Test with Rust
Write-Host "`nTesting ONNX Runtime with Rust..." -ForegroundColor Gray
$testCode = @'
fn main() {
    println!("ORT_DYLIB_PATH: {:?}", std::env::var("ORT_DYLIB_PATH"));
    match ort::init() {
        Ok(_) => println!("✓ ONNX Runtime initialized successfully!"),
        Err(e) => eprintln!("✗ Failed to initialize ONNX Runtime: {}", e),
    }
}
'@

$testDir = "$env:TEMP\ort_test_$([System.Guid]::NewGuid())"
New-Item -ItemType Directory -Path $testDir -Force | Out-Null

try {
    # Create test project
    Push-Location $testDir
    cargo init --name ort_test --quiet
    Add-Content -Path "Cargo.toml" -Value "`n[dependencies]`nort = `"2.0.0-rc.4`""
    $testCode | Out-File -FilePath "src\main.rs" -Encoding UTF8
    
    # Run test
    cargo run --quiet 2>&1 | ForEach-Object { Write-Host $_ }
} catch {
    Write-Host "Test failed: $_" -ForegroundColor Yellow
} finally {
    Pop-Location
    Remove-Item -Path $testDir -Recurse -Force -ErrorAction SilentlyContinue
}

# Instructions
Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host "Installation Complete!" -ForegroundColor Green
Write-Host "`nTo use ONNX Runtime in your current session:" -ForegroundColor Yellow
Write-Host "  1. Run: .\setup_ort_env.bat" -ForegroundColor White
Write-Host "  2. Or set manually:" -ForegroundColor White
Write-Host "     set ORT_DYLIB_PATH=$dllPath" -ForegroundColor DarkGray
Write-Host "     set PATH=$libPath;%PATH%" -ForegroundColor DarkGray

if (-not $AddToPath) {
    Write-Host "`nTo make changes permanent, run with -AddToPath flag" -ForegroundColor Yellow
}

Write-Host "`nYou can now use real ONNX models in MAGRAY CLI!" -ForegroundColor Green