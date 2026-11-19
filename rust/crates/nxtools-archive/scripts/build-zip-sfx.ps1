param([Parameter(Mandatory=$true)][string]$EvidenceDirectory)

$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
if (-not $env:PI_SCRATCH_DIR) { throw 'PI_SCRATCH_DIR is required for build intermediates' }
$env:TEMP=$env:PI_SCRATCH_DIR
$env:TMP=$env:PI_SCRATCH_DIR
if ($env:CARGO_TARGET_DIR -or $env:CARGO_ENCODED_RUSTFLAGS) { throw 'Use the normal workspace target directory and RUSTFLAGS for this build' }
$repoRoot=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..\..\..')).Path
$crateRelative='rust\crates\nxtools-archive'
$evidence=[System.IO.Path]::GetFullPath($EvidenceDirectory)
if (Test-Path -LiteralPath $evidence) { throw 'Use a fresh evidence directory; previous build evidence is immutable' }
New-Item -ItemType Directory -Path $evidence -ErrorAction Stop | Out-Null

function Wait-BuildLane {
    $deadline=(Get-Date).AddMinutes(10)
    do {
        $memory=Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory -ErrorAction Stop
        $available=[double]($memory.CommitLimit-$memory.CommittedBytes)
        $busy=@(Get-CimInstance Win32_Process -ErrorAction Stop | Where-Object {
            $_.Name -match '^(cargo|rustc|clippy-driver|MSBuild|link|NxTools\.ArchiveWorker|nxtools-archive-worker)\.exe$' -or
            ($_.Name -eq 'dotnet.exe' -and $_.CommandLine -match '(?:build|test|publish|run|msbuild)(?:\s|$)|MSBuild\.dll') -or
            ($_.ExecutablePath -like '*\target\*\deps\*' -and $_.Name -ne 'conhost.exe')
        })
        $status="$(Get-Date -Format o) processes=$($busy.Count) available_commit_GiB=$([math]::Round($available/1GB,2))"
        Add-Content -LiteralPath (Join-Path $evidence 'lane.log') -Value $status
        Write-Output $status
        if ($busy.Count -eq 0 -and $available -ge 8GB) { return }
        $busy | Select-Object ProcessId,Name,CommandLine | Format-List | Out-String | Add-Content -LiteralPath (Join-Path $evidence 'lane.log')
        if ((Get-Date) -ge $deadline) { throw 'Build lane unavailable after bounded wait' }
        Start-Sleep -Seconds 15
    } while ($true)
}

Push-Location $repoRoot
try {
    Wait-BuildLane
    $sourceRoot=Join-Path $evidence 'source-inputs'
    $files=@('Cargo.toml','Cargo.lock','rust-toolchain.toml',"$crateRelative\Cargo.toml","$crateRelative\build.rs","$crateRelative\src\sfx_main.rs")
    foreach ($folder in @("$crateRelative\src\sfx","$crateRelative\notices\zip-sfx","$crateRelative\scripts")) {
        $files+=@(Get-ChildItem -LiteralPath $folder -File -Recurse | ForEach-Object { $_.FullName.Substring($repoRoot.Length+1) })
    }
    $facts=@(foreach ($relative in $files) {
        $destination=Join-Path $sourceRoot $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force -ErrorAction Stop | Out-Null
        Copy-Item -LiteralPath $relative -Destination $destination -ErrorAction Stop
        [pscustomobject]@{ path=$relative; bytes=(Get-Item -LiteralPath $relative).Length }
    })
    $facts | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $evidence 'source-inputs.json')
    $command='cargo rustc --locked --offline -p nxtools-archive --bin nxtools-zip-sfx --release -- -C target-feature=+crt-static'
    @("workspace=$repoRoot",'CARGO_BUILD_JOBS=1','CARGO_INCREMENTAL=0','RUSTFLAGS=-C codegen-units=256',"TEMP=$env:TEMP","TMP=$env:TMP",$command) | Set-Content -LiteralPath (Join-Path $evidence 'build-command.txt')
    Wait-BuildLane
    cmd /d /c 'rustc -Vv 2>&1' > (Join-Path $evidence 'rustc-Vv.txt')
    $compilerExit=$LASTEXITCODE
    Set-Content -LiteralPath (Join-Path $evidence 'rustc-Vv.exit') -Value $compilerExit
    if ($compilerExit -ne 0) { throw 'Toolchain query failed' }
    $compiler=Get-Content -LiteralPath (Join-Path $evidence 'rustc-Vv.txt') -Raw
    if ($compiler -notmatch 'release: 1\.96\.0\s' -or $compiler -notmatch 'host: x86_64-pc-windows-msvc\s') { throw 'Toolchain must match embedded Rust 1.96.0 Windows x64 notices' }
    Wait-BuildLane
    cmd /d /c "$command 2>&1" > (Join-Path $evidence 'build.log')
    $buildExit=$LASTEXITCODE
    Set-Content -LiteralPath (Join-Path $evidence 'build.exit') -Value $buildExit
    if ($buildExit -ne 0) { throw 'Recipient build failed; retain log and inspect surviving compiler processes before another build' }
    python (Join-Path $PSScriptRoot 'inspect-zip-sfx.py') --evidence $evidence > (Join-Path $evidence 'inspection.log') 2>&1
    $inspectionExit=$LASTEXITCODE
    Set-Content -LiteralPath (Join-Path $evidence 'inspection.exit') -Value $inspectionExit
    if ($inspectionExit -ne 0) { throw 'Bare PE inspection/bundle failed; retain the exact built file and inspection log' }
    Write-Output "build_exit=$buildExit inspection_exit=$inspectionExit evidence=$evidence"
} finally {
    Pop-Location
}
