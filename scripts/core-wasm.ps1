# Сборка ядра игры для сервера: openheart-core → core.wasm.
#
# Тот же код, что линкуется в клиент, но собранный под wasm — его грузит oh-server
# через wazero и крутит авторитетный тик (docs/MULTIPLAYER.md §4).
#
#   .\scripts\core-wasm.ps1            # release, копия в server\core.wasm
#   .\scripts\core-wasm.ps1 -Debug     # без оптимизаций

param([switch]$Debug)

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent

if (-not (rustup target list --installed | Select-String -Quiet "wasm32-unknown-unknown")) {
    Write-Host "[core] ставлю таргет wasm32-unknown-unknown..." -ForegroundColor Yellow
    rustup target add wasm32-unknown-unknown
}

$profile = if ($Debug) { "debug" } else { "release" }
$cargoArgs = @("build", "-p", "openheart-core", "--target", "wasm32-unknown-unknown", "--features", "abi")
if (-not $Debug) { $cargoArgs += "--release" }

Push-Location $Root
try {
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
    Pop-Location
}

$out = "$Root\target\wasm32-unknown-unknown\$profile\openheart_core.wasm"
Copy-Item $out "$Root\server\core.wasm" -Force

$size = [math]::Round((Get-Item $out).Length / 1KB)
Write-Host "[OK] core.wasm ($size КБ) → server\core.wasm" -ForegroundColor Green
Write-Host "     запуск: cd server; go run ./cmd/oh-server -core core.wasm" -ForegroundColor Gray
