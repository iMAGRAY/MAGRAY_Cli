#!/usr/bin/env bash
set -euo pipefail

INTERVAL=30
LOG_DIR="logs"

# Parse args
while [[ "${1-}" != "" ]]; do
  case "$1" in
    -i|--interval)
      shift
      INTERVAL=${1:-30}
      ;;
    -h|--help)
      echo "Usage: $0 [--interval N]"
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 1
      ;;
  esac
  shift || true
done

mkdir -p "$LOG_DIR"

summarize_file() {
  local file_name="$1"
  local title="$2"
  local path="$LOG_DIR/$file_name"

  if [[ -f "$path" ]]; then
    local mtime
    if mtime=$(date -Is -r "$path" 2>/dev/null); then :; else mtime=$(stat -c %y "$path" 2>/dev/null || echo "unknown"); fi
    local last_result
    last_result=$(grep -a 'test result:' "$path" | tail -n 1 | tr -d '\r' || true)
    local last_error
    last_error=$(grep -aE 'error: (could not compile|linking with|test failed)' "$path" | tail -n 2 | tr -d '\r' || true)
    local last_panic
    last_panic=$(grep -aE 'panicked at' "$path" | tail -n 1 | tr -d '\r' || true)

    echo "• $title (mtime: $mtime)"
    if [[ -n "$last_result" ]]; then
      echo "  last: $last_result"
    else
      echo "  last: -"
    fi
    if [[ -n "$last_error" ]]; then
      echo "  err:  $(echo "$last_error" | tr '\n' ' ' | sed 's/  */ /g')"
    fi
    if [[ -n "$last_panic" ]]; then
      echo "  panic: $last_panic"
    fi
  else
    echo "• $title: not found"
  fi
}

while true; do
  echo "=== CI Watchdog $(date -Is) ==="
  summarize_file "ci-local-latest.log" "ci-local"
  summarize_file "ci-local-all-latest.log" "ci-local-all"
  summarize_file "ci-local-persistence-latest.log" "ci-local-persistence"
  summarize_file "llm-advanced-latest.log" "llm-advanced"
  echo ""
  sleep "$INTERVAL"
done