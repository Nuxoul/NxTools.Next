# 2026-10-28 archive feedback / HZ2 — PARTIAL handoff

## Current conclusion

Production implementation and the executed functional regressions are present. **This round is PARTIAL, not full seven-item/HZ2 acceptance.** Scoped rustfmt check fails; `progress.rs` reached the per-turn failed-edit stop and was not subsequently edited/formatted through a bypass. The request's complete HZ2 timing/button/render matrix is not fully demonstrated. The two divider lines remain unidentified without an annotated full-window screenshot. Actual Explorer, physical DPI/IME/UIA/user visual and installed-release acceptance were not run.

No packaging/signing/install/update/product launch/Registry/service/ACL/UserChoice/source recycling/commit/push. No HZ3–HZ5 or other-format write engine.

Opening boundaries and preservation: [round boundaries](feedback-hz2-boundaries.md). Detailed native evidence: [native drag](feedback-drag-native-outcomes.md); preview/capability source and REDs: [preview/capabilities](feedback-preview-capabilities-outcomes.md).

## Running identity

Read-only opening observation: PID15556, `C:\ProgramData\NxTools\NxTools.Next\Service\NxTools.exe`, SHA256 `DDD0DA472A50626CD76A2B9032E295617EADA6D4D96242669837188DA42D9F74`, equal to local artifacts. Release32 / `73b6a27d-d4ad-4c8a-95da-b4eea608183e`. File/product version resources blank. That running installation does **not** contain the later HZ1/current feedback source. No automatic installation/launch was performed; two other Host identities were inaccessible and remain unknown.

## Per-item result

### 1. Drag extraction / target blocking — implemented, isolated boundary verified

Confirmed original cause: `ArchiveVirtualFileDataObject.GetData(FileContents)` synchronously awaited original materialization. The actual controlled native consumer's UI phase could not return while the original worker remained stopped at a600ms checkpoint (baseline1823ms total). Full SDK `IDataObjectAsyncCapability` now allows a negotiating receiver to return its UI phase while the original extraction/cache continues on its original owner. Set/GetAsyncMode, Start/In/EndOperation, COM refs/vtable, both End/loop orderings, cancellation, STA pump, returned-stream lifetime and synchronous fallback are implemented. One original held-pointer OLE gesture/STA/DoDragDrop remains;32roots/128descriptors/32files/256requests/512MiB limits and password preauthorization remain.

Native demand progress uses the original bounded transport and original application, with one UI-only presentation slot in Jobs and the same JobProgressView. Window creation is hidden/focus:false; success ACKs sequence NOACTIVATE -> final device bounds -> visibility. No modal from GetData, no target-path inference. Close only hides presentation; pause/cancel are disabled on this surface rather than pretending support after stream transfer. The original browser source lifecycle still cancels its own native drag. Terminal-after-receipt and disconnect are represented; cache-ready/receiver-copying/receiver-reported-complete/ole-only are distinct.

Native RED/GREEN: `drag-native-red-run.log` RUN1FAIL1; delegate final20PASS. Parent fresh isolated worker + current C# final selected39RUN39PASS39 includes original20drag fixtures,6Shell-readback,1result-location and12HZ1/destination fixtures. Slow consumer target phase1ms on latest parent run; **not real Explorer evidence**. Actual Explorer responsiveness and target writes remain NOT RUN. A receiver withholding EndOperation retains the original sole native slot; full EntrySession disconnect can truthfully leave cleanup incomplete.

### 2. Completed extraction location — implemented, synthetic app/native output verified

Confirmed gaps: Desktop used `request.destination` (base), not actual Name/Smart collision folder; ordinary Shell Open only launched without target/foreground readback. Native OutputSession now derives a display/Shell fact before disposal from committed transactions and held directories; Runner -> JobSession -> optional protocol `result_location` -> original logical/attempt/generation terminal gate. Sole directory enters it; sole file selects/reveals it; multiple roots enter verified base once. C# fixtures verify Here multiple/onefile/onedirectory, Smart sole directory and Name/Smart collisions `(2)/(3)`, without guessing path suffixes from request.

Same Shell service gets explicit `open_in_explorer` action: existing exact active directory view first, directory entry versus file reveal, bounded original target/foreground readback. Archive mode uses normal SetForegroundWindow without AttachThreadInput/forced-topmost loop. Target confirmation and denied foreground are separate. Original JobRecord tracks pending receipt/12s timeout/status/manual action; duplicates do not auto-open twice, explicit false remains false, failed/cancelled/cleanup-unknown terminal does not auto-open. Existing missing-setting default true already met the requirement; no preference migration/reset was needed. Active native browser drag delays location; external drop target remains unknown.

Native behavioral RED: successful extraction produced actual collision directory but no terminal location (`feedback-location-native-red-{list,run}.log`: LIST1/RUN1/FAIL1). GREEN latest shape fixture1PASS plus6existing Shell readback fixtures; app tests cover preference, duplicate terminal, failure/cancel/cleanup and multiple records. Real existing Explorer/focus behavior NOT RUN. Warning-success auto-location remains conservative (only Succeeded); no claim of broader success policy.

### 3. Decimal sizes/rates — implemented, tested display boundary

Original central formatter divided by1024. It now uses1000 and B/KB/MB/GB/TB; browser/space/prompts/progress/terminal share it. Speed uses same grammar /s. Preview resource hints are decimal65.5KB/8.4MB while actual byte caps are unchanged. Split input remains MiB with original checked conversion; no budget/protocol capacity change. Genuine RED0PASS2FAIL included1000B showing1000B rather than1.0KB and actual progress lacking measured rate. Both GREEN; browser2,021,888B now2.0MB. Binary units in technical safety-budget explanations and historical reports were not globally rewritten.

### 4. Two divider lines — BLOCKED / not changed

No current screenshot accompanied this request. An annotated full-window screenshot identifying the two lines was requested. Existing general geometry tests pass but cannot identify which two user-reported boundaries are wrong. No guessed line/offset/border repair or physical-DPI acceptance is claimed.

### 5. Built-in MD/TXT/image preview — implemented, TestWindow/lifecycle verified

Confirmed default-off/deep View-menu-only discovery and absent persisted preview preference, not a missing decoder. Visible official Medium outline Preview toggle in Toolbar.content avoids Toolbar.child ghost/compact override. Additive original UiState missing-field default true, explicit false remembered. Original resizable pane, selection/latest token/EntrySession/cache/source/password/decode/lease and shared passive MarkdownDocument/owned image remain. No external program/network/active HTML/new dependency. Long document actual wheel, decoded MD/TXT/image drawing, state matrix, explicit-off restart, min/themed splitter and original doubleclick tests pass.

Integration exposed hidden-tab preview token/content not retired; suspend now immediately invalidates token/drops local content and defers original cancel. Return keeps original selection/model/entity but reseeds a fresh preview token. Default-on also exposed passive requests using replacement connection: original IPC router atomically checks captured read origin before enqueue. No second queue/reservation. Old fixtures now install the same original connection; exact opened-lease Release is counted amid legitimate passive preview cancellation. Current `archive`298PASS includes original lifecycle/held lease/source/disconnect/ABA and7named feedback_preview tests. Source/TestWindow is not installed/user acceptance.

### 6. Non-ZIP capabilities — implemented, tested

Modification restrictions no longer become a permanent global error banner for healthy readable non-ZIP formats. Existing read/extract/preview remain independent; attempted unavailable modification explains “此格式支持浏览和解压，暂不支持修改压缩包内容。” via original official Dialog/Label. Unknown/not-ready does not promise read capability. Ordinary/split ZIP,7z/RAR/TAR/streams, real unsupported/corrupt/password failures remain separate; no write engine or loosened editing authority. Four named feedback_capabilities tests PASS including actual attempted modification Dialog and rendered banner geometry.

### 7. HZ2 — implemented base, acceptance remains PARTIAL

Original ProgressSnapshot/View and archive_job events enhanced, no task system/timer. Bounded event-driven5s/64samples, coalesced>=100ms observations,>=1s history, original monotonic elapsed, processed-byte speed; reliable remaining total and positive speed only for ETA. Wait/pause/pending-control/state/attempt/counter reset/terminal clear samples; old attempt and same-phase regressing counters rejected. Current member comes from original worker output and is UI-only; actual terminal committed/verified/skipped/cleanup remains primary and stale runtime lines disappear. Buttons guard repeated local dispatch; cancel remains possible during pending pause. Existing background/keep-open owners remain. Official Progress/Alert/DescriptionList/outlineButtons, one titlebar, scroll body/fixed footer.

Six production-model/TestWindow progress tests PASS, including insufficient history/window/coalescing/zero/unknown/attempt/reset, pause/terminal clearing, actual MB/s rendering and480x260/560x300 Light/Dark long-member footer geometry. Worker2tests prove1000updates do not bypass100ms framed output and current member uses original snapshot.

**Remaining concrete HZ2 boundary:** no-event stall may retain last displayed speed/ETA until the next event/render snapshot; the full requested fake-clock/button-once/long-error/terminal-cleanup matrix is not fully covered. Drag metrics are sealed-cache milestones and intentionally have no fabricated decoding speed/ETA. Ordinary phases reflect existing real events only; no new fake worker state. Do not call the complete HZ2 slice accepted from the six tests.

## Verification actually executed

All parent Cargo commands checked shared lane and set CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256 in the same process; --locked --offline, long timeouts. No duplicate build or name-wide kill. Other-session chrome_store builds were observed and left untouched.

| Check | Actual final result |
| --- | --- |
| Desktop `cargo test ... archive -- --test-threads=1` |298PASS/0FAIL/0ignored,1159filtered |
| Protected subsets within that run |HZ118, icon8, WAC12 allPASS |
| Core / IPC / Protocol library tests |93 /43 /87 PASS,0FAIL/ignored |
| Strict Clippy Desktop+Archive+IPC+Protocol --tests -Dwarnings |PASS; one internal Delivery::Finished boxed for larger terminal DTO, no policy change/suppression |
| cargo check --workspace --tests |PASS |
| Archive worker progress / job_execution_contract |2PASS /2PASS |
| Fresh worker isolated-codecs build |PASS, SHA256 EED2F89901670AA9BE4EFA4D03BE5BC65BE37885CFA854B565D0CB4991666770 |
| Current C# build |0warnings/0errors |
| Same-filter C# final LIST/RUN |39/39/39PASS,0SKIP/FAIL/not_run |
| Scoped git diff --check |PASS |
| Scoped rustfmt --check |**FAIL,18paths; not hidden** |
| spec/release/editor/catalog/approved-icons/CLI/drag/tree/selection/desktop lifecycle/source-only release-chain gates |PASS (source/resource-only) |
| Seven protected opening hashes |unchanged: password,space,space_requests,archive_space,installerhelper,Cargo.lock,Cargo.toml |

Raw logs/accounting in session scratch: `feedback-final-accounting.json`, `feedback-archive-final.log`, `feedback-packages.log`, `feedback-clippy-final.log`, `feedback-workspace.log`, `feedback-worker-{progress,isolated-build}.log`, `feedback-job-execution.log`, `feedback-native-all-final-{list,run}.log`, `feedback-scopedfmt{,-files}.log/txt`, protected-hashes JSON. PowerShell redirected logs are UTF16 and may include NativeCommandError wrappers for ordinary Cargo stderr; exact native exit/result summaries above are authoritative.

Intermediate failures retained: first compiled archive260PASS5FAIL, third253PASS12FAIL, then final298PASS. The five first failures exposed real tab/token/origin compatibility plus fixture observations; third-run fixture missing origin and old-token assertions were repaired without bypassing production guard. Compile errors/zero selection are not behavioral RED. First native integrated24PASS3SKIP was rerun with explicit existing encrypted fixture directories to27PASS and then fresh-worker39PASS. No skip was counted as PASS.

## Editing/quality blocker

`progress.rs` hit repeated tool `EDIT_LINES_UNSEEN` during this turn (including unseen235 and154/203/294); edits stopped after recognizing the per-turn limit, and no whole-file rewrite/shell patch/delegate/formatter was used to bypass it. Compiled tests and strict Clippy pass but scopedfmt still fails on18changed files (full list in scratch). No formatting cleanup was applied after the failed gate. This is a genuine remaining delivery blocker; a new authorized repair turn is required for that path and final scope formatting. No final complete/ready claim.

The checkpoint opening155472bytes is preserved as an exact suffix by the top-only append. Existing independent HZ1 acceptance and historical failure reports remain unchanged.
