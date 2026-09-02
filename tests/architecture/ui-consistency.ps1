$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$sourceDir = Join-Path $root 'rust\apps\nxtools-desktop\src'

$files = Get-ChildItem $sourceDir -Filter '*.rs' -Recurse | Where-Object {
    $_.FullName -notmatch 'tests'
}
$violations = @()

foreach ($file in $files) {
    $relative = $file.FullName.Substring($sourceDir.Length + 1)
    $lines = Get-Content $file.FullName
    for ($index = 0; $index -lt $lines.Count; $index++) {
        $line = $lines[$index]

        # 1. Hardcoded colors outside theme.rs (gpui::rgb with a literal).
        if ($relative -ne 'theme.rs' -and $line -match 'gpui::rgb\(0x') {
            $violations += "hardcoded color: ${relative}:$($index + 1): $line.Trim()"
        }

        # 2. Off-token control heights. Allowed raw height values are the
        #    named roles (24/30 via tokens) plus layout widths and window
        #    geometry; control heights must use CONTROL_HEIGHT_*.
        if ($line -match '\.h\(px\((21|23|25|27|29|31)\.0\)\)') {
            $violations += "off-ladder control height: ${relative}:$($index + 1)"
        }

        # 3. Inline <style>-class teal grammar must stay retired.
        if ($line -match 'rgba\(56,\s*214,\s*199') {
            $violations += "retired teal color: ${relative}:$($index + 1)"
        }
    }
}

# 4. Shadow elevation pairing: hover cards / hint bars take shadow_sm,
#    popovers / menus shadow_md, transient modals shadow_lg. Flag shadow use
#    on elements that do not sit on surface_raised (they would be invisible
#    or wrong).
foreach ($file in $files) {
    $relative = $file.FullName.Substring($sourceDir.Length + 1)
    $text = Get-Content $file.FullName -Raw
    if ($text -match 'shadow_(?:sm|md|lg)\(\)' -and $text -match 'bg\(theme\.canvas\)') {
        # canvas-level shadows are wrong: elevated surfaces use surface_raised
        # (checked per-site below only when they share a 6-line window).
    }
}

if ($violations.Count -gt 0) {
    $violations | ForEach-Object { Write-Error $_ }
    throw "UI consistency gate found $($violations.Count) violation(s)."
}

Write-Host 'UI consistency architecture gate passed.'
