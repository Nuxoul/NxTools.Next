$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$build = Get-Content -Raw -LiteralPath (Join-Path $root 'scripts\build.ps1')
$package = Get-Content -Raw -LiteralPath (Join-Path $root 'scripts\package.ps1')
$runner = Get-Content -Raw -LiteralPath (Join-Path $root 'csharp\NxTools.PlatformHost.Tests\TestSelection.cs')
$catalog = Get-Content -Raw -LiteralPath (Join-Path $root 'csharp\NxTools.PlatformHost.Tests\Program.cs')
$effects = Get-Content -Raw -LiteralPath (Join-Path $root 'csharp\NxTools.PlatformHost.Tests\TestEffects.cs')
if ($build -notmatch '--no-build -- --fixtures' -or $build -match '--authorize-native|--native') {
    throw 'Ordinary build verification must select isolated C# fixtures explicitly, never native acceptance.'
}
if ($package -notmatch "build\.ps1" -or $package -match '--authorize-native|--native') {
    throw 'Package verification must inherit the fixture-only build entry without native authorization.'
}
if ($runner -notmatch 'No tests match' -or $runner -notmatch 'explicit name filters' -or
    $runner -notmatch 'catch \(TestSkippedException' -or $runner -notmatch 'SUMMARY selected=') {
    throw 'Runner must reject zero/unsafe selections and separately count skip/pass/fail.'
}
if ($catalog -notmatch 'TestEffects.Create\(test.Name, test.Run\)' -or $catalog -match 'Initial classification' -or
    $effects -notmatch 'Unclassified test symbol' -or $effects -notmatch 'RestrictedFixture') {
    throw 'Every catalog entry must explicitly declare its reviewed effects, with no implicit safe default.'
}
foreach ($relative in @('scripts\build.ps1', 'scripts\package.ps1', 'tests\native\gpui-kit-acceptance.ps1')) {
    $parseTokens = $null
    $parseErrors = $null
    $null = [Management.Automation.Language.Parser]::ParseFile((Join-Path $root $relative), [ref]$parseTokens, [ref]$parseErrors)
    if ($parseErrors.Count -ne 0) { throw "PowerShell syntax failed: $relative" }
}
$native = Get-Content -Raw -LiteralPath (Join-Path $root 'tests\native\gpui-kit-acceptance.ps1')
if ($native -notmatch '--native --authorize-native') {
    throw 'The separately invoked owned GPUI native wrapper must carry explicit authorization.'
}
Write-Host 'PASS fixture-only build/package selection, explicit native entry, zero-match and result accounting (source only)'
