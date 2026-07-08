#!/usr/bin/env bash
# build.sh — локальная релизная сборка OpenHeart (Linux / macOS)
#
# Использование:
#   ./build.sh linux           # Linux x86_64
#   ./build.sh macos           # macOS Universal (fat dylib)
#   ./build.sh android         # Android arm64 + armeabi-v7a (нужен NDK)
#   ./build.sh web             # Web/WASM (нужен Emscripten 3.1.39 + Rust nightly)
#   ./build.sh linux --skip-export   # только Rust, без Godot экспорта
#   ./build.sh linux --debug         # debug-сборка
#
# Зависимости:
#   - Rust (rustup)
#   - Godot 4.7-stable с экспортными шаблонами
#     Linux/macOS:  https://godotengine.org/download
#
# Android: ANDROID_NDK_HOME должен указывать на NDK r23c (23.2.8568313)
# Web:     установите emsdk 3.1.39, активируйте через `source emsdk_env.sh`

set -euo pipefail

PLATFORM="${1:-}"
ROOT="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$ROOT/rust"
GODOT_DIR="$ROOT/godot"
SKIP_EXPORT=false
DEBUG=false

# ─── Аргументы ────────────────────────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --skip-export) SKIP_EXPORT=true ;;
        --debug)       DEBUG=true ;;
        linux|macos|android|web) PLATFORM="$arg" ;;
    esac
done

if [[ -z "$PLATFORM" ]]; then
    echo "Использование: $0 [linux|macos|android|web] [--skip-export] [--debug]"
    exit 1
fi

# ─── Цвета ────────────────────────────────────────────────────────────────────
C_CYAN='\033[36m'; C_GREEN='\033[32m'; C_YELLOW='\033[33m'; C_RED='\033[31m'; C_RESET='\033[0m'
step()  { echo -e "\n${C_CYAN}[$*]${C_RESET}"; }
ok()    { echo -e "  ${C_GREEN}OK${C_RESET}  $*"; }
warn()  { echo -e "  ${C_YELLOW}!  ${C_RESET}  $*"; }
fail()  { echo -e "  ${C_RED}ERR${C_RESET} $*"; exit 1; }

# ─── Найти Godot ──────────────────────────────────────────────────────────────
find_engine() {
    for cmd in godot godot4 redot; do
        if command -v "$cmd" &>/dev/null; then echo "$cmd"; return 0; fi
    done
    # macOS
    if [[ -f /Applications/Godot.app/Contents/MacOS/Godot ]]; then
        echo /Applications/Godot.app/Contents/MacOS/Godot; return 0
    fi
    return 1
}

CARGO_FLAGS=()
[[ "$DEBUG" == false ]] && CARGO_FLAGS+=(--release)

# ─── Шаг 1: Rust ──────────────────────────────────────────────────────────────

CARGO_SUFFIX=$([ "$DEBUG" = true ] && echo "debug" || echo "release")
step "1/3  Building Rust  ($PLATFORM, $CARGO_SUFFIX)"

mkdir -p "$GODOT_DIR/bin"
cd "$RUST_DIR"

case "$PLATFORM" in
  linux)
    cargo build "${CARGO_FLAGS[@]}"
    cp "target/$CARGO_SUFFIX/libopenheart.so" "$GODOT_DIR/bin/libopenheart.so"
    ok "libopenheart.so → godot/bin/"
    ;;

  macos)
    rustup target add x86_64-apple-darwin aarch64-apple-darwin 2>/dev/null || true
    cargo build "${CARGO_FLAGS[@]}" --target x86_64-apple-darwin
    cargo build "${CARGO_FLAGS[@]}" --target aarch64-apple-darwin
    lipo -create \
      "target/x86_64-apple-darwin/$CARGO_SUFFIX/libopenheart.dylib" \
      "target/aarch64-apple-darwin/$CARGO_SUFFIX/libopenheart.dylib" \
      -output "$GODOT_DIR/bin/libopenheart.dylib"
    ok "libopenheart.dylib (fat) → godot/bin/"
    ;;

  android)
    if ! command -v cargo-ndk &>/dev/null; then
        warn "cargo-ndk не найден. Устанавливаю..."
        cargo install cargo-ndk --locked
    fi
    NDK="${ANDROID_NDK_HOME:-${ANDROID_SDK_ROOT:-}/ndk/23.2.8568313}"
    [[ -d "$NDK" ]] || fail "Android NDK не найден. Укажите ANDROID_NDK_HOME."
    export ANDROID_NDK_HOME="$NDK"
    rustup target add aarch64-linux-android armv7-linux-androideabi 2>/dev/null || true
    mkdir -p "$GODOT_DIR/bin/android"
    cargo ndk -t arm64-v8a -t armeabi-v7a -o "$GODOT_DIR/bin/android" build "${CARGO_FLAGS[@]}"
    ok "libopenheart.so → godot/bin/android/{arm64-v8a,armeabi-v7a}/"
    ;;

  web)
    if ! command -v emcc &>/dev/null; then
        fail "Emscripten не найден. Активируйте emsdk: source /path/to/emsdk/emsdk_env.sh"
    fi
    rustup toolchain install nightly 2>/dev/null || true
    rustup target add wasm32-unknown-emscripten --toolchain nightly 2>/dev/null || true
    mkdir -p "$GODOT_DIR/bin/web"
    EMCC_CFLAGS="-sSIDE_MODULE=2 -O2" \
    cargo +nightly build -Z build-std=std,panic_abort \
        "${CARGO_FLAGS[@]}" \
        --target wasm32-unknown-emscripten
    cp "target/wasm32-unknown-emscripten/$CARGO_SUFFIX/libopenheart.so" \
       "$GODOT_DIR/bin/web/libopenheart.wasm"
    ok "libopenheart.wasm → godot/bin/web/"
    ;;
esac

# ─── Шаг 2: Godot export ──────────────────────────────────────────────────────

if [[ "$SKIP_EXPORT" == true ]]; then
    warn "--skip-export: Godot export пропущен. Rust-библиотека готова."
    exit 0
fi

step "2/3  Godot export  ($PLATFORM)"

ENGINE=$(find_engine 2>/dev/null || true)
if [[ -z "$ENGINE" ]]; then
    warn "Godot 4.7 не найден в PATH или /Applications."
    echo "  Скачайте: https://godotengine.org/download"
    echo "  Или установите через пакетный менеджер:"
    echo "    Ubuntu:  sudo apt install godot   (или flatpak install org.godotengine.Godot)"
    echo "    macOS:   brew install --cask godot"
    echo ""
    echo -e "  ${C_GREEN}Rust-библиотека готова. Откройте godot/project.godot в редакторе.${C_RESET}"
    exit 0
fi
ok "Движок: $ENGINE"

cd "$GODOT_DIR"

case "$PLATFORM" in
  linux)
    mkdir -p dist/linux
    "$ENGINE" --headless --export-release "Linux/X11" dist/linux/OpenHeart.x86_64
    ;;
  macos)
    mkdir -p dist/macos
    "$ENGINE" --headless --export-release "macOS" dist/macos/OpenHeart.zip
    ;;
  android)
    mkdir -p dist/android
    "$ENGINE" --headless --export-release "Android" dist/android/OpenHeart.apk
    ;;
  web)
    mkdir -p dist/web
    "$ENGINE" --headless --export-release "Web" dist/web/index.html
    ;;
esac

# ─── Шаг 3: Итог ──────────────────────────────────────────────────────────────

step "3/3  Done"
case "$PLATFORM" in
  linux)   ok "Сборка: godot/dist/linux/OpenHeart.x86_64" ;;
  macos)   ok "Сборка: godot/dist/macos/OpenHeart.zip" ;;
  android) ok "Сборка: godot/dist/android/OpenHeart.apk" ;;
  web)     ok "Сборка: godot/dist/web/index.html" ;;
esac
