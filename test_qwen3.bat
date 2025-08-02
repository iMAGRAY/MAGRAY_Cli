@echo off
echo =======================================
echo Testing Qwen3 Models Integration
echo =======================================
echo.

REM Set environment variables
set RUST_LOG=info
set RUST_BACKTRACE=1

REM Run the test example
echo Running Qwen3 models test...
cargo run --package ai --example test_qwen3_models

echo.
echo =======================================
echo Test completed!
echo =======================================
pause