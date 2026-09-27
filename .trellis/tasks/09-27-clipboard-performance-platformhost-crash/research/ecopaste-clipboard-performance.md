# EcoPaste Clipboard Performance Review

## Reference

Read-only review of `D:\Code\EcoPaste-master`; no files were modified and no EcoPaste tests were run.

## Useful patterns

- `src-tauri/src/clipboard/watcher.rs:204` keeps native clipboard handles on a dedicated watcher thread. Owned payloads cross into async persistence; platform handles do not cross threads.
- `src-tauri/src/db/init.rs:10` enables SQLite WAL/`NORMAL` setup.
- `src-tauri/migrations/0001_init.sql:20` defines clipboard storage, a content-hash index, and an FTS5 trigram external-content table with synchronization triggers.
- `src-tauri/src/db/items.rs:150` / `:459` separate page fetch from count and apply `LIMIT/OFFSET` ordering.
- `src-tauri/src/commands/clipboard.rs:520` returns a bounded list projection; full content is fetched through preview/writeback paths instead of every list row.
- `src/hooks/useClipboardItems.ts:79` uses range loading, request tokens, stale-result rejection, page-aligned ranges, and bounded range cache trimming (`:341`, `:383`).
- `src/pages/Clipboard/components/List.tsx:731` / `:822` use a virtualized list with placeholders and total-count rendering.
- `src/pages/Clipboard/components/Header.tsx:91` debounces keyword search and invalidates work when clearing the query.
- `src/pages/Preview/hooks.ts:54` and `src/pages/Preview/cache.ts:32` use generation-aware preview requests and an LRU cache keyed by item/version state.
- `src-tauri/src/clipboard/storage.rs:105` creates thumbnails lazily and caches them; list payloads do not require all image work up front.

## Important tradeoffs

EcoPaste also performs page plus count queries and detached async persistence does not provide an explicit bounded queue. FTS5, React virtualization, cache sizes, and detached persistence are comparison points, not designs to copy.

EcoPaste has no NxTools C# PlatformHost, Windows STA event window, or Rust/C# IPC boundary. It cannot attribute NxTools `0xe0434352`.

## NxTools application

Evaluate bounded range/window state, request cancellation/coalescing, list summary/detail separation, lazy payload work, FTS/index options, and preview generation caches against measured NxTools behavior. Preserve Rust worker ownership, GPUI lifecycle, and C# native ownership.
