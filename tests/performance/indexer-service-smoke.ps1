param(
    [string]$Query = 'NxTools.Indexer.exe',
    [ValidateRange(1, 200)][int]$Limit = 20,
    [int]$TimeoutMs = 10000,
    [string]$ExpectedPath
)

$ErrorActionPreference = 'Stop'
$serviceName = 'NxToolsIndexerV1'
$expectedExecutable = Join-Path $env:ProgramData 'NxTools\NxTools.Next\Service\NxTools.Indexer.exe'
$packagedExecutable = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\artifacts\win-x64\NxTools.Indexer.exe'))
$service = Get-CimInstance Win32_Service -Filter "Name='$serviceName'"
if (-not $service -or $service.State -ne 'Running' -or $service.StartName -ne 'LocalSystem') {
    throw 'The packaged LocalSystem indexer must be running.'
}
$registeredExecutable = if ($service.PathName -match '^"([^"]+)"') { $Matches[1] } else { ($service.PathName -split '\s+', 2)[0] }
if ([IO.Path]::GetFullPath($registeredExecutable) -ne $expectedExecutable) {
    throw 'Indexer service registration does not point to the protected package.'
}
$serviceKey = Get-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Services\$serviceName"
$approvedHash = [string]$serviceKey.NxToolsBinarySha256
if (-not $approvedHash -or
    $approvedHash -ne (Get-FileHash $packagedExecutable -Algorithm SHA256).Hash -or
    $approvedHash -ne (Get-FileHash $expectedExecutable -Algorithm SHA256).Hash) {
    throw 'Running protected indexer does not match the approved package.'
}
if (-not $ExpectedPath -and $Query -eq 'NxTools.Indexer.exe') { $ExpectedPath = $expectedExecutable }
. (Join-Path $PSScriptRoot 'indexer-desktop-probe.ps1')
$search = Invoke-IndexerDesktopSearch -Query $Query -ExpectedPath $ExpectedPath -TimeoutMs $TimeoutMs
$finalService = Get-CimInstance Win32_Service -Filter "Name='$serviceName'"
if ($finalService.State -ne 'Running' -or $finalService.ProcessId -ne $service.ProcessId) {
    throw 'Indexer service process changed during the search.'
}
[pscustomobject]@{
    Phase = 'search_verified'
    ServicePid = $service.ProcessId
    DesktopPid = $search.DesktopPid
    Query = $Query
    ResultCount = $search.ResultCount
    ReportedResultCount = [Math]::Min($Limit, $search.ResultCount)
    MatchedCount = $search.MatchedCount
    WallMs = $search.WallMs
    FirstPath = $search.FirstPath
    InputRestored = $search.InputRestored
}
