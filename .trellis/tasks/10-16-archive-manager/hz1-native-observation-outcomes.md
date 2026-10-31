# HZ1 — bounded C# native observation outcomes

2026-10-28. Independent native observation slice only; parent owns Rust/protocol Rust/forms/jobs and icon agent owns icons/browser layout. No Cargo invocation, package/install/product launch, dependency/vendor/installer/password-focus change, Registry/service/ACL mutation, commit or push. Existing unrelated dirty files were preserved. Main checkpoint and parent HZ1 report were not edited.

## Reference and invariants

Read HZ1 user brief lines90–197, native/shared specs, `docs/reference-code/archive.md`, `hatchzip.md`, `hatchzip/space-preflight.md` and `hatchzip/source-manifest.json`. Re-read actual `D:\Code\hatchzip-main\crates\core\src\session.rs:306–403`, `engine.rs:438–469`, `fsutil.rs:10–38,63–75`. All three SHA256 values still match the manifest; evidence: `$env:PI_SCRATCH_DIR\hz1-native-reference-hashes.json`. No upstream code executed or copied. Existing archive note records no old NxTools general archive-window implementation; unrelated old ZIP installers are not behavioral authority for this feature.

Original call chain retained: pipe route -> original `PlatformCommandSequenceDispatcher` detached archive-entry lane -> original `BoundedStaOperationRunner` max2/5-second logical wait -> `ArchiveVolumeSources` / `ArchiveSourceLease` and `ArchiveOutputSession.Prepare` -> native held-directory observation. No new service/queue/timeout replacement worker. Source parents/file and target ancestors stay held without delete sharing; source is read-only. Missing target uses the nearest verified existing ancestor; observation never calls `CreatePreparedRoot`, creates a directory/probe/partial, modifies/truncates/punches source, or widens UNC/device/reparse admission. Cancellation invalidates receipt ownership, not the uninterruptible native call; the original slot survives until real cleanup.

Original execution `ArchiveWorkerRunner.CheckDestination` -> `InspectDestination` -> `OutputNative.AvailableSpace`, missing/low-space continue/cancel prompt, actual output limits/readmission/Job/concurrency and reserve0 remain unchanged. Advisory free bytes are neither reservation nor approval. No source/overwrite cleanup credit is added.

## API and identity semantics

DTO shape unchanged (`Protocol/ArchiveSpaceContract.cs:6–23`): command version/op/session/generation/source/destination/optional expected_source/expected_target; observation version/op/session/generation/available_bytes/allocation_bytes/target_identity/volume_identity/destination_missing/source_matches. Strict unmapped-field rejection, v1 IDs, original local paths and private command ToString retained. `source_matches=true` means the supplied expected family identity matched, or no expectation was supplied; it is not proof that arbitrary bytes form a valid archive.

`Windows/Archive/ArchiveOutputSession.cs:130–145`: `ObserveSpace` uses original writable-directory access check (open only, no probe), GetDiskFreeSpaceExW, GetDiskFreeSpaceW and before/after full native IDs. Target identity is `v1:<64-bit volume serial>:<128-bit held ancestor ID>:<SHA256 exact UTF16 missing suffix>`. Empty suffix represents an existing target. This fixes collisions between different nonexistent destinations at equal depth. Suffix remains exact/case-conservative; it is not a plaintext path and is not logged. Identity is opaque to Rust, not a new DTO field. New target creation/replacement or nearest-ancestor change invalidates it.

`OutputNative.AllocationSpace` (same file, ~657): volume identity contains actual held handle's volume serial plus GUID from `GetFinalPathNameByHandleW(VOLUME_NAME_GUID)`, never drive-letter equality. Allocation is real sectors×bytes, rejected as unknown if zero/non-power-of-two/>16MiB. No guessed4KiB fallback. A reparse/mount component is rejected rather than followed.

`Windows/Archive/ArchiveSpacePreflight.cs`: expected source uses original `ArchiveVolumeSources.Identity()` (one-file native serial/full ID/size/mtime; original multi-volume family digest); final source verification stays original. Original finite source family/header ceilings4MiB metadata/100GiB encoded are NOT raised: too large/incomplete/unsupported family returns honest `resource_limit`/original error, no zero estimate/full codec scan. Family enumeration limits remain original1024volumes/16384directory candidates.

Failures preserve native codes: missing source `source_unavailable`2/3; access denied `source_denied`5; sharing `source_busy`32/33; destination wrong kind `target_wrong_kind`267; original writable-open failure `target_denied` with native code; original `reparse_component`, changed source/target, allocation failures and `space_busy_or_timeout` remain distinct. Native non-directory source open on a directory yields actual Win325 here, so it is honestly reported as source_denied, not falsely relabeled from path guessing.

`Protocol/PlatformPipeClient.cs:319–326` now calls tested `ArchiveSpacePreflight.DispatchAsync`, preserving original dispatcher/reader route and suppressing post-cancel publish. Capacity fixture delegates replace ONLY available bytes after actual held/native observation. `Run` remains original fixed2workers/5seconds. `Observe` rechecks cancellation after observation seam, fixing late success. Diagnostics use operation correlation plus session/generation, fixed stage/native/elapsed fields; accepted/started/finished/failed and separate wait_ended distinguish logical timeout from later cleanup. Throwing sinks never alter results; no names/paths/contents/identity tokens/passwords logged.

## Behavioral RED -> GREEN evidence

All logs below are under session scratch `C:\Users\Administrator\.pi-desktop\scratch\9d8169e5-656b-47ae-9113-af5789f5091c`.

- Inherited original stub RED `hz1-native-red.log`: compiled successfully, selected1/RUN1/FAIL1 at capacity observation. Inherited `hz1-native-green.log`:1/1PASS. Parent changed tiny invalid `source.zip` bytes to signature-neutral `source.bin` after stub RED; this fixture correction is NOT claimed as a codec regression. Current tiny fixture is `private-source.bin`.
- Before fixes, expanded test build `hz1-native-expanded-red-build.log`:0warnings/0errors. Each regression was separately LISTed then RUN with the same exact positional name filter (LIST1/RUN1/FAIL1):
  - `hz1-native-behavior-red-binding{,-list}.log`: different missing suffixes shared expected-target identity.
  - `hz1-native-behavior-red-cancel{,-list}.log`: cancellation at native observation returned success.
  - `hz1-native-behavior-red-errors{,-list}.log`: missing source returned raw open_component2 rather than source_unavailable2.
- Initial attempted literal `--filter` invocation is unsupported by this existing harness: `hz1-native-expanded-red-{binding,cancel,errors}{,-list}.log` each exited2/selected0/RUN0. These are selection errors, NOT behavioral RED. Harness preserves positional substring filters; no selection code changed.
- Expanded intermediate integrated GREEN `hz1-native-expanded-green.log`:10/10PASS; later final/verified selections include additional cancellation and original execution fixture, below.

## Exact final verification

Executed serially; no shared compiler/build-server processes created (`-nr:false`, `UseSharedCompilation=false`).

```powershell
dotnet build csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -nr:false -p:UseSharedCompilation=false
dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list 'archive space fixture' 'archive destination fixture missing low space confirmation'
dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures 'archive space fixture' 'archive destination fixture missing low space confirmation'
& tests\architecture\platformhost-test-selection.ps1
```

- Final logs `hz1-native-verified-build.log`, `hz1-native-verified-list.log`, `hz1-native-verified-run.log`: build exit0/0warnings/0errors; LIST12/RUN0; RUN12/PASS12/SKIP0/FAIL0/not_run0. Ordered LIST/RUN names exactly match (`hz1-native-final-accounting.json`).11new/refined observation tests plus original1destination-execution test. Earlier final12/12 (`hz1-native-final-*`) also retained.
- `Tests/Program.cs:74–84` and `Tests/TestEffects.cs:70–80` register exactly11 reviewed GUID-owned tiny IO/native-handle/STA tests, no broad implicit classification. Tests call real observer/native held owners; no replacement observation calculator.
- Coverage: missing nested target/no probe/no source byte+mtime effects; independent native allocation/full128-bit ID/volume GUID; missing suffix binding/target appearance; real source and target replacement; injected zero/UInt64.Max available only; source missing/real Win325/32 and target267/writable-sharing denial32; UNC/device/traversal/ADS rejection; actual owned source+directory symlinks; pre/during cancellation; strict wire and throwing-sink privacy/terminal validation; actual original STA max2 retained after cancellation AND5s timeout with no replacements; source-write/target-rename denied while held; later dispatcher command responsive; exact cancelled dispatch gets no late receipt.
- Original `ArchiveDestinationPreflightTests.MissingAndLowSpaceRequireOriginalConfirmation` passed standalone1/1 (`hz1-native-execution-confirmation{,-list}.log`) and both integrated12/12runs. It exercised original real worker/Job/native publication: missing vs injected-zero existing target × approve/cancel (4extract cases), plus create approve/cancel2cases; stale session/generation and consumed tickets reject. No worker source/budget/test edits.
- Existing worker `target\debug\nxtools-archive-worker.exe` was present and newer than all archive crate source/test .rs/.toml inputs: UTC2026-10-27T09:39:29.5568973Z,10994688bytes, SHA256 `04C6863DBA4B46ACEF8E736DAFAC904A12E2155044F1F788A6B11D25E091BFE0`. Existing executable used, never Cargo-built here. Passing original wire/publication test is current behavioral compatibility evidence, not reproducible binary provenance or release approval.
- Selection gate passed (`hz1-native-selection-gate.log`), source-only.
- `python tests\architecture\spec-contracts.py` passed:114documents/14workspaceowners (`hz1-native-spec-contracts.log`), source/navigation evidence only.
- Scoped `git diff --check` passed; only Git CRLF→LF notices for pre-existing large Program/PlatformPipeClient, no whitespace errors (`hz1-native-diff-check.log`). No whole-file rewrite of those owners.
- Final process audit `hz1-native-final-accounting.json`: task-owned dotnet/build/test processes remaining0. Two unrelated Unity dotnet processes (23316 Burst persistent server,1968 Unity VBCSCompiler) were preserved. Therefore do NOT claim the whole workstation has zero dotnet processes.

## Remaining limits / explicit NOTRUN

No real ACL mutation or restricted-user ACL acceptance; native Win325 is exercised with a wrong-kind source directory, destination denial with owned sharing-only handle. Actual ACL denial follows the same preserved native-code route but remains NOTRUN. No real volume/mount replacement, quota/FAT/exFAT/remote volume, disk exhaustion, huge archive, destructive source operation, physical UI/DPI/focus, real production pipe end-to-end connection, packaged/native-user acceptance or release validation. Dispatcher test uses the production dispatch helper called by the real pipe route and fake receipt sink/later callback; it is not a complete named-pipe integration test.

More-than100GiB encoded or family/header work beyond finite ceilings can honestly fail advisory observation. No larger-file allocation or full codec scan was attempted to evade limits. Cancellation cannot interrupt a hung Win32 call; at most2original slots remain occupied until actual return, and logical busy/timeout is reported. Observation is not an exact peak/reservation; Rust owns bounded estimate/unknown/reserve0/generation/UI semantics and parent owns Rust validation. No HZ2–HZ5 work included.
