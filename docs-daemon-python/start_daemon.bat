@echo off
REM Startup script for CTL Sync Daemon
REM Automatically starts the daemon in background watch mode

echo Starting CTL Sync Daemon in background...

cd /d "%~dp0"

REM Stop any existing daemon processes
python daemon_manager.py stop 2>nul

REM Start daemon in background watch mode
start /b ctl-sync watch

echo Daemon started successfully!
echo Use 'python daemon_manager.py validate' to check status
echo Use 'python daemon_manager.py stop' to stop the daemon

pause