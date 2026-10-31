param(
    [Parameter(Mandatory = $true)]
    [string]$DesktopExecutable,

    [string]$ExpectedBrandIcon = (
        Join-Path $PSScriptRoot '..\..\rust\apps\nxtools-desktop\assets\brand\nxtools-32.png'
    )
)

$ErrorActionPreference = 'Stop'

$desktopPath = [System.IO.Path]::GetFullPath($DesktopExecutable)
$brandPath = [System.IO.Path]::GetFullPath($ExpectedBrandIcon)
if (-not [System.IO.File]::Exists($desktopPath)) {
    throw "Desktop executable does not exist: $desktopPath"
}
if (-not [System.IO.File]::Exists($brandPath)) {
    throw "Expected brand icon does not exist: $brandPath"
}

# Pure PE parser checks both RT_GROUP_ICON IDs and all18 real decoded sizes.
# Keep the existing associated-icon pixel check below as a separate Windows boundary.
& python (Join-Path $PSScriptRoot 'approved-icons.py') --desktop-pe $desktopPath
if ($LASTEXITCODE -ne 0) { throw 'Desktop embedded application/archive resources differ from approved icons.' }

try {
    Add-Type -AssemblyName System.Drawing.Common -ErrorAction Stop
}
catch {
    Add-Type -AssemblyName System.Drawing -ErrorAction Stop
}
$associated = [System.Drawing.Icon]::ExtractAssociatedIcon($desktopPath)
if ($null -eq $associated) {
    throw 'NxTools.exe does not expose an associated Windows icon.'
}

$actual = $null
$expected = $null
try {
    $actual = $associated.ToBitmap()
    $expected = [System.Drawing.Bitmap]::FromFile($brandPath)
    if ($actual.Width -ne 32 -or $actual.Height -ne 32) {
        throw "NxTools.exe associated icon is $($actual.Width)x$($actual.Height), expected 32x32."
    }
    if ($expected.Width -ne 32 -or $expected.Height -ne 32) {
        throw "Formal brand asset is $($expected.Width)x$($expected.Height), expected 32x32."
    }

    for ($y = 0; $y -lt 32; $y++) {
        for ($x = 0; $x -lt 32; $x++) {
            if ($actual.GetPixel($x, $y).ToArgb() -ne $expected.GetPixel($x, $y).ToArgb()) {
                throw "NxTools.exe associated icon differs from the formal brand asset at ($x,$y)."
            }
        }
    }
}
finally {
    if ($null -ne $expected) { $expected.Dispose() }
    if ($null -ne $actual) { $actual.Dispose() }
    $associated.Dispose()
}

Write-Host 'Desktop product icon gate passed.'
