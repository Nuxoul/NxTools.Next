param([Parameter(Mandatory=$true)][string]$Name,[Parameter(Mandatory=$true)][string]$Executable,[string[]]$TestArgs,[int]$Seconds=60)
$ErrorActionPreference='Stop'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$busy=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe' });if($busy.Count){throw 'Rust not naturally idle'}
$folder=$PSScriptRoot;$log=Join-Path $folder ($Name+'.log');$record=Join-Path $folder ($Name+'.json');if((Test-Path $log)-or(Test-Path $record)){throw 'Immutable log exists'}
$start=[DateTime]::UtcNow;$info=New-Object System.Diagnostics.ProcessStartInfo;$info.FileName=$Executable;$info.Arguments=$TestArgs -join ' ';$info.UseShellExecute=$false;$info.CreateNoWindow=$true;$info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true
$proc=New-Object System.Diagnostics.Process;$proc.StartInfo=$info;[void]$proc.Start();$stdout=$proc.StandardOutput.ReadToEndAsync();$stderr=$proc.StandardError.ReadToEndAsync();$settled=$proc.WaitForExit($Seconds*1000);if(-not $settled){$proc.Kill();$proc.WaitForExit()};$code=$proc.ExitCode
$text=$stdout.GetAwaiter().GetResult()+$stderr.GetAwaiter().GetResult();[IO.File]::WriteAllText($log,$text,(New-Object Text.UTF8Encoding($false)))
@{executable=$Executable;args=$TestArgs;process_id=$proc.Id;completed=$settled;exit=$code;watchdog_seconds=$Seconds;started_utc=$start.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');environment=@{CARGO_BUILD_JOBS=$env:CARGO_BUILD_JOBS;CARGO_INCREMENTAL=$env:CARGO_INCREMENTAL;RUSTFLAGS=$env:RUSTFLAGS}} | ConvertTo-Json -Depth 4 | Set-Content $record -Encoding UTF8
Get-Content $log -Tail 24;if(-not $settled){exit 124};exit $code
