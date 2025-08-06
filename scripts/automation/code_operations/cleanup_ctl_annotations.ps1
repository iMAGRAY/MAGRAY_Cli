#!/usr/bin/env pwsh

<#
.SYNOPSIS
    PowerShell обертка для скрипта удаления CTL аннотаций

.DESCRIPTION
    Удобный интерфейс для запуска ctl_annotation_remover.py с различными опциями.

.PARAMETER DryRun
    Запуск в режиме предварительного просмотра

.PARAMETER ProjectRoot
    Путь к корневой директории проекта

.PARAMETER SaveReport
    Сохранить подробный отчет в JSON файл

.PARAMETER SkipConfirmation
    Пропустить подтверждение перед удалением

.EXAMPLE
    .\cleanup_ctl_annotations.ps1 -DryRun
    Предварительный просмотр изменений

.EXAMPLE
    .\cleanup_ctl_annotations.ps1 -SaveReport
    Полное удаление с сохранением отчета
#>

param(
    [switch]$DryRun,
    [string]$ProjectRoot = "",
    [switch]$SaveReport,
    [switch]$SkipConfirmation
)

# Функция для вывода цветного текста
function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "White"
    )
    
    $colorMap = @{
        "Green" = "DarkGreen"
        "Red" = "DarkRed" 
        "Yellow" = "DarkYellow"
        "Blue" = "DarkBlue"
        "Cyan" = "DarkCyan"
        "White" = "White"
    }
    
    Write-Host $Message -ForegroundColor $colorMap[$Color]
}

# Функция для проверки зависимостей
function Test-Dependencies {
    Write-ColorOutput "Проверка зависимостей..." "Blue"
    
    # Проверка Python
    try {
        $pythonVersion = py --version 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "Python не найден"
        }
        Write-ColorOutput "OK Python: $pythonVersion" "Green"
    }
    catch {
        Write-ColorOutput "ERROR Python не найден в PATH" "Red"
        exit 1
    }
    
    # Проверка Rich библиотеки
    try {
        py -c "import rich" 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-ColorOutput "Установка зависимостей..." "Yellow"
            pip install rich
            if ($LASTEXITCODE -ne 0) {
                throw "Ошибка установки зависимостей"
            }
        }
        Write-ColorOutput "OK Rich библиотека доступна" "Green"
    }
    catch {
        Write-ColorOutput "ERROR Не удалось установить зависимости" "Red"
        exit 1
    }
}

# Функция определения корня проекта  
function Get-ProjectRoot {
    if ($ProjectRoot -ne "") {
        if (Test-Path $ProjectRoot) {
            return Resolve-Path $ProjectRoot
        } else {
            Write-ColorOutput "ERROR Указанный путь не существует: $ProjectRoot" "Red"
            exit 1
        }
    }
    
    # Автоопределение корня проекта
    $scriptDir = Split-Path $MyInvocation.MyCommand.Path -Parent
    $projectRoot = Resolve-Path (Join-Path $scriptDir "../../..")
    
    $cargoToml = Join-Path $projectRoot "Cargo.toml"
    $cratesDir = Join-Path $projectRoot "crates"
    
    if ((Test-Path $cargoToml) -and (Test-Path $cratesDir)) {
        return $projectRoot
    }
    
    Write-ColorOutput "ERROR Не удалось найти корень проекта" "Red"
    exit 1
}

# Функция подтверждения действия
function Confirm-Action {
    param([string]$Message)
    
    if ($SkipConfirmation) {
        return $true
    }
    
    Write-Host "$Message (y/N): " -NoNewline
    $response = Read-Host
    
    return ($response.ToLower() -eq "y" -or $response.ToLower() -eq "yes")
}

# Основная функция
function Main {
    Write-ColorOutput "CTL Annotation Remover v1.0.0" "Cyan"
    Write-ColorOutput "================================" "Cyan"
    
    # Проверка зависимостей
    Test-Dependencies
    
    # Определение проекта
    $detectedProjectRoot = Get-ProjectRoot
    Write-ColorOutput "Проект: $detectedProjectRoot" "Blue"
    
    # Подготовка аргументов
    $pythonArgs = @("--project-root", "`"$detectedProjectRoot`"")
    
    if ($DryRun) {
        $pythonArgs += "--dry-run"
        Write-ColorOutput "Режим: Предварительный просмотр" "Yellow"
    } else {
        Write-ColorOutput "Режим: Реальное удаление" "Red"
    }
    
    # Подготовка отчета
    if ($SaveReport -or (-not $DryRun)) {
        $timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
        $reportFile = "ctl_cleanup_report_$timestamp.json"
        $pythonArgs += @("--report", $reportFile)
        Write-ColorOutput "Отчет: $reportFile" "Blue"
    }
    
    # Получение подтверждения
    if (-not $DryRun) {
        Write-ColorOutput "" "White"
        Write-ColorOutput "ВНИМАНИЕ: Это удалит все CTL аннотации!" "Red"
        Write-ColorOutput "Будет создан backup в папке backups/" "Yellow"
        
        if (-not (Confirm-Action "Продолжить?")) {
            Write-ColorOutput "Операция отменена" "Yellow"
            exit 0
        }
    }
    
    # Запуск Python скрипта
    Write-ColorOutput "" "White"
    Write-ColorOutput "Запуск скрипта очистки..." "Green"
    
    $scriptPath = Join-Path (Split-Path $MyInvocation.MyCommand.Path -Parent) "ctl_annotation_remover.py"
    
    try {
        & py $scriptPath @pythonArgs
        $exitCode = $LASTEXITCODE
        
        if ($exitCode -eq 0) {
            Write-ColorOutput "" "White"
            Write-ColorOutput "Операция завершена успешно!" "Green"
        } else {
            Write-ColorOutput "Операция завершена с ошибкой" "Red"
            exit $exitCode
        }
    }
    catch {
        Write-ColorOutput "Критическая ошибка: $($_.Exception.Message)" "Red"
        exit 1
    }
}

# Обработка прерывания
trap {
    Write-ColorOutput "Операция прервана пользователем" "Yellow"
    exit 130
}

# Запуск
Main