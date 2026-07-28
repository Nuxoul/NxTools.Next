$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$adapterPath = Join-Path $repositoryRoot "csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.cs"
$placementPath = Join-Path $repositoryRoot "csharp\NxTools.PlatformHost\Windows\AnchoredWindowPlacement.cs"

$adapter = Get-Content -LiteralPath $adapterPath -Raw
$placement = Get-Content -LiteralPath $placementPath -Raw

if ($placement -notmatch '\bSwpNoZOrder\b') {
    throw "File-dialog search placement can reorder or activate the foreground window."
}
if ($adapter -match '\bFileDialogGeometryTimerId\b' -or
    $adapter -match '\bFileDialogGeometryIntervalMs\b') {
    throw "File-dialog search following still has a fixed timer delay."
}
if ($adapter -notmatch '\bWmFileDialogGeometryChanged\b') {
    throw "File-dialog search following is not coalesced through an immediate window message."
}
if ($placement -notmatch '\bSwpNoActivate\b') {
    throw "File-dialog search placement must remain non-activating."
}
if ($adapter -notmatch 'AnchoredWindowPlacement\.Follow\(') {
    throw "File-dialog drag following still uses the calibrated initial-placement path."
}

Write-Output "PASS file-dialog search never reorders foreground windows and follows without a fixed-frame delay"
