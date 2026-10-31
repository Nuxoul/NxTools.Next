# Archive feedback and dual-progress results

## Delivered

- Drag rejection, submission failure, native failure and unlock feedback publish through the existing deferred NotificationCenter. The extra `archive-drag-feedback` banner and its layout state are removed. Existing archive read/stale/recovery states remain local to the page. Stable notification identity bounds repeated feedback; retired replies remain quiet.
- ZIP backslashes and forward slashes share one presentation/selection projection. Immutable original names, source binding and ordinals remain unchanged. Tree/list drag, selected extraction and existing edit comparisons use that projection. Native descriptor components use UTF-16 units, preserve invalid/device-name checks, and retain the full260-unit descriptor capacity boundary. Drive roots are accepted as ordinary destinations in Rust and C#.
- Contextual Extract fixes its source/selection to the opened object and removes source replacement. Global empty-source Extract retains source selection and subsequent replacement. Destination Browse sends the matching correlation ID; rejection, expiry and disconnect clear the original form's pending state. A missing directory hint falls back to the system chooser default. Only picker receipts use the existing native two-minute lifetime; other receipts keep10seconds.
- Extraction renders independent current-file and overall official Progress controls. Worker snapshots carry actual current bytes/known size plus selected-plan totals; fields pass through C#, protocol and the original generation-scoped event path. Current file resets independently, unknown/zero totals do not fabricate percentages, and terminal native publication/cleanup remains authoritative.
- The progress layout retains production560×260/default and480×260/minimum bounds. Current filename is single-line with its full accessible label; aggregate bytes are not repeated in a second paragraph. File count, elapsed/rate/ETA and original pause/stop/background/keep-open behavior remain.

## Verification

| Check | Result |
| --- | --- |
| Desktop archive regression suite, final code |353 passed,0 failed |
| Archive backend unit regressions with isolated-codecs |51 passed,0 failed |
| Protocol unit regressions including root/UTF-16 boundaries |98 passed,0 failed |
| IPC unit regressions including picker lifetime |44 passed,0 failed |
| Scoped strict Clippy: Desktop/archive/protocol/IPC tests |Passed |
| PlatformHost/test build |0 warnings,0 errors |
| C# dual-progress wire and Windows path/descriptor fixtures |RUN2/PASS2/SKIP0/FAIL0 |
| Spec/catalog/release/editor source gates |Passed |
| Scoped formatting/whitespace |Passed |

Red-capable evidence includes the production drag freezer rejecting the supplied raw member, the actual parser rejecting an uncorrelated picker command, extraction-progress tests missing required APIs, and root/UTF-16 assertions failing on the original validators. Regressions retain invalid-path, stale-result, cleanup, cancellation and no-fabricated-percentage cases.

The actual Debug progress renderer was opened without runtime/platform/services using synthetic independent25%/40% values and unknown current size. Inspected captures:
- [Dark,560×260](assets/dual-progress-dark.png)
- [Light minimum,480×260](assets/dual-progress-light-min.png)
- [Unknown current size,560×260](assets/dual-progress-unknown-light.png)

Both progress groups and fixed controls are visible at the tested sizes. These are inspected isolated images, not user visual approval or physical/DPI/provider acceptance. All owned preview processes were verified exited.

## Supplied sample

The original `xiugou-tv-windows-x64-v1.0.3-portable.zip` was opened read-only through the unchanged owned-test-package native seam with the new Debug worker and PlatformHost DLL. The test process uses this session's scratch as its temporary directory, satisfying the existing owned-root guard. Worker approval and source/output identity validation were not altered.

- Browser returned215 members and the actual selected ordinal0;8,625.7682ms, succeeded/cleanup complete.
- Materialize returned the intended8,324,412-byte file;18,967.6833ms, succeeded/cleanup complete; lease released.
- Ordinary selected extraction returned succeeded/cleanup complete in27,101.1222ms, with exactly one committed/published8,324,412-byte file and no skipped files.
- The observed progress snapshot carried `current_bytes_done=current_bytes_total=8324412`, `bytes_done=bytes_total=8324412`, and `files_done=files_total=1`, with the correct current member. The harness drains the latest progress when session events arrive, so this confirms native field transport, not intermediate update cadence.
- Streaming byte comparison of the extracted file against the decoded ZIP member passed for all8,324,412bytes. The original ZIP and existing outputs were not modified; no extracted code was executed.

Evidence root: session scratch `archive-product-af7f2c17-5670-4a09-a101-695369337fd6`; structured `selected-member-results.json`, `dual-progress-output-comparison.json` and the final scoped verification logs are retained there/in session scratch.

## Remaining issues

- Extraction is still slow: this Debug sample run succeeded in about27seconds. Repeated whole-archive revisions and synchronous JSON output remain confirmed costs, but their stage-level durations have not been isolated. No performance-fix or stable timeout-resolution claim is made; the original production-UI >60second report remains unverified.
- Closing a form retires its Rust picker state, but an already-open native picker lacks a per-operation native cancellation command. Live owner/focus/close behavior remains unverified; it can remain open until user dismissal or the bounded native wait.
- Actual external OLE target interaction, system picker interaction, other DPI/font/provider states and installed/protected release behavior remain unverified. No package, installation, service operation, commit or push was performed.
