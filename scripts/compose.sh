#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
stack=app
if [[ "${1:-}" == presentation || "${1:-}" == app ]]; then
  stack="$1"
  shift
fi
if [[ -n "${COMPOSE_BIN:-}" ]]; then
  compose=("$COMPOSE_BIN")
elif docker compose version >/dev/null 2>&1; then
  compose=(docker compose)
elif command -v docker-compose >/dev/null 2>&1 && [[ "$(docker-compose version --short 2>/dev/null)" == v2.* || "$(docker-compose version --short 2>/dev/null)" == 2.* ]]; then
  compose=(docker-compose)
elif [[ -x "$project_root/infra/bin/docker-compose" ]]; then
  compose=("$project_root/infra/bin/docker-compose")
else
  printf '%s\n' 'Docker Compose v2 не найден. Установите локально: bash scripts/install-compose.sh' >&2
  exit 127
fi
if [[ "$stack" == presentation ]]; then
  options=(-f infra/docker-compose.presentation.yml)
else
  if [[ ! -f infra/.env ]]; then
    printf '%s\n' 'Нет infra/.env. Выполните: node scripts/setup-gastro.mjs' >&2
    exit 1
  fi
  options=(--env-file infra/.env -f infra/docker-compose.gastro.yml)
  smtp_host_network="${SMTP_HOST_NETWORK:-}"
  if [[ -z "$smtp_host_network" ]]; then
    while IFS= read -r line || [[ -n "$line" ]]; do
      line="${line%$'\r'}"
      case "$line" in
        SMTP_HOST_NETWORK=true|SMTP_HOST_NETWORK=\"true\"|SMTP_HOST_NETWORK=\'true\') smtp_host_network=true ;;
      esac
    done < infra/.env
  fi
  if [[ "$smtp_host_network" == true ]]; then
    options+=(-f infra/docker-compose.smtp-host.yml)
  fi

fi
if [[ $# == 0 ]]; then set -- up --build -d; fi
exec "${compose[@]}" "${options[@]}" "$@"
