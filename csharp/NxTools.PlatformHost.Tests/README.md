# PlatformHost verification versus native acceptance

The executable is a console harness, not a framework auto-discovery runner. Program.Catalog retains original names/delegates/order; TestEffects classifies every exact function symbol from inspected bodies/callees. New/unreviewed symbols fail closed. TestSelection parses/selects; TestRunner executes and counts. No production native policy is changed by this separation.

## Isolated verification
```powershell
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures
# Existing case-insensitive substring filters retain union, catalog order and no duplicate execution:
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- "installed native fixture" "test selection"
```
No arguments also selects only reviewed Fixture entries. scripts/build.ps1 explicitly passes --fixtures; package.ps1 verification invokes that build path without native authorization. This is C# selection safety, not permission to run unrestricted Rust/Indexer suites or Release packaging on a workstation.

Current224-entry checkpoint contains197Fixture,23NativeAcceptance and4RestrictedFixture entries. CRX package verification now carries a GUID-owned explicit temporary PEM and is Fixture; seven signing identity/isolation regressions were added. Self-created temporary files, fake Windows operations and self-owned synthetic HWND/STA tests may be Fixture; native/real wording is not classification. Existing user windows, registry/tray/Shell/global hooks and external GPUI handles require native acceptance. Four remaining fixtures reach production icon cache or a fixed unowned temporary path and remain RestrictedFixture until isolated.

## Inspect without execution
```powershell
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --native --list
```
Each LIST line includes effect, original name and reviewed scope. RUN is0 and nothing invokes the test delegate. Native listing includes restricted fixtures with their isolation reason.

## Explicitly authorized acceptance
After separate user authorization for the exact operation, use:
```powershell
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --native --authorize-native "real Directory Opus navigation reaches C drive targets"
```
This example performs live navigation; do not run it as ordinary verification. Native execution requires both switches AND explicit filters; there is no unrestricted run-all native mode. Matching a native/restricted entry through legacy/default fixture filters rejects the whole selection rather than silently dropping it. Cross-effect filters reject. The separately invoked tests/native/gpui-kit-acceptance.ps1 wrapper requires -AuthorizeNative before any fixture launch and passes its original exact filters with these switches.

## Results and errors
- Invalid/unknown/unclassified/zero-match/cross-effect/unauthorized selection: SELECTION ERROR,exit2,RUN0.
- Assertion/operation failure:FAIL,exit1,existing fail-fast ordering retained.
- Unavailable supported live environment:typed TestSkippedException,SKIP with reason,never a simultaneous PASS.
- Success:exit0. Always inspect summary; skipped acceptance is not passed acceptance.
- SUMMARY: selected,run,pass,skip,fail,not_run. `run=pass+skip+fail`; `selected=run+not_run`.

Tests must not print SKIP then return normally. Use the typed result so the runner has truthful accounting. Six selection/accounting regressions exercise this with synthetic delegates only. tests/architecture/platformhost-test-selection.ps1 protects wrapper arguments and script syntax without executing wrappers.

Latest signing checkpoint executed16precisely filtered signing/selection/package/registration-fixture tests,not all197Fixture entries. Native/restricted cases were listed only,not executed. Earlier selection checkpoint19fixtures remains historical evidence. Live/package/DPI/provider/service acceptance remains separate.

## Archive tree creation fixtures
`archive tree creation fixture` selects the reviewed GUID-owned synthetic7z/TAR/TGZ/TXZ
Start→Job→Rust-worker→protected-native-output cases. Supply the explicit built worker through
NXTOOLS_ARCHIVE_WORKER_FIXTURE, Python through NXTOOLS_ARCHIVE_PYTHON_ORACLE, fixture scripts
through NXTOOLS_ARCHIVE_ZIP_FIXTURES and the existing stock7-Zip oracle through
NXTOOLS_ARCHIVE_7ZIP_ORACLE. Oracle passwords use stdin only; no download/install/PATH change.
Independent tools read only synthetic outputs and materialize into their own temporary folder.
Missing explicit inputs are typed SKIP,not PASS; exact current run/counts live in the archive
task's phase-a-checkpoint.md. Always --list the intended selection before execution.
No release/registry/service/user archive/Shell/UI acceptance is implied.

## Archive source-delete foundation fixtures
`archive source delete fixture` selects nine reviewed isolated cases using the actual built
worker,GUID-owned sources/output and recording backend through original productionRun/session.
It never invokes Shell recycle APIs. Includes no-Shell DELETE-access sharing32/close->name-resolve
race proof, typed results/once-only/consent/coverage/failure/cancel/privacy. Supply worker and codec
fixture env keys above; LIST/RUN/PASS counts/names must match, no skipped case is acceptance.
Production ForPackage keeps native backend absent and returns native_protection_unproven;
source retained. This is not authority to recycle even the fixture files into the real bin.
Retired-evidence replacement/newlease/requestclone regression must use the original captured
private session's restricted observer;no static qualification by callertrue/new source set.
Only original completionowner qualifies/executes. Observer cannot start backend;retire refuses
late/concurrent/replayed attempts. Keep original117case names union new2when running this repair.
