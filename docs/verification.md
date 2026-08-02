# Phase 1 verification record

## Windows download UI automation

`tests/runtime/download-ui-smoke.ps1` runs the Stage 3 download acceptance path without manual
clicking. It launches or attaches to NxTools, opens the real GPUI new-download form, enters a URL,
uses a unique output under `artifacts/ui-automation/downloads`, starts the transfer, captures each
window, verifies the taskbar overlay before and after a foreground switch, and waits for the final
artifact. Each run writes PNG evidence and `report.json` under a timestamped
`artifacts/ui-automation` directory.

Use the deterministic local slow fixture by omitting `-DownloadUrl`, or exercise a real endpoint:

```powershell
tests/runtime/download-ui-smoke.ps1 `
  -DownloadUrl 'http://hkg.download.datapacket.com/100mb.bin'
```

The lower-level `tests/runtime/windows-ui-automation.ps1` accepts JSON scenarios for window
selection, non-interactive client-area clicks, text/key input, foreground activation, screenshots,
native visible/topmost assertions and downloaded-file assertions.

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

## 2026-08-02 download Phase 4 browser takeover gate

- `scripts/package.ps1 -Configuration Release -SkipVerification`: pass after the complete Debug
  workspace gate; the Release package includes the original Chromium/Firefox extension sources,
  Chrome/Edge ZIP, Firefox XPI and `NxTools.Cli.exe` Native Messaging host.
- Chrome, Edge and Firefox user-level Native Messaging manifests resolve to the protected CLI.
  Packaged/protected SHA-256 hashes match for desktop, PlatformHost, indexer and CLI.
- `tests/runtime/browser-native-host-smoke.ps1 -TestBatch -TestBlob`: pass against the packaged CLI.
  It covers framing/correlation, persistent duplicate replay, two-item batch replay, sequential Blob
  staging, tokenized loopback Range transfer and byte-identical atomic output.
- `npm install --ignore-scripts` followed by `tests/runtime/browser-extension-smoke.ps1`: pass with
  Chrome for Testing 143 through Puppeteer's DevTools pipe. The test installs the existing fixed-ID
  extension, checks Native Messaging, creates the supplied 100 MiB browser download, verifies one new
  browser-origin NxTools task and no in-progress browser duplicate, then captures the popup.
- `node --test browser_extension/nxtools-interceptor/tests/*.test.cjs`: 10/10 pass, covering durable
  acceptance-before-cancel, failure fallback, cancellation compensation, bounded/correlated transport,
  Blob chunk order, fixed identities, context-menu routing, Manifest V3 non-blocking permissions,
  popup CSP compatibility and the no-WebSocket/no-database boundary.
- Final browser smoke tasks and files are removed by exact task ID; the screenshot remains at
  `artifacts/browser-automation/browser-extension-popup.png`.
- The 0.5.1 regression reruns the real browser gate against both the source extension and the packaged
  unpacked extension. The automation now fails on Service Worker, extension-log or popup errors, and
  removes its accepted task by exact ID. Both runs passed with the supplied `100mb.bin` URL; packaged
  Chrome/Edge ZIP and Firefox XPI contain neither a blocking listener nor `webRequestBlocking`.
- The packaged extension additionally passed the same isolated-profile gate on the workstation's
  installed Google Chrome 150.0.7871.187, again with no background errors, duplicate browser item or
  residual test task.
- The browser front-end regression now requires a visible `NxTools.NewDownload` window instead of
  treating a hidden `Created` task as acceptance. A GPUI lifecycle test verifies the external request
  hydrates URL, ordinary headers, Cookie, Referrer, filename hint and declared size while the task list
  remains unchanged. Native UI IPC tests cover open and correlated dismiss responses.
- `tests/runtime/browser-extension-smoke.ps1` now drives the real confirmation and progress windows,
  captures them, and validates the final artifact. A deterministic 33,554,432-byte zero payload passed
  exact length plus SHA-256 (`83ee47245398adee79bd9c0a8bc57b821e92aba10f5f9ade8a5d1fae4d8c4302`).
  The supplied DataPacket `100mb.bin` URL then passed on the final protected package at exactly
  100,000,000 bytes with no browser duplicate. Screenshots and reports are under
  `artifacts/browser-automation-final`.
- The final package and protected service copies match for `NxTools.exe`, `NxTools.Cli.exe`,
  `NxTools.PlatformHost.exe` and `NxTools.Indexer.exe`. The full Release gate had one known concurrent
  CDN fixture timing failure; its exact Release rerun passed immediately before packaging.
