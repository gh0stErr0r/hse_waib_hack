#!/bin/sh
set -eu
attempt=1
until "$@"; do
    if [ "$attempt" -ge 3 ]; then
        printf 'Command failed after %s attempts\n' "$attempt" >&2
        exit 1
    fi
    printf 'Retrying after failed attempt %s/3\n' "$attempt" >&2
    sleep "$((attempt * 2))"
    attempt="$((attempt + 1))"
done
