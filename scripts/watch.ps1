# watch.ps1 — авто-пересборка при изменении .rs файлов
# Требует cargo-watch: cargo install cargo-watch
# Использование: .\watch.ps1

$Root = Split-Path $PSScriptRoot -Parent   # скрипты лежат в scripts/
New-Item -ItemType Directory -Force -Path "$Root\game\bin" | Out-Null
Set-Location $Root

$cargoWatch = Get-Command "cargo-watch" -ErrorAction SilentlyContinue
if (-not $cargoWatch) {
    Write-Host "cargo-watch не установлен. Устанавливаю..." -ForegroundColor Yellow
    cargo install cargo-watch
    if ($LASTEXITCODE -ne 0) { exit 1 }
}

Write-Host "[watch] Слежу за src/*.rs — пересобираю при изменениях." -ForegroundColor Cyan
Write-Host "        DLL копируется в godot/bin; после сборки переключись в редактор Redot/Godot — она перезагрузится автоматически." -ForegroundColor Gray
Write-Host "        Ctrl+C для остановки." -ForegroundColor Gray
Write-Host ""

# --use-shell=cmd: на Windows cargo-watch >=7.8 по умолчанию гонит -s через
# PowerShell, где `&&` (PS 5.1) и `copy /Y` не работают.
cargo watch -w client/src -w core/src --use-shell=cmd -s "cargo build -p openheart && copy /Y target\debug\openheart.dll ..\godot\bin\openheart.dll"
