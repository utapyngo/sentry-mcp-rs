#!/bin/bash
SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
cd "$SCRIPT_DIR" || exit 1
if [ -f "$SCRIPT_DIR/.env" ]; then
    set -a
    source "$SCRIPT_DIR/.env"
    set +a
fi
BINARY="./target/release/sentry-rs"
[ -f "$BINARY" ] || cargo build --release
exec "$BINARY" "$@"
