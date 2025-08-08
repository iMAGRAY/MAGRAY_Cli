#!/bin/bash
# ONNX Runtime environment setup
echo "Setting ONNX Runtime environment variables..."
export ORT_DYLIB_PATH="./scripts/onnxruntime/lib/libonnxruntime.so"
export LD_LIBRARY_PATH="./scripts/onnxruntime/lib:${LD_LIBRARY_PATH}"
echo "Environment configured for ONNX Runtime 1.22.0"
