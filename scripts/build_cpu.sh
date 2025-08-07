#!/bin/bash
# MAGRAY CLI - CPU Build Script
# Builds full CPU version (~20MB) with all features except GPU

set -e

echo "🔨 Building MAGRAY CLI - CPU Version"
echo "===================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build configuration
BINARY_NAME="magray"
BUILD_MODE="release"
TARGET_DIR="target/cpu"
FEATURES="cpu"

echo -e "${YELLOW}Configuration:${NC}"
echo "  - Binary: $BINARY_NAME"
echo "  - Mode: $BUILD_MODE"  
echo "  - Features: $FEATURES"
echo "  - Target: $TARGET_DIR"
echo ""

# Clean previous builds
echo -e "${YELLOW}Cleaning previous builds...${NC}"
cargo clean --target-dir $TARGET_DIR

# Check system dependencies
echo -e "${YELLOW}Checking system dependencies...${NC}"
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found. Please install Rust toolchain.${NC}"
    exit 1
fi

# Build CPU version with optimizations
echo -e "${YELLOW}Building CPU version with optimizations...${NC}"
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
        
        # Feature availability test
        echo -e "${YELLOW}Testing feature availability...${NC}"
        if "$BINARY_PATH" --version >/dev/null 2>&1; then
            echo -e "${GREEN}✅ Version check passed${NC}"
        else
            echo -e "${RED}⚠️  Warning: Version check failed${NC}"
        fi
        
        # Test AI features (if available)
        echo -e "${YELLOW}Testing AI features...${NC}"
        if "$BINARY_PATH" models list >/dev/null 2>&1; then
            echo -e "${GREEN}✅ AI features available${NC}"
        else
            echo -e "${YELLOW}ℹ️  AI features not tested (may require setup)${NC}"
        fi
        
    else
        echo -e "${RED}❌ Build failed: Binary not found${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}🎉 CPU build completed successfully!${NC}"
echo -e "${GREEN}Use: $BINARY_PATH${NC}"
echo -e "${YELLOW}Note: This build includes full AI/ML functionality on CPU${NC}"