#!/usr/bin/env bash
# Publish dist/ artifacts as a GitHub release. Usage: publish.sh [--draft]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

DRAFT=0
for arg in "$@"; do
    case "$arg" in
        --draft) DRAFT=1 ;;
        -h|--help) echo "Usage: $0 [--draft]"; exit 0 ;;
        *) echo "Usage: $0 [--draft]" >&2; exit 2 ;;
    esac
done

if ! command -v gh >/dev/null 2>&1; then
    echo "publish: gh not installed; see https://cli.github.com" >&2
    exit 1
fi

# Validate everything local before any network/auth call, so an empty or
# wrong-version dist/ fails with an actionable message even offline.
VERSION="$(bash "$ROOT/scripts/version.sh")"
TAG="v$VERSION"
shopt -s nullglob
ASSETS=("$ROOT/dist/zipp-$VERSION-"*)
if [ "${#ASSETS[@]}" -eq 0 ]; then
    echo "publish: no dist/ artifacts for $VERSION; run scripts/release.sh first" >&2
    exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
    echo "publish: gh is not authenticated; run 'gh auth login'" >&2
    exit 1
fi

if ! git -C "$ROOT" rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
    git -C "$ROOT" tag -a "$TAG" -m "Zipp $VERSION"
fi
git -C "$ROOT" push origin "$TAG"

ARGS=("$TAG" "${ASSETS[@]}" --title "Zipp $VERSION" --generate-notes --verify-tag)
if [ "$DRAFT" -eq 1 ]; then ARGS+=(--draft); fi
gh release create "${ARGS[@]}"
