#!/usr/bin/env bash
# Installs the official standalone Compose binary inside this repository only.
set -euo pipefail
project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
version=v2.40.3
if [[ "$(uname -s)" != Linux ]]; then
  printf '%s\n' 'Этот установщик предназначен для Linux. На macOS/Windows используйте Docker Desktop с Compose v2.' >&2
  exit 1
fi
case "$(uname -m)" in
  x86_64|aarch64) architecture="$(uname -m)" ;;
  arm64) architecture=aarch64 ;;
  *) printf '%s\n' 'Неподдерживаемая архитектура процессора' >&2; exit 1 ;;
esac
asset="docker-compose-linux-$architecture"
url="https://github.com/docker/compose/releases/download/$version"
cache_dir="$(mktemp -d "${TMPDIR:-/tmp}/gastro-compose.XXXXXX")"
trap 'rm -rf -- "$cache_dir"' EXIT
curl --fail --location --retry 2 --connect-timeout 10 "$url/$asset" --output "$cache_dir/$asset"
curl --fail --location --retry 2 --connect-timeout 10 "$url/checksums.txt" --output "$cache_dir/checksums.txt"
awk -v name="$asset" '$2 == name || $2 == "*" name {print}' "$cache_dir/checksums.txt" > "$cache_dir/selected-checksum.txt"
if [[ ! -s "$cache_dir/selected-checksum.txt" ]]; then
  printf '%s\n' 'В официальном списке не найдена контрольная сумма Compose' >&2
  exit 1
fi
(cd "$cache_dir" && sha256sum --check selected-checksum.txt)
mkdir -p "$project_root/infra/bin"
install -m 0755 "$cache_dir/$asset" "$project_root/infra/bin/docker-compose"
"$project_root/infra/bin/docker-compose" version
printf '%s\n' 'Compose установлен локально. Запуск презентации: bash scripts/compose.sh presentation up --build -d'
