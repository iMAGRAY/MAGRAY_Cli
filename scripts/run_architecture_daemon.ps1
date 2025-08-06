#!/usr/bin/env pwsh
# Запуск ультракомпактного архитектурного демона для MAGRAY CLI

param(
    [switch]$Watch,
    [string]$ProjectRoot = "."
)

# Определяем пути
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$DaemonScript = Join-Path $ScriptDir "architecture_daemon.py"

# Проверяем наличие Python
try {
    $PythonVersion = py --version 2>$null
    if (-not $PythonVersion) {
        throw "Python не найден"
    }
    Write-Host "[INFO] Python найден: $PythonVersion" -ForegroundColor Green
}
catch {
    Write-Host "[ERROR] Python не установлен или недоступен" -ForegroundColor Red
    Write-Host "Установите Python 3.8+ и попробуйте снова" -ForegroundColor Yellow
    exit 1
}

# Проверяем зависимости
Write-Host "[INFO] Проверка зависимостей..." -ForegroundColor Cyan
try {
    py -c "import toml, watchdog" 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[INFO] Установка зависимостей..." -ForegroundColor Yellow
        py -m pip install toml watchdog
    }
    Write-Host "[OK] Зависимости готовы" -ForegroundColor Green
}
catch {
    Write-Host "[ERROR] Ошибка зависимостей" -ForegroundColor Red
    exit 1
}

# Запускаем демон
Write-Host ""
Write-Host "=== MAGRAY CLI Architecture Daemon ===" -ForegroundColor Magenta
Write-Host ""

if ($Watch) {
    Write-Host "[INFO] Запуск демона в watch режиме..." -ForegroundColor Cyan
    py $DaemonScript --project-root $ProjectRoot --watch
}
else {
    Write-Host "[INFO] Единократное обновление архитектуры..." -ForegroundColor Cyan
    py $DaemonScript --project-root $ProjectRoot
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "[SUCCESS] Демон завершен успешно!" -ForegroundColor Green
        Write-Host ""
        Write-Host "Результат:" -ForegroundColor Cyan
        Write-Host "- Создана/обновлена секция AUTO-GENERATED ARCHITECTURE в CLAUDE.md" -ForegroundColor White
        Write-Host "- Генерирована компактная Mermaid диаграмма архитектуры" -ForegroundColor White
        Write-Host "- Обновлена статистика проекта: 8 crates и зависимости" -ForegroundColor White
        
        Write-Host ""
        Write-Host "Для автоматических обновлений используйте:" -ForegroundColor Yellow
        Write-Host "  .\run_architecture_daemon.ps1 -Watch" -ForegroundColor Cyan
    }
}