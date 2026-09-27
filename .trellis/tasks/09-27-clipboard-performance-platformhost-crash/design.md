# Clipboard Performance and PlatformHost Crash Design

## Decision boundary

Rust owns clipboard history state, SQLite queries, bounded worker operations, list projections, and GPUI data state. C# PlatformHost owns Windows clipboard APIs, STA/native event handling, native payload capture, and IPC publication. The browser/desktop reference projects are behavioral evidence only.

The deterministic baseline is now measured. Count-on-initial/search only, a stable cursor for forward pages, and a bounded bidirectional UI window are selected; FTS is deferred because the cursor path materially removes deep-page ordering/skip cost and the remaining search scan has not justified a schema migration.

## Data flow

```text
Windows clipboard event / capture
  -> PlatformHost native read and bounded payload staging
  -> IPC clipboard_captured event
  -> ClipboardCaptureWorkerHandle bounded queue
  -> ClipboardHistory SQLite transaction
  -> ClipboardPanel query controller
  -> bounded list window + lazy preview/detail
```

Search and paging use one generation-scoped controller. A new query/category/scope/delete refresh invalidates the previous generation, prevents stale commits, and coalesces queued work. A running SQLite statement may finish, but obsolete work must not keep accumulating behind newer requests.

## Candidate changes, selected by measurement

1. Repair the existing CPU harness and add a deterministic database/worker fixture. Measure query wait, page/count duration, row materialization, render/projection, queue depth, retained rows, and process memory.
2. Replace unbounded `Rc<Vec<ClipboardItem>>` retention with a bounded range/window cache if the retention slope is confirmed. Keep total count and selection identity independent from loaded rows; preserve page order and scroll anchor.
3. Add explicit query coalescing/cancellation at the worker boundary. A query request carries generation and can be skipped before execution when superseded. Delete/refresh requests share the current query controller rather than adding another full query behind it.
4. If SQL is dominant, compare: count-on-initial/search only, page-plus-one instead of count-on-every-page, keyset pagination, and FTS/index-backed search. Migrations must be backward compatible and query semantics must remain content/extended-scope compatible.
5. If render/materialization is dominant, introduce a bounded list summary projection and move image/native-format/file metadata/preview work to selected or hovered rows with version/generation-aware caching.
6. For PlatformHost, add a bounded capture-publication tail or coalescing boundary, verify payload limits before expensive conversion where possible, and add top-level exception diagnostics around message/event and asynchronous boundaries. Diagnostics record exception type/HResult/stack context/correlation, never clipboard payloads.

## Compatibility and rollback

- Existing worker ownership, SQLite schema, clipboard event contract, generation validation, and GPUI deferred updates remain authoritative.
- Preserve content-only and extended search semantics, category/source filters, delete/undo behavior, selection, preview suppression, and capture deduplication.
- Keep the old query path behind a test seam or compatibility branch until the new benchmark and contract tests pass.
- If a candidate optimization regresses ordering, selection, preview, error state, or capture reliability, revert only that candidate and retain the measurements.

## Crash evidence boundary

`0xe0434352` is insufficient for attribution. The task must correlate PlatformHost process role, exception type/HResult, faulting module, managed/native stack, dump/WER data, and nearby low-frequency diagnostic events. Without a dump/stack/WER match, report the evidence gap and harden the boundary without naming a root cause.

## Selected Implementation

- `ClipboardHistory::query_category_page_after` uses the existing usage ordering as a cursor boundary and fetches one extra row to preserve `has_more` semantics. The existing offset path remains available for compatibility and backward page loads.
- `ClipboardCaptureWorkerHandle` coalesces query generations before execution and exposes both offset and cursor page requests through the same typed async boundary.
- `ClipboardPanel` tracks the database offset of the first loaded row, the next forward cursor, and the total independently. The loaded payload window is capped at `HISTORY_PAGE_SIZE * 4` (320 rows); front eviction is used while moving forward and tail eviction while moving backward. `ListState::splice` adjusts the logical anchor for each replacement.
- PlatformHost exception diagnostics and clipboard publication continuation containment are boundary hardening only. They do not claim a root cause for `0xe0434352` without WER/dump/stack evidence.

## Review Corrections

- Query cursors carry the complete ordering tuple instead of rereading mutable rows by ID.
- Query cancellation is presentation-neutral, and scroll pagination is blocked while a replacement query is pending.
- Native callbacks and publication tails have bounded/diagnostic failure paths; staging cleanup is required on drop/failure.
- Redownload replacement state is durable across the replacement/cleanup crash window, and compact summaries preserve final-path search/display data.
