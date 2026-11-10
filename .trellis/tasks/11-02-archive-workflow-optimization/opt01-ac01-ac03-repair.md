# OPT-01 — AC-01/02/03 repair evidence

## Disposition

AC-01, AC-02 and AC-03 have focused fixes and passing production-entry automated regressions. **OPT-00/OPT-01 remain `in_progress`; G-00 remains incomplete; OPT-02 through OPT-07 are not started.** Parent status remains **Phase E — Format expansion and ZIP editing**.

The [independent acceptance](opt00-opt01-independent-acceptance.md) remains the original review verdict. This record adds repair evidence without replacing that verdict, the [original results](opt00-opt01-results.md), or the 216 retained supplemental native observations. The independently accepted shared-read/Seal-size slice is unchanged.

## Repairs and boundary evidence

| Item | Original-owner change | Verified behavior |
| --- | --- | --- |
| AC-01 | `jobs.rs::background_job` reuses `restore_prompt_surface` after taking the window and pruning retired jobs | Prompt-before-background and prompt-between-dispatch-and-defer; seven-second notification expiry; all five prompt kinds; one restored window/current ticket; one actual decision response; duplicate ticket/callback refusal; terminal, disconnect and replacement-attempt retirement |
| AC-02 | `ArchiveJobSession.Operation` stamps optional `progress.sequence` under its existing gate at publication; `job_notification` filters superseded observations | Hold a frame read from actual coalesced `Progress`, publish a later lossless `Events` phase, apply newer then held through serialization/validation/original notification; phase, processed counters and observation time remain current; native Verifying/Committing still disable Pause |
| AC-03 | Original `ProgressSnapshot` carries the frozen modify request's format into `JobProgressView` | Each SevenZ/Tar/TarGz/TarXz request consumes its own C# serialized phases. SevenZ retains its non-solid LZMA2 description; TAR/TAR.GZ/TAR.XZ preparation and rewrite descriptions do not claim 7z/LZMA2. Processed bytes, candidate verification and confirmed publication remain distinct |

AC-01 also exercises the registered native-close callback after a prompt is already painted, for all five kinds, then submits the restored prompt's own decision. The locked GPUI TestWindow helper reinstalls its close handler after invocation even when the callback removed that window. This fixture catches only that final `window not found` assertion, independently proves the old handle is gone, and checks restoration and once-only replies. It does not catch a restoration or reply assertion failure. No GPUI patch or product test switch is added.

Sequence is a positive `u64`/`ulong`, omitted when absent. Zero, negative, fractional and string values are rejected by the respective schema/validation boundary. Legacy frames retain existing same-phase counter checks. Once sequenced progress is installed, missing/equal/older sequence cannot overwrite it. A newer sequence may restart an earlier phase or reset same-phase counters: there is no global phase ranking. Original password Dialog replies and replacement `send_job_attempt` exercise those transitions; the latter clears progress and accepts the new operation's sequence 1 while refusing old-operation frames. The sequence is presentation order, not source/control/publication authority.

The UI shape remains the existing 560×260 progress window with 480×260 minimum, original Root/Dialog/DialogFooter and official Medium actions. No control size, layout, parent adaptation, notification manager, scheduler or transport is added. Existing `archive.job.presentation_closed`, `archive.job.prompt_restored` and phase/state/terminal diagnostics retain job/attempt/generation correlation; no per-frame diagnostic stream is introduced.

## Red-capable regressions

Tests were executed against the uncorrected product entry points before the fixes:

| Counterexample | Observed failure |
| --- | --- |
| Both background/prompt orders | Current prompt window absent after original background callback and notification expiry |
| C# publication | `AC-02 publication order must survive the two original channels` |
| Rust two-channel consumption | `edit_reading`, bytes4 replaced `edit_verifying`, bytes8 |
| Format presentation | Matching Tar request's `edit_planning` displayed `准备 7z 编辑；容量需求未知` |

Retained session logs: `ac01-background-red.log`, `ac01-ac03-csharp-red.log`, `ac01-ac03-desktop-red.log`. Final evidence: `ac01-ac03-green-build.log`, `ac01-ac03-green-list.log`, `ac01-ac03-csharp-green.log`, `ac01-ac03-jobs-final.log`, `ac01-ac03-progress-final.log`, `ac01-ac03-protocol-final.log`, `ac01-ac03-clippy-final.log`. These are under the session scratch root `D:\.pi-desktop\data\scratch\a2072588-1f47-4232-be7a-3951a25df615`; replay generates fresh exports and does not require that old directory.

## Executed checks

| Check | Result |
| --- | --- |
| C# test-host build, no restore, single MSBuild lane, shared compilation disabled | PASS; zero warnings/errors |
| Exact LIST then identical RUN: extraction dual-progress wire, original editing-progress wire, AC repair production-progress wire | 3 selected / 3 run / 3 passed / 0 skipped |
| Desktop original jobs tests, explicitly using both newly generated C# exports | 83 passed, including seven AC repair tests and the original 15 OPT-01 tests |
| Desktop original progress/metrics tests | 15 passed |
| Rust protocol library | 101 passed |
| Desktop + protocol strict Clippy `--tests -- -D warnings` | PASS |
| Scoped rustfmt and tracked-file diff whitespace | PASS |
| Spec navigation/evidence contracts; GPUI release; pinned editor source contracts | PASS; 120 documents/14 owners; Kit0.8.0/pre0.3.9 unchanged |
| Task JSON/status and related-file links | PASS; `in_progress`, no completion date, original independent verdict retained |

C# `RepairProgressProductionWire` uses the original nested Operation, `OnFrame`, channel readers and envelope serializer. It exports 24 format-phase events, eight newer/held pairs, and four retry observations. It creates/removes an owned temporary root; it does not start a worker or access an archive. Rust rebinding changes only operation/session/generation identities to the corresponding live test job; format and progress observations remain paired. Missing AC export fails explicitly rather than substituting synthetic progress.

The bounded source review found no repair-code behavioral defect. Its requested title-close and restored-decision coverage is included in the final 83-test run. No second review pass or performance rerun was used.

## Replay

From the workspace root in Windows PowerShell, use the current session scratch directory:

```powershell
$env:TEMP=$env:PI_SCRATCH_DIR
$env:TMP=$env:PI_SCRATCH_DIR
$env:NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE="$env:PI_SCRATCH_DIR\ac01-ac03-progress.json"
$env:NXTOOLS_ARCHIVE_EDIT_PROGRESS_WIRE="$env:PI_SCRATCH_DIR\ac01-ac03-six-phases.json"
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -p:UseSharedCompilation=false
$testHost='.\csharp\NxTools.PlatformHost.Tests\bin\Debug\net8.0-windows\NxTools.PlatformHost.Tests.exe'
$selection=@('archive extraction dual progress fixture wire', 'archive OPT01 CSharp production editing progress wire', 'archive OPT01 AC repair production progress wire')
& $testHost --fixtures --list @selection
# Confirm exactly these three fixture-effect rows before executing the same selection.
& $testHost --fixtures @selection
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
Push-Location rust
cargo test -p nxtools-desktop --offline archive::application::jobs::tests -- --test-threads=1
cargo test -p nxtools-desktop --offline archive::progress:: -- --test-threads=1
cargo test -p nxtools-protocol --offline --lib -- --test-threads=1
cargo clippy -p nxtools-desktop -p nxtools-protocol --offline --tests -- -D warnings
Pop-Location
```

Use one Cargo lane, sufficient operation timeout and the environment values in every Rust verification invocation. A failed build or fixture run blocks consuming its exports.

## Limitations and remaining gates

- This is source/compiled production-entry evidence, not physical user acceptance. Real GUI/Explorer, DPI, screen-reader, installation and Release validation remain unexecuted.
- G-00 still lacks actual total source reads, exclusive internal format phase costs, block decode accounting and the required original 137,737,166-byte sample comparison. Observable ZIP/TAR/7z work and unavailable locked UnRAR internal events remain separate gaps; no old numbers are backfilled.
- Shared-engine/cache/native data-protection and performance evidence remains at its independently accepted scope. This repair does not remeasure or broaden it.
- No packaging, signing, service/registry operation, user-archive mutation, recycling, HZ5 change, commit or push was performed.
