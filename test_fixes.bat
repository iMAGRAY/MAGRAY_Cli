@echo off
echo Testing MAGRAY CLI runtime fixes...
echo.

echo Test 1: Simple status check with timeout
timeout 10 target\release\magray.exe status
if %ERRORLEVEL% EQU 0 (
    echo ✓ Status command completed successfully
) else (
    echo ✗ Status command failed or timed out
)
echo.

echo Test 2: Simple chat message test with pipe input
echo "What is MAGRAY?" | timeout 30 target\release\magray.exe chat
if %ERRORLEVEL% EQU 0 (
    echo ✓ Chat command with pipe input completed successfully
) else (
    echo ✗ Chat command with pipe input failed or timed out
)
echo.

echo Test 3: Chat command with argument
timeout 20 target\release\magray.exe chat "Hello MAGRAY"
if %ERRORLEVEL% EQU 0 (
    echo ✓ Chat command with argument completed successfully
) else (
    echo ✗ Chat command with argument failed or timed out
)
echo.

echo Test 4: Health check command
timeout 15 target\release\magray.exe health
if %ERRORLEVEL% EQU 0 (
    echo ✓ Health check completed successfully
) else (
    echo ✗ Health check failed or timed out
)
echo.

echo All tests completed. Check results above.
pause