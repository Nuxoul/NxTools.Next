param(
    [Parameter(Mandatory = $true)]
    [string]$ImagePath
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$resolved = Resolve-Path -LiteralPath $ImagePath
$bitmap = [System.Drawing.Bitmap]::FromFile($resolved)
try {
    if ($bitmap.Width -lt 637 -or $bitmap.Height -lt 419) {
        throw "Quick-launch capture is smaller than the expected 620 x 410 client area plus frame."
    }

    $emptyRows = @()
    for ($row = 0; $row -lt 9; $row++) {
        $top = 64 + (40 * $row)
        $colors = @{}
        for ($x = 30; $x -le 58; $x++) {
            for ($y = $top + 4; $y -le $top + 35; $y++) {
                $argb = $bitmap.GetPixel($x, $y).ToArgb()
                $colors[$argb] = 1 + [int]($colors[$argb])
            }
        }

        $dominant = $colors.GetEnumerator() |
            Sort-Object Value -Descending |
            Select-Object -First 1
        $nonDominantPixels = 0
        foreach ($entry in $colors.GetEnumerator()) {
            if ($entry.Key -ne $dominant.Key) {
                $nonDominantPixels += $entry.Value
            }
        }
        if ($nonDominantPixels -lt 12) {
            $emptyRows += $row + 1
        }
    }

    if ($emptyRows.Count -gt 0) {
        throw "Quick-launch result icon rail is visually empty in rows: $($emptyRows -join ', ')."
    }
}
finally {
    $bitmap.Dispose()
}

Write-Host 'PASS visible quick-launch rows contain rendered icon pixels'
