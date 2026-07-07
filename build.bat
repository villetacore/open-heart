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
if not exist "%~dp0godot\bin" mkdir "%~dp0godot\bin"
copy /Y "%~dp0rust\target\debug\openheart.dll" "%~dp0godot\bin\openheart.dll" >nul
echo.
echo [OK] openheart.dll built and copied to godot\bin\openheart.dll
