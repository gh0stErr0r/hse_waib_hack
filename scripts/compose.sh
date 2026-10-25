#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"
# Read only named boolean flags; never source the secrets file as shell code.
env_flag_enabled() {
  local key="$1" value line pattern enabled=false
  value="$(printenv "$key" 2>/dev/null || true)"
  if [[ -n "$value" ]]; then
    if [[ "$value" == true ]]; then return 0; else return 1; fi
  fi
  pattern="^[[:space:]]*${key}[[:space:]]*=[[:space:]]*(true|false|\"true\"|\"false\"|'true'|'false')[[:space:]]*(#.*)?$"
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%$'\r'}"
    if [[ "$line" =~ $pattern ]]; then
      case "${BASH_REMATCH[1]}" in
        true|\"true\"|\'true\') enabled=true ;;
        *) enabled=false ;;
      esac
    fi
  done < infra/.env
  [[ "$enabled" == true ]]
}

stack=app
if [[ "${1:-}" == presentation || "${1:-}" == app || "${1:-}" == ollama ]]; then
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
  if [[ "$stack" == ollama ]]; then
    options+=(-f infra/docker-compose.ollama.yml)
    if env_flag_enabled OLLAMA_GPU; then
      options+=(-f infra/docker-compose.ollama-gpu.yml)
    fi
  fi
  if env_flag_enabled SMTP_HOST_NETWORK; then
    options+=(-f infra/docker-compose.smtp-host.yml)
  fi
  if env_flag_enabled APP_CUSTOM_DNS; then
    options+=(-f infra/docker-compose.dns.yml)
  fi

fi
if [[ $# == 0 ]]; then set -- up --build -d; fi
exec "${compose[@]}" "${options[@]}" "$@"
