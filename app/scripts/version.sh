#!/usr/bin/env bash
# Print the zipp package version. Single source of truth: rust/Cargo.toml.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cargo metadata --locked --format-version 1 --no-deps \
    --manifest-path "$ROOT/rust/Cargo.toml" \
    | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "zipp"))'
