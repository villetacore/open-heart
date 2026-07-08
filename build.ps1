# build.ps1 — локальная релизная сборка OpenHeart
#
# Использование:
#   .\build.ps1                        # Windows x64 (по умолчанию)
#   .\build.ps1 -Platform android      # Android arm64+armeabi-v7a (нужен NDK)
#   .\build.ps1 -SkipExport            # только собрать Rust DLL, без Godot экспорта
#   .\build.ps1 -Debug                 # debug-сборка Rust (быстрее, но не для релиза)
#
# Требования для Windows-сборки:
#   - Rust stable (rustup)
#   - Godot 4.7-stable с экспортными шаблонами (один из вариантов ниже)
#
# Godot можно установить через WinGet:
#   winget install GodotEngine.GodotEngine
# Или скачать вручную с https://godotengine.org/download

param(
    [ValidateSet("windows", "android")]
    [string]$Platform = "windows",

    [switch]$SkipExport,
    [switch]$Debug
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root     = $PSScriptRoot
$RustDir  = "$Root\rust"
$GodotDir = "$Root\godot"

# ─── Вспомогательные функции ───────────────────────────────────────────────────

function Write-Step([string]$msg) {
    Write-Host "`n[$msg]" -ForegroundColor Cyan
}
function Write-OK([string]$msg) {
    Write-Host "  OK  $msg" -ForegroundColor Green
}
function Write-Fail([string]$msg) {
    Write-Host "  ERR $msg" -ForegroundColor Red
}
function Write-Warn([string]$msg) {
    Write-Host "  !   $msg" -ForegroundColor Yellow
}

function Find-GodotEngine {
    $candidates = @(
        # WinGet
        "$env:LOCALAPPDATA\Microsoft\WinGet\Links\godot.exe",
        "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\GodotEngine.GodotEngine_Microsoft.Winget.Source_8wekyb3d8bbwe\Godot_v4.7-stable_win64.exe",
        # Ручные установки
        "C:\Godot\godot.exe",
        "C:\tools\godot\godot.exe",
        "$env:LOCALAPPDATA\Godot\godot.exe",
        "C:\Program Files\Godot\godot.exe"
    )
    # PATH
    $fromPath = Get-Command godot -ErrorAction SilentlyContinue
    if ($fromPath) { return $fromPath.Source }

    foreach ($c in $candidates) {
        if ($c -and (Test-Path $c)) { return $c }
    }
    return $null
}

# ─── Шаг 1: Rust ──────────────────────────────────────────────────────────────

Write-Step "1/3  Building Rust  ($Platform$(if ($Debug) {', debug'} else {', release'}))"

Push-Location $RustDir

switch ($Platform) {
    "windows" {
        $cargoArgs = @("build")
        if (-not $Debug) { $cargoArgs += "--release" }
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) { Write-Fail "cargo build failed"; exit 1 }

        $suffix = if ($Debug) { "debug" } else { "release" }
        $dll    = "$RustDir\target\$suffix\openheart.dll"

        New-Item -ItemType Directory -Force "$GodotDir\bin" | Out-Null
        Copy-Item $dll "$GodotDir\bin\openheart.dll" -Force
        Write-OK "openheart.dll  →  godot\bin\"
    }

    "android" {
        # Проверяем cargo-ndk
        if (-not (Get-Command cargo-ndk -ErrorAction SilentlyContinue)) {
            Write-Warn "cargo-ndk не найден. Устанавливаю..."
            cargo install cargo-ndk --locked
            if ($LASTEXITCODE -ne 0) { Write-Fail "Не удалось установить cargo-ndk"; exit 1 }
        }

        # NDK путь
        $ndk = $env:ANDROID_NDK_HOME
        if (-not $ndk) {
            $ndk = "$env:ANDROID_SDK_ROOT\ndk\23.2.8568313"
        }
        if (-not (Test-Path $ndk)) {
            Write-Fail "Android NDK не найден. Укажите ANDROID_NDK_HOME или установите ndk;23.2.8568313 через sdkmanager."
            exit 1
        }
        $env:ANDROID_NDK_HOME = $ndk

        # Добавляем таргеты
        rustup target add aarch64-linux-android armv7-linux-androideabi | Out-Null

        $outDir = "$GodotDir\bin\android"
        New-Item -ItemType Directory -Force $outDir | Out-Null

        $cargoArgs = @("-t", "arm64-v8a", "-t", "armeabi-v7a", "-o", $outDir, "build")
        if (-not $Debug) { $cargoArgs += "--release" }
        & cargo ndk @cargoArgs
        if ($LASTEXITCODE -ne 0) { Write-Fail "cargo ndk build failed"; exit 1 }

        Write-OK "libopenheart.so  →  godot\bin\android\{arm64-v8a,armeabi-v7a}\"
    }
}

Pop-Location

# ─── Шаг 2: Godot export ──────────────────────────────────────────────────────

if ($SkipExport) {
    Write-Warn "SkipExport: Godot export пропущен. DLL готова."
    exit 0
}

Write-Step "2/3  Godot export  ($Platform)"

$engine = Find-GodotEngine
if (-not $engine) {
    Write-Warn "Godot 4.7 не найден автоматически."
    Write-Host "  Установите через WinGet:  winget install GodotEngine.GodotEngine" -ForegroundColor Gray
    Write-Host "  Или укажите вручную:      `$env:PATH += ';C:\путь\к\godot'" -ForegroundColor Gray
    Write-Host ""
    Write-Host "  Rust DLL уже готова: godot\bin\openheart.dll" -ForegroundColor Green
    Write-Host "  Откройте проект в Godot вручную: godot\project.godot" -ForegroundColor Green
    exit 0
}
Write-OK "Движок: $engine"

Push-Location $GodotDir

switch ($Platform) {
    "windows" {
        New-Item -ItemType Directory -Force "dist\windows" | Out-Null
        & $engine --headless --export-release "Windows Desktop" "dist\windows\OpenHeart.exe" 2>&1
    }
    "android" {
        New-Item -ItemType Directory -Force "dist\android" | Out-Null
        & $engine --headless --export-release "Android" "dist\android\OpenHeart.apk" 2>&1
    }
}

$exitCode = $LASTEXITCODE
Pop-Location

if ($exitCode -ne 0) {
    Write-Fail "Godot export вернул код $exitCode."
    Write-Warn "Убедитесь, что экспортные шаблоны установлены:"
    Write-Host "  В редакторе Godot: Editor → Export Templates → Download" -ForegroundColor Gray
    exit 1
}

# ─── Шаг 3: Итог ──────────────────────────────────────────────────────────────

Write-Step "3/3  Done"

switch ($Platform) {
    "windows" { Write-OK "Сборка: godot\dist\windows\OpenHeart.exe" }
    "android" { Write-OK "Сборка: godot\dist\android\OpenHeart.apk" }
}
