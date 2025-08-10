#!/usr/bin/env bash
set -euo pipefail

log() { echo -e "[$(date +%H:%M:%S)] $*"; }
warn() { echo -e "[$(date +%H:%M:%S)] [warn] $*" >&2; }
err() { echo -e "[$(date +%H:%M:%S)] [error] $*" >&2; }

NON_INTERACTIVE=0
for a in "$@"; do
  case "$a" in
    --non-interactive|--yes|-y) NON_INTERACTIVE=1 ;;
  esac
done

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd -P)"
export PATH="/usr/local/cargo/bin:$HOME/.cargo/bin:$PATH"

log "Bootstrap: Rust toolchain"
if ! command -v cargo >/dev/null 2>&1; then
  if command -v rustup >/dev/null 2>&1; then
    :
  else
    if command -v curl >/dev/null 2>&1; then
      curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal || warn "rustup install failed"
    elif command -v wget >/dev/null 2>&1; then
      wget -qO- https://sh.rustup.rs | sh -s -- -y --profile minimal || warn "rustup install failed"
    else
      warn "Neither curl nor wget available to install rustup"
    fi
  fi
fi
# shellcheck disable=SC1090
. "$HOME/.cargo/env" 2>/dev/null || true

if command -v rustup >/dev/null 2>&1; then
  rustup toolchain install nightly -y --profile minimal || true
  rustup default nightly || true
  rustup component add rustfmt clippy || true
fi

log "Bootstrap: system deps (optional)"
APT_OK=0
if command -v apt-get >/dev/null 2>&1; then
  if [ "${NON_INTERACTIVE}" = "1" ]; then
    if [ "$(id -u)" = "0" ] || (command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null); then
      SUDO=""; [ "$(id -u)" != "0" ] && SUDO="sudo -n"
      $SUDO apt-get update -y || true
      $SUDO apt-get install -y pkg-config libssl-dev ca-certificates || true
      APT_OK=1
    else
      warn "No privileges for apt-get install; skipping system deps"
    fi
  else
    warn "Interactive mode: skip apt-get (use --non-interactive to auto-install)"
  fi
fi

log "Bootstrap: OpenSSL detection"
if command -v pkg-config >/dev/null 2>&1; then
  if pkg-config --exists openssl 2>/dev/null; then
    export OPENSSL_LIB_DIR="$(pkg-config --variable=libdir openssl)"
    export OPENSSL_INCLUDE_DIR="$(pkg-config --variable=includedir openssl)"
    export PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1
    log "OpenSSL via pkg-config detected"
  else
    warn "openssl.pc not found; some crates may require libssl-dev"
  fi
else
  warn "pkg-config not found; skipping OpenSSL autodetect"
fi

log "Bootstrap: ONNX Runtime (optional)"
if [ -x "$ROOT_DIR/crates/cli/setup_ort_env.sh" ]; then
  bash "$ROOT_DIR/crates/cli/setup_ort_env.sh" || true
fi

log "Bootstrap: cargo fetch/check"
if command -v cargo >/dev/null 2>&1; then
  cargo fetch || true
  cargo check -q || true
fi

log "Bootstrap: done"