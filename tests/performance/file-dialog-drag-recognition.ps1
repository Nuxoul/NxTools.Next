param(
    [Parameter(Mandatory = $true)]
    [string]$LogPath,
    [Parameter(Mandatory = $true)]
    [DateTimeOffset]$StartTime,
    [Parameter(Mandatory = $true)]
    [DateTimeOffset]$EndTime,
    [int]$MaximumRecognitionsPerSecond = 2,
    [double]$MaximumRecognitionMilliseconds = 16
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $LogPath)) {
    throw "PlatformHost log does not exist: $LogPath"
}
if ($EndTime -le $StartTime) {
    throw "EndTime must be later than StartTime."
}

$recognitions = @(
    Get-Content -LiteralPath $LogPath |
        ForEach-Object {
            try {
                $record = $_ | ConvertFrom-Json
                if ($record.event -ne "quick_launch.file_dialog_recognized") {
                    return
                }
                $timestamp = [DateTimeOffset]$record.timestamp
                if ($timestamp -ge $StartTime -and $timestamp -lt $EndTime) {
                    [pscustomobject]@{
                        Timestamp = $timestamp
                        ElapsedMilliseconds = [double]$record.elapsed_ms
                    }
                }
            }
            catch {
            }
        }
)

if ($recognitions.Count -eq 0) {
    throw "The selected trace contains no file-dialog recognition events."
}

$buckets = @(
    $recognitions |
        Group-Object {
            [Math]::Floor($_.Timestamp.ToUnixTimeMilliseconds() / 1000)
        }
)
$peakPerSecond = ($buckets | Measure-Object -Property Count -Maximum).Maximum
$peakElapsed = (
    $recognitions |
        Measure-Object -Property ElapsedMilliseconds -Maximum
).Maximum

Write-Output (
    "file-dialog drag recognition: events={0}, peak_per_second={1}, peak_elapsed_ms={2:N2}" -f
        $recognitions.Count,
        $peakPerSecond,
        $peakElapsed
)

if ($peakPerSecond -gt $MaximumRecognitionsPerSecond) {
    throw (
        "Repeated structural recognition exceeded the drag budget: {0}/s > {1}/s." -f
            $peakPerSecond,
            $MaximumRecognitionsPerSecond
    )
}
if ($peakElapsed -gt $MaximumRecognitionMilliseconds) {
    throw (
        "Structural recognition exceeded the frame budget: {0:N2} ms > {1:N2} ms." -f
            $peakElapsed,
            $MaximumRecognitionMilliseconds
    )
}

Write-Output "PASS file-dialog drag does not re-run structural recognition"
