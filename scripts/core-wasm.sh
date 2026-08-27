#!/usr/bin/env bash
# Сборка ядра игры для сервера: openheart-core → core.wasm.
#
# Тот же код, что линкуется в клиент, но собранный под wasm — его грузит oh-server
# через wazero и крутит авторитетный тик (docs/MULTIPLAYER.md §4).
#
#   ./scripts/core-wasm.sh          # release, копия в server/core.wasm
#   ./scripts/core-wasm.sh --debug  # без оптимизаций
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="release"
CARGO_FLAGS=(--release)
if [[ "${1:-}" == "--debug" ]]; then
    PROFILE="debug"
    CARGO_FLAGS=()
fi

if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "[core] ставлю таргет wasm32-unknown-unknown..."
    rustup target add wasm32-unknown-unknown
fi

cd "$ROOT"
cargo build -p openheart-core --target wasm32-unknown-unknown --features abi "${CARGO_FLAGS[@]}"

OUT="$ROOT/target/wasm32-unknown-unknown/$PROFILE/openheart_core.wasm"
cp "$OUT" "$ROOT/server/core.wasm"

SIZE=$(( $(wc -c < "$OUT") / 1024 ))
echo "[OK] core.wasm (${SIZE} КБ) → server/core.wasm"
echo "     запуск: cd server && go run ./cmd/oh-server -core core.wasm"
