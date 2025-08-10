#!/usr/bin/env bash
set -euo pipefail

INTERVAL=30
LOG_DIR="logs"
SCHEDULE_MINUTES=0 # 0 = disabled

# Parse args
while [[ "${1-}" != "" ]]; do
  case "$1" in
    -i|--interval)
      shift
      INTERVAL=${1:-30}
      ;;
    --schedule-all-min)
      shift
      SCHEDULE_MINUTES=${1:-0}
      ;;
    -h|--help)
      echo "Usage: $0 [--interval N] [--schedule-all-min M]"
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

should_trigger_all() {
  # return 0 if should trigger
  [[ "$SCHEDULE_MINUTES" -gt 0 ]] || return 1
  local latest="$LOG_DIR/ci-local-all-latest.log"
  # If no log yet, trigger
  [[ -f "$latest" ]] || return 0
  # If recent run active, skip
  if pgrep -af "cargo test .*ci-local-all" >/dev/null 2>&1; then return 1; fi
  # If log older than SCHEDULE_MINUTES, trigger
  local now=$(date +%s)
  local mtime=$(stat -c %Y "$latest" 2>/dev/null || echo 0)
  local age=$(( now - mtime ))
  local threshold=$(( SCHEDULE_MINUTES * 60 ))
  [[ "$age" -ge "$threshold" ]]
}

trigger_all_if_needed() {
  if should_trigger_all; then
    local ts=$(date +%Y%m%d_%H%M%S)
    local lock="$LOG_DIR/.ci-all.lock"
    if mkdir "$lock" 2>/dev/null; then
      (
        set -o pipefail
        echo "=== ci-local-all SCHEDULED $(date -Is) ===" | tee -a "$LOG_DIR/ci-local-all_${ts}.log"
        CI=1 MAGRAY_NO_ANIM=1 MAGRAY_SKIP_AUTO_INSTALL=1 MAGRAY_FORCE_NO_ORT=1 timeout 1200s \
        cargo test -q --features="cpu,extended-tests,orchestrated-search,keyword-search,hnsw-index" --tests -- --nocapture \
        2>&1 | tee -a "$LOG_DIR/ci-local-all_${ts}.log" || true
        ln -sf "ci-local-all_${ts}.log" "$LOG_DIR/ci-local-all-latest.log"
        rmdir "$lock" || true
      ) &
      disown || true
    fi
  fi
}

while true; do
  echo "=== CI Watchdog $(date -Is) ==="
  summarize_file "ci-local-latest.log" "ci-local"
  summarize_file "ci-local-all-latest.log" "ci-local-all"
  summarize_file "ci-local-persistence-latest.log" "ci-local-persistence"
  summarize_file "llm-advanced-latest.log" "llm-advanced"
  echo ""
  trigger_all_if_needed
  sleep "$INTERVAL"
done