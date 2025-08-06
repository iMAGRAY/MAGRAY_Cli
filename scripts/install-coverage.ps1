param(
    [switch]$InstallLLVMCov = $true
)

Write-Host "Installing coverage tools for MAGRAY CLI..." -ForegroundColor Green

# Create coverage directories
$dirs = @("target/coverage", "target/tarpaulin", "target/coverage/core", "target/coverage/llm", "target/coverage/tools")
foreach ($dir in $dirs) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        Write-Host "  Created directory: $dir"
    }
}

# Install cargo-llvm-cov if requested
if ($InstallLLVMCov) {
    Write-Host "`nInstalling cargo-llvm-cov..." -ForegroundColor Yellow
    
    try {
        cargo llvm-cov --help | Out-Null
        Write-Host "  cargo-llvm-cov already installed"
    }
    catch {
        Write-Host "  Installing cargo-llvm-cov..."
        cargo install cargo-llvm-cov
        Write-Host "  cargo-llvm-cov installed successfully"
    }
}

Write-Host "`nCoverage tools setup completed!" -ForegroundColor Green