#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd -P)"
OUT_DIR="$ROOT_DIR/docs/agent_reports"
mkdir -p "$OUT_DIR"
STAMP="$(date +%Y%m%d_%H%M%S)"
OUT_FILE="$OUT_DIR/report_${STAMP}.md"

section() { echo -e "\n### $*\n" >> "$OUT_FILE"; }
line() { echo "- $*" >> "$OUT_FILE"; }
code() { echo -e "\n\t$*\n" >> "$OUT_FILE"; }

echo "## MAGRAY Agent Report — ${STAMP}" > "$OUT_FILE"
section "Environment"
line "hostname: $(hostname || echo n/a)"
line "kernel: $(uname -r || echo n/a)"
line "rustc: $(rustc --version 2>/dev/null || echo n/a)"
line "cargo: $(cargo --version 2>/dev/null || echo n/a)"

section "Workspace Summary"
line "packages: $(jq -r '.packages|length' <(cargo metadata --no-deps --format-version=1 2>/dev/null) 2>/dev/null || echo n/a)"
line "git branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo n/a)"
line "git status: $(git status --porcelain=v1 2>/dev/null | wc -l | tr -d ' ' || echo n/a) changes"

section "Code Stats"
RS_FILES=$(find "$ROOT_DIR" -type f -name "*.rs" | wc -l | tr -d ' ')
line "*.rs files: ${RS_FILES}"
RUST_LOC=$(find "$ROOT_DIR" -type f -name "*.rs" -print0 | xargs -0 cat | wc -l | tr -d ' ')
line "rust LOC: ${RUST_LOC}"

section "Suspicious Patterns"
if command -v rg >/dev/null 2>&1; then
  {
    echo "panic!:"; rg -n "\bpanic!\(" -g '!target' -S || true; echo;
    echo "unwrap():"; rg -n "\bunwrap\(\)" -g '!target' -S || true; echo;
    echo "expect():"; rg -n "\bexpect\(" -g '!target' -S || true; echo;
    echo "unsafe:"; rg -n "\bunsafe\b" -g '!target' -S || true; echo;
  } | sed 's/^/    /' >> "$OUT_FILE"
else
  echo "rg not found; skipping" >> "$OUT_FILE"
fi

section "Build Health"
{
  echo "cargo fmt --check:"; cargo fmt --all -- --check 2>&1 || true; echo;
  echo "cargo clippy:"; cargo clippy --workspace -q -D warnings 2>&1 || true; echo;
  echo "cargo check:"; cargo check --workspace --all-features 2>&1 || true; echo;
} | sed 's/^/    /' >> "$OUT_FILE"

section "Recommendations"
line "Починить предупреждения clippy и форматирование fmt."
line "Сократить количество unwrap/expect; заменить на надёжную обработку ошибок."
line "Рассмотреть контейнерный smoke-тест (make smoke-cpu) в CI."

echo "Report written to: $OUT_FILE"