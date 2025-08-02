# Minimal CUDA libraries setup for ONNX Runtime GPU
# This script copies only essential CUDA DLLs for ONNX Runtime

$ErrorActionPreference = "Stop"

Write-Host "Minimal CUDA Setup for ONNX Runtime" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan

# Find CUDA installation
$cudaPath = $env:CUDA_PATH
if (-not $cudaPath) {
    $cudaPath = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
    if (-not (Test-Path $cudaPath)) {
        Write-Host "ERROR: CUDA not found at $cudaPath" -ForegroundColor Red
        exit 1
    }
}

Write-Host "CUDA Path: $cudaPath" -ForegroundColor Green

# ONNX Runtime lib directory
$ortLibPath = ".\onnxruntime\lib"
if (-not (Test-Path $ortLibPath)) {
    Write-Host "ERROR: ONNX Runtime lib directory not found at $ortLibPath" -ForegroundColor Red
    exit 1
}

# Essential CUDA libraries for ONNX Runtime
$essentialLibs = @(
    "cublas64_12.dll",
    "cublasLt64_12.dll",
    "cudart64_12.dll",
    "cufft64_11.dll",
    "curand64_10.dll",
    "cusparse64_12.dll",
    "cusolver64_11.dll",
    "cusolverMg64_11.dll"
)

Write-Host "`nCopying essential CUDA libraries..." -ForegroundColor Yellow

$cudaBinPath = Join-Path $cudaPath "bin"
$copiedCount = 0

foreach ($lib in $essentialLibs) {
    $sourcePath = Join-Path $cudaBinPath $lib
    if (Test-Path $sourcePath) {
        $targetPath = Join-Path $ortLibPath $lib
        Copy-Item -Path $sourcePath -Destination $targetPath -Force
        Write-Host "  + $lib" -ForegroundColor Green
        $copiedCount++
    } else {
        # Try without version suffix
        $libBase = $lib -replace '64_\d+', '64_*'
        $found = Get-ChildItem -Path $cudaBinPath -Filter $libBase | Select-Object -First 1
        if ($found) {
            $targetPath = Join-Path $ortLibPath $found.Name
            Copy-Item -Path $found.FullName -Destination $targetPath -Force
            Write-Host "  + $($found.Name)" -ForegroundColor Green
            $copiedCount++
        } else {
            Write-Host "  - $lib (not found)" -ForegroundColor Yellow
        }
    }
}

Write-Host "`nCopied $copiedCount essential CUDA libraries" -ForegroundColor Green

# Check if we can bypass cuDNN requirement
Write-Host "`nChecking ONNX Runtime GPU providers..." -ForegroundColor Yellow

$providers = @(
    "onnxruntime_providers_cuda.dll",
    "onnxruntime_providers_shared.dll"
)

foreach ($provider in $providers) {
    $providerPath = Join-Path $ortLibPath $provider
    if (Test-Path $providerPath) {
        Write-Host "  + $provider (found)" -ForegroundColor Green
    } else {
        Write-Host "  - $provider (missing)" -ForegroundColor Red
    }
}

# Create test script
$testScript = @'
# Test GPU with minimal setup
import os
os.add_dll_directory(r".\onnxruntime\lib")

try:
    import onnxruntime as ort
    print(f"ONNX Runtime version: {ort.__version__}")
    print(f"Available providers: {ort.get_available_providers()}")
    
    # Try to create session with CUDA
    providers = ['CUDAExecutionProvider', 'CPUExecutionProvider']
    print(f"\nTrying providers: {providers}")
    
except Exception as e:
    print(f"Error: {e}")
'@

$testScript | Out-File -FilePath "test_minimal_gpu.py" -Encoding UTF8

Write-Host "`nCreated test_minimal_gpu.py" -ForegroundColor Gray
Write-Host "Run: python test_minimal_gpu.py" -ForegroundColor Cyan

# Alternative: Download cuDNN-free ONNX Runtime build
Write-Host "`nAlternative Option:" -ForegroundColor Yellow
Write-Host "For cuDNN-free operation, consider using DirectML provider instead:" -ForegroundColor White
Write-Host "1. Install: pip install onnxruntime-directml" -ForegroundColor Gray
Write-Host "2. Use 'DmlExecutionProvider' instead of 'CUDAExecutionProvider'" -ForegroundColor Gray
Write-Host "3. Works with any DirectX 12 compatible GPU (NVIDIA, AMD, Intel)" -ForegroundColor Gray

Write-Host "`nDone!" -ForegroundColor Green