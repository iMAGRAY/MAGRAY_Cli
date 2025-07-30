@echo off
echo Setting ONNX Runtime environment variables...
set ORT_DYLIB_PATH=.\onnxruntime\lib\onnxruntime.dll
set PATH=.\onnxruntime\lib;%PATH%
echo Environment configured for ONNX Runtime 1.22.0
