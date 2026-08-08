param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
)

$ErrorActionPreference = "Stop"

$sourcePath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\ui\mod.rs"
$source = Get-Content -LiteralPath $sourcePath -Raw
$handlerStart = $source.IndexOf('pub(crate) fn handle_command_result(')
$handlerEnd = $source.IndexOf('pub(crate) fn handle_hide_failure(', $handlerStart)
if ($handlerStart -lt 0 -or $handlerEnd -le $handlerStart) {
    throw "Could not isolate the clipboard command-result handler."
}

$handler = $source.Substring($handlerStart, $handlerEnd - $handlerStart)
if ($handler -match 'self\s*\.application\s*\.update\(cx') {
    throw @'
Clipboard action completion runs while ClipboardApplication is already leased by
the platform notification dispatcher. It must not synchronously update the same
application or GPUI will terminate with double_lease_panic / 0xc0000409.
'@
}
if ([regex]::Matches($handler, 'schedule_entity_update\(').Count -lt 2 -or
    -not $handler.Contains('application.mark_item_used(item_id)', [StringComparison]::Ordinal)) {
    throw "Clipboard usage persistence and copy-only refresh must cross separate deferred turns."
}
if ($handler.Contains('self.refresh_entries(cx)', [StringComparison]::Ordinal)) {
    throw "Copy-only completion must not synchronously query ClipboardApplication while it is leased."
}

Write-Host "Clipboard command-result lifecycle architecture gate passed."
