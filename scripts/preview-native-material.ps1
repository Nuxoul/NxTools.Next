[CmdletBinding(DefaultParameterSetName = 'Preview')]
param(
    [ValidateSet('Workbench', 'Clipboard', 'Search')][string]$Surface = 'Workbench',
    [ValidateSet('Acrylic', 'Solid')][string]$Material = 'Acrylic',
    [switch]$Light,
    [Parameter(ParameterSetName = 'Preview')][switch]$NoBuild,
    [Parameter(ParameterSetName = 'Test', Mandatory = $true)][switch]$TestOnly,
    [Parameter(ParameterSetName = 'Build', Mandatory = $true)][switch]$BuildOnly,
    [Parameter(ParameterSetName = 'Preview')][switch]$PassThru
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')
Initialize-LowMemoryRustBuildEnvironment
$correlation = [Guid]::NewGuid()
$started = [Diagnostics.Stopwatch]::StartNew()
$stage = 'accepted'
$launchedProcessId = 0
$session = $null
$sessionTransferred = $false
Write-Host "ui.native_material.preview.accepted correlation=$correlation"

try {
    $targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
    if (-not [IO.Path]::IsPathRooted($targetRoot)) { $targetRoot = Join-Path $root $targetRoot }
    $targetRoot = [IO.Path]::GetFullPath($targetRoot)
    $candidateExecutable = Join-Path $targetRoot 'debug\preview-native-material\quick_launch_surface.exe'
    $buildExecutable = Join-Path $targetRoot 'debug\quick_launch_surface.exe'
    $powershellExecutable = Join-Path $PSHOME 'powershell.exe'

    if (-not $NoBuild) {
        $stage = 'build_ownership'
        # Only these exact output images block a writer; other preview variants may remain open.
        $running = @(Get-CimInstance Win32_Process -Filter "Name = 'quick_launch_surface.exe'" |
            Where-Object { $_.ExecutablePath -and
                ([string]::Equals($_.ExecutablePath, $candidateExecutable, [StringComparison]::OrdinalIgnoreCase) -or
                 [string]::Equals($_.ExecutablePath, $buildExecutable, [StringComparison]::OrdinalIgnoreCase)) })
        if ($running.Count) { throw 'Close the native-material candidate before writing its build outputs.' }

        $stage = 'native_fixture'
        & $powershellExecutable -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'tests\graphics\native-material-preview.ps1')
        $fixtureExitCode = $LASTEXITCODE
        if ($fixtureExitCode -eq 3) {
            Write-Host "ui.native_material.preview.skipped correlation=$correlation stage=native_fixture exit_code=3 elapsed_ms=$($started.ElapsedMilliseconds)"
            exit 3
        }
        if ($fixtureExitCode -ne 0) { throw 'Native material fixture failed.' }

        $stage = 'build'
        $buildMode = if ($TestOnly) { 'Test' } else { 'Build' }
        Write-Host "ui.native_material.preview.build_started correlation=$correlation"
        & $powershellExecutable -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'build-quick-launch-under-border.ps1') `
            -TargetDirectory $targetRoot -CandidateDirectory 'preview-native-material' -Mode $buildMode
        $buildExitCode = $LASTEXITCODE
        if ($buildExitCode -ne 0) { throw 'Native material isolated builder failed.' }
        Write-Host "ui.native_material.preview.build_finished correlation=$correlation exit_code=$buildExitCode elapsed_ms=$($started.ElapsedMilliseconds)"
        if ($TestOnly -or $BuildOnly) { return }
    }

    $stage = 'candidate'
    if (-not (Test-Path -LiteralPath $candidateExecutable -PathType Leaf)) {
        throw 'Build the native material candidate before using -NoBuild.'
    }
    $stage = 'helper_compile'
    if (-not ('NativeMaterialPreviewWindow' -as [type])) {
        Add-Type -Path (Join-Path $PSScriptRoot 'preview-native-material-window.cs')
    }
    $stage = 'platform'
    $windowsBuild = [NativeMaterialPreviewWindow]::GetWindowsBuild()
    if ($windowsBuild -lt 22621) {
        throw 'The native Accent material proof requires Windows build 22621 or newer for both modes.'
    }

    $stage = 'launch'
    # Every argument is a fixed flag, a ValidateSet token, or a GUID: no shell or path interpolation.
    $previewArguments = @('--native-material-preview', ('--surface=' + $Surface.ToLowerInvariant()),
        ('--material=' + $Material.ToLowerInvariant()), "--correlation=$correlation")
    if ($Light) { $previewArguments += '--light' }
    $scratchRoot = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
    $launchLogs = Join-Path $scratchRoot "native-material-$correlation"
    New-Item -ItemType Directory -Path $launchLogs -ErrorAction Stop | Out-Null
    Write-Host "ui.native_material.preview.launch_started correlation=$correlation"
    $session = New-Object NativeMaterialPreviewSession -ArgumentList $candidateExecutable, ($previewArguments -join ' '), $root, $correlation, `
        (Join-Path $launchLogs 'stdout.log'), (Join-Path $launchLogs 'stderr.log')
    $session.Start(($Material -eq 'Acrylic'), [bool]$Light)
    $launchedProcessId = $session.ProcessId
    # DWM38 is None in both modes. The released GPUI Accent policy alone selects native blur;
    # BOOL acceptance is not pixel evidence. The scalar receipt reports requested Accent state.
    # FrameBefore*/FrameAfter* report physical client-preserving fixed-frame normalization.
    Write-Host ('ui.native_material.preview.native_receipt ' + ($session.Receipt | ConvertTo-Json -Compress))
    Write-Host "ui.native_material.preview.ready correlation=$correlation pid=$launchedProcessId elapsed_ms=$($started.ElapsedMilliseconds)"
    # -PassThru transfers ownership: keep this PowerShell host alive, then WaitForExit(), inspect Failure,
    # and Dispose(). Only Receipt/LastThemeReceipt are scalar JSON contracts, never the session itself.
    if ($PassThru) {
        Write-Host "ui.native_material.preview.session_transferred correlation=$correlation pid=$launchedProcessId requires_wait_and_dispose=true"
        $sessionTransferred = $true
        return $session
    }
    $stage = 'preview_wait'
    $previewExitCode = $session.WaitForExit()
    Write-Host "ui.native_material.preview.process_exited correlation=$correlation pid=$launchedProcessId exit_code=$previewExitCode theme_apply_count=$($session.ThemeApplyCount)"
    if ($session.Failure) { throw $session.Failure }
    if ($previewExitCode -ne 0) { throw 'Native material preview returned a nonzero exit code.' }
}
catch {
    $failure = $_.Exception
    while ($failure.InnerException) { $failure = $failure.InnerException }
    $nativeResult = 0
    if ($failure.GetType().Name -eq 'NativeMaterialPreviewException') {
        $stage = $failure.Stage
        $nativeResult = $failure.NativeResult
    }
    Write-Host "ui.native_material.preview.failed correlation=$correlation stage=$stage pid=$launchedProcessId hresult=$($failure.HResult) native_result=$nativeResult elapsed_ms=$($started.ElapsedMilliseconds)"
    # Leave even the owned preview available for inspection; never terminate another comparison.
    throw 'Native material preview failed; inspect the correlated stage and native results.'
}
finally {
    if ($session -and -not $sessionTransferred) {
        $session.Dispose()
        Write-Host "ui.native_material.preview.cleaned_up correlation=$correlation pid=$($session.ProcessId) exit_code=$($session.ExitCode) elapsed_ms=$($started.ElapsedMilliseconds)"
    }
}
