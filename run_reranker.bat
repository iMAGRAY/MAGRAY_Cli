@echo off
set ORT_DYLIB_PATH=%~dp0scripts\onnxruntime\lib\onnxruntime.dll
cd /d %~dp0crates\memory
cargo run --example test_reranker --release