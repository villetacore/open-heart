# run.ps1 - build Rust and launch the Redot/Godot EDITOR (for content editing)
# Usage: .\run.ps1                     # build DLL and open the EDITOR (OpenHeart tab; F5 to play)
#   or:  .\run.ps1 -Game               # launch the GAME directly, without the editor
#   or:  .\run.ps1 -Engine "C:\path\to\godot.exe"

param(
    [string]$Engine = "",
    [switch]$Game        # -Game launches the game directly (default opens the editor)
)

Set-Location $PSScriptRoot

# 1. Build Rust
Write-Host "[1/2] Building Rust..." -ForegroundColor Cyan
Push-Location "$PSScriptRoot\rust"
cargo build
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Rust build failed" -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location

# DLL lives inside the project (godot/bin) so the game export can bundle it
New-Item -ItemType Directory -Force -Path "$PSScriptRoot\godot\bin" | Out-Null
Copy-Item "$PSScriptRoot\rust\target\debug\openheart.dll" "$PSScriptRoot\godot\bin\openheart.dll" -Force
Write-Host "[OK] openheart.dll ready (godot\bin)" -ForegroundColor Green

# 2. Find engine (Redot / Godot)
$found = $null

$redotCmd = Get-Command "redot" -ErrorAction SilentlyContinue
$godotCmd  = Get-Command "godot"  -ErrorAction SilentlyContinue
$godot4Cmd = Get-Command "godot4" -ErrorAction SilentlyContinue

$candidates = @(
    $Engine,
    "$env:LOCALAPPDATA\Microsoft\WinGet\Links\godot.exe",
    "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\GodotEngine.GodotEngine_Microsoft.Winget.Source_8wekyb3d8bbwe\Godot_v4.7-stable_win64.exe",
    "$env:LOCALAPPDATA\Redot\redot.exe",
    "$env:LOCALAPPDATA\Redot Engine\redot.exe",
    "C:\Program Files\Redot\redot.exe",
    "C:\tools\redot\redot.exe",
    "$env:LOCALAPPDATA\Godot\godot.exe",
    "$env:LOCALAPPDATA\Godot Engine\godot.exe",
    "C:\Program Files\Godot\godot.exe",
    "C:\tools\godot\godot.exe"
)

if ($redotCmd) { $candidates += $redotCmd.Source }
if ($godotCmd)  { $candidates += $godotCmd.Source }
if ($godot4Cmd) { $candidates += $godot4Cmd.Source }

foreach ($c in $candidates) {
    if ($c -and (Test-Path $c)) {
        $found = $c
        break
    }
}

if (-not $found) {
    Write-Host ""
    Write-Host "[2/2] Engine not found automatically." -ForegroundColor Yellow
    Write-Host "Specify path manually:" -ForegroundColor Yellow
    Write-Host "  .\run.ps1 -Engine 'C:\path\to\redot.exe'" -ForegroundColor White
    Write-Host ""
    Write-Host "Or open project manually in Redot/Godot editor:" -ForegroundColor Yellow
    Write-Host "  Import -> $PSScriptRoot\godot\project.godot" -ForegroundColor White
    Write-Host ""
    Write-Host "DLL is ready. Press F5 in editor to play." -ForegroundColor Green
    exit 0
}

# 3. Launch - editor by default (-e flag), or the game with -Game
$projectDir = "$PSScriptRoot\godot"
if ($Game) {
    Write-Host "[2/2] Launching GAME: $found" -ForegroundColor Cyan
    Start-Process -FilePath $found -ArgumentList "--path `"$projectDir`""
    Write-Host "[OK] Game launched." -ForegroundColor Green
} else {
    Write-Host "[2/2] Launching EDITOR: $found" -ForegroundColor Cyan
    Start-Process -FilePath $found -ArgumentList "-e --path `"$projectDir`""
    Write-Host "[OK] Editor launched. Open the OpenHeart tab at the top; press F5 to play." -ForegroundColor Green
}
