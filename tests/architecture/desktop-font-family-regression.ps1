$ErrorActionPreference = 'Stop'

# Exercise the exact predicate used by the shell gate, without running its unrelated checks.
$gateSource = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'desktop-shell-lifecycle.ps1') -Raw
$predicate = [regex]::Match($gateSource, '(?m)^if \(\$desktopSources -match ''(?<pattern>[^'']+)''\)')
if (-not $predicate.Success) { throw 'Cannot locate the shell gate font-family predicate.' }
$fontPattern = $predicate.Groups['pattern'].Value
$cases = @(
    @{ Name='direct CSS list'; Reject=$true; Source='div().font_family("A, B")' },
    @{ Name='UI font constant list'; Reject=$true; Source='pub(crate) const UI_FONT_FAMILY: &str = "A, B";' },
    @{ Name='mono font constant list'; Reject=$true; Source='pub(crate) const UI_MONO_FONT_FAMILY: &str = "A, B";' },
    @{ Name='spaced multiline direct list'; Reject=$true; Source="div().font_family (`n    `"A, B`",`n)" },
    @{ Name='multiline constant list'; Reject=$true; Source="const UI_FONT_FAMILY:`n    &str =`n    `"A, B`";" },
    @{ Name='static font list'; Reject=$true; Source='static UI_FONT_FAMILY: &str = "A, B";' },
    @{ Name='single family literal'; Reject=$false; Source='div().font_family("Microsoft YaHei UI")' },
    @{ Name='single family constants'; Reject=$false; Source="const UI_FONT_FAMILY: &str = `"Microsoft YaHei UI`";`nconst UI_MONO_FONT_FAMILY: &str = `"Consolas`";" },
    @{ Name='UI font variable'; Reject=$false; Source='div().font_family(UI_FONT_FAMILY)' },
    @{ Name='multiline UI font variable'; Reject=$false; Source="div().font_family(`n    UI_FONT_FAMILY,`n)" },
    @{ Name='variable then unrelated assignment'; Reject=$false; Source="div().font_family(UI_FONT_FAMILY).w(px(width));`nlet source = `"ordinary text`";`nlet (_, window) = next();" },
    @{ Name='variable then damaged decoded string'; Reject=$false; Source="div().font_family(UI_FONT_FAMILY);`nlet source = `"ordinary text;`nlet (_, window) = next();" },
    @{ Name='mono variable then comma text'; Reject=$false; Source="div().font_family(UI_MONO_FONT_FAMILY);`nlet label = `"ordinary, text`";" },
    @{ Name='literal then comma text'; Reject=$false; Source='div().font_family("Consolas").child("ordinary, text")' },
    @{ Name='constant then comma text'; Reject=$false; Source='const UI_FONT_FAMILY: &str = "Microsoft YaHei UI"; let label = "ordinary, text";' },
    @{ Name='adjacent source fragments'; Reject=$false; Source=(@('div().font_family(UI_FONT_FAMILY)', 'let source = "ordinary text"; let (_, window) = next();') -join "`n") }
)
$failures = @()
foreach ($case in $cases) {
    $actualReject = $case.Source -match $fontPattern
    if ($actualReject -ne $case.Reject) {
        $failures += $case.Name
        Write-Host "FAIL $($case.Name): expected reject=$($case.Reject), actual=$actualReject"
    } else {
        Write-Host "PASS $($case.Name)"
    }
}
Write-Host "SUMMARY cases=$($cases.Count) passed=$($cases.Count-$failures.Count) failed=$($failures.Count)"
if ($failures.Count -ne 0) { throw "Font-family detector regression failed: $($failures -join '; ')" }
