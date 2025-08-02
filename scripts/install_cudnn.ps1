# cuDNN Installation Helper for ONNX Runtime
# Installs cuDNN 9.x for CUDA 12.x

param(
    [string]$CudnnPath = "",
    [switch]$AutoDetect = $true
)

$ErrorActionPreference = "Stop"

Write-Host "cuDNN Installation Helper for ONNX Runtime" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

# Check CUDA installation
Write-Host "`nChecking CUDA installation..." -ForegroundColor Yellow
$cudaPath = $env:CUDA_PATH
if (-not $cudaPath) {
    # Try to find CUDA in standard locations
    $possiblePaths = @(
        "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9",
        "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.8",
        "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.7",
        "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6",
        "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.5",
        "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.4",
        "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.3",
        "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.2",
        "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.1",
        "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.0"
    )
    
    foreach ($path in $possiblePaths) {
        if (Test-Path $path) {
            $cudaPath = $path
            Write-Host "Found CUDA at: $cudaPath" -ForegroundColor Green
            break
        }
    }
    
    if (-not $cudaPath) {
        Write-Host "ERROR: CUDA not found. Install CUDA Toolkit from https://developer.nvidia.com/cuda-downloads" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "CUDA_PATH: $cudaPath" -ForegroundColor Green
}

# Get CUDA version
$cudaVersion = Split-Path $cudaPath -Leaf
Write-Host "CUDA Version: $cudaVersion" -ForegroundColor Cyan

# Instructions for downloading cuDNN
Write-Host "`nInstructions to install cuDNN:" -ForegroundColor Yellow
Write-Host "1. Go to https://developer.nvidia.com/cudnn" -ForegroundColor White
Write-Host "2. Login to NVIDIA Developer Account (free registration)" -ForegroundColor White
Write-Host "3. Download cuDNN 9.x for CUDA 12.x (Windows)" -ForegroundColor White
Write-Host "4. Extract archive to any folder" -ForegroundColor White
Write-Host "5. Run this script again with -CudnnPath <path_to_extracted_folder>" -ForegroundColor White

Write-Host "`nDirect download link:" -ForegroundColor Cyan
Write-Host "https://developer.nvidia.com/downloads/compute/cudnn/secure/9.5.1/local_installers/cudnn-windows-x86_64-9.5.1.17_cuda12-archive.zip" -ForegroundColor Gray
Write-Host "(requires NVIDIA account login)" -ForegroundColor Gray

if ($CudnnPath) {
    # Install cuDNN
    Write-Host "`nInstalling cuDNN from: $CudnnPath" -ForegroundColor Cyan
    
    if (-not (Test-Path $CudnnPath)) {
        Write-Host "ERROR: Path does not exist: $CudnnPath" -ForegroundColor Red
        exit 1
    }
    
    # Check cuDNN structure
    $requiredDirs = @("bin", "include", "lib")
    $allFound = $true
    foreach ($dir in $requiredDirs) {
        $dirPath = Join-Path $CudnnPath $dir
        if (-not (Test-Path $dirPath)) {
            Write-Host "ERROR: Directory not found: $dir" -ForegroundColor Red
            $allFound = $false
        }
    }
    
    if (-not $allFound) {
        Write-Host "ERROR: Invalid cuDNN structure. Make sure you specified the root folder with bin/, include/, lib/" -ForegroundColor Red
        exit 1
    }
    
    # Copy files
    Write-Host "`nCopying cuDNN files to CUDA..." -ForegroundColor Yellow
    
    # bin/*.dll -> CUDA/bin/
    $sourceBin = Join-Path $CudnnPath "bin"
    $targetBin = Join-Path $cudaPath "bin"
    Write-Host "  Copying DLL files..." -ForegroundColor Gray
    Get-ChildItem -Path $sourceBin -Filter "*.dll" | ForEach-Object {
        $targetFile = Join-Path $targetBin $_.Name
        Copy-Item -Path $_.FullName -Destination $targetFile -Force
        Write-Host "    + $($_.Name)" -ForegroundColor Green
    }
    
    # include/*.h -> CUDA/include/
    $sourceInclude = Join-Path $CudnnPath "include"
    $targetInclude = Join-Path $cudaPath "include"
    Write-Host "  Copying header files..." -ForegroundColor Gray
    Get-ChildItem -Path $sourceInclude -Filter "*.h" | ForEach-Object {
        $targetFile = Join-Path $targetInclude $_.Name
        Copy-Item -Path $_.FullName -Destination $targetFile -Force
        Write-Host "    + $($_.Name)" -ForegroundColor Green
    }
    
    # lib/*.lib -> CUDA/lib/x64/
    $sourceLib = Join-Path $CudnnPath "lib\x64"
    if (-not (Test-Path $sourceLib)) {
        $sourceLib = Join-Path $CudnnPath "lib"
    }
    $targetLib = Join-Path $cudaPath "lib\x64"
    Write-Host "  Copying libraries..." -ForegroundColor Gray
    Get-ChildItem -Path $sourceLib -Filter "*.lib" | ForEach-Object {
        $targetFile = Join-Path $targetLib $_.Name
        Copy-Item -Path $_.FullName -Destination $targetFile -Force
        Write-Host "    + $($_.Name)" -ForegroundColor Green
    }
    
    Write-Host "`ncuDNN successfully installed!" -ForegroundColor Green
    
    # Copy necessary DLLs to onnxruntime/lib
    $ortLibPath = "..\onnxruntime\lib"
    if (Test-Path $ortLibPath) {
        Write-Host "`nCopying cuDNN DLLs to ONNX Runtime..." -ForegroundColor Yellow
        
        $cudnnDlls = @("cudnn64_9.dll", "cudnn_ops64_9.dll", "cudnn_cnn64_9.dll", "cudnn_graph64_9.dll")
        foreach ($dll in $cudnnDlls) {
            $sourceDll = Join-Path $targetBin $dll
            if (Test-Path $sourceDll) {
                $targetDll = Join-Path $ortLibPath $dll
                Copy-Item -Path $sourceDll -Destination $targetDll -Force
                Write-Host "  + $dll" -ForegroundColor Green
            }
        }
    }
    
    Write-Host "`nDone! Now you can run GPU acceleration tests." -ForegroundColor Green
    Write-Host "Use: cargo test --features gpu test_gpu_acceleration" -ForegroundColor Cyan
    
} else {
    # Check for existing cuDNN
    Write-Host "`nChecking for installed cuDNN..." -ForegroundColor Yellow
    
    $cudnnDll = Join-Path $cudaPath "bin\cudnn64_9.dll"
    if (Test-Path $cudnnDll) {
        Write-Host "cuDNN is already installed!" -ForegroundColor Green
        
        # Check version
        $fileInfo = Get-Item $cudnnDll
        Write-Host "  File: $($fileInfo.Name)" -ForegroundColor Gray
        Write-Host "  Size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor Gray
        Write-Host "  Date: $($fileInfo.LastWriteTime)" -ForegroundColor Gray
        
        # Copy to onnxruntime/lib if needed
        $ortLibPath = "..\onnxruntime\lib"
        if (Test-Path $ortLibPath) {
            $ortCudnnDll = Join-Path $ortLibPath "cudnn64_9.dll"
            if (-not (Test-Path $ortCudnnDll)) {
                Write-Host "`nCopying cuDNN to ONNX Runtime..." -ForegroundColor Yellow
                Copy-Item -Path $cudnnDll -Destination $ortCudnnDll -Force
                Write-Host "Copied to $ortCudnnDll" -ForegroundColor Green
            }
        }
        
    } else {
        Write-Host "cuDNN is not installed" -ForegroundColor Red
        Write-Host "  Expected file: $cudnnDll" -ForegroundColor Gray
        
        # Auto-detect cuDNN
        if ($AutoDetect) {
            Write-Host "`nSearching for cuDNN in system..." -ForegroundColor Yellow
            
            $searchPaths = @(
                "C:\Program Files\NVIDIA\CUDNN",
                "C:\Tools\cuda",
                "C:\cudnn",
                "$env:USERPROFILE\Downloads"
            )
            
            foreach ($path in $searchPaths) {
                if (Test-Path $path) {
                    $found = Get-ChildItem -Path $path -Recurse -Filter "cudnn64_9.dll" -ErrorAction SilentlyContinue | Select-Object -First 1
                    if ($found) {
                        Write-Host "Found cuDNN at: $($found.DirectoryName)" -ForegroundColor Green
                        $parentPath = $found.DirectoryName | Split-Path -Parent
                        Write-Host "Run: .\install_cudnn.ps1 -CudnnPath '$parentPath'" -ForegroundColor Cyan
                        break
                    }
                }
            }
        }
    }
}

Write-Host "`nAdditional information:" -ForegroundColor Cyan
Write-Host "- cuDNN 9.x is compatible with CUDA 12.x" -ForegroundColor Gray
Write-Host "- For TensorRT support, install TensorRT SDK separately" -ForegroundColor Gray
Write-Host "- Restart PowerShell after installation" -ForegroundColor Gray