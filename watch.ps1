# watch.ps1 — авто-пересборка при изменении .rs файлов
# Требует cargo-watch: cargo install cargo-watch
# Использование: .\watch.ps1

Set-Location "$PSScriptRoot\rust"

$cargoWatch = Get-Command "cargo-watch" -ErrorAction SilentlyContinue
if (-not $cargoWatch) {
    Write-Host "cargo-watch не установлен. Устанавливаю..." -ForegroundColor Yellow
    cargo install cargo-watch
    if ($LASTEXITCODE -ne 0) { exit 1 }
}

Write-Host "[watch] Слежу за src/*.rs — пересобираю при изменениях." -ForegroundColor Cyan
Write-Host "        После сборки переключись в редактор Redot/Godot — DLL перезагрузится автоматически." -ForegroundColor Gray
Write-Host "        Ctrl+C для остановки." -ForegroundColor Gray
Write-Host ""

cargo watch -x build -w src
