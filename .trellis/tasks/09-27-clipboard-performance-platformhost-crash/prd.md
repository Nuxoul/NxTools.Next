# Diagnose clipboard performance and PlatformHost crash

## Goal

Make long-running clipboard history browsing and repeated search/delete workflows remain responsive, and establish an evidence-backed diagnosis and fix for the reported PlatformHost `0xe0434352` application error. Use Ditto and EcoPaste as behavioral/performance references without copying their implementation or assuming Rust/C++ alone explains the difference.

## Confirmed Facts

- The user reports progressive lag while scrolling deep clipboard history and after repeated `search -> delete -> search` cycles.
- The user reports a long-session `NxTools.PlatformHost.exe` exception with code `0xe0434352`; the current report has no faulting module, stack, dump, or matching WER record.
- NxTools already owns clipboard history in Rust and exposes a bounded clipboard worker queue (`async_channel` capacity 256).
- Clipboard history uses paged queries with `LIMIT/OFFSET`, but each page query also performs a full matching `COUNT(*)`; search uses lowercased `instr()` predicates over text and metadata rather than a measured FTS/index path.
- Desktop UI state uses query/load generations and a worker boundary, but the long-session behavior and stale-result/backlog cost are not yet measured with a red-capable loop.
- PlatformHost clipboard capture catches several expected clipboard exceptions, but asynchronous/native boundaries and process-level unhandled exception evidence still need inspection.

## Reference Findings

- EcoPaste uses a dedicated clipboard watcher thread, owned payloads crossing into async persistence, SQLite WAL, FTS5 trigram search for longer tokens, bounded range/page loading, a virtualized list, request tokens for stale-result rejection, a bounded frontend range cache, and lazy preview/thumbnail work.
- EcoPaste still performs page plus count queries and does not cancel detached persistence work; these are comparison points, not proof that its design is universally optimal.
- EcoPaste has no NxTools PlatformHost/C# STA/IPC boundary and therefore cannot attribute the reported `0xe0434352` crash.
- Existing NxTools reference guidance requires worker-owned history, bounded pages, generation/query/category/scope validation, deferred GPUI updates, and distinct loading/error/empty states (`docs/reference-code/clipboard.md:349-379`).

## Ranked Hypotheses

1. **Stale query backlog**: every debounced search and delete-triggered refresh still executes serially on the worker after its result becomes obsolete. Prediction: a stress loop shows queue wait and query completions growing with rapid search/delete cycles; coalescing or cancellation removes the growth.
2. **Unbounded panel window**: deep scroll appends every page into `Rc<Vec<ClipboardItem>>`, so retained payloads and list bookkeeping grow with history depth. Prediction: working set and row/projection work grow with pages even when the visible viewport is unchanged; a bounded window removes the slope.
3. **Database scan cost**: each page performs a full matching `COUNT(*)`, lowercased `instr()` search, and increasing `OFFSET`. Prediction: first-page vs deep-page query timing and `EXPLAIN QUERY PLAN` expose a growing scan/skip cost; removing count or changing the search/index path changes it materially.
4. **Payload/render cost**: per-row text normalization, file-path formatting, image filesystem checks, and preview/icon work dominate UI frames. Prediction: typed/image/file-heavy fixtures cost more than text-only rows at equal row counts; summary-only rows reduce render time.
5. **PlatformHost capture tail/backpressure**: clipboard capture reads/decodes payloads on the STA path and chains thread-pool publish continuations. Prediction: a clipboard storm causes capture-tail growth, private-memory growth, event drops, or an unhandled process exit even when Rust history queries are idle.

## Feedback Loop Status

- Existing `tests/performance/clipboard-panel-cpu.ps1` can drive the real packaged panel and sample desktop/PlatformHost CPU and working set, but its embedded `Add-Type` C# uses `out var`, which fails on the current PowerShell compiler before the app starts. The harness must be repaired or replaced in the task before it can be a red-capable baseline.

## Requirements

- Build a deterministic performance loop for deep scrolling and repeated search/delete cycles using realistic text/image/file fixtures. Measure query latency, render/projection latency, outstanding work, and bounded memory/row counts.
- Trace the full clipboard flow: PlatformHost capture -> IPC event -> Rust worker/storage -> GPUI query state -> history list/preview. Identify whether the dominant cost is SQL, payload/materialization, stale async results, queue backlog, image/path loading, or rendering.
- Review `D:\Code\EcoPaste-master` with a read-only subagent and record relevant Rust patterns for pagination, search cancellation, caching, virtualization, worker ownership, and payload lifetime. Compare behavior and invariants, not filenames.
- Preserve clipboard payload privacy in diagnostics. Do not log clipboard text, image/file paths, window titles, credentials, or raw payloads.
- Reproduce or instrument the PlatformHost crash boundary without attributing it to a module until faulting-module, stack, dump, or matching WER evidence exists. Add a regression/diagnostic gate at the narrowest real boundary available.
- Keep Rust responsible for clipboard state/storage/UI data flow and C# responsible for Windows APIs/native clipboard integration. Do not replace the architecture with a copied third-party design.

## Out of Scope

- Copying Ditto, EcoPaste, Vue/Pinia, or other third-party source code.
- Replacing SQLite, the Rust clipboard worker, GPUI lifecycle, or PlatformHost with a new framework without measured evidence.
- Claiming a crash root cause from the exception code alone.
- Broad visual redesign or unrelated clipboard feature changes.

## Acceptance Criteria

- [ ] A repeatable unattended performance loop reproduces the reported scrolling/search-delete workload and records baseline timings and outstanding-work/memory signals.
- [ ] The loop demonstrates the dominant cost and a regression test or benchmark fails before the fix and passes after it; if the real UI seam cannot be automated, the limitation and fallback measurement are documented.
- [ ] Deep scrolling uses bounded page/window state and does not accumulate full-history rows, duplicate async jobs, stale result application, or unbounded preview/materialization work.
- [ ] Repeated search/delete cycles cancel or supersede stale work, keep only the current query result authoritative, and remain bounded under stress.
- [ ] EcoPaste findings are recorded with exact paths/functions and translated into NxTools-specific decisions; no source is copied.
- [ ] PlatformHost long-session failure handling has a red-capable regression/diagnostic gate. Any claimed root cause is supported by faulting-module, stack, dump, or WER evidence; otherwise the remaining evidence gap is explicit.
- [ ] Relevant Rust/C# tests, performance checks, formatting, and scoped diff checks pass, with unrelated dirty worktree changes preserved.

## Deferred Items

- Initial performance acceptance uses relative regression and boundedness: deep-page/search-delete p95 must not show a sustained growth slope versus first-page/idle baseline, and retained rows, worker backlog, and private memory must remain bounded. Absolute latency targets can be chosen after the baseline is measured.
- If no local reproduction, dump, stack, or WER record is available for `0xe0434352`, the task will harden exception boundaries and diagnostics while explicitly preserving the crash attribution gap.

## Measured Results

- The repaired deterministic fixture ran with 10k and 100k rows and emitted payload-free JSONL. After reusing the initial matching total, the 100k content page measured about `1.4 ms` at offset 0 and `95 ms` at offset 2960; the cursor page for the same deep result measured about `15 ms`.
- The 100k extended page measured about `1.2 ms` at offset 0 and `188 ms` at offset 8680 with `OFFSET`; the equivalent cursor page measured about `21 ms`. Recent unfiltered pages remained dominated by the indexed ordering path (`0.6 ms` first page and `17 ms` deep page in this run), so FTS was not added.
- Forward paging now uses a stable ordering cursor and reads the cursor row's sort key inside the worker-owned SQLite connection. The old offset API remains for compatibility and backward-window loads.
- `ClipboardPanel` keeps `total_entries` independent from a bounded 320-row window, can prepend and append pages, and uses `ListState::splice` to preserve the visible anchor while evicting rows outside the window. Query generations still supersede obsolete queued work before SQLite execution.
- Automated verification passed: `nxtools-clipboard` 39/39 tests, `nxtools-desktop` 414/414 tests, PlatformHost Debug build and test suite, `cargo fmt --all -- --check`, and `git diff --check`.
- The packaged interactive CPU/working-set harness was repaired but was not run because this session has no confirmed packaged desktop/display soak. A live clipboard storm/working-set measurement is therefore still open.
- PlatformHost diagnostics now preserve exception type, base type, HResult, stack, role and termination context without exception messages or clipboard payloads. No faulting module, managed/native stack, dump, or WER record for `0xe0434352` was found, so the crash remains explicitly unattributed.

## Review Fixes

- Fixed review findings for immutable clipboard cursors, `QuerySuperseded` UI handling, search/scroll replacement races, mixed image/file asset cleanup, bounded PlatformHost publication tail, native callback exception boundaries, helper-role diagnostics, and payload/path-free logging.
- Fixed download redownload reconciliation, partial media cleanup ordering, browser replacement task identity, stale progress rejection, compact final-path projection/filtering, and snapshot progress-cache reset.
- The packaged interactive clipboard CPU/working-set soak and WER/dump correlation remain evidence gaps; no crash root cause is claimed.

## Latest PixPin image-paste diagnosis

- Live PlatformHost evidence confirmed PixPin screenshots carry PNG, `CF_DIB`, `CF_DIBV5`, `CF_HDROP`, and source-owned `DataObject`/`PixPinData`/`Ole Private Data` registrations.
- The active image fallback previously replayed those private registrations as ordinary global-memory formats. The fix now canonicalizes the image to PNG plus top-down 32bpp `CF_DIBV5`/`CF_DIB`, filters source-private formats, and retains the bounded `CF_HDROP` path fallback.
- Added PlatformHost regressions for private-format canonicalization, rewrite triggering, and valid DIB/DIBV5 headers. Debug build and the complete PlatformHost suite pass.
- Browser right-click `Copy image` is covered by the same path: HTML/URL/Chromium metadata formats trigger canonical rewrite and are removed while PNG, top-down `CF_DIBV5`/`CF_DIB`, safe custom formats and bounded `CF_HDROP` remain.
- The Release package and Inno Setup installer were built successfully, and the installer was smoke-installed into an isolated user directory. Live Pi-desktop/QQ/Codex acceptance remains pending because the currently running protected PlatformHost has a different hash from the new package and was not stopped.
- Live PixPin/QQ/Pi/Codex end-to-end verification still requires a rebuilt/running package and the user's target input controls; the existing target-window activation failure evidence remains a separate issue from image-format compatibility.
