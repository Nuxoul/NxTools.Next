# Clipboard SQLite and assets

Evidence: `repo:rust/crates/nxtools-clipboard/src/lib.rs#ClipboardHistory`, `repo:rust/crates/nxtools-clipboard/src/storage.rs`, `repo:rust/crates/nxtools-clipboard/src/archive.rs`.

History owns Connection/item/format/category SQL; current initialization uses CREATE TABLE/additive ensure_column, not a claimed versioned schema/WAL contract. Serialized worker stays outside GPUI. Assets are referenced resources. Online backup then VACUUM; migration stages/validates then worker commits config before switch. Retention max(7,days); late import Partial cleans only proven new orphans. [Quality](quality-guidelines.md), [verification](../../shared/verification.md); retain cursor contract below.

## Scenario: Bounded clipboard history pagination

### 1. Scope / Trigger
- Trigger: any change to `ClipboardHistory` paging, cursor construction, or worker-owned history projection.
- Purpose: keep query order stable while the UI loads a bounded payload window.

### 2. Signatures
- `ClipboardPageCursor { is_pinned, last_used_at_unix_ms, captured_at_unix_ms, id }` is the complete usage-order boundary.
- `query_category_page_with_total(query, category, scope, offset, limit, known_total)` is the compatibility/initial-page API.
- `query_category_page_after(query, category, scope, cursor, limit, known_total)` is the forward-page API.

### 3. Contracts
- Every `ClipboardItem` query projection includes `last_used_at_unix_ms`; the cursor is copied from the returned row and is never reread by ID for pagination.
- Cursor SQL uses descending `(is_pinned, last_used_at_unix_ms, captured_at_unix_ms, id)` ordering and fetches `limit + 1` rows to derive `has_more`.
- A known filtered total may be reused for appended/prepended pages; a fresh query must calculate it.

### 4. Validation & Error Matrix
- Missing category -> `CategoryNotFound`.
- Invalid/zero page size -> clamp to `1..=500`.
- Deleted or reordered cursor row -> continue from the immutable cursor tuple; never return an empty page solely because the cursor ID disappeared.
- Worker generation superseded -> `QuerySuperseded`; the UI treats this as cancellation, not a query failure.

### 5. Good/Base/Bad Cases
- Good: initial page stores total and the last row's full cursor; next page preserves ordering after a mark-used/delete mutation.
- Base: legacy offset calls remain available for compatibility and backward-window loading.
- Bad: cursor contains only an ID, or a page repeats `COUNT(*)` after the total is already known.

### 6. Tests Required
- Assert cursor and offset pages have equal items, totals, and `has_more`.
- Mutate and delete the cursor row, then assert continuation has no duplicate cursor item and still returns later rows.
- Run 10k/100k content and extended fixtures and record offset vs cursor timings.

### 7. Wrong vs Correct
#### Wrong
```rust
let cursor = ClipboardPageCursor { id: last.id };
// Query the mutable row again to recover its sort fields.
```
#### Correct
```rust
let cursor = ClipboardPageCursor::from(&last);
history.query_category_page_after(query, category, scope, cursor, limit, Some(total))?;
```

Explicit result navigation uses `ClipboardHistory::page_containing_item` in query.rs on the same serialized worker/connection. Resolve the active ID's rank with the complete descending pin/last-use/capture/ID tuple, then load one bounded original page under the same SQLite snapshot. Missing/deleted ID is ItemNotFound, never first-row fallback; locating does not mark used, copy/paste, alter content or create another store. Test exact page equality beyond the first page, pinned ordering, deleted/missing IDs and bounded payload. Evidence: `repo:rust/crates/nxtools-clipboard/src/query.rs#page_containing_item`.

## Scenario: Lightweight history projection and exact full-item effects
### 1. Scope / Trigger
List payload projection or any consumer of a projected ClipboardItem.
### 2. Signatures
`ClipboardItemSummary { item, text_character_count }`; `summary_category_page_with_total`, `summary_category_page_after`, `summary_page_containing_item`; `active_item(id)` and `active_item_with_native_formats(id)`.
Evidence: `repo:rust/crates/nxtools-clipboard/src/query.rs#register_summary_functions`, `repo:rust/apps/nxtools-desktop/src/ui/efficiency.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard_worker/thumbnails.rs`.
### 3. Contracts
Summary text is at most400Unicode scalar values; full character count includes embeddedNUL. Connection-local deterministic nxt_clip_summary/nxt_clip_chars use raw SQLite text because built-in length/substr stop atNUL. Filtering searches full stored text; full-item/archive APIs remain unchanged. Action/native formats hydrate the same active row in one transaction before effect. UI preserves cursor/order/four-page owner and prunes summary/target/thumbnail metadata to retained rows. Edit/extension/preview/open-target consume full-by-ID data, not truncated text; query/session/operation guards remain current after worker completion.
### 4. Validation & Error Matrix
Missing/deleted row -> ItemNotFound/no native dispatch. Stale query/closed/disconnected/cancelled preparation -> retire without effect. Superseded page -> quiet cancellation. Derivative failure -> placeholder, never original image decoded as list fallback. Cache corruption -> bounded decode validation then regeneration.
### 5. Good/Base/Bad Cases
Good: tail search returns a400-scalar summary and exact full-body paste/edit. Base: full APIs retain all native formats/order. Bad: execute a URL cut at summary boundary, use SQLite length onNUL text, or replace domain image_path with derivative.
### 6. Tests Required
summary_seam_tests assert Unicode/NUL/full-tail search, cursor equality, page location, active/native snapshot. Production efficiency_tests assert hydrated paste/edit/deleted/stale target and query cancellation. Thumbnail fixtures assert alpha/no-upscale/original bytes/corruption/prune/purge; production resource cache contains derivative, not original. Real native/DPI/visual acceptance remains separate.
### 7. Wrong vs Correct
Wrong: summary.item.text -> native command or editor. Correct: immutableID -> active full snapshot -> current operation/query check -> existing effect owner.
Disposable list-thumbnails-v1 stays bounded512files/128MiB,640x288PNG/no enlargement; source<=64MiB/32Mpixels, decoder allocation<=192MiB. Serialized worker alone generates/removes files; migration invalidates old paths and derivatives regenerate lazily.
