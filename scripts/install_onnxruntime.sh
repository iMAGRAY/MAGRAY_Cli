#!/bin/bash
# ONNX Runtime 1.22.0 Installation Script for Linux/macOS
# This script downloads and installs ONNX Runtime 1.22.0 required by ort 2.0.0-rc.4

set -e

# Configuration
ONNX_VERSION="1.22.0"
INSTALL_PATH="${1:-./onnxruntime}"
ADD_TO_PROFILE="${2:-false}"

# Allow skipping the Rust test stage via env var
ORT_NO_TEST="${ORT_NO_TEST:-0}"
# Timeout for the optional test stage (seconds)
TIMEOUT_SECS="${TIMEOUT_SECS:-300}"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Map architecture names
case "$ARCH" in
    x86_64)
        ARCH="x64"
        ;;
    aarch64|arm64)
        ARCH="arm64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
 esac

# Set download URL based on OS
case "$OS" in
    linux)
        DOWNLOAD_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ONNX_VERSION}/onnxruntime-linux-${ARCH}-${ONNX_VERSION}.tgz"
        LIB_EXT="so"
        ;;
    darwin)
        DOWNLOAD_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ONNX_VERSION}/onnxruntime-osx-${ARCH}-${ONNX_VERSION}.tgz"
        LIB_EXT="dylib"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
 esac

echo "ONNX Runtime ${ONNX_VERSION} Installation Script"
echo "============================================"
echo "OS: $OS, Architecture: $ARCH"

echo "ORT_NO_TEST=$ORT_NO_TEST, TIMEOUT_SECS=$TIMEOUT_SECS"

# Check if already installed
if [ -d "$INSTALL_PATH" ]; then
    echo "ONNX Runtime already installed at $INSTALL_PATH"
    read -p "Do you want to reinstall? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 0
    fi
    rm -rf "$INSTALL_PATH"
fi

# Create installation directory
echo -e "\nCreating installation directory: $INSTALL_PATH"
mkdir -p "$INSTALL_PATH"

# Download ONNX Runtime
TEMP_FILE="/tmp/onnxruntime-${ONNX_VERSION}.tgz"
echo -e "\nDownloading ONNX Runtime ${ONNX_VERSION}..."
echo "URL: $DOWNLOAD_URL"

if command -v wget > /dev/null; then
    wget -q --show-progress -O "$TEMP_FILE" "$DOWNLOAD_URL"
elif command -v curl > /dev/null; then
    curl -L -# -o "$TEMP_FILE" "$DOWNLOAD_URL"
else
    echo "Error: Neither wget nor curl is available"
    exit 1
fi

# Extract archive
echo -e "\nExtracting ONNX Runtime..."
tar -xzf "$TEMP_FILE" -C "$INSTALL_PATH" --strip-components=1

# Clean up
rm -f "$TEMP_FILE"

# Verify installation
LIB_PATH="${INSTALL_PATH}/lib"
DLL_PATH="${LIB_PATH}/libonnxruntime.${LIB_EXT}"

if [ -f "$DLL_PATH" ]; then
    echo -e "\nONNX Runtime installed successfully!"
    echo "Library location: $DLL_PATH"
else
    # Try alternative location
    DLL_PATH="${LIB_PATH}/libonnxruntime.${LIB_EXT}.${ONNX_VERSION}"
    if [ -f "$DLL_PATH" ]; then
        echo -e "\nONNX Runtime installed successfully!"
        echo "Library location: $DLL_PATH"
    else
        echo -e "\nError: libonnxruntime.${LIB_EXT} not found in expected location"
        exit 1
    fi
fi

# Set environment variables for current session
export ORT_DYLIB_PATH="$DLL_PATH"
export LD_LIBRARY_PATH="${LIB_PATH}:${LD_LIBRARY_PATH}"
if [ "$OS" = "darwin" ]; then
    export DYLD_LIBRARY_PATH="${LIB_PATH}:${DYLD_LIBRARY_PATH}"
fi

echo -e "\nEnvironment variables set:"
echo "ORT_DYLIB_PATH=$ORT_DYLIB_PATH"
echo "LD_LIBRARY_PATH includes $LIB_PATH"

# Create environment setup script
SETUP_SCRIPT="setup_ort_env.sh"
cat > "$SETUP_SCRIPT" << EOF
#!/bin/bash
# ONNX Runtime environment setup
echo "Setting ONNX Runtime environment variables..."
export ORT_DYLIB_PATH="$DLL_PATH"
export LD_LIBRARY_PATH="${LIB_PATH}:\${LD_LIBRARY_PATH}"
EOF

if [ "$OS" = "darwin" ]; then
    echo "export DYLD_LIBRARY_PATH=\"${LIB_PATH}:\${DYLD_LIBRARY_PATH}\"" >> "$SETUP_SCRIPT"
fi

echo "echo \"Environment configured for ONNX Runtime ${ONNX_VERSION}\"" >> "$SETUP_SCRIPT"
chmod +x "$SETUP_SCRIPT"

echo -e "\nCreated $SETUP_SCRIPT for easy environment configuration"

# Add to shell profile if requested
if [ "$ADD_TO_PROFILE" = "true" ]; then
    PROFILE_FILE=""
    
    # Determine shell profile file
    if [ -n "$BASH_VERSION" ]; then
        PROFILE_FILE="$HOME/.bashrc"
    elif [ -n "$ZSH_VERSION" ]; then
        PROFILE_FILE="$HOME/.zshrc"
    else
        PROFILE_FILE="$HOME/.profile"
    fi
    
    echo -e "\nAdding to $PROFILE_FILE..."
    
    # Add to profile if not already present
    if ! grep -q "ORT_DYLIB_PATH" "$PROFILE_FILE" 2>/dev/null; then
        echo "" >> "$PROFILE_FILE"
        echo "# ONNX Runtime configuration" >> "$PROFILE_FILE"
        echo "export ORT_DYLIB_PATH=\"$DLL_PATH\"" >> "$PROFILE_FILE"
        echo "export LD_LIBRARY_PATH=\"${LIB_PATH}:\$LD_LIBRARY_PATH\"" >> "$PROFILE_FILE"
        if [ "$OS" = "darwin" ]; then
            echo "export DYLD_LIBRARY_PATH=\"${LIB_PATH}:\$DYLD_LIBRARY_PATH\"" >> "$PROFILE_FILE"
        fi
        echo "Added ONNX Runtime configuration to $PROFILE_FILE"
    fi
fi

# Optional: Test with Rust (can be skipped)
if [ "$ORT_NO_TEST" != "1" ]; then
    echo -e "\nTesting ONNX Runtime with Rust (timeout ${TIMEOUT_SECS}s)..."
    TEST_DIR="/tmp/ort_test_$$"
    mkdir -p "$TEST_DIR"
    cd "$TEST_DIR"

    # Create test project
    cargo init --name ort_test --quiet
    echo '[dependencies]' >> Cargo.toml
    echo 'ort = "2.0.0-rc.4"' >> Cargo.toml

    cat > src/main.rs << 'EOF'
fn main() {
    println!("ORT_DYLIB_PATH: {:?}", std::env::var("ORT_DYLIB_PATH"));
    match ort::init() {
        Ok(_) => println!("✓ ONNX Runtime initialized successfully!"),
        Err(e) => eprintln!("✗ Failed to initialize ONNX Runtime: {}", e),
    }
}
EOF

    # Run test with timeout; do not fail script on test errors
    set +e
    timeout "${TIMEOUT_SECS}s" cargo run --quiet 2>&1 || echo "Test failed or timed out - you may need to source the environment first"
    set -e

    # Clean up
    cd - > /dev/null
    rm -rf "$TEST_DIR"
else
    echo -e "\nSkipping Rust ONNX Runtime test (ORT_NO_TEST=${ORT_NO_TEST})"
fi

# Instructions
echo -e "\n============================================"
echo -e "Installation Complete!\n"
echo "To use ONNX Runtime in your current session:"
echo "  source ./setup_ort_env.sh"
echo
echo "Or set manually:"
echo "  export ORT_DYLIB_PATH=\"$DLL_PATH\""
echo "  export LD_LIBRARY_PATH=\"${LIB_PATH}:\$LD_LIBRARY_PATH\""
if [ "$OS" = "darwin" ]; then
    echo "  export DYLD_LIBRARY_PATH=\"${LIB_PATH}:\$DYLD_LIBRARY_PATH\""
fi

if [ "$ADD_TO_PROFILE" != "true" ]; then
    echo -e "\nTo make changes permanent, run:"
    echo "  $0 \"$INSTALL_PATH\" true"
fi

echo -e "\nYou can now use real ONNX models in MAGRAY CLI!"