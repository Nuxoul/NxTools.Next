param([string]$HelperPath = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (!$HelperPath) { $HelperPath = Join-Path $root 'installer\close-owned-processes.ps1' }
. $HelperPath -LibraryOnly
Initialize-UpgradeNative
$work = Join-Path $env:PI_SCRATCH_DIR ('installer-exit-race-' + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($work) | Out-Null
$checks = 0
function Check($Condition, [string]$Label) {
    if (!$Condition) { throw "FAIL $Label" }
    $script:checks++; Write-Output "PASS $Label"
}
$children = @()
try {
    # Keep the original process object/handle after exit: Windows still permits OpenProcess,
    # but QueryFullProcessImageName fails31. No real application is discovered or stopped.
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = Join-Path $env:SystemRoot 'System32\cmd.exe'
    $info.Arguments = '/d /c exit 0'
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $exited = New-Object Diagnostics.Process
    $exited.StartInfo = $info
    Check ($exited.Start()) 'owned short child started'
    $children += $exited
    $retainedHandle = $exited.Handle
    $exitedId = $exited.Id
    Check ($exited.WaitForExit(10000)) 'owned short child exited with retained native handle'
    try { $closed = [NxToolsUpgradeProcess]::Stop($exitedId, $info.FileName) }
    catch {
        $failure = $_.Exception
        while ($failure.InnerException) { $failure = $failure.InnerException }
        throw "RED exited retained process must be harmless; actual native=$($failure.NativeErrorCode) bits=$([IntPtr]::Size*8)"
    }
    Check (!$closed) 'already exited same handle returns no termination instead of native31'
    $script:staleDelivered = $false
    $script:staleProcessId = $exitedId
    $script:staleProcessImage = $info.FileName
    function Get-CimInstance { param($ClassName,$Filter)
        if (!$script:staleDelivered -and $Filter -eq "Name='NxTools.exe'") {
            $script:staleDelivered = $true
            return [pscustomobject]@{ProcessId=$script:staleProcessId;ExecutablePath=$script:staleProcessImage}
        }
        return @()
    }
    $staleTargets = @{ $info.FileName = 'owned-fixture' }
    Check ((Stop-UpgradeTargets $staleTargets) -eq 0 -and $script:staleDelivered) 'original discovery loop tolerates exact stale exited candidate without counting termination'
    $script:staleDelivered = $false
    $script:staleProcessImage = $null
    Check ((Stop-UpgradeTargets $staleTargets) -eq 0 -and $script:staleDelivered) 'CIM-null exited candidate is checked at original discovery boundary, not reported as access denied'
    $script:staleProcessImage = $info.FileName


    $fixture = Join-Path $work 'owned.exe'
    Add-Type -TypeDefinition 'public static class ExitRaceFixture { public static void Main() { System.Threading.Thread.Sleep(60000); } }' -OutputAssembly $fixture -OutputType ConsoleApplication
    $live = Start-Process -FilePath $fixture -PassThru -WindowStyle Hidden
    $children += $live
    Check ([NxToolsUpgradeProcess]::QueryImage($live.Id) -eq $fixture) 'query-only native discovery identifies live owned child without CIM path'
    Check (![NxToolsUpgradeProcess]::Stop($live.Id, (Join-Path $work 'foreign.exe'))) 'live mismatched image refused without termination'
    $live.Refresh()
    Check (!$live.HasExited) 'mismatched live child remains alive'
    $script:staleDelivered = $false
    $script:staleProcessId = $live.Id
    $script:staleProcessImage = $null
    Check ((Stop-UpgradeTargets @{ $fixture='owned-fixture' }) -eq 1) 'CIM-null live owned child reaches original exact-image stop boundary'
    Check ($live.WaitForExit(5000)) 'exact owned child exit confirmed'

    # Deterministic API-boundary races: run the real production Stop body with only native
    # API answers controlled. Live failures must remain errors, never blanket31 suppression.
    $text = [IO.File]::ReadAllText($HelperPath)
    $body = [regex]::Match($text, "(?s)Add-Type -TypeDefinition @'\r?\n(.*?)\r?\n'@").Groups[1].Value
    if (!$body) { throw 'Missing production native type' }
    $body = $body.Replace('public static class NxToolsUpgradeProcess', 'public static class ExitRaceNativeFixture')
    $nativeDeclarations = '(?m)^\s*\[DllImport\("kernel32.dll"[^\r\n]*\)\]\s*static extern (?:IntPtr OpenProcess|bool QueryFullProcessImageName|bool TerminateProcess|uint WaitForSingleObject|bool CloseHandle)[^\r\n]*;'
    $nativeMatches = [regex]::Matches($body, $nativeDeclarations)
    Check ($nativeMatches.Count -eq 5) 'exactly five native process APIs isolated, production Stop body retained'
    $body = [regex]::Replace($body, $nativeDeclarations, '')
    # Managed API substitutes cannot set CLR's P/Invoke last-error cache; substitute only
    # that native error observation, keeping every production control-flow branch intact.
    $body = $body.Replace('Marshal.GetLastWin32Error()', 'LastError')
    $fake = @'
    public static string Scenario;
    public static int QueryCalls, TerminateCalls, WaitCalls, CloseCalls;
    public static uint LastAccess;
    static int LastError;
    static void SetLastError(uint code) { LastError=(int)code; }
    public static void Reset(string value) { Scenario=value; QueryCalls=0; TerminateCalls=0; WaitCalls=0; CloseCalls=0; }
    static IntPtr OpenProcess(uint access, bool inherit, int id) {
        LastAccess=access;
        if (Scenario=="open-denied") { LastError=5; return IntPtr.Zero; }
        if (Scenario=="open-gone") { LastError=87; return IntPtr.Zero; }
        return new IntPtr(123);
    }
    static bool QueryFullProcessImageName(IntPtr handle, uint flags, StringBuilder path, ref int size) {
        QueryCalls++;
        if (Scenario.StartsWith("query")) { SetLastError(Scenario=="query-pending5"?5u:31u); return false; }
        path.Append(@"C:\owned\fixture.exe"); return true;
    }
    static bool TerminateProcess(IntPtr handle, uint code) { TerminateCalls++; SetLastError(Scenario=="terminate-live5" || Scenario=="terminate-pending5"?5u:31u); return false; }
    static uint WaitForSingleObject(IntPtr handle, uint ms) {
        WaitCalls++;
        if (Scenario=="exited" || Scenario=="query-exit" && QueryCalls>0 || Scenario=="terminate-exit" && TerminateCalls>0) return 0;
        if (ms>0 && (Scenario=="query-pending5" && QueryCalls>0 || Scenario=="terminate-pending5" && TerminateCalls>0)) return 0;
        if (Scenario=="wait-failed") { SetLastError(6); return 0xFFFFFFFF; }
        return 258;
    }
    static bool CloseHandle(IntPtr handle) { CloseCalls++; return true; }
'@
    $body = $body.Replace('public static class ExitRaceNativeFixture {', 'public static class ExitRaceNativeFixture {' + [Environment]::NewLine + $fake)
    Add-Type -TypeDefinition $body
    foreach ($scenario in @('exited','query-exit','query-pending5','terminate-exit','terminate-pending5','query-live','terminate-live31','terminate-live5','wait-failed')) {
        [ExitRaceNativeFixture]::Reset($scenario)
        $failure = $null; $closed = $null
        try { $closed = [ExitRaceNativeFixture]::Stop(123, 'C:\owned\fixture.exe') }
        catch { $failure=$_.Exception; while($failure.InnerException){$failure=$failure.InnerException} }
        if ($scenario -in @('exited','query-exit','query-pending5','terminate-exit','terminate-pending5')) {
            Check ($null -eq $failure -and $closed -eq $false) "$scenario accepts only same-handle observed exit"
        } else {
            $expected = if ($scenario -eq 'terminate-live5') {5} elseif ($scenario -eq 'wait-failed') {6} else {31}
            Check ($failure -is [ComponentModel.Win32Exception] -and $failure.NativeErrorCode -eq $expected) "$scenario retains real native failure$expected"
        }
        Check ([ExitRaceNativeFixture]::CloseCalls -eq 1) "$scenario closes exact acquired handle once"
        if ($scenario -eq 'exited') { Check ([ExitRaceNativeFixture]::QueryCalls -eq 0 -and [ExitRaceNativeFixture]::TerminateCalls -eq 0) 'exited handle never queries image or terminates' }
        if ($scenario -eq 'query-live') { Check ([ExitRaceNativeFixture]::TerminateCalls -eq 0) 'unknown live image never reaches termination' }
    }
    foreach ($scenario in @('exited','query-exit','query-pending5','query-live','wait-failed','live','open-denied','open-gone')) {
        [ExitRaceNativeFixture]::Reset($scenario)
        $failure=$null; $image=$null
        try { $image=[ExitRaceNativeFixture]::QueryImage(123) }
        catch { $failure=$_.Exception; while($failure.InnerException){$failure=$failure.InnerException} }
        if ($scenario -in @('exited','query-exit','query-pending5','open-gone')) { Check ($null-eq $failure -and $null-eq $image) "$scenario query discovery proves exit before ignoring candidate" }
        elseif($scenario-eq 'live') { Check ($image-eq 'C:\owned\fixture.exe') 'live query returns exact native path' }
        else { Check ($failure-is [ComponentModel.Win32Exception] -and ($scenario-ne 'open-denied' -or $failure.NativeErrorCode-eq 5)) "$scenario query failure is not silently ignored" }
        $expectedClose=if($scenario.StartsWith('open-')){0}else{1}
        Check ([ExitRaceNativeFixture]::LastAccess-eq 0x00101000 -and [ExitRaceNativeFixture]::TerminateCalls-eq 0 -and [ExitRaceNativeFixture]::CloseCalls-eq $expectedClose) "$scenario discovery uses query/synchronize only and closes its handle"
    }
    Write-Output "PASS installer exit race $checks checks bits=$([IntPtr]::Size*8); owned children/native API fixtures only; evidence=$work"
} finally {
    foreach ($child in $children) {
        $child.Refresh()
        if (!$child.HasExited) { $child.Kill(); $child.WaitForExit(5000) | Out-Null }
        $child.Dispose()
    }
}
