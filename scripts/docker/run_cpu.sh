#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd -P)"
repo_root="$(cd "$here/../.." && pwd -P)"
compose_file="$repo_root/scripts/docker/docker-compose.yml"
dockerfile_cpu="$repo_root/scripts/docker/Dockerfile.cpu"
image_cpu="magray:cpu"

has_cmd() { command -v "$1" >/dev/null 2>&1; }

run_compose() {
  echo "[info] Using docker compose with file: $compose_file"
  (cd "$repo_root/scripts/docker" && docker compose --profile cpu build)
  (cd "$repo_root/scripts/docker" && docker compose --profile cpu up -d)
  (cd "$repo_root/scripts/docker" && docker compose ps)
  echo "[info] Smoke test: magray --version"
  (cd "$repo_root/scripts/docker" && docker compose --profile cpu exec -T magray-cpu /usr/local/bin/magray --version)
  echo "[info] Smoke test: magray status (may return non-zero if endpoints unavailable)"
  (cd "$repo_root/scripts/docker" && docker compose --profile cpu exec -T magray-cpu /usr/local/bin/magray status || true)
}

run_direct() {
  echo "[warn] docker compose not available, falling back to direct build/run"
  (cd "$repo_root" && docker build -t "$image_cpu" -f "$dockerfile_cpu" .)
  echo "[info] Smoke test: magray --version"
  docker run --rm "$image_cpu" /usr/local/bin/magray --version
  echo "[info] Help preview"
  docker run --rm "$image_cpu" /usr/local/bin/magray --help | head -n 20 || true
}

main() {
  if ! has_cmd docker; then
    echo "[error] docker not found. Please install Docker and start the daemon." >&2
    exit 127
  fi
  if docker compose version >/dev/null 2>&1; then
    run_compose
  elif has_cmd docker-compose; then
    echo "[info] Using legacy docker-compose"
    (cd "$repo_root/scripts/docker" && docker-compose --profile cpu build)
    (cd "$repo_root/scripts/docker" && docker-compose --profile cpu up -d)
    (cd "$repo_root/scripts/docker" && docker-compose ps)
    (cd "$repo_root/scripts/docker" && docker-compose --profile cpu exec -T magray-cpu /usr/local/bin/magray --version)
    (cd "$repo_root/scripts/docker" && docker-compose --profile cpu exec -T magray-cpu /usr/local/bin/magray status || true)
  else
    run_direct
  fi
}

main "$@"