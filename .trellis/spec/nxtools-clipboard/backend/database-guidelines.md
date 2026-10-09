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
