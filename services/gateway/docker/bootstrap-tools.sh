#!/bin/sh
set -eu

cache_dir="${BUILD_TOOLS_CACHE:-/root/.cache/gastro-build-tools}"
mkdir -p "$cache_dir"

verify() {
    printf '%s  %s\n' "$2" "$1" | sha512sum --check --status
}

fetch() {
    name="$1"
    url="$2"
    checksum="$3"
    target="$cache_dir/$name"
    if [ -f "$target" ] && verify "$target" "$checksum"; then
        printf 'Using verified cached %s\n' "$name"
        return
    fi
    printf 'Downloading %s\n' "$name"
    curl --fail --location --show-error --silent \
        --retry 3 --retry-all-errors --retry-delay 2 --retry-max-time 180 \
        --connect-timeout 15 --max-time 90 \
        "$url" --output "$target.part"
    if ! verify "$target.part" "$checksum"; then
        printf 'SHA-512 verification failed for %s\n' "$name" >&2
        exit 1
    fi
    mv "$target.part" "$target"
}

# Build Hex with the image's compiler: old precompiled BEAM archives can be
# incompatible with newer OTP patch releases. Production Hex has no dependencies.
fetch hex-2.6.0-source.tar.gz \
    "${HEX_SOURCE_URL:-https://github.com/hexpm/hex/archive/refs/tags/v2.6.0.tar.gz}" \
    ab3528a1029bdbac7b6d3cbd798209013935bf1e257b12f2b41c2c8dfe1519f309843ad117a9c8c0fe926177858753d2ca0f3d93b1558cd530d3061715cde07e
# Pin the official portable escript, rather than fetching an installer manifest.
fetch rebar3-3.25.1 \
    "${REBAR3_ARCHIVE_URL:-https://github.com/erlang/rebar3/releases/download/3.25.1/rebar3}" \
    69073f6ad163f74971545015238614c327893960c1b3f26df5377df135c773a0716b48b65c2a48cef878f185dd92805abc69894adfa3fd27a90c62a64ba371e2

work_dir="$(mktemp -d)"
trap 'rm -rf -- "$work_dir"' EXIT
tar -xzf "$cache_dir/hex-2.6.0-source.tar.gz" -C "$work_dir" --strip-components=1
(cd "$work_dir" && MIX_ENV=prod mix archive.build -o "$work_dir/hex.ez")
mix archive.install "$work_dir/hex.ez" --force
install -m 0755 "$cache_dir/rebar3-3.25.1" /usr/local/bin/rebar3
/usr/local/bin/rebar3 version
