@echo off
REM MAGRAY CLI - GPU Build Script (Windows)
REM Builds full GPU version (~50MB) with CUDA/TensorRT support

echo.
echo 🔨 Building MAGRAY CLI - GPU Version
echo ====================================

REM Build configuration
set BINARY_NAME=magray
set BUILD_MODE=release
set TARGET_DIR=target\gpu
set FEATURES=gpu

echo Configuration:
echo   - Binary: %BINARY_NAME%
echo   - Mode: %BUILD_MODE%
echo   - Features: %FEATURES%
echo   - Target: %TARGET_DIR%
echo.

REM Check system dependencies
echo Checking system dependencies...
cargo --version >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ❌ Error: cargo not found. Please install Rust toolchain.
    exit /b 1
)

REM Check CUDA availability
echo Checking CUDA availability...
nvcc --version >nul 2>&1
if %ERRORLEVEL% equ 0 (
    echo ✅ CUDA toolkit found
    for /f "tokens=*" %%i in ('nvcc --version ^| findstr "release"') do set CUDA_INFO=%%i
    echo %CUDA_INFO%
) else (
    echo ⚠️  CUDA not found. GPU features may not work optimally.
    echo    Download from: https://developer.nvidia.com/cuda-downloads
)

REM Check for ONNX Runtime GPU libraries
if exist "scripts\onnxruntime\lib\onnxruntime.dll" (
    echo ✅ ONNX Runtime GPU libraries found
) else (
    echo ⚠️  ONNX Runtime GPU libraries not found in scripts\onnxruntime\lib
    echo    Run: scripts\download_onnxruntime_gpu.ps1 to download
)

REM Clean previous builds
echo Cleaning previous builds...
cargo clean --target-dir %TARGET_DIR%
if %ERRORLEVEL% neq 0 (
    echo ❌ Failed to clean target directory
    exit /b 1
)

REM Set environment for GPU build
if defined CUDA_PATH (
    echo Using CUDA_PATH: %CUDA_PATH%
) else (
    set CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA
    echo Using default CUDA_PATH: %CUDA_PATH%
)

REM Build GPU version with optimizations
echo Building GPU version with CUDA optimizations...
set RUSTFLAGS=-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1
cargo build ^
    --release ^
    --no-default-features ^
    --features=%FEATURES% ^
    --target-dir=%TARGET_DIR% ^
    --bin=%BINARY_NAME%

if %ERRORLEVEL% neq 0 (
    echo ❌ Build failed
    echo Troubleshooting:
    echo   - Ensure CUDA toolkit is installed
    echo   - Check ONNX Runtime GPU libraries are available
    echo   - Verify CUDA_PATH environment variable
    echo   - Run scripts\download_onnxruntime_gpu.ps1 if needed
    exit /b 1
)

REM Check build success
set BINARY_PATH=%TARGET_DIR%\release\%BINARY_NAME%.exe
if exist "%BINARY_PATH%" (
    echo ✅ Build successful!
    echo Binary location: %BINARY_PATH%
    
    REM Get binary size
    for %%A in ("%BINARY_PATH%") do set BINARY_SIZE=%%~zA
    set /a BINARY_SIZE_MB=%BINARY_SIZE%/1024/1024
    echo Binary size: ~%BINARY_SIZE_MB%MB
    
    REM GPU availability test
    echo Testing GPU availability...
    "%BINARY_PATH%" gpu info >nul 2>&1
    if %ERRORLEVEL% equ 0 (
        echo ✅ GPU detection working
    ) else (
        echo ℹ️  GPU detection not tested (may require GPU hardware^)
    )
    
    REM Version test
    "%BINARY_PATH%" --version >nul 2>&1
    if %ERRORLEVEL% equ 0 (
        echo ✅ Version check passed
    ) else (
        echo ⚠️  Warning: Version check failed
    )
) else (
    echo ❌ Build failed: Binary not found
    exit /b 1
)

echo.
echo 🎉 GPU build completed successfully!
echo Use: %BINARY_PATH%
echo Note: This build includes full GPU acceleration support
echo Requires: CUDA-compatible GPU, ONNX Runtime GPU libraries