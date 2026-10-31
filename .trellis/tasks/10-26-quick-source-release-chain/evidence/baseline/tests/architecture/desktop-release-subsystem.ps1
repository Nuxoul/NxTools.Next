param(
    [Parameter(Mandatory = $true)]
    [string]$DesktopExecutable
)

$ErrorActionPreference = 'Stop'
$dotnetRoot = Split-Path -Parent ((Get-Command dotnet.exe -ErrorAction Stop).Source)
$metadataAssembly = Get-ChildItem -LiteralPath (Join-Path $dotnetRoot 'sdk') `
    -Filter 'System.Reflection.Metadata.dll' `
    -File `
    -Recurse `
    -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\tools\\net472\\System\.Reflection\.Metadata\.dll$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $metadataAssembly) {
    throw 'The dotnet SDK does not provide System.Reflection.Metadata.dll for the PE subsystem gate.'
}
Add-Type -Path $metadataAssembly.FullName -ErrorAction Stop
$resolved = [System.IO.Path]::GetFullPath($DesktopExecutable)
if (-not [System.IO.File]::Exists($resolved)) {
    throw "Desktop executable does not exist: $resolved"
}

$stream = [System.IO.File]::OpenRead($resolved)
try {
    $reader = [System.Reflection.PortableExecutable.PEReader]::new($stream)
    try {
        $subsystem = $reader.PEHeaders.PEHeader.Subsystem.ToString()
        if ($subsystem -ne 'WindowsGui') {
            throw "Release desktop subsystem is '$subsystem', expected WindowsGui."
        }
    }
    finally {
        $reader.Dispose()
    }
}
finally {
    $stream.Dispose()
}

Write-Host 'Desktop Release PE subsystem gate passed.'
