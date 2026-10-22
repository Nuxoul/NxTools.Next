# Preserved Phase A historical fixture reproduction

Moved from phase-a-checkpoint.md reproduction section to retain its history while the unique
current checkpoint stays below task-context injection bounds. Historical counts/filters below
are NOT current product-read execution evidence. Current exact126 list/run is in
[product read evidence](../product-read-evidence.md).

Fixture inputs (test copies refuse an existing fixed helper and remove only their own copy):
```powershell
$env:NXTOOLS_ARCHIVE_WORKER_FIXTURE=(Resolve-Path target\debug\nxtools-archive-worker.exe).Path
$env:NXTOOLS_ARCHIVE_CODEC_FIXTURES=(Resolve-Path rust\crates\nxtools-archive\tests\fixtures\upstream).Path
$env:NXTOOLS_ARCHIVE_ZIP_FIXTURES=(Resolve-Path rust\crates\nxtools-archive\tests\fixtures).Path
$env:NXTOOLS_ARCHIVE_PYTHON_ORACLE='C:\Users\Administrator\AppData\Local\Programs\Python\Python313\python.exe' # verify actual version/hash first
$env:NXTOOLS_ARCHIVE_7ZIP_ORACLE='<explicit hash-verified portable7z.exe>' # new AES create oracle
$fixtureFilters=@('archive output fixture native failure','archive output fixture nested','archive output fixture existing','archive output fixture concurrent','archive worker','archive codec actual Rust','archive extraction actual Rust','extension process fixture owned echo','extension process fixture input and native process limits','archive remediation','archive output fixture rechecked','archive overwrite fixture','archive outcome fixture','archive stream fixture','archive XZ fixture','archive ZIP method fixture','archive ZIP crypto fixture','archive ZIP remediation fixture','archive ZIP creation fixture','archive admission fixture','archive source binding fixture','archive tree creation fixture')
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list @fixtureFilters
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures @fixtureFilters
```
Historical selection89/run89/pass89 belongs to sevenz-create-typed-io-remediation.md.
The later source-deletion-remediation.md selected119; do not reuse old filter/count as current.
This is not unrestricted/default C# or native-user acceptance. Documentation/check/compilation
do not close full Phase A/native/release exits.
