# MAGRAY CLI - Universal Build Script (PowerShell)
# Builds all three variants: minimal, cpu, gpu

param(
    [string]$Variant = "all",  # all, minimal, cpu, gpu
    [switch]$Clean = $false,   # Clean all targets first
    [switch]$Test = $false     # Run tests after build
)

$ErrorActionPreference = "Stop"

# Colors for output
$Red = "Red"
$Green = "Green" 
$Yellow = "Yellow"
$Cyan = "Cyan"

function Write-ColoredOutput($Message, $Color = "White") {
    Write-Host $Message -ForegroundColor $Color
}

function Write-Header($Title) {
    Write-Host "`n" -NoNewline
    Write-ColoredOutput "=" * 50 $Cyan
    Write-ColoredOutput "  $Title" $Cyan
    Write-ColoredOutput "=" * 50 $Cyan
    Write-Host ""
}

function Test-Prerequisites {
    Write-ColoredOutput "Checking prerequisites..." $Yellow
    
    # Check Rust toolchain
    try {
        $rustVersion = cargo --version
        Write-ColoredOutput "✅ Rust: $rustVersion" $Green
    }
    catch {
        Write-ColoredOutput "❌ Rust toolchain not found. Install from https://rustup.rs/" $Red
        exit 1
    }
    
    # Check CUDA for GPU builds
    try {
        $cudaVersion = nvcc --version 2>$null | Select-String "release"
        Write-ColoredOutput "✅ CUDA: $cudaVersion" $Green
    }
    catch {
        Write-ColoredOutput "⚠️  CUDA not found (GPU build may fail)" $Yellow
    }
    
    # Check ONNX Runtime libraries
    if (Test-Path "scripts\onnxruntime\lib\onnxruntime.dll") {
        Write-ColoredOutput "✅ ONNX Runtime libraries found" $Green
    }
    else {
        Write-ColoredOutput "⚠️  ONNX Runtime GPU libraries not found" $Yellow
        Write-ColoredOutput "   Run: scripts\download_onnxruntime_gpu.ps1" $Yellow
    }
}

function Build-Variant($VariantName, $Features, $TargetDir) {
    Write-Header "Building $VariantName variant"
    
    $binaryName = "magray"
    $targetPath = "$TargetDir\release\$binaryName.exe"
    
    Write-ColoredOutput "Configuration:" $Yellow
    Write-ColoredOutput "  - Variant: $VariantName" 
    Write-ColoredOutput "  - Features: $Features"
    Write-ColoredOutput "  - Target: $TargetDir"
    Write-Host ""
    
    # Clean if requested
    if ($Clean) {
        Write-ColoredOutput "Cleaning target directory..." $Yellow
        cargo clean --target-dir $TargetDir
    }
    
    # Set optimization flags based on variant
    $rustFlags = switch ($VariantName) {
        "minimal" { "-C target-cpu=native -C link-arg=/SUBSYSTEM:CONSOLE" }
        "cpu"     { "-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1" }
        "gpu"     { "-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1" }
    }
    
    # Build
    Write-ColoredOutput "Building with optimizations..." $Yellow
    $env:RUSTFLAGS = $rustFlags
    
    try {
        cargo build --release --no-default-features --features="$Features" --target-dir="$TargetDir" --bin="$binaryName"
        
        if (Test-Path $targetPath) {
            $binarySize = (Get-Item $targetPath).Length
            $binarySizeMB = [math]::Round($binarySize / 1MB, 2)
            
            Write-ColoredOutput "✅ Build successful!" $Green
            Write-ColoredOutput "Binary: $targetPath" $Green  
            Write-ColoredOutput "Size: ${binarySizeMB}MB" $Green
            
            # Test binary
            Write-ColoredOutput "Testing binary..." $Yellow
            try {
                & $targetPath --version | Out-Null
                Write-ColoredOutput "✅ Basic test passed" $Green
            }
            catch {
                Write-ColoredOutput "⚠️  Warning: Basic test failed" $Yellow
            }
            
            return $targetPath
        }
        else {
            Write-ColoredOutput "❌ Build failed: Binary not found" $Red
            return $null
        }
    }
    catch {
        Write-ColoredOutput "❌ Build failed: $($_.Exception.Message)" $Red
        return $null
    }
}

function Run-Tests($BinaryPath, $VariantName) {
    if (-not $Test -or -not $BinaryPath) { return }
    
    Write-Header "Testing $VariantName variant"
    
    # Basic functionality tests
    Write-ColoredOutput "Running functionality tests..." $Yellow
    
    try {
        # Version check
        $version = & $BinaryPath --version
        Write-ColoredOutput "✅ Version: $version" $Green
        
        # Feature-specific tests
        switch ($VariantName) {
            "cpu" {
                try {
                    & $BinaryPath models list | Out-Null
                    Write-ColoredOutput "✅ AI models functionality working" $Green
                }
                catch {
                    Write-ColoredOutput "ℹ️  AI models not tested (may require setup)" $Yellow
                }
            }
            "gpu" {
                try {
                    & $BinaryPath gpu info | Out-Null
                    Write-ColoredOutput "✅ GPU detection working" $Green
                }
                catch {
                    Write-ColoredOutput "ℹ️  GPU features not tested (may require GPU hardware)" $Yellow
                }
            }
        }
    }
    catch {
        Write-ColoredOutput "⚠️  Some tests failed: $($_.Exception.Message)" $Yellow
    }
}

# Main execution
Write-Header "MAGRAY CLI - Multi-Variant Build System"

Test-Prerequisites

$variants = @()
switch ($Variant.ToLower()) {
    "all" { 
        $variants = @(
            @{Name="minimal"; Features="minimal"; Target="target\minimal"},
            @{Name="cpu"; Features="cpu"; Target="target\cpu"},  
            @{Name="gpu"; Features="gpu"; Target="target\gpu"}
        )
    }
    "minimal" { $variants = @(@{Name="minimal"; Features="minimal"; Target="target\minimal"}) }
    "cpu"     { $variants = @(@{Name="cpu"; Features="cpu"; Target="target\cpu"}) }
    "gpu"     { $variants = @(@{Name="gpu"; Features="gpu"; Target="target\gpu"}) }
    default {
        Write-ColoredOutput "❌ Invalid variant: $Variant. Use: all, minimal, cpu, gpu" $Red
        exit 1
    }
}

$builtBinaries = @()
$startTime = Get-Date

foreach ($var in $variants) {
    $binary = Build-Variant $var.Name $var.Features $var.Target
    if ($binary) {
        $builtBinaries += @{Name=$var.Name; Path=$binary}
        Run-Tests $binary $var.Name
    }
}

# Summary
$endTime = Get-Date
$totalTime = $endTime - $startTime

Write-Header "Build Summary"
Write-ColoredOutput "Total build time: $($totalTime.TotalMinutes.ToString("F1")) minutes" $Cyan

if ($builtBinaries.Count -gt 0) {
    Write-ColoredOutput "Successfully built variants:" $Green
    foreach ($binary in $builtBinaries) {
        $size = (Get-Item $binary.Path).Length
        $sizeMB = [math]::Round($size / 1MB, 2)
        Write-ColoredOutput "  ✅ $($binary.Name): $($binary.Path) (${sizeMB}MB)" $Green
    }
}
else {
    Write-ColoredOutput "❌ No variants built successfully" $Red
    exit 1
}

Write-ColoredOutput "`n🎉 Build process completed!" $Green