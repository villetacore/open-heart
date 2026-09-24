param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ClientArgs
)

$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    & cargo run -p openheart-terminal -- @ClientArgs
    if ($LASTEXITCODE -ne 0) {
        throw "oh-terminal exited with code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}
