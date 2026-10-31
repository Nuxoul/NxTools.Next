param(
    [ValidateSet('all','key','trust','label','preflight')][string]$Scenario = 'all',
    [string]$EvidenceRoot = $env:PI_SCRATCH_DIR
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$work = Join-Path $EvidenceRoot ('staging-cleanup-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
$previousScratch = $env:PI_SCRATCH_DIR
$previousKey = $env:NXTOOLS_RELEASE_SIGNING_KEY
$checks = 0
try {
    foreach ($case in $(if ($Scenario -eq 'all') { @('key','trust','label','preflight') } else { @($Scenario) })) {
        $area = Join-Path $work $case
        $scripts = Join-Path $area 'scripts'
        New-Item -ItemType Directory -Path $scripts | Out-Null
        $foreignKey = Join-Path $area 'foreign-key.keep'
        [IO.File]::WriteAllText($foreignKey, 'unrelated owner marker')
        # Run an unchanged copy of the actual wrapper. Only external tool boundaries are replaced.
        $wrapper = Join-Path $scripts 'package.ps1'
        Copy-Item -LiteralPath (Join-Path $root 'scripts\package.ps1') -Destination $wrapper
        if ((Get-FileHash $wrapper).Hash -ne (Get-FileHash (Join-Path $root 'scripts\package.ps1')).Hash) { throw 'Wrapper copy differs.' }
        [IO.File]::WriteAllText((Join-Path $scripts 'new-release-signing-key.ps1'), @'
param([string]$KeyPath)
[IO.File]::WriteAllText($KeyPath, 'synthetic owned-key marker; not a private key')
[IO.File]::WriteAllText((Join-Path $env:PI_SCRATCH_DIR 'created-key-path.txt'), $KeyPath)
if ((Split-Path -Leaf $env:PI_SCRATCH_DIR) -eq 'key') { throw 'fixture_key_failed_after_create' }
'@)
        [IO.File]::WriteAllText((Join-Path $scripts 'write-update-trust.ps1'), @'
param([string]$SigningKeyPath,[string]$OutputPath)
if (!(Test-Path -LiteralPath $SigningKeyPath)) { throw 'fixture_key_not_created' }
if ((Split-Path -Leaf $env:PI_SCRATCH_DIR) -eq 'trust') { throw 'fixture_trust_failed_after_key' }
[IO.File]::WriteAllText($OutputPath, '// synthetic test trust only')
'@)
        [IO.File]::WriteAllText((Join-Path $scripts 'windows-build-tools.ps1'), @'
function Initialize-LowMemoryRustBuildEnvironment {
    $env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'
}
function Initialize-GpuiReleaseEnvironment { }
function Assert-ReleaseBuildCommitHeadroom {
    param($MinimumGiB)
    [IO.File]::WriteAllText((Join-Path $env:PI_SCRATCH_DIR 'preflight-reached.txt'), 'before Cargo')
    throw 'fixture_preflight_failed_after_key'
}
function Assert-CargoBuildLaneIdle { throw 'Forbidden Cargo boundary reached.' }
'@)
        $env:PI_SCRATCH_DIR = $area
        $env:NXTOOLS_RELEASE_SIGNING_KEY = ''
        $destination = if ($case -eq 'label') { Join-Path $area 'missing-parent\stage' } else { Join-Path $area 'stage' }
        $failed = $false; $failure = ''
        try { & $wrapper -Configuration Release -OutputPath $destination -SkipVerification -SkipInstaller -IsolatedStaging *> (Join-Path $area 'wrapper.log') }
        catch { $failed = $true; $failure = $_.Exception.Message }
        $keyPath = [IO.File]::ReadAllText((Join-Path $area 'created-key-path.txt'))
        $keyExists = Test-Path -LiteralPath $keyPath -PathType Leaf
        $expected = switch ($case) {
            'key' { $failure -like '*fixture_key_failed_after_create*' }
            'trust' { $failure -like '*fixture_trust_failed_after_key*' }
            'label' { $failure -like '*STAGING-NOT-INSTALLABLE*' -and !(Test-Path (Split-Path -Parent $destination)) }
            'preflight' { $failure -like '*fixture_preflight_failed_after_key*' -and (Test-Path (Join-Path $area 'preflight-reached.txt')) }
        }
        [pscustomobject]@{scenario=$case;failed=$failed;expected_failure=$expected;key_remaining=$keyExists;output_created=(Test-Path $destination);cargo_started=$false} |
            ConvertTo-Json | Set-Content (Join-Path $area 'result.json') -Encoding UTF8
        # The test only cleans its exact recorded synthetic marker if the old implementation leaked it.
        if ($keyExists) {
            if (![IO.Path]::GetFullPath($keyPath).StartsWith($area + '\', [StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($keyPath) -ne 'TEST-ONLY-private.pem') { throw 'Unexpected fixture key owner.' }
            Remove-Item -LiteralPath $keyPath -Force
        }
        $foreignIntact = (Test-Path -LiteralPath $foreignKey) -and [IO.File]::ReadAllText($foreignKey) -eq 'unrelated owner marker'
        if (!$failed -or !$expected -or $keyExists -or (Test-Path $destination) -or !$foreignIntact) { throw "FAIL staging cleanup $case : expected_failure=$expected key_remaining=$keyExists output_created=$(Test-Path $destination) foreign_intact=$foreignIntact" }
        $checks += 5
        Write-Output "PASS staging cleanup $case : failure returned, exact owned key absent, pre-Cargo only"
    }
    Write-Output "PASS staging key cleanup: $checks assertions; no Cargo/product/service/installer or private-key content; evidence=$work"
} finally { $env:PI_SCRATCH_DIR=$previousScratch; $env:NXTOOLS_RELEASE_SIGNING_KEY=$previousKey }
