# NxTools Clipboard and PlatformHost Evidence

## Clipboard flow

- `rust/apps/nxtools-desktop/src/clipboard_worker.rs:24` uses a bounded async worker queue with capacity 256; the worker serializes capture, cleanup, mutations, and query jobs (`:610`).
- `rust/apps/nxtools-desktop/src/ui/mod.rs:365` debounces search, `:582` dispatches paged queries, and `:614` rejects stale results by generation/query/category/scope.
- `rust/apps/nxtools-desktop/src/ui/mod.rs:500` triggers page prefetch near the visible range. `:637` extends `Rc<Vec<ClipboardItem>>` with every loaded page, so retained row/payload state grows with scroll depth.
- `rust/apps/nxtools-desktop/src/ui/mod.rs:1078` soft-deletes an item and `:1092` refreshes the current query. Search/delete cycles can therefore enqueue serialized queries behind earlier obsolete work.
- `rust/crates/nxtools-clipboard/src/lib.rs:1221` performs a matching `COUNT(*)` for each page; `:1232` uses `instr(lower(...))`; `:1299` orders and applies `LIMIT/OFFSET`.
- `rust/apps/nxtools-desktop/src/ui/mod.rs:1921` renders a list callback over the retained entries; `ui/history.rs` performs per-row text/path/image preparation and preview callbacks.

## Existing performance harness

`tests/performance/clipboard-panel-cpu.ps1` drives the packaged panel and samples desktop/PlatformHost CPU and working set. The current embedded `Add-Type` C# uses `out var`, which fails with the current PowerShell compiler before application startup; repair this harness before using it as a red-capable baseline.

## PlatformHost evidence

- `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs:390` receives clipboard updates and `:502` reads clipboard payloads on the native event path.
- `:633` catches expected clipboard read exceptions, but `WndProc` and all asynchronous/native boundaries are not one uniform process-level exception boundary.
- `:753` chains capture publication continuations and may retain payload references while downstream IPC is busy.
- `csharp/NxTools.PlatformHost/Program.cs:75` catches cancellation only around the main pipe loop.
- No checked-in WER, dump, faulting module, or stack evidence for `0xe0434352` was found. Do not attribute the crash until such evidence exists.

## Red-test seams

- Rust fixture: seed 10k/100k rows and measure first/deep page, count/page query, and search scopes with SQLite query-plan evidence.
- Worker stress: block one query, enqueue rapid generations plus delete/refresh cycles, then assert stale work is coalesced or skipped before execution and latest result wins.
- UI stress: exercise scroll/search/delete and measure retained rows, outstanding jobs, query wait, and working-set slope.
- PlatformHost soak: controlled clipboard storms and payload sizes; record capture-tail/drop events, private memory, process exit, and diagnostic exception fields without payload contents.

## Measured Implementation Update

- The original retained-vector path has been replaced by `ClipboardPanel`'s bounded window state in `rust/apps/nxtools-desktop/src/ui/mod.rs`; total count, first logical offset, and forward cursor are separate from retained payload rows.
- `rust/crates/nxtools-clipboard/src/lib.rs` keeps the existing content/extended predicates and usage ordering. `query_category_page_after` uses the stable ordering cursor for forward pages; the 100k fixture reduced deep content paging from about `95 ms` offset execution to `15 ms` cursor execution, and deep extended paging from about `188 ms` to `21 ms` in the recorded run.
- `rust/apps/nxtools-desktop/src/clipboard_worker.rs` skips superseded queued queries before calling SQLite. The desktop regression suite proves the worker reply path remains asynchronous and the latest generation remains authoritative.
- `tests/performance/clipboard-panel-cpu.ps1` now compiles its embedded probe under the current PowerShell compiler, but no packaged interactive display run was available in this session. Retained-row and private-memory soak values are therefore not claimed.
- PlatformHost diagnostics were added at process, pipe-loop, and clipboard publication boundaries. The `0xe0434352` report still has no faulting module, stack, dump, or WER match; the evidence boundary remains open.

## Review Corrections

- `QuerySuperseded` is an expected cancellation result and must not clear committed UI rows or render a query error.
- A bounded publication tail must drop and clean staged payloads before the bounded IPC event queue is reached; otherwise a serialized continuation chain still retains payloads without bound.
- Native callback and helper-role exception boundaries must be registered at the process role that owns the callback, not only in the normal PlatformHost role.
- Clipboard diagnostics redact messages, paths, URLs, cookies and window-title fields; failure events use stable error kinds instead of exception text.
