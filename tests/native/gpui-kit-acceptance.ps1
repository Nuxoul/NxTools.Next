param(
    [ValidateSet('quick','clipboard')][string]$Mode='quick',
    [ValidateSet('All','ClientAndUia')][string]$Check='All',
    [switch]$AuthorizeNative
)
$ErrorActionPreference='Stop'
if (-not $AuthorizeNative) {throw 'Owned native acceptance requires explicit -AuthorizeNative approval before starting a fixture.'}
$workspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if (-not $env:PI_SCRATCH_DIR) {throw 'Explicit session scratch directory required.'}
$fixtureExe=Join-Path $workspaceRoot 'target\debug\nxtools-desktop.exe'
$harnessExe=Join-Path $workspaceRoot 'csharp\NxTools.PlatformHost.Tests\bin\Debug\net8.0-windows\NxTools.PlatformHost.Tests.exe'
if (-not (Test-Path $fixtureExe) -or -not (Test-Path $harnessExe)) {throw 'Build Debug desktop and exact C# test harness first; this script never builds or packages.'}
$fixtureLog=Join-Path $env:PI_SCRATCH_DIR "native-acceptance-$Mode.log"
$fixtureErr=Join-Path $env:PI_SCRATCH_DIR "native-acceptance-$Mode.err.log"
$environmentNames=@('NXTOOLS_PREVIEW_NATIVE_ACCEPTANCE','NXTOOLS_ACCEPTANCE_PID','NXTOOLS_ACCEPTANCE_EXE','NXTOOLS_ACCEPTANCE_PRIMARY','NXTOOLS_ACCEPTANCE_PASSIVE','NXTOOLS_ACCEPTANCE_WIDTH','NXTOOLS_ACCEPTANCE_HEIGHT')
$previousEnvironment=@{}
foreach($name in $environmentNames){$previousEnvironment[$name]=[Environment]::GetEnvironmentVariable($name,'Process')}
$fixtureProcess=$null
$results=@()
try {
    $env:NXTOOLS_PREVIEW_NATIVE_ACCEPTANCE=$Mode
    $fixtureProcess=Start-Process -FilePath $fixtureExe -ArgumentList @('--ui-preview','foundation') -WorkingDirectory $workspaceRoot -RedirectStandardOutput $fixtureLog -RedirectStandardError $fixtureErr -PassThru
    $null=$fixtureProcess.Handle
    $deadline=[DateTime]::UtcNow.AddSeconds(30)
    do {
        Start-Sleep -Milliseconds 100
        $fixtureProcess.Refresh()
        if($fixtureProcess.HasExited){throw 'Synthetic acceptance fixture exited before ready.'}
        $text=if(Test-Path $fixtureLog){[string](Get-Content -LiteralPath $fixtureLog -Raw)}else{''}
        $primary=[regex]::Match($text,'ui.acceptance.primary hwnd=(\d+) scale=([\d.]+)')
        $passive=[regex]::Match($text,'ui.acceptance.passive hwnd=(\d+) scale=([\d.]+) width=([\d.]+) height=([\d.]+) hidden=true')
    }until(($primary.Success -and $passive.Success) -or [DateTime]::UtcNow -ge $deadline)
    if(-not $primary.Success -or -not $passive.Success){throw 'Owned HWND ready markers unavailable.'}
    if($fixtureProcess.Path -ne $fixtureExe){throw 'Fixture executable identity mismatch.'}
    $env:NXTOOLS_ACCEPTANCE_PID=$fixtureProcess.Id.ToString()
    $env:NXTOOLS_ACCEPTANCE_EXE=$fixtureExe
    $env:NXTOOLS_ACCEPTANCE_PRIMARY=$primary.Groups[1].Value
    $env:NXTOOLS_ACCEPTANCE_PASSIVE=$passive.Groups[1].Value
    $env:NXTOOLS_ACCEPTANCE_WIDTH=$passive.Groups[3].Value
    $env:NXTOOLS_ACCEPTANCE_HEIGHT=$passive.Groups[4].Value
    $filters=@('owned GPUI native client and UIA acceptance')
    if($Check -eq 'All'){$filters=@('owned GPUI passive preview rejects mouse activation')+$filters}
    foreach($filter in $filters){
        $suffix=if($filter -like '*mouse activation'){'passive'}else{'uia'}
        $testLog=Join-Path $env:PI_SCRATCH_DIR "native-acceptance-$Mode-$suffix.log"
        $testErr=Join-Path $env:PI_SCRATCH_DIR "native-acceptance-$Mode-$suffix.err.log"
        # Exact, preselected filters only. No unfiltered native or DOpus harness.
        $testProcess=Start-Process -FilePath $harnessExe -ArgumentList ('--native --authorize-native "'+$filter+'"') -WorkingDirectory $workspaceRoot -RedirectStandardOutput $testLog -RedirectStandardError $testErr -PassThru
        $null=$testProcess.Handle
        if(-not $testProcess.WaitForExit(30000)){
            if($testProcess.Path -eq $harnessExe){$testProcess.Kill();$testProcess.WaitForExit()}
            throw 'Owned native acceptance harness timed out.'
        }
        $testProcess.Refresh()
        $testExit=$testProcess.ExitCode
        if($null -eq $testExit){throw 'Actual native harness exit code unavailable.'}
        $out=[IO.File]::ReadAllText($testLog)
        $err=[IO.File]::ReadAllText($testErr)
        $runNames=@([regex]::Matches($out,'(?m)^RUN (.+)\r?$') | ForEach-Object {$_.Groups[1].Value.TrimEnd()})
        if($runNames.Count -ne 1 -or $runNames[0] -ne $filter){throw 'Native test filter expanded or matched zero tests.'}
        $passed=$testExit -eq 0 -and $out.Contains("PASS $filter")
        $results+=[pscustomobject]@{Test=$filter;Exit=$testExit;Passed=$passed;Output=$out;Error=$err}
    }
    [pscustomobject]@{Mode=$Mode;Check=$Check;FixturePid=$fixtureProcess.Id;Primary=[long]$env:NXTOOLS_ACCEPTANCE_PRIMARY;Passive=[long]$env:NXTOOLS_ACCEPTANCE_PASSIVE;Scale=$primary.Groups[2].Value;LogicalWidth=$env:NXTOOLS_ACCEPTANCE_WIDTH;LogicalHeight=$env:NXTOOLS_ACCEPTANCE_HEIGHT;Results=$results} | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $env:PI_SCRATCH_DIR "native-acceptance-$Mode-evidence.json") -Encoding UTF8
    $results | ConvertTo-Json -Depth 5
}finally{
    if($fixtureProcess){
        $fixtureProcess.Refresh()
        if(-not $fixtureProcess.HasExited -and $fixtureProcess.Path -eq $fixtureExe){
            [void]$fixtureProcess.CloseMainWindow()
            if(-not $fixtureProcess.WaitForExit(3000)){$fixtureProcess.Kill();$fixtureProcess.WaitForExit()}
        }
    }
    foreach($name in $environmentNames){[Environment]::SetEnvironmentVariable($name,$previousEnvironment[$name],'Process')}
}
if(@($results | Where-Object {-not $_.Passed}).Count -gt 0){exit 1}
