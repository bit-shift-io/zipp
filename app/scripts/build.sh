#!/usr/bin/env bash
# arm64 Android build. Run with bash scripts/build.sh [debug|release].
# Adapted from qlaunch (bitshift, MIT).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-release}"
case "$MODE" in debug|release) ;; *) echo "Usage: $0 [debug|release]" >&2; exit 2;; esac
if [[ -f "$ROOT/android/gradlew" && -f "$ROOT/android/gradle/wrapper/gradle-wrapper.jar" ]]; then
    GRADLE=(bash "$ROOT/android/gradlew")
elif command -v gradle >/dev/null; then
    GRADLE=(gradle)
else
    echo "Install Gradle 9.8.1 or restore the Android Gradle wrapper." >&2
    exit 1
fi
# Gradle/AGP need a full JDK (one that ships javac); a JRE-only install fails
# with "Toolchain installation ... does not provide the required capabilities:
# [JAVA_COMPILER]". This build is launched from many contexts that never source
# the shell rc files - notably grit, which runs it from a systemd user service -
# so JAVA_HOME can be unset or point at a bare JRE, and Gradle then falls back to
# the java on PATH (Fedora's /usr/lib/jvm/java-25-openjdk, headless/JRE-only).
# Detect a real JDK here so the build is self-sufficient regardless of caller.
is_jdk() { [[ -n "${1:-}" && -x "$1/bin/java" && -x "$1/bin/javac" ]]; }
if ! is_jdk "${JAVA_HOME:-}"; then
    unset JAVA_HOME
    for candidate in \
        "$HOME/Android/android-studio/jbr" \
        "/opt/jbr-21" \
        "/usr/lib/jvm/java-21-openjdk" \
        "/usr/lib/jvm/java-17-openjdk" \
        "/usr/lib/jvm/java-latest-openjdk" \
        "/usr/lib/jvm/java-25-openjdk"; do
        if is_jdk "$candidate"; then
            export JAVA_HOME="$candidate"
            break
        fi
    done
fi
if ! is_jdk "${JAVA_HOME:-}"; then
    jdk_bin="$(command -v javac 2>/dev/null || true)"
    if [[ -n "$jdk_bin" ]]; then
        export JAVA_HOME="$(dirname "$(dirname "$(readlink -f "$jdk_bin")")")"
    fi
fi
if ! is_jdk "${JAVA_HOME:-}"; then
    echo "No JDK with javac found (JAVA_HOME=${JAVA_HOME:-unset}). Install one:" >&2
    echo "  sudo dnf install java-latest-openjdk-devel   # Fedora" >&2
    echo "  sudo apt install default-jdk                  # Debian/Ubuntu" >&2
    exit 1
fi

export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/28.2.13676358}"
TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
if [[ ! -x "$TOOLCHAIN/clang" ]]; then
    echo "Incomplete NDK at $ANDROID_NDK_HOME (clang is missing)." >&2
    echo "Set ANDROID_NDK_HOME to a complete NDK; this build uses 28.2.13676358." >&2
    exit 1
fi
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN/aarch64-linux-android34-clang"
export CC_aarch64_linux_android="$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
export AR_aarch64_linux_android="$TOOLCHAIN/llvm-ar"
# Retain compatibility with Android devices using 16 KiB pages.
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"
FLAGS=()
if [[ "$MODE" == release ]]; then FLAGS+=(--release); TASK=assembleRelease; else TASK=assembleDebug; fi
cargo build --locked --manifest-path "$ROOT/rust/Cargo.toml" -p zipp --lib \
    --target aarch64-linux-android "${FLAGS[@]}"
mkdir -p "$ROOT/android/app/src/main/jniLibs/arm64-v8a"
cp "$ROOT/rust/target/aarch64-linux-android/$MODE/libzipp.so" \
    "$ROOT/android/app/src/main/jniLibs/arm64-v8a/"
JAVA_SRC="$(cargo metadata --locked --format-version 1 --filter-platform aarch64-linux-android --manifest-path "$ROOT/rust/Cargo.toml" | \
    python3 -c 'import json,sys,pathlib; p=next(p for p in json.load(sys.stdin)["packages"] if p["name"]=="iced_android"); print(pathlib.Path(p["manifest_path"]).parent / "java")')"
# The workspace Cargo version is the single source of truth; Android's
# versionCode falls out of YYYY.MM.DD as YYYYMMDD.
VERSION="$(bash "$ROOT/scripts/version.sh")"
IFS=. read -r V_MAJOR V_MINOR V_PATCH <<<"$VERSION"
VERSION_CODE=$((10#$V_MAJOR * 10000 + 10#$V_MINOR * 100 + 10#$V_PATCH))
"${GRADLE[@]}" -p "$ROOT/android" "-PicedAndroidJava=$JAVA_SRC" \
    "-PzippVersionName=$VERSION" "-PzippVersionCode=$VERSION_CODE" "$TASK"
