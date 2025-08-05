@echo off
echo Installing CTL Sync Python Daemon...
echo ====================================

rem Install package in development mode
python -m pip install -e .

if %ERRORLEVEL% NEQ 0 (
    echo Error: Failed to install package
    pause
    exit /b 1
)

echo.
echo Installation complete!
echo.
echo Usage:
echo   ctl-sync once    - One-time sync
echo   ctl-sync watch   - Watch mode
echo   ctl-sync stats   - Show statistics
echo.
echo Running demo...
python test_demo.py

pause