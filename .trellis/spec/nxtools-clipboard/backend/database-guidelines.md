# Database Guidelines

> Database patterns and conventions for this project.

---

## Overview

<!--
Document your project's database conventions here.

Questions to answer:
- What ORM/query library do you use?
- How are migrations managed?
- What are the naming conventions for tables/columns?
- How do you handle transactions?
-->

(To be filled by the team)

---

## Query Patterns

<!-- How should queries be written? Batch operations? -->

(To be filled by the team)

---

## Migrations

<!-- How to create and run migrations -->

(To be filled by the team)

---

## Naming Conventions

<!-- Table names, column names, index names -->

(To be filled by the team)

---

## Common Mistakes

<!-- Database-related mistakes your team has made -->


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
(To be filled by the team)
