# run.ps1 - build Rust and launch Redot/Godot
# Usage: .\run.ps1
#   or:  .\run.ps1 -Engine "C:\path\to\redot.exe"

param(
    [string]$Engine = ""
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
Write-Host "[OK] openheart.dll ready" -ForegroundColor Green

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

# 3. Launch
Write-Host "[2/2] Launching: $found" -ForegroundColor Cyan
$projectDir = "$PSScriptRoot\godot"
Start-Process -FilePath $found -ArgumentList "--path `"$projectDir`""
Write-Host "[OK] Editor launched. Press F5 to play." -ForegroundColor Green
