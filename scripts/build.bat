@echo off
echo [OpenHeart] Building Rust...
cd /d "%~dp0.."
cargo build -p openheart
if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Build failed!
    pause
    exit /b 1
)
if not exist "%~dp0..\game\bin" mkdir "%~dp0..\game\bin"
copy /Y "%~dp0..\target\debug\openheart.dll" "%~dp0..\game\bin\openheart.dll" >nul
echo.
echo [OK] openheart.dll built and copied to game\bin\openheart.dll
