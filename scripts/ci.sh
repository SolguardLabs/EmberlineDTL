#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"

add_path() {
    if [[ -d "$1" ]]; then
        export PATH="$1:$PATH"
    fi
}

add_path "$HOME/.cargo/bin"
add_path "$HOME/.bun/bin"
add_path "/mnt/c/Program Files/nodejs"

if [[ -d "/mnt/c/Users" ]]; then
    for WINDOWS_HOME in /mnt/c/Users/*; do
        add_path "$WINDOWS_HOME/.cargo/bin"
        add_path "$WINDOWS_HOME/AppData/Roaming/npm"
    done
fi

resolve_cmd() {
    local name="$1"
    local path=""
    if command -v "$name" >/dev/null 2>&1; then
        path="$(command -v "$name")"
        if "$path" --version >/dev/null 2>&1; then
            echo "$path"
            return 0
        fi
    fi
    if command -v "$name.exe" >/dev/null 2>&1; then
        path="$(command -v "$name.exe")"
        if "$path" --version >/dev/null 2>&1; then
            echo "$path"
            return 0
        fi
    fi
    return 1
}

if [[ -z "${CARGO_BIN:-}" ]] && ! CARGO_BIN="$(resolve_cmd cargo)"; then
    echo "No se encontro un cargo ejecutable en PATH." >&2
    exit 127
fi

if [[ -z "${NODE_BIN:-}" ]] && ! NODE_BIN="$(resolve_cmd node)"; then
    echo "No se encontro un node ejecutable en PATH." >&2
    exit 127
fi

if [[ -z "${NPM_BIN:-}" ]] && ! NPM_BIN="$(resolve_cmd npm)"; then
    echo "No se encontro un npm ejecutable en PATH." >&2
    exit 127
fi

"$CARGO_BIN" fmt --all -- --check
"$CARGO_BIN" build --all-targets --locked
"$CARGO_BIN" test --locked
"$CARGO_BIN" clippy --all-targets --all-features --locked -- -D warnings
"$NPM_BIN" run fmt:check
"$NODE_BIN" scripts/check-loc.mjs
"$NODE_BIN" --test "tests/node/*.test.js"
"$NPM_BIN" audit --audit-level=high
"$NODE_BIN" scripts/verify-release.mjs

if [[ "${CI:-}" == "true" ]] && [[ -z "${WSL_DISTRO_NAME:-}" ]] && [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
    echo "La verificacion ha modificado archivos versionados." >&2
    git status --short
    exit 1
fi
