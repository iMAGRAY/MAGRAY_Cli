#!/bin/bash
# MAGRAY CLI - GPU Build Script
# Builds full GPU version (~50MB) with CUDA/TensorRT support

set -e

echo "🔨 Building MAGRAY CLI - GPU Version"
echo "===================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build configuration
BINARY_NAME="magray"
BUILD_MODE="release"
TARGET_DIR="target/gpu"
FEATURES="gpu"

echo -e "${YELLOW}Configuration:${NC}"
echo "  - Binary: $BINARY_NAME"
echo "  - Mode: $BUILD_MODE"  
echo "  - Features: $FEATURES"
echo "  - Target: $TARGET_DIR"
echo ""

# Check system dependencies
echo -e "${YELLOW}Checking system dependencies...${NC}"
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found. Please install Rust toolchain.${NC}"
    exit 1
fi

# Check CUDA availability
echo -e "${YELLOW}Checking CUDA availability...${NC}"
if command -v nvcc &> /dev/null; then
    CUDA_VERSION=$(nvcc --version | grep "release" | sed 's/.*release //' | sed 's/,.*//')
    echo -e "${GREEN}✅ CUDA found: $CUDA_VERSION${NC}"
else
    echo -e "${YELLOW}⚠️  CUDA not found. GPU features may not work optimally.${NC}"
fi

# Check for ONNX Runtime GPU libraries
if [ -d "scripts/onnxruntime/lib" ]; then
    echo -e "${GREEN}✅ ONNX Runtime GPU libraries found${NC}"
else
    echo -e "${YELLOW}⚠️  ONNX Runtime GPU libraries not found in scripts/onnxruntime/lib${NC}"
    echo -e "${YELLOW}   Run: scripts/download_onnxruntime_gpu.ps1 (on Windows) or download manually${NC}"
fi

# Clean previous builds
echo -e "${YELLOW}Cleaning previous builds...${NC}"
cargo clean --target-dir $TARGET_DIR

# Set environment for GPU build
export CUDA_PATH=${CUDA_PATH:-"/usr/local/cuda"}
export LD_LIBRARY_PATH="$CUDA_PATH/lib64:$LD_LIBRARY_PATH"

# Build GPU version with optimizations
echo -e "${YELLOW}Building GPU version with CUDA optimizations...${NC}"
RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1" \
cargo build \
    --release \
    --no-default-features \
    --features="$FEATURES" \
    --target-dir="$TARGET_DIR" \
    --bin="$BINARY_NAME"

# Check build success
if [ $? -eq 0 ]; then
    BINARY_PATH="$TARGET_DIR/release/$BINARY_NAME"
    if [ -f "$BINARY_PATH" ]; then
        BINARY_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
        echo -e "${GREEN}✅ Build successful!${NC}"
        echo -e "${GREEN}Binary location: $BINARY_PATH${NC}"
        echo -e "${GREEN}Binary size: $BINARY_SIZE${NC}"
        
        # GPU availability test
        echo -e "${YELLOW}Testing GPU availability...${NC}"
        if "$BINARY_PATH" gpu info >/dev/null 2>&1; then
            echo -e "${GREEN}✅ GPU detection working${NC}"
        else
            echo -e "${YELLOW}ℹ️  GPU detection not tested (may require GPU hardware)${NC}"
        fi
        
        # Version test
        if "$BINARY_PATH" --version >/dev/null 2>&1; then
            echo -e "${GREEN}✅ Version check passed${NC}"
        else
            echo -e "${RED}⚠️  Warning: Version check failed${NC}"
        fi
        
    else
        echo -e "${RED}❌ Build failed: Binary not found${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ Build failed${NC}"
    echo -e "${YELLOW}Troubleshooting:${NC}"
    echo "  - Ensure CUDA toolkit is installed"
    echo "  - Check ONNX Runtime GPU libraries are available"
    echo "  - Verify environment variables: CUDA_PATH, LD_LIBRARY_PATH"
    exit 1
fi

echo ""
echo -e "${GREEN}🎉 GPU build completed successfully!${NC}"
echo -e "${GREEN}Use: $BINARY_PATH${NC}"
echo -e "${YELLOW}Note: This build includes full GPU acceleration support${NC}"
echo -e "${YELLOW}Requires: CUDA-compatible GPU, ONNX Runtime GPU libraries${NC}"