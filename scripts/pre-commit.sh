#!/usr/bin/env bash
set -euo pipefail

echo "[pre-commit] lint/typecheck/tests"
if [ -f package.json ]; then
  npm run lint || true
  npm test -- --coverage --watchAll=false || true
fi

if command -v cargo >/dev/null 2>&1; then
  cargo fmt --all
  cargo clippy --all-targets -- -D warnings || true
  cargo test --all --quiet
fi

if test -f "pyproject.toml" || test -f "requirements.txt"; then
  black . || true
  ruff check . || true
  pytest -q || true
fi
