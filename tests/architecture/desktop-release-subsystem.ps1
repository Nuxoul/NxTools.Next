param(
    [Parameter(Mandatory = $true)]
    [string]$DesktopExecutable
)

$ErrorActionPreference = 'Stop'
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
