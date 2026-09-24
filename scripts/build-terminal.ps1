param([string]$OutputDirectory = 'target/terminal-dist')

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    & cargo build -p openheart-terminal --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Terminal client build failed' }
    & cargo build -p openheart-core --target wasm32-unknown-unknown --features abi --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'core.wasm build failed; install wasm32-unknown-unknown using rustup' }
    & go -C server build -o oh-server.exe ./cmd/oh-server
    if ($LASTEXITCODE -ne 0) { throw 'Go server build failed' }

    $destination = [System.IO.Path]::GetFullPath((Join-Path $projectRoot $OutputDirectory))
    # Use a fresh destination so obsolete content cannot change the preset hash.
    if (Test-Path -LiteralPath $destination) {
        throw "Output already exists: $destination. Choose a new -OutputDirectory."
    }
    New-Item -ItemType Directory -Path $destination | Out-Null
    Copy-Item -LiteralPath 'target/release/oh-terminal.exe' -Destination $destination
    Copy-Item -LiteralPath 'server/oh-server.exe' -Destination $destination
    Copy-Item -LiteralPath 'target/wasm32-unknown-unknown/release/openheart_core.wasm' -Destination (Join-Path $destination 'core.wasm')
    Copy-Item -LiteralPath 'game/presets' -Destination $destination -Recurse
    Copy-Item -LiteralPath 'terminal/README.md' -Destination $destination
    Write-Output "Ready: $destination"
} finally {
    Pop-Location
}
