$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$sourcePath = Join-Path $repositoryRoot "rust\apps\nxtools-desktop\src\quick_launch\file_dialog_search.rs"
$source = Get-Content -Raw $sourcePath
$rowMatch = [regex]::Match(
    $source,
    '(?s)fn row\(&self, index: usize, cx: &mut Context<Self>\).*?(?=\n    \})')
if (-not $rowMatch.Success) {
    throw "Could not locate the file-dialog search result-row renderer."
}
if ($rowMatch.Value -notmatch '\.w_full\(\)') {
    throw "File-dialog result selection only paints the content width instead of the full row."
}

Write-Output "PASS file-dialog search selection and hover surfaces fill each result row"
