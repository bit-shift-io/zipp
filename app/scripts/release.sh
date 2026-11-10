#!/usr/bin/env bash
# Build versioned release artifacts into dist/. Usage: release.sh [all|android|desktop]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-all}"
case "$MODE" in all|android|desktop) ;; *) echo "Usage: $0 [all|android|desktop]" >&2; exit 2;; esac

VERSION="$(bash "$ROOT/scripts/version.sh")"
mkdir -p "$ROOT/dist"
# Keep only the current version, but allow building each target separately.
find "$ROOT/dist" -maxdepth 1 -type f ! -name "zipp-$VERSION-*" -delete

if [[ "$MODE" == all || "$MODE" == desktop ]]; then
    cargo build --locked --release --manifest-path "$ROOT/rust/Cargo.toml" -p zipp --bin zipp
    install -m 0755 "$ROOT/rust/target/release/zipp" \
        "$ROOT/dist/zipp-$VERSION-linux-$(uname -m)"
fi

if [[ "$MODE" == all || "$MODE" == android ]]; then
    bash "$ROOT/scripts/build.sh" release
    cp "$ROOT/build/app/outputs/apk/release/app-release.apk" \
        "$ROOT/dist/zipp-$VERSION-android-arm64.apk"
fi

echo "release: $VERSION"
ls -1 "$ROOT/dist"
