# Phase 1 verification record

Date: 2026-07-18

Machine: Windows 10.0.19045, AMD Radeon RX 580 2048SP

## Automated

- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass, including clipboard favorite/pin persistence, pinned-first
  ordering, soft-delete/restore, plain-text editing, category persistence/source filtering and
  category wheel/drop behavior, plus embedded Material Symbol availability
- Rust protocol/runtime tests: pass
- .NET build with warnings as errors: pass
- C# protocol wire and bounded-reader tests: pass
- Rust/C# named-pipe handshake: pass
- foreground context event received by Rust: pass
- packaged desktop launches PlatformHost with the Rust process as parent: pass
- graceful desktop close stops the supervised PlatformHost: pass
- forced/crashed parent termination is detected and the PlatformHost exits: pass
- second desktop instance rejected with non-zero exit: pass
- Release/package build with GPUI shaders: pass; package produced at `artifacts/win-x64`

## Performance baseline

Debug build, window open and idle after a five-second settling period, sampled for five seconds:

| Process | CPU | Working set |
| --- | ---: | ---: |
| `nxtools-desktop` | 0.130% | 54.4 MiB |
| `NxTools.PlatformHost` | 0.000% | 27.1 MiB |

This is a diagnostic baseline, not the `<1%` Release acceptance result.

## Phase 2 index scale baseline

Release synthetic snapshots were measured on 2026-07-21 with
`tests/performance/indexer-scale-baseline.ps1` on a 12-logical-processor machine:

| Entries | Build | Query P50 | Query P95 | One-row USN | Working-set delta |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000,000 | 401 ms | 31.0 ms | 33.5 ms | 27 µs | 226.4 MiB |
| 3,000,000 | 1,413 ms | 85.0 ms | 107.6 ms | 23 µs | 642.9 MiB |

The query runs on the indexer worker and remains generation-cancellable; it does not scan on the
GPUI thread. Raw JSON is written to `artifacts/performance/indexer-scale-baseline.json`.

The packaged `NxToolsIndexerV1` LocalSystem service was then restarted against its persisted cache.
It reported `ready` immediately with 2,785,412 restored entries. An eight-second idle sample averaged
0.13% normalized CPU (0.13% peak) with a 12.5 MiB peak working set. The corrected real-pipe
smoke test ignored intervening status events, completed in 139 ms
(131,516 µs service time), and returned six real paths. Fifty reconnect/search sessions and a
100-generation latest-query cancellation stress run also passed.

## 2026-07-22 USN recovery and index-scope controls

- Win32 1181 (`ERROR_JOURNAL_ENTRY_DELETED`) regression: pass; it requests an affected-volume
  rebuild instead of remaining as a generic `PARTIAL` polling error.
- GPUI index settings: pass; discovered volumes use toggles and excluded directories use native
  folder picking plus removable entries, with no semicolon-delimited text fields.
- Packaged LocalSystem restart: `ready`, 2,787,250 restored items. Eight-second normalized CPU
  average 1.85% against the 2.0% regression threshold.
- Post-recovery status: `ready`, 2,793,802 items, C:/D: online, no `last_error`.
- Real service search: six results for `NxTools.Indexer.exe`, 739 ms wall time and 731,545 µs
  service time.

## Open environment gates

- Windows 10 and 11 per-monitor DPI: requires manual testing on both operating systems.
- Chinese IME: the proof now contains a real GPUI input control; composition still requires
  manual verification with the target IMEs.
- Custom commands: automated coverage verifies configuration round-trip and normalization,
  case-insensitive keyword matching, quoted argument parsing, `%s`/`%sN` substitution, index
  bypass, typed Rust protocol round-trip and C# `ProcessStartInfo.ArgumentList` mapping. Manual
  verification should add one normal command and one elevated/hidden command, summon Quick Launch,
  execute each, copy the rendered command from the local action menu, and confirm a failed launch
  leaves the panel visible with an actionable error.
- Native Windows open/save dialog navigation: automated coverage verifies strict `#32770`
  structure matching, frozen HWND routing, bounded control messaging, post-submit current-directory
  verification, focus restoration intent and late-foreground-event suppression. A real dialog still requires manual verification: summon Quick
  Launch, switch to two different directories consecutively in the same dialog, confirm no Open/Save
  action is triggered, then close/stale the host and confirm the popup remains with an actionable error.

## 2026-07-27 launcher and clipboard UI/resource pass

- `cargo test --workspace --no-fail-fast -j 1`: pass; 100%/125%/150% attached-search geometry,
  narrow dialog clamping, compact metadata preview, clipboard safe insets, image aspect/cap,
  bounded image-resource cache and Zed native-image-only paste are covered.
- `cargo clippy --workspace --all-targets -j 1 -- -D warnings`: pass.
- `dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj -c Release`:
  pass, including the optional FileDrop representation at the Windows data-object boundary.
- `tests/performance/clipboard-panel-cpu.ps1` now supports `-ExerciseScroll -HoverFirstItem` and
  records desktop/host CPU plus desktop working-set/private-memory samples.
- No desktop UI or performance harness was launched during this pass. Native screenshot comparison,
  real Zed paste behavior, and Release active-scroll memory/CPU remain manual gates.

## 2026-07-27 download Phase 1A automated gate

- `cargo test -p nxtools-download -j 1`: pass, 45 tests.
- `cargo test -p nxtools-desktop -j 1`: pass, 163 tests.
- `cargo clippy -p nxtools-download -p nxtools-desktop --all-targets -j 1 -- -D warnings`: pass.
- Covered boundaries include HEAD/Range fallback, redirects, streaming writes, pause/cancel,
  target collision, server-derived filename recovery, permission/disk-full faults at the artifact
  boundary, pre-rename crash boundary, atomic task/artifact rollback, versioned commands and
  sequenced/coalesced events.
- The desktop UI was not launched. Real-network download, pause/restart/resume, native directory
  picking, notifications and filesystem cleanup remain the Phase 1A manual acceptance gate.

## Stage 5 real-media acceptance

The checked-in runtime smoke uses two public, externally hosted samples without copying media into
the repository:

- Apple Bip Bop: the source preflight checks the official master playlist and a finite plaintext
  MPEG-TS media playlist. The runtime task uses the lower-bandwidth media playlist so CLI acceptance
  does not automatically select and download the largest 30-minute variant.
- DASH-IF/CTA WAVE Tears of Steel: a 60-second, one-Period static MPD with one AVC video track, one
  AAC audio track and finite SegmentTimeline expansion. It exercises the user-managed FFmpeg mux
  boundary.

Use PowerShell 7 or newer. First verify that the bounded process-tree runner and remote fixtures have
not changed outside the supported Stage 5 profile:

```powershell
.\tests\runtime\download-media-smoke.ps1 -ContractTestOnly
.\tests\runtime\download-media-smoke.ps1 -SourcePreflightOnly
```

For the full smoke, package the current commit, launch that exact `NxTools.exe` (or allow it to
redirect to the byte-identical protected desktop), configure a valid FFmpeg executable under
“下载设置 → 媒体工具”, and use a new explicit output directory. The script validates the atomic
desktop/indexer/PlatformHost package, invokes every CLI command with `--connect-only`, bounds and
drains every child process, kills timed-out descendant trees, and uses the unique requested artifact
identity to reconcile a task when the add response is lost. Cooperative failure/interrupt cleanup
requests cancellation and boundedly verifies the durable outcome; forcibly terminating the
PowerShell host remains outside that guarantee. It cannot silently start a hidden desktop after a
pipe disconnect. It retains both completed artifacts for inspection and accepts an optional
user-supplied `ffprobe.exe` for stream and fixture-specific duration-range validation:

```powershell
.\tests\runtime\download-media-smoke.ps1 `
  -PackageDirectory .\artifacts\win-x64 `
  -OutputDirectory "$env:USERPROFILE\Downloads\NxTools-Media-Acceptance" `
  -FfprobePath C:\Tools\ffmpeg\bin\ffprobe.exe
```

Acceptance still requires opening both retained files and confirming picture, sound, duration and
seeking. In the GUI, separately submit the Apple master playlist to verify explicit quality
selection, then pause/restart/resume one media task and confirm completed fragments are reused.
The deterministic blocking-child tests remain the gate for pause/cancel during the normally brief
FFmpeg version/mux windows; do not claim a human timed a real stream-copy cancellation unless the
finalization state was actually observed before the action.
