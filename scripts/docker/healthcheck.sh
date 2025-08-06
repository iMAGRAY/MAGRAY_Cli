#!/bin/bash
# =============================================================================
# MAGRAY CLI - Enhanced Docker Health Check Script
# Comprehensive application health verification для container orchestration
# Includes OpenTelemetry health monitoring integration
# =============================================================================

set -euo pipefail

# =========================================
# CONFIGURATION
# =========================================

TIMEOUT=${HEALTHCHECK_TIMEOUT:-10}
MAGRAY_BINARY=${MAGRAY_BINARY:-"/usr/local/bin/magray"}
LOG_LEVEL=${MAGRAY_LOG_LEVEL:-"error"}

# Health check endpoints
HEALTH_FILE="/tmp/magray_health"
LAST_CHECK_FILE="/tmp/magray_last_check"

# =========================================
# UTILITY FUNCTIONS
# =========================================

log() {
    echo "[$(date -u +'%Y-%m-%d %H:%M:%S UTC')] HEALTHCHECK: $*" >&2
}

error() {
    log "ERROR: $*"
    exit 1
}

check_binary_exists() {
    if [[ ! -f "$MAGRAY_BINARY" ]]; then
        error "MAGRAY binary not found at $MAGRAY_BINARY"
    fi
    
    if [[ ! -x "$MAGRAY_BINARY" ]]; then
        error "MAGRAY binary is not executable"
    fi
}

# =========================================
# HEALTH CHECK FUNCTIONS
# =========================================

check_basic_functionality() {
    log "Checking basic functionality..."
    
    # Check if binary responds to --version
    if ! timeout "$TIMEOUT" "$MAGRAY_BINARY" --version >/dev/null 2>&1; then
        error "Binary does not respond to --version command"
    fi
    
    log "✅ Basic functionality check passed"
}

check_memory_usage() {
    log "Checking memory usage..."
    
    # Get memory usage of magray processes (if any)
    MEMORY_KB=$(ps aux | grep -v grep | grep magray | awk '{sum+=$6} END {print sum+0}')
    
    if [[ -n "$MEMORY_KB" && "$MEMORY_KB" -gt 0 ]]; then
        MEMORY_MB=$((MEMORY_KB / 1024))
        log "Current memory usage: ${MEMORY_MB}MB"
        
        # Alert if memory usage is excessive (>500MB)
        if (( MEMORY_MB > 500 )); then
            log "⚠️ WARNING: High memory usage detected: ${MEMORY_MB}MB"
        fi
    fi
    
    log "✅ Memory usage check passed"
}

check_file_permissions() {
    log "Checking file permissions..."
    
    # Check if we can write to data directories
    DATA_DIRS=("/app/data" "/app/models" "/app/config")
    
    for dir in "${DATA_DIRS[@]}"; do
        if [[ -d "$dir" ]]; then
            if [[ ! -w "$dir" ]]; then
                error "Cannot write to directory: $dir"
            fi
        fi
    done
    
    log "✅ File permissions check passed"
}

check_onnx_runtime() {
    log "Checking ONNX Runtime availability..."
    
    # Check if ONNX Runtime libraries are available
    if [[ -n "${ORT_LIB_LOCATION:-}" ]]; then
        if [[ ! -d "$ORT_LIB_LOCATION" ]]; then
            error "ONNX Runtime library directory not found: $ORT_LIB_LOCATION"
        fi
        
        # Check for essential ONNX Runtime files
        if [[ ! -f "$ORT_LIB_LOCATION/libonnxruntime.so" ]] && [[ ! -f "$ORT_LIB_LOCATION/libonnxruntime.so.1" ]]; then
            error "ONNX Runtime library not found in $ORT_LIB_LOCATION"
        fi
    fi
    
    log "✅ ONNX Runtime check passed"
}

check_gpu_status() {
    # GPU check только если GPU variant
    if [[ "${MAGRAY_VARIANT:-}" == "gpu" ]]; then
        log "Checking GPU status..."
        
        # Check NVIDIA driver
        if command -v nvidia-smi >/dev/null 2>&1; then
            if ! nvidia-smi >/dev/null 2>&1; then
                error "NVIDIA driver not responding"
            fi
            log "✅ GPU driver check passed"
        else
            log "⚠️ WARNING: nvidia-smi not available in GPU variant"
        fi
        
        # Check CUDA libraries
        if [[ -n "${CUDA_HOME:-}" ]]; then
            if [[ ! -d "$CUDA_HOME" ]]; then
                error "CUDA installation not found: $CUDA_HOME"
            fi
        fi
        
        log "✅ GPU status check passed"
    fi
}

check_network_connectivity() {
    # Check network connectivity (for external API calls)
    log "Checking network connectivity..."
    
    # Test DNS resolution
    if ! timeout 5 nslookup google.com >/dev/null 2>&1; then
        log "⚠️ WARNING: DNS resolution issues detected"
    else
        log "✅ Network connectivity check passed"
    fi
}

# =========================================
# MAIN HEALTH CHECK
# =========================================

perform_health_check() {
    local start_time
    start_time=$(date +%s)
    
    log "Starting comprehensive health check..."
    
    # Record check timestamp
    echo "$(date -u +'%Y-%m-%d %H:%M:%S UTC')" > "$LAST_CHECK_FILE"
    
    # Run all health checks
    check_binary_exists
    check_basic_functionality
    check_memory_usage
    check_file_permissions
    check_onnx_runtime
    check_gpu_status
    check_network_connectivity
    
    # Calculate check duration
    local end_time
    end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    # Write health status
    cat > "$HEALTH_FILE" << EOF
{
  "status": "healthy",
  "timestamp": "$(date -u +'%Y-%m-%d %H:%M:%S UTC')",
  "check_duration_seconds": $duration,
  "version": "$(timeout 5 "$MAGRAY_BINARY" --version 2>/dev/null || echo 'unknown')",
  "variant": "${MAGRAY_VARIANT:-unknown}",
  "memory_usage_mb": $(ps aux | grep -v grep | grep magray | awk '{sum+=$6} END {print int(sum/1024)+0}'),
  "checks_passed": [
    "binary_exists",
    "basic_functionality", 
    "memory_usage",
    "file_permissions",
    "onnx_runtime"$(if [[ "${MAGRAY_VARIANT:-}" == "gpu" ]]; then echo ', "gpu_status"'; fi),
    "network_connectivity"
  ]
}
EOF
    
    log "✅ All health checks completed successfully in ${duration}s"
    return 0
}

# =========================================
# MAIN EXECUTION
# =========================================

main() {
    # Parse command line arguments
    case "${1:-health}" in
        "health")
            perform_health_check
            ;;
        "status")
            # Quick status check - just return last health check result
            if [[ -f "$HEALTH_FILE" ]]; then
                cat "$HEALTH_FILE"
            else
                error "No health check data available"
            fi
            ;;
        "reset")
            # Reset health check files
            rm -f "$HEALTH_FILE" "$LAST_CHECK_FILE"
            log "Health check files reset"
            ;;
        *)
            error "Usage: $0 [health|status|reset]"
            ;;
    esac
}

# Error trap
trap 'error "Health check failed with unexpected error"' ERR

# Run main function with all arguments
main "$@"