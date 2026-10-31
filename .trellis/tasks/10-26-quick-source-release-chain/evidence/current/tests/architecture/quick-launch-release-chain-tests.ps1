param([switch]$SourceOnly)
$ErrorActionPreference = 'Stop'
if (-not $SourceOnly) { throw 'This regression requires -SourceOnly; product scripts are never executed.' }
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $PSScriptRoot 'quick-launch-release-chain.ps1')
$gate = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'quick-launch-lifecycle.ps1')
# Reuse the real gate's source-loader and exact security condition. The selected loader
# only reads source files (and imports this checker), never invokes build/publish/native code.
$loadStart = $gate.IndexOf('$desktopMainPath =')
$loadEnd = $gate.IndexOf('if ($source -notmatch', $loadStart)
if ($loadStart -lt 0 -or $loadEnd -le $loadStart) { throw 'Missing production source-loader boundary.' }
Invoke-Expression $gate.Substring($loadStart, $loadEnd - $loadStart)
$quickLaunchSource = (@(Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\quick_launch.rs')) +
    @(Get-ChildItem (Join-Path $root 'rust\apps\nxtools-desktop\src\quick_launch') -Filter '*.rs' |
        ForEach-Object { Get-Content -Raw $_.FullName })) -join "`n"
$publisherSource = Get-Content -Raw -LiteralPath (Join-Path $root 'scripts\publish-release-hosts.ps1')
$gateTokens = $null; $gateErrors = $null
$gateAst = [Management.Automation.Language.Parser]::ParseInput($gate, [ref]$gateTokens, [ref]$gateErrors)
if ($gateErrors.Count) { throw 'Production gate must parse before fixtures.' }
$conditions = @($gateAst.FindAll({ param($node)
    $node -is [Management.Automation.Language.IfStatementAst] -and
    $node.Extent.Text.StartsWith('if ($protocolSource -notmatch ''ElevatedDirectoryHelperRequested''')
}, $true))
if ($conditions.Count -ne 1) { throw 'Exactly one production elevated-navigation condition is required.' }
$productionCondition = [scriptblock]::Create($conditions[0].Extent.Text)
function Invoke-ConditionMutation([string]$Variable, [string]$Value) {
    if ($Variable) { Set-Variable -Name $Variable -Value $Value -Scope Local }
    & $productionCondition
}
$assertions = 0
function Assert-SourceRejected([string]$Name, [string]$Variable, [string]$Value) {
    $rejected = $false
    try { Invoke-ConditionMutation $Variable $Value }
    catch {
        if ($_.Exception.Message -notlike 'Release chain source:*' -and
            $_.Exception.Message -notlike 'Quick-launch elevated-navigation regression:*') { throw }
        $rejected = $true
    }
    if (-not $rejected) { throw "Source mutation unexpectedly accepted: $Name" }
    $script:assertions++
    Write-Output "PASS reject $Name"
}
# This is red on the original obsolete direct-writer predicate, before gate repair.
Invoke-ConditionMutation '' ''
$assertions++
Write-Output 'PASS actual delegated production condition (source only)'

$delegateText = "& (Join-Path `$PSScriptRoot 'publish-release-hosts.ps1') -Configuration `$Configuration ``" + "`n" +
    "        -GeneratedDirectory (Join-Path `$hostBuildRoot 'generated') -BuildDirectory `$hostBuildRoot"
$writerText = "& (Join-Path `$PSScriptRoot 'write-release-binary-manifest.ps1') -Profile `$profile -TargetDirectory `$TargetDirectory -ShellHelperDirectory `$helperPublish -OutputPath `$releasePath -ArchiveOutputPath `$archivePath"
if (-not $buildScriptSource.Contains($delegateText) -or -not $publisherSource.Contains($writerText)) {
    throw 'Mutation fixture must locate the exact actual calls.'
}
foreach ($case in @(
    @('delegation removed', 'buildScriptSource', $buildScriptSource.Replace($delegateText, '')),
    @('delegation comment only', 'buildScriptSource', $buildScriptSource.Replace($delegateText, '<# ' + $delegateText + ' #>')),
    @('delegation dead branch', 'buildScriptSource', $buildScriptSource.Replace($delegateText, 'if ($false) { ' + $delegateText + ' }')),
    @('delegation dormant function', 'buildScriptSource', $buildScriptSource.Replace($delegateText, 'function Never-Publish { ' + $delegateText + ' }')),
    @('delegation wrong sibling', 'buildScriptSource', $buildScriptSource.Replace("'publish-release-hosts.ps1'", "'other-publisher.ps1'")),
    @('delegation duplicate', 'buildScriptSource', $buildScriptSource.Replace($delegateText, $delegateText + "`n" + $delegateText)),
    @('delegation wrong config', 'buildScriptSource', $buildScriptSource.Replace('-Configuration $Configuration', '-Configuration $otherConfiguration')),
    @('delegation skip host', 'buildScriptSource', $buildScriptSource.Replace($delegateText, $delegateText + ' -SkipPlatformHost')),
    @('writer removed', 'publisherSource', $publisherSource.Replace($writerText, '')),
    @('writer comment only', 'publisherSource', $publisherSource.Replace($writerText, '# ' + $writerText)),
    @('writer dead branch', 'publisherSource', $publisherSource.Replace($writerText, 'if ($false) { ' + $writerText + ' }')),
    @('writer wrong sibling', 'publisherSource', $publisherSource.Replace("'write-release-binary-manifest.ps1'", "'other-writer.ps1'")),
    @('writer wrong input', 'publisherSource', $publisherSource.Replace('-TargetDirectory $TargetDirectory', '-TargetDirectory $otherTarget')),
    @('writer wrong output', 'publisherSource', $publisherSource.Replace('-OutputPath $releasePath -ArchiveOutputPath', '-OutputPath $otherOutput -ArchiveOutputPath')),
    @('writer before helper', 'publisherSource', $publisherSource.Replace($writerText, '').Replace('$identityPath =', $writerText + "`n" + '$identityPath =')),
    @('helper identity broken', 'publisherSource', $publisherSource.Replace('ShellSelectionIdentityPath=$identityPath', 'ShellSelectionIdentityPath=$otherIdentity')),
    @('host generated input broken', 'publisherSource', $publisherSource.Replace('ReleaseBinaryManifestPath=$releasePath', 'ReleaseBinaryManifestPath=$otherManifest')),
    @('publisher ignores errors', 'publisherSource', $publisherSource.Replace("`$ErrorActionPreference = 'Stop'", "`$ErrorActionPreference = 'Continue'")),
    @('helper exit failure swallowed', 'publisherSource', $publisherSource.Replace('throw "ShellSelectionHelper publish failed: $LASTEXITCODE"', 'Write-Output "ignored"')),
    @('host exit failure swallowed', 'publisherSource', $publisherSource.Replace('throw "PlatformHost publish failed: $LASTEXITCODE"', 'Write-Output "ignored"')),
    @('generated path string changed', 'publisherSource', $publisherSource.Replace("'ReleaseBinaryManifest.g.cs'", "'ReleaseBinaryManifest .g.cs'")),
    @('writer SHA256 weakened', 'manifestWriterSource', $manifestWriterSource.Replace('-Algorithm SHA256', '-Algorithm SHA1')),
    @('writer missing input accepted', 'manifestWriterSource', $manifestWriterSource.Replace('throw "Trusted release binary is missing: $Path"', 'return ""')),
    @('writer Desktop wrong hash', 'manifestWriterSource', $manifestWriterSource.Replace("Get-RequiredHash (Join-Path `$TargetDirectory 'nxtools-desktop.exe')", "Get-RequiredHash (Join-Path `$TargetDirectory 'other.exe')")),
    @('writer helper member missing', 'manifestWriterSource', $manifestWriterSource.Replace(", 'NxTools.ShellSelectionHelper.runtimeconfig.json'", '')),
    @('writer computed identity replaced', 'manifestWriterSource', $manifestWriterSource.Replace('DesktopSha256 = "$desktopHash"', 'DesktopSha256 = "literal"'))
)) { Assert-SourceRejected $case[0] $case[1] $case[2] }

# Preserve the original security predicates, not just the release location assumption.
$securityPredicates = @([regex]::Matches($conditions[0].Clauses[0].Item1.Extent.Text,
    '\$(\w+)\s+(-notmatch|-match)\s+''([^'']*)'''))
$securityCount = 0
foreach ($predicate in $securityPredicates) {
    if ($predicate.Groups[1].Value -eq 'buildScriptSource') { throw 'Obsolete direct-writer predicate is still present.' }
    $variable = $predicate.Groups[1].Value; $pattern = $predicate.Groups[3].Value
    $value = Get-Variable -Name $variable -ValueOnly
    $mutated = if ($predicate.Groups[2].Value -eq '-notmatch') {
        [regex]::Replace($value, $pattern, '__REMOVED_SOURCE_CONTRACT__', [Text.RegularExpressions.RegexOptions]::IgnoreCase)
    } else { $value + "`nrunas" }
    Assert-SourceRejected ($variable + ':' + $pattern) $variable $mutated
    $securityCount++
}
if ($securityCount -ne 73) { throw "Expected all 73 retained security predicates, observed $securityCount." }
Write-Output "PASS Quick Launch release-chain SourceOnly: $assertions assertions; 26 broken chains + 73 retained security predicates; no product/native command executed."
