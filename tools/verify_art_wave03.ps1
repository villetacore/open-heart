# Validate the actual assets, dialogue consumers, dimensions and saved provenance.
# Read-only: does not launch the game or touch campaign saves.
$ErrorActionPreference = 'Stop'
$artRoot = Split-Path $PSScriptRoot -Parent
$gameRoot = Join-Path $artRoot 'game'
$manifest = Get-Content (Join-Path $gameRoot 'assets/illustrations/neighbors/places_manifest.json') -Raw | ConvertFrom-Json
$dialogues = Get-Content (Join-Path $gameRoot 'presets/core/dialogues.json') -Raw | ConvertFrom-Json
$dialogueRon = Get-Content (Join-Path $gameRoot 'presets/core/dialogues.ron') -Raw
$consumer = Get-Content (Join-Path $artRoot 'core/src/data/dialogue.rs') -Raw
$expected = @('ivo','noel','lucien','ash','emil','yves')
if ($manifest.entries.Count -ne 6) { throw 'Expected exactly six entries.' }
if (($manifest.entries.id | Sort-Object -Unique).Count -ne 6) { throw 'Duplicate IDs.' }
if (-not $consumer.Contains('scene.ends_with("_city")')) { throw 'Dialogue image dispatch changed; review bindings.' }
Add-Type -AssemblyName System.Drawing
foreach ($entry in $manifest.entries) {
    if ($entry.id -notin $expected) { throw "Unexpected ID: $($entry.id)" }
    if ($entry.image -ne "res://assets/illustrations/neighbors/$($entry.id)_place_v1.png") { throw 'Unexpected image path.' }
    if ($entry.resource -ne "res://assets/illustrations/neighbors/$($entry.id).tres") { throw 'Unexpected resource path.' }
    $scene = @($dialogues | Where-Object id -eq $entry.dialogue_scene)
    if ($scene.Count -ne 1) { throw "Missing/duplicate dialogue: $($entry.dialogue_scene)" }
    if ($entry.id -notin $scene[0].lines.portrait) { throw "Portrait ID mismatch: $($entry.id)" }
    if (-not $dialogueRon.Contains('"' + $entry.dialogue_scene + '"')) { throw 'Scene absent from preferred RON preset.' }
    $png = Join-Path $gameRoot $entry.image.Substring(6)
    $resource = Join-Path $gameRoot $entry.resource.Substring(6)
    $texture = [System.Drawing.Image]::FromFile($png)
    try {
        if ($texture.Width -ne $entry.width -or $texture.Height -ne $entry.height) { throw "Dimension mismatch: $png" }
        if ($texture.RawFormat.Guid -ne [System.Drawing.Imaging.ImageFormat]::Png.Guid) { throw "Not PNG: $png" }
    } finally { $texture.Dispose() }
    $tres = Get-Content $resource -Raw
    if (-not $tres.Contains('path="' + $entry.image + '"')) { throw "Wrong PNG binding: $resource" }
    if (-not $tres.Contains("region = Rect2(0, 0, $($entry.width), $($entry.height))")) { throw "Wrong texture region: $resource" }
    if ([string]::IsNullOrWhiteSpace($entry.prompt) -or [string]::IsNullOrWhiteSpace($entry.source_file)) { throw 'Missing provenance.' }
    Write-Output "PASS $($entry.id): PNG $($entry.width)x$($entry.height), resource, JSON/RON scene, provenance"
}
Write-Output 'ART_WAVE03: 6/6 static checks passed. Godot runtime and export are separate checks.'
