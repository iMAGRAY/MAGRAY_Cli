# PowerShell script to download ONNX Runtime GPU with CUDA/TensorRT support
$ErrorActionPreference = "Stop"

Write-Host "Loading ONNX Runtime GPU v1.22.0 for Windows x64..." -ForegroundColor Green

# ONNX Runtime GPU version
$version = "1.22.0"
$gpuUrl = "https://github.com/microsoft/onnxruntime/releases/download/v$version/onnxruntime-win-x64-gpu-$version.zip"
$tempFile = "onnxruntime-gpu.zip"
$targetDir = ".\onnxruntime-gpu"

# Remove old version if exists
if (Test-Path $targetDir) {
    Write-Host "Removing old version..." -ForegroundColor Yellow
    Remove-Item -Path $targetDir -Recurse -Force
}

if (Test-Path $tempFile) {
    Remove-Item -Path $tempFile -Force
}

# Create directory if not exists
if (!(Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir | Out-Null
}

# Download GPU version with CUDA/TensorRT support
Write-Host "Downloading GPU version with CUDA/TensorRT support..." -ForegroundColor Yellow
Write-Host "URL: $gpuUrl" -ForegroundColor Cyan

try {
    Invoke-WebRequest -Uri $gpuUrl -OutFile $tempFile -UseBasicParsing
    Write-Host "Download completed successfully" -ForegroundColor Green
} catch {
    Write-Host "Error downloading GPU version: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Trying fallback source..." -ForegroundColor Yellow
    
    # Try fallback URL (sometimes GPU builds are in different location)
    $fallbackUrl = "https://github.com/microsoft/onnxruntime/releases/download/v$version/onnxruntime-win-x64-$version.zip"
    Write-Host "Fallback URL: $fallbackUrl" -ForegroundColor Cyan
    
    try {
        Invoke-WebRequest -Uri $fallbackUrl -OutFile $tempFile -UseBasicParsing
        Write-Host "Fallback download completed" -ForegroundColor Green
    } catch {
        Write-Host "Failed to download both GPU and CPU versions" -ForegroundColor Red
        exit 1
    }
}

# Extract archive
Write-Host "Extracting archive..." -ForegroundColor Yellow
Expand-Archive -Path $tempFile -DestinationPath "temp_ort_gpu" -Force

# Copy files
$sourceDir = Get-ChildItem -Path "temp_ort_gpu" -Directory | Select-Object -First 1
if ($sourceDir) {
    Write-Host "Copying files..." -ForegroundColor Yellow
    
    # Copy lib files
    $libSource = Join-Path $sourceDir.FullName "lib"
    $libTarget = Join-Path $targetDir "lib"
    if (!(Test-Path $libTarget)) {
        New-Item -ItemType Directory -Path $libTarget | Out-Null
    }
    Copy-Item -Path "$libSource\*" -Destination $libTarget -Force -Recurse
    
    # Copy include files if exists
    $includeSource = Join-Path $sourceDir.FullName "include"
    if (Test-Path $includeSource) {
        $includeTarget = Join-Path $targetDir "include"
        if (!(Test-Path $includeTarget)) {
            New-Item -ItemType Directory -Path $includeTarget | Out-Null
        }
        Copy-Item -Path "$includeSource\*" -Destination $includeTarget -Force -Recurse
    }
    
    # Copy version info and license
    Copy-Item -Path "$($sourceDir.FullName)\VERSION_NUMBER" -Destination $targetDir -Force -ErrorAction SilentlyContinue
    Copy-Item -Path "$($sourceDir.FullName)\LICENSE" -Destination $targetDir -Force -ErrorAction SilentlyContinue
    Copy-Item -Path "$($sourceDir.FullName)\README.md" -Destination $targetDir -Force -ErrorAction SilentlyContinue
}

# Clean temp files
Write-Host "Cleaning temp files..." -ForegroundColor Yellow
Remove-Item -Path $tempFile -Force -ErrorAction SilentlyContinue
Remove-Item -Path "temp_ort_gpu" -Recurse -Force -ErrorAction SilentlyContinue

# Check for GPU support files
$dllPath = Join-Path $targetDir "lib\onnxruntime.dll"
if (Test-Path $dllPath) {
    Write-Host ""
    Write-Host "ONNX Runtime GPU installed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Installed to: $targetDir" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Set environment variable:" -ForegroundColor Yellow
    Write-Host "   set ORT_DYLIB_PATH=$pwd\onnxruntime-gpu\lib\onnxruntime.dll" -ForegroundColor White
    Write-Host ""
    Write-Host "Or for PowerShell:" -ForegroundColor Yellow
    Write-Host "   `$env:ORT_DYLIB_PATH = `"$pwd\onnxruntime-gpu\lib\onnxruntime.dll`"" -ForegroundColor White
    Write-Host ""
    
    # Check for GPU-specific files
    $cudaProviderPath = Join-Path $targetDir "lib\onnxruntime_providers_cuda.dll"
    $tensorrtProviderPath = Join-Path $targetDir "lib\onnxruntime_providers_tensorrt.dll"
    
    if (Test-Path $cudaProviderPath) {
        Write-Host "CUDA provider found: onnxruntime_providers_cuda.dll" -ForegroundColor Green
    } else {
        Write-Host "CUDA provider NOT found" -ForegroundColor Yellow
    }
    
    if (Test-Path $tensorrtProviderPath) {
        Write-Host "TensorRT provider found: onnxruntime_providers_tensorrt.dll" -ForegroundColor Green
    } else {
        Write-Host "TensorRT provider NOT found" -ForegroundColor Yellow
    }
    
    Write-Host ""
    Write-Host "GPU support requirements:" -ForegroundColor Cyan
    Write-Host "   - CUDA 12.x (CUDA 11.x no longer supported)" -ForegroundColor White
    Write-Host "   - cuDNN 9.x" -ForegroundColor White
    Write-Host "   - TensorRT 10.8+ (optional)" -ForegroundColor White
    Write-Host "   - Visual C++ Redistributable 14.38+" -ForegroundColor White
    
} else {
    Write-Host "Error: onnxruntime.dll not found!" -ForegroundColor Red
    exit 1
}