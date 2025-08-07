@echo off
REM MAGRAY CLI - CPU Build Script (Windows)
REM Builds full CPU version (~20MB) with all features except GPU

echo.
echo 🔨 Building MAGRAY CLI - CPU Version
echo ====================================

REM Build configuration
set BINARY_NAME=magray
set BUILD_MODE=release
set TARGET_DIR=target\cpu
set FEATURES=cpu

echo Configuration:
echo   - Binary: %BINARY_NAME%
echo   - Mode: %BUILD_MODE%
echo   - Features: %FEATURES%
echo   - Target: %TARGET_DIR%
echo.

REM Clean previous builds
echo Cleaning previous builds...
cargo clean --target-dir %TARGET_DIR%
if %ERRORLEVEL% neq 0 (
    echo ❌ Failed to clean target directory
    exit /b 1
)

REM Check system dependencies
echo Checking system dependencies...
cargo --version >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ❌ Error: cargo not found. Please install Rust toolchain.
    exit /b 1
)

REM Build CPU version with optimizations
echo Building CPU version with optimizations...
set RUSTFLAGS=-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1
cargo build ^
    --release ^
    --no-default-features ^
    --features=%FEATURES% ^
    --target-dir=%TARGET_DIR% ^
    --bin=%BINARY_NAME%

if %ERRORLEVEL% neq 0 (
    echo ❌ Build failed
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
    
    REM Feature availability test
    echo Testing feature availability...
    "%BINARY_PATH%" --version >nul 2>&1
    if %ERRORLEVEL% equ 0 (
        echo ✅ Version check passed
    ) else (
        echo ⚠️  Warning: Version check failed
    )
    
    REM Test AI features (if available)
    echo Testing AI features...
    "%BINARY_PATH%" models list >nul 2>&1
    if %ERRORLEVEL% equ 0 (
        echo ✅ AI features available
    ) else (
        echo ℹ️  AI features not tested (may require setup^)
    )
) else (
    echo ❌ Build failed: Binary not found
    exit /b 1
)

echo.
echo 🎉 CPU build completed successfully!
echo Use: %BINARY_PATH%
echo Note: This build includes full AI/ML functionality on CPU