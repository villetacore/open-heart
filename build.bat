@echo off
echo [OpenHeart] Building Rust...
cd /d "%~dp0rust"
cargo build
if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Build failed!
    pause
    exit /b 1
)
echo.
echo [OK] openheart.dll built: rust\target\debug\openheart.dll
