# Pure orchestration parsing plus hidden, self-owned HWND fixtures. No candidate launch.
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$runner = Join-Path $PSScriptRoot 'native-material-benchmark.ps1'
$parseTokens = $null; $parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($runner, [ref]$parseTokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw ($parseErrors | Out-String) }
. $runner
$script:assertions = 0
function Assert-True([bool] $Condition, [string] $Message) {
    if (-not $Condition) { throw $Message }
    $script:assertions++
}
$correlation = [Guid]::NewGuid()
foreach ($phase in @('idle', 'input', 'scroll')) {
    $start = ConvertFrom-MaterialBenchmarkEvent "2026-11-21 INFO ui.material_benchmark.phase_started correlation=$correlation phase=$phase generation=1 updates=0 visible_row=0" $correlation
    Assert-True ($start.Event -eq 'phase_started' -and $start.Phase -eq $phase -and $start.Generation -eq 1 -and $start.Updates -eq 0 -and $start.VisibleRow -eq 0) 'Start parser must retain all scalar fields.'
    $finish = ConvertFrom-MaterialBenchmarkEvent "ui.material_benchmark.phase_finished correlation=$correlation phase=$phase generation=1 updates=12 visible_row=0 max_rendered_row=6`r" $correlation
    Assert-True ($finish.Event -eq 'phase_finished' -and $finish.Updates -eq 12 -and $finish.VisibleRow -eq 0 -and $finish.MaxRenderedRow -eq 6) 'Finish parser must retain actual rendered-row evidence and CRLF.'
}
$completeLine = "ui.material_benchmark.completed correlation=$correlation"
Assert-True ((ConvertFrom-MaterialBenchmarkEvent $completeLine $correlation).Event -eq 'completed') 'Completion must parse.'
Assert-True ($null -eq (ConvertFrom-MaterialBenchmarkEvent $completeLine ([Guid]::NewGuid()))) 'Foreign correlations cannot advance a run.'
foreach ($invalidLine in @('unrelated log', "ui.material_benchmark.phase_started correlation=$correlation phase=background generation=1 updates=0 visible_row=0", "ui.material_benchmark.phase_finished correlation=$correlation phase=scroll generation=1 updates=-1 visible_row=0", ($completeLine + ' payload=unexpected'))) {
    Assert-True ($null -eq (ConvertFrom-MaterialBenchmarkEvent $invalidLine $correlation)) 'Malformed event must not match.'
}
Assert-True (((Get-MaterialBenchmarkOrder 1) -join ',') -eq 'solid,acrylic') 'First repetition must start Solid.'
Assert-True (((Get-MaterialBenchmarkOrder 2) -join ',') -eq 'acrylic,solid') 'Second repetition must reverse order.'
Assert-True (((Get-MaterialBenchmarkOrder 3) -join ',') -eq 'solid,acrylic') 'Third repetition must restore order.'
$logPath = Join-Path $env:PI_SCRATCH_DIR ('benchmark-log-fixture-' + [Guid]::NewGuid().ToString('N') + '.log')
$logWriter = [IO.StreamWriter]::new($logPath, $false, [Text.UTF8Encoding]::new($false))
try {
    $logWriter.WriteLine($completeLine); $logWriter.Flush()
    Assert-True ((Read-MaterialBenchmarkLog $logPath).Trim() -eq $completeLine) 'Polling must read the log while its redirect writer remains open.'
    $logWriter.Write('partial'); $logWriter.Flush()
    Assert-True ((Read-MaterialBenchmarkLog $logPath).EndsWith('partial')) 'Polling must preserve partial output for the next pass.'
}
finally { $logWriter.Dispose(); Remove-Item -LiteralPath $logPath }

# Compile runner-local WM_CLOSE helper without executing the runner.
$closeSource = $ast.FindAll({ param($node) $node -is [Management.Automation.Language.StringConstantExpressionAst] -and $node.Value -like '*public static class NativeMaterialBenchmarkClose*' }, $true)[0].Value
Add-Type -TypeDefinition $closeSource
$helperPath = Join-Path $PSScriptRoot '..\..\scripts\preview-native-material-window.cs'
$fixture = @'
public static class NativeMaterialBenchmarkWindowFixture {
    [System.Runtime.InteropServices.DllImport("user32.dll", CharSet=System.Runtime.InteropServices.CharSet.Unicode)]
    private static extern System.IntPtr CreateWindowEx(uint extended, string cls, string title, uint style, int x, int y, int w, int h, System.IntPtr parent, System.IntPtr menu, System.IntPtr instance, System.IntPtr parameter);
    [System.Runtime.InteropServices.DllImport("user32.dll")]
    private static extern bool DestroyWindow(System.IntPtr window);
    public static int Run() {
        int processId = System.Diagnostics.Process.GetCurrentProcess().Id;
        const string title = "NxTools \u2014 Native Material Preview (F2 / Esc)";
        System.IntPtr[] windows = new System.IntPtr[4];
        int count = 0;
        try {
            for (int i=0; i<4; i++) {
                windows[i] = CreateWindowEx(0, "STATIC", i==3 ? title+" other" : title, 0x80000000u, 50+i*100, 50, 90, 90, System.IntPtr.Zero, System.IntPtr.Zero, System.IntPtr.Zero, System.IntPtr.Zero);
                Check(windows[i] != System.IntPtr.Zero); count++;
            }
            long[] found = NativeMaterialPreviewWindow.FindWindows(processId);
            Check(found.Length == 3); count++;
            for (int i=0; i<3; i++) { Check(System.Array.IndexOf(found, windows[i].ToInt64()) >= 0); count++; }
            Check(NativeMaterialPreviewWindow.FindWindows(int.MaxValue).Length == 0); count++;
            try { NativeMaterialPreviewWindow.FindWindows(0); throw new System.Exception("Invalid PID accepted"); }
            catch (NativeMaterialPreviewException failure) { Check(failure.Stage == "process_identity"); count++; }
            System.Guid correlation = System.Guid.NewGuid();
            for (int i=0; i<3; i++) {
                NativeMaterialPreviewReceipt receipt = NativeMaterialPreviewWindow.Prepare(processId, windows[i].ToInt64(), i==1, false, correlation);
                Check(receipt.WindowHandle == windows[i].ToInt64() && receipt.Correlation == correlation && receipt.AccentAccepted && receipt.RequestedAccentState == (i==1 ? 4 : 0) && receipt.FrameClientGeometryPreserved && receipt.FrameForegroundPreserved); count++;
            }
            try { NativeMaterialPreviewWindow.Prepare(processId, windows[3].ToInt64(), false, false, correlation); throw new System.Exception("Wrong title accepted"); }
            catch (NativeMaterialPreviewException failure) { Check(failure.Stage == "window_identity"); count++; }
            try { NativeMaterialPreviewWindow.Prepare(int.MaxValue, windows[0].ToInt64(), false, false, correlation); throw new System.Exception("Wrong PID accepted"); }
            catch (NativeMaterialPreviewException failure) { Check(failure.Stage == "window_identity"); count++; }
            return count;
        }
        finally { foreach (System.IntPtr window in windows) if (window != System.IntPtr.Zero) DestroyWindow(window); }
    }
    private static void Check(bool condition) { if (!condition) throw new System.Exception("Owned HWND fixture assertion failed"); }
}
'@
Add-Type -TypeDefinition ([IO.File]::ReadAllText((Resolve-Path -LiteralPath $helperPath).Path) + "`n" + $fixture)
$nativeAssertions = [NativeMaterialBenchmarkWindowFixture]::Run()
[pscustomobject]@{ Result = 'passed'; ParserAndOrderAssertions = $script:assertions; NativeWindowAssertions = $nativeAssertions; CandidateLaunches = 0 }
