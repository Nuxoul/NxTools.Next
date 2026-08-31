param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
)

$ErrorActionPreference = "Stop"

# 13.1 design-token gate: outside the typography/metrics/motion token modules,
# production UI must not write font families, text sizes, line heights, corner
# radii or component-theme font sizes as raw string/numeric literals. Values
# must come from ui::typography and ui::metrics constants (or be derived from a
# named layout constant such as a circle's own diameter).

$srcRoot = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src"
$typographyPath = Join-Path $srcRoot "ui\typography.rs"
$metricsPath = Join-Path $srcRoot "ui\metrics.rs"

$typography = Get-Content -LiteralPath $typographyPath -Raw
$metrics = Get-Content -LiteralPath $metricsPath -Raw

foreach ($token in @(
    'pub(crate) const UI_FONT_FAMILY: &str = "Microsoft YaHei UI";',
    'pub(crate) const UI_MONO_FONT_FAMILY: &str = "Consolas";',
    'pub(crate) const TEXT_TITLE: f32 = 14.0;',
    'pub(crate) const TEXT_BODY: f32 = 12.0;',
    'pub(crate) const TEXT_LABEL: f32 = 11.0;',
    'pub(crate) const TEXT_META: f32 = 10.0;',
    'pub(crate) const TEXT_STATUS_GLYPH: f32 = 28.0;'
)) {
    if (-not $typography.Contains($token)) {
        throw "Typography token contract is missing: $token"
    }
}

foreach ($token in @(
    'pub(crate) const RADIUS_NONE: f32 = 0.0;',
    'pub(crate) const RADIUS_CONTROL: f32 = 4.0;',
    'pub(crate) const RADIUS_FLOATING: f32 = 6.0;',
    'pub(crate) const CONTROL_HEIGHT_COMPACT: f32 = 18.0;',
    'pub(crate) const CONTROL_HEIGHT_DEFAULT: f32 = 22.0;',
    'pub(crate) const CONTROL_HEIGHT_ACTION: f32 = 28.0;',
    'pub(crate) const CONTROL_HEIGHT_INPUT: f32 = 32.0;'
)) {
    if (-not $metrics.Contains($token)) {
        throw "Metrics token contract is missing: $token"
    }
}

$tokenModules = @(
    (Join-Path $srcRoot "ui\typography.rs"),
    (Join-Path $srcRoot "ui\metrics.rs"),
    (Join-Path $srcRoot "ui\motion.rs")
)

$patterns = @(
    @{ Name = "font family literal"; Regex = '(font_family\s*=\s*"|\.font_family\("|"(Microsoft YaHei|Consolas|Segoe))' },
    @{ Name = "raw text size"; Regex = '\.text_size\(px\(-?[0-9]+(?:\.[0-9]+)?\)\)' },
    @{ Name = "raw line height"; Regex = '\.line_height\(px\(-?[0-9]+(?:\.[0-9]+)?\)\)' },
    @{ Name = "raw corner radius"; Regex = '\.rounded\(px\(-?[0-9]+(?:\.[0-9]+)?\)\)' },
    @{ Name = "raw component font size"; Regex = '(mono_)?font_size\s*=\s*px\(-?[0-9]+(?:\.[0-9]+)?\)' }
)

$violations = @()

Get-ChildItem -LiteralPath $srcRoot -Recurse -Filter *.rs | ForEach-Object {
    $sourceFile = $_
    foreach ($tokenModule in $tokenModules) {
        if ($sourceFile.FullName -eq $tokenModule) {
            return
        }
    }

    $lineNumber = 0
    foreach ($line in (Get-Content -LiteralPath $sourceFile.FullName)) {
        $lineNumber++
        foreach ($pattern in $patterns) {
            if ($line -match $pattern.Regex) {
                $relativePath = $sourceFile.FullName.Substring($srcRoot.Length + 1)
                $violations += "{0}:{1}: {2}: {3}" -f $relativePath, $lineNumber, $pattern.Name, $line.Trim()
            }
        }
    }
}

if ($violations.Count -gt 0) {
    $violations | ForEach-Object { Write-Host $_ }
    throw "UI token boundary gate found $($violations.Count) hardcoded value(s) outside the token modules."
}

Write-Host "UI token boundary architecture gate passed."
