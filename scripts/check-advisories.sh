#!/usr/bin/env bash
# Checks Cargo.lock against the RustSec advisory database with a pinned, checksum-verified cargo-deny.
# Set CARGO_DENY to an existing cargo-deny binary to skip the download.
set -euo pipefail

version=0.22.0
digest=98c56e6a408bfa5c3c60744b6015aaa6aa08f743fdeb88386d06fca035bb1786

cd "$(dirname "${BASH_SOURCE[0]}")/.."
if [[ -z "${CARGO_DENY:-}" ]]; then
    if [[ "$(uname -sm)" != "Linux x86_64" ]]; then
        echo "Set CARGO_DENY to a cargo-deny $version binary on this platform." >&2
        exit 2
    fi
    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT
    archive="$work/cargo-deny.tar.gz"
    curl -fsSL --proto '=https' --tlsv1.2 -o "$archive" \
        "https://github.com/EmbarkStudios/cargo-deny/releases/download/$version/cargo-deny-$version-x86_64-unknown-linux-musl.tar.gz"
    echo "$digest  $archive" | sha256sum --check --quiet -
    tar -xzf "$archive" -C "$work" --strip-components=1
    CARGO_DENY="$work/cargo-deny"
fi
"$CARGO_DENY" --locked check advisories
