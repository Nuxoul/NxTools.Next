$ErrorActionPreference = 'Stop'
$samplerPath = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\scripts\observe-platformhost-memory.ps1'))
$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) ('nxtools-memory-sampler-test-' + [Guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $fixtureRoot
$fixtureHostPath = Join-Path $fixtureRoot 'NxTools.PlatformHost.exe'
$fixtureDesktopPath = Join-Path $fixtureRoot 'NxTools.exe'
$fixtureCreated = [DateTime]::Parse('2026-11-06T00:00:10Z').ToUniversalTime()
$fixtureParent = [pscustomobject]@{ ProcessId=12000; ExecutablePath=$fixtureDesktopPath; SessionId=1; CreationDate=$fixtureCreated.AddSeconds(-10) }
$fixtureHost = [pscustomobject]@{ ProcessId=12004; ParentProcessId=12000; ExecutablePath=$fixtureHostPath; SessionId=1;
    CreationDate=$fixtureCreated; CommandLine=('"' + $fixtureHostPath + '" --parent-pid 12000') }
$global:HostMemorySamplerFixture = @{ Facts=@($fixtureHost); Parent=$fixtureParent; HostQueries=0; Launches=0; Kills=0;
    NativeReads=0; Mode='complete'; Arguments=@(); NativeDisposals=0; CollectorDisposals=0 }
$passed = 0
function Assert-Fixture([bool]$Condition, [string]$Name) {
    if (-not $Condition) { throw "FAIL $Name" }
    $script:passed++
    Write-Output "PASS $Name"
}
# These functions shadow OS/tool calls only inside this isolated test process.
function Get-CimInstance {
    [CmdletBinding()]param($ClassName, [string]$Filter, $OperationTimeoutSec)
    if ($Filter -like 'Name =*') {
        $global:HostMemorySamplerFixture.HostQueries++
        if ($global:HostMemorySamplerFixture.Mode -eq 'replaced' -and $global:HostMemorySamplerFixture.HostQueries -gt 1) {
            $replacement = $global:HostMemorySamplerFixture.Facts[0].PSObject.Copy()
            $replacement.CreationDate = $replacement.CreationDate.AddMinutes(1)
            return $replacement
        }
        return $global:HostMemorySamplerFixture.Facts
    }
    return $global:HostMemorySamplerFixture.Parent
}
function Get-Command {
    [CmdletBinding()]param($Name, $CommandType)
    return [pscustomobject]@{ Source='C:\owned-fixture\dotnet-counters.exe' }
}
function Start-Process {
    [CmdletBinding()]param($FilePath, $ArgumentList, [switch]$PassThru, [switch]$NoNewWindow, $RedirectStandardOutput, $RedirectStandardError)
    $global:HostMemorySamplerFixture.Launches++
    $global:HostMemorySamplerFixture.Arguments = $ArgumentList
    $runtime = $ArgumentList[[Array]::IndexOf($ArgumentList, '--output') + 1].Trim('"')
    $counterRows = @([pscustomobject]@{ Timestamp='2026-11-06 00:00:10'; Provider='System.Runtime'; 'Counter Name'='GC Heap Size (MB)'; 'Mean/Increment'=1 })
    if ($global:HostMemorySamplerFixture.Mode -ne 'missing_gc') {
        $counterRows += [pscustomobject]@{ Timestamp='2026-11-06 00:00:10'; Provider='System.Runtime'; 'Counter Name'='GC Committed Bytes (MB)'; 'Mean/Increment'=2 }
    }
    $counterRows | Export-Csv -LiteralPath $runtime -NoTypeInformation -Encoding UTF8
    $fake = [pscustomobject]@{ HasExited=($global:HostMemorySamplerFixture.Mode -eq 'counter_error'); ExitCode=0 }
    if ($fake.HasExited) { $fake.ExitCode=1 }
    $fake | Add-Member ScriptMethod WaitForExit { param($Milliseconds) $this.HasExited=$true; return $true }
    $fake | Add-Member ScriptMethod Kill { $global:HostMemorySamplerFixture.Kills++; $this.HasExited=$true }
    $fake | Add-Member ScriptMethod Dispose { $global:HostMemorySamplerFixture.CollectorDisposals++ }
    return $fake
}
function Get-Process {
    [CmdletBinding()]param($Id)
    $global:HostMemorySamplerFixture.NativeReads++
    $fake = [pscustomobject]@{ StartTime=$fixtureCreated; PrivateMemorySize64=(100MB + $global:HostMemorySamplerFixture.NativeReads * 1MB);
        WorkingSet64=40MB; HandleCount=100; Threads=@(1,2) }
    $fake | Add-Member ScriptMethod Refresh { }
    $fake | Add-Member ScriptMethod Dispose { $global:HostMemorySamplerFixture.NativeDisposals++ }
    return $fake
}
try {
    . $samplerPath -HostExePath $fixtureHostPath -DesktopExePath $fixtureDesktopPath -OutputDirectory (Join-Path $fixtureRoot 'unused')
    Assert-Fixture (Test-HostMemoryIdentity $fixtureHost $fixtureParent $fixtureHostPath $fixtureDesktopPath 12000) 'ordinary exact identity accepted'
    foreach ($case in @(
        @{Name='foreign Host image'; Field='ExecutablePath'; Value='C:\foreign\NxTools.PlatformHost.exe'},
        @{Name='unreadable Host image'; Field='ExecutablePath'; Value=$null},
        @{Name='wrong native parent'; Field='ParentProcessId'; Value=999},
        @{Name='wrong parent argument'; Field='CommandLine'; Value='fixture.exe --parent-pid 999'},
        @{Name='trusted update role'; Field='CommandLine'; Value='fixture.exe --trusted-update-service'},
        @{Name='elevated helper role'; Field='CommandLine'; Value='fixture.exe --elevated-directory-helper pipe opaque'},
        @{Name='archive CLI role'; Field='CommandLine'; Value='fixture.exe --archive-cli --parent-pid 12000 --pipe-name fixture'},
        @{Name='different session'; Field='SessionId'; Value=2},
        @{Name='missing start time'; Field='CreationDate'; Value=$null}
    )) {
        $changed = $fixtureHost.PSObject.Copy()
        $changed.($case.Field) = $case.Value
        Assert-Fixture (-not (Test-HostMemoryIdentity $changed $fixtureParent $fixtureHostPath $fixtureDesktopPath 12000)) $case.Name
    }
    $foreignParent = $fixtureParent.PSObject.Copy(); $foreignParent.ExecutablePath='C:\foreign\NxTools.exe'
    Assert-Fixture (-not (Test-HostMemoryIdentity $fixtureHost $foreignParent $fixtureHostPath $fixtureDesktopPath)) 'foreign Desktop image rejected'
    Assert-Fixture (-not (Test-HostMemoryIdentity $fixtureHost $fixtureParent $fixtureHostPath $fixtureDesktopPath 999)) 'explicit parent mismatch rejected'
    foreach ($mode in @('absent','ambiguous','complete','missing_gc','replaced','counter_error')) {
        $global:HostMemorySamplerFixture.Mode=$mode
        $global:HostMemorySamplerFixture.HostQueries=0; $global:HostMemorySamplerFixture.Launches=0
        $global:HostMemorySamplerFixture.Kills=0; $global:HostMemorySamplerFixture.NativeReads=0
        $global:HostMemorySamplerFixture.NativeDisposals=0; $global:HostMemorySamplerFixture.CollectorDisposals=0
        $global:HostMemorySamplerFixture.Facts = switch ($mode) { 'absent' { @() } 'ambiguous' { @($fixtureHost,$fixtureHost) } default { @($fixtureHost) } }
        $captureDirectory = Join-Path $fixtureRoot $mode
        $text = & $samplerPath -HostExePath $fixtureHostPath -DesktopExePath $fixtureDesktopPath -OutputDirectory $captureDirectory -DurationSeconds 5 -IntervalSeconds 5
        $captureExit = $LASTEXITCODE
        $result = ($text -join "`n") | ConvertFrom-Json
        if ($mode -in @('absent','ambiguous')) {
            Assert-Fixture ($captureExit -eq 2 -and $global:HostMemorySamplerFixture.Launches -eq 0 -and -not (Test-Path $captureDirectory)) "$mode does not launch or create capture files"
        } elseif ($mode -eq 'complete') {
            Assert-Fixture ($captureExit -eq 0 -and $result.Status -eq 'completed' -and $result.GcCountersPresent -and $result.Samples -eq 2) 'bounded overlapping capture with required GC counters'
            $arguments = $global:HostMemorySamplerFixture.Arguments
            Assert-Fixture ($arguments -contains '--resume-runtime:false' -and $arguments -contains 'EventCounters\System.Runtime[gc-heap-size,gc-committed,gen-2-gc-count,alloc-rate]') 'aggregate-only provider and no automatic resume'
            Assert-Fixture ($global:HostMemorySamplerFixture.NativeDisposals -eq 2 -and $global:HostMemorySamplerFixture.CollectorDisposals -eq 1) 'sampler and collector handles released'
            Assert-Fixture (-not $text.Contains('CommandLine')) 'command lines absent from report'
        } elseif ($mode -eq 'missing_gc') {
            Assert-Fixture ($captureExit -eq 1 -and $result.Status -eq 'incomplete_counters' -and -not $result.GcCountersPresent) 'missing GC commit is not a successful capture or zero'
        } elseif ($mode -eq 'replaced') {
            Assert-Fixture ($captureExit -eq 1 -and $result.Status -eq 'target_identity_lost' -and $result.Samples -eq 0 -and $global:HostMemorySamplerFixture.Kills -eq 1) 'replacement stops capture and only the owned collector'
        } else {
            Assert-Fixture ($captureExit -eq 1 -and $result.Status -eq 'counter_exited_early' -and $result.Samples -eq 0) 'counter failure cannot claim completed evidence'
        }
    }
    Write-Output "SUMMARY pass=$passed fail=0 (synthetic process/counter boundaries only)"
} finally {
    Remove-Variable -Name HostMemorySamplerFixture -Scope Global
    Remove-Item -LiteralPath $fixtureRoot -Recurse -Force
}
