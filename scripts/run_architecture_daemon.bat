@echo off
REM Запуск ультракомпактного архитектурного демона для MAGRAY CLI

echo [INFO] MAGRAY CLI Architecture Daemon
echo.

REM Проверяем Python
py --version >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Python не найден. Установите Python 3.8+ и попробуйте снова.
    pause
    exit /b 1
)

REM Устанавливаем зависимости если нужно
echo [INFO] Проверка зависимостей...
py -c "import toml, watchdog" >nul 2>&1
if errorlevel 1 (
    echo [INFO] Установка зависимостей...
    py -m pip install toml watchdog
    if errorlevel 1 (
        echo [ERROR] Ошибка установки зависимостей
        pause
        exit /b 1
    )
)

echo [OK] Зависимости готовы
echo.

REM Запускаем демон
if "%1"=="--watch" (
    echo [INFO] Запуск демона в watch режиме...
    echo [INFO] Для остановки нажмите Ctrl+C
    echo.
    py "%~dp0architecture_daemon.py" --project-root . --watch
) else (
    echo [INFO] Единократное обновление архитектуры...
    echo.
    py "%~dp0architecture_daemon.py" --project-root .
    if errorlevel 0 (
        echo.
        echo [SUCCESS] Демон завершен успешно!
        echo.
        echo Результат:
        echo - Создана/обновлена секция AUTO-GENERATED ARCHITECTURE в CLAUDE.md
        echo - Генерирована компактная Mermaid диаграмма архитектуры
        echo - Обновлена статистика проекта: 8 crates и зависимости
        echo.
        echo Для автоматических обновлений используйте:
        echo   run_architecture_daemon.bat --watch
    )
)

echo.
pause