param(
    [Parameter(Mandatory)] [string]$Source,
    [Parameter(Mandatory)] [string[]]$Names
)

Add-Type -AssemblyName System.Drawing

$columns = 8
$rows = $Names.Count
$cellWidth = 128
$cellHeight = 256
$targetHeight = 226
$baseline = 246
$charactersDir = Join-Path $PSScriptRoot "..\godot\assets\sprites\characters"
$sourceBitmap = [System.Drawing.Bitmap]::new($Source)

function Test-KeyColor {
    param([System.Drawing.Color]$Color)
    $Color.R -ge 215 -and $Color.B -ge 215 -and $Color.G -le 75
}

for ($row = 0; $row -lt $rows; $row++) {
    $sourceCells = @()
    $ratios = @()

    for ($column = 0; $column -lt $columns; $column++) {
        $left = [int]($column * $sourceBitmap.Width / $columns)
        $top = [int]($row * $sourceBitmap.Height / $rows)
        $width = [int]($sourceBitmap.Width / $columns)
        $height = [int]($sourceBitmap.Height / $rows)
        $minX = $width
        $minY = $height
        $maxX = -1
        $maxY = -1

        for ($y = 0; $y -lt $height; $y += 2) {
            for ($x = 0; $x -lt $width; $x += 2) {
                if (Test-KeyColor $sourceBitmap.GetPixel($left + $x, $top + $y)) { continue }
                $minX = [Math]::Min($minX, $x)
                $minY = [Math]::Min($minY, $y)
                $maxX = [Math]::Max($maxX, $x)
                $maxY = [Math]::Max($maxY, $y)
            }
        }

        if ($maxX -lt 0) { throw "$($Names[$row]) frame $column is empty" }
        $bounds = [System.Drawing.Rectangle]::new(
            $left + $minX, $top + $minY, $maxX - $minX + 3, $maxY - $minY + 3
        )
        $sourceCells += $bounds
        $ratios += $bounds.Width / $bounds.Height
    }

    $sortedRatios = $ratios | Sort-Object
    $medianRatio = $sortedRatios[[int]($sortedRatios.Count / 2)]
    $targetWidth = [Math]::Min(108, [Math]::Max(32, [int]($targetHeight * $medianRatio)))
    $output = [System.Drawing.Bitmap]::new(
        1024, $cellHeight, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
    )
    $graphics = [System.Drawing.Graphics]::FromImage($output)
    $graphics.Clear([System.Drawing.Color]::Transparent)
    $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half

    for ($frame = 0; $frame -lt $columns; $frame++) {
        $x = $frame * $cellWidth + [int](($cellWidth - $targetWidth) / 2)
        $y = $baseline - $targetHeight
        $destination = [System.Drawing.Rectangle]::new($x, $y, $targetWidth, $targetHeight)
        $graphics.DrawImage(
            $sourceBitmap, $destination, $sourceCells[$frame], [System.Drawing.GraphicsUnit]::Pixel
        )
    }
    $graphics.Dispose()

    for ($y = 0; $y -lt $output.Height; $y++) {
        for ($x = 0; $x -lt $output.Width; $x++) {
            $pixel = $output.GetPixel($x, $y)
            if (Test-KeyColor $pixel) {
                $output.SetPixel($x, $y, [System.Drawing.Color]::Transparent)
            }
        }
    }

    $target = Join-Path $charactersDir $Names[$row]
    $output.Save($target, [System.Drawing.Imaging.ImageFormat]::Png)
    $output.Dispose()
    Write-Output "Imported $($Names[$row])"
}

$sourceBitmap.Dispose()
