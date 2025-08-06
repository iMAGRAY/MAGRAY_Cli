# =============================================================================
# MAGRAY CLI - Coverage Tools Setup Script
# Устанавливает инструменты для измерения тестового покрытия кода
# =============================================================================

param(
    [switch]$InstallTarpaulin = $false,
    [switch]$InstallLLVMCov = $true,
    [switch]$Verbose = $false
)

Write-Host "🧪 Настройка инструментов для тестового покрытия MAGRAY CLI..." -ForegroundColor Green

# Создание директорий для coverage данных
$coverageDirs = @(
    "target/coverage",
    "target/tarpaulin", 
    "target/coverage/core",
    "target/coverage/llm",
    "target/coverage/tools"
)

foreach ($dir in $coverageDirs) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        Write-Host "  ✅ Создана директория: $dir"
    }
}

# Функция проверки наличия cargo команды
function Test-CargoCommand($command) {
    try {
        cargo $command --help | Out-Null
        return $true
    }
    catch {
        return $false
    }
}

# 1. Установка cargo-llvm-cov (рекомендуемый инструмент)
if ($InstallLLVMCov) {
    Write-Host "`n📊 Установка cargo-llvm-cov..." -ForegroundColor Yellow
    
    if (Test-CargoCommand "llvm-cov") {
        Write-Host "  ✅ cargo-llvm-cov уже установлен"
    } else {
        Write-Host "  🔄 Установка cargo-llvm-cov..."
        cargo install cargo-llvm-cov
        
        if (Test-CargoCommand "llvm-cov") {
            Write-Host "  ✅ cargo-llvm-cov успешно установлен"
        } else {
            Write-Host "  ❌ Ошибка установки cargo-llvm-cov" -ForegroundColor Red
        }
    }
}

Write-Host "`n🎉 Настройка coverage инструментов завершена!" -ForegroundColor Green