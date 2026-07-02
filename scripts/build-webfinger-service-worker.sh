#!/usr/bin/env sh
set -eu

# Cloudflare's dashboard/Git deploy image can start as a Node-only environment. Local developers
# usually already have Rust, so keep this bootstrap conditional and let existing toolchains win.
if ! command -v cargo >/dev/null 2>&1; then
    if ! command -v curl >/dev/null 2>&1; then
        echo "cargo is missing and curl is unavailable, so Rust cannot be bootstrapped" >&2
        exit 1
    fi

    rustup_installer="$(mktemp)"
    trap 'rm -f "$rustup_installer"' EXIT HUP INT TERM
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$rustup_installer"
    sh "$rustup_installer" -y --profile minimal --default-toolchain stable

    # rustup writes cargo's environment hook here for non-interactive installs.
    # shellcheck disable=SC1090
    . "${CARGO_HOME:-"$HOME/.cargo"}/env"
fi

# The service compiles to Wasm. This explicit add handles fresh Cloudflare build images.
if command -v rustup >/dev/null 2>&1; then
    rustup target add wasm32-unknown-unknown
else
    echo "rustup is unavailable; assuming wasm32-unknown-unknown is already installed" >&2
fi

worker_build_version="0.8.5"
if ! command -v worker-build >/dev/null 2>&1 ||
    [ "$(worker-build --version)" != "$worker_build_version" ]; then
    cargo install -q worker-build --version "$worker_build_version"
fi

cd webfinger-service-worker
worker-build --release
