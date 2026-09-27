# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

<!--
Document your project's quality standards here.

Questions to answer:
- What patterns are forbidden?
- What linting rules do you enforce?
- What are your testing requirements?
- What code review standards apply?
-->

(To be filled by the team)

---

## Forbidden Patterns

<!-- Patterns that should never be used and why -->

(To be filled by the team)

---

## Required Patterns

<!-- Patterns that must always be used -->

(To be filled by the team)

---

## Testing Requirements

<!-- What level of testing is expected -->

(To be filled by the team)

---

## Code Review Checklist

<!-- What reviewers should check -->


## Scenario: Generation-scoped clipboard window

### 1. Scope / Trigger
- Trigger: changes to `ClipboardPanel` query dispatch, scroll prefetch, selection, or worker reply handling.
- Purpose: prevent stale query replies and unbounded payload retention across GPUI turns.

### 2. Signatures
- `request_entries(query, generation, offset, request, known_total, cursor, cx)` owns replace/append/prepend transitions.
- `ClipboardPageRequest` is `Replace`, `Append`, or `Prepend`.
- `ClipboardCaptureWorkerHandle::query_category_page_after_async` is used for forward pages.

### 3. Contracts
- `QuerySuperseded` is an expected cancellation result: clear pending state without clearing committed rows or showing an error.
- Scroll prefetch is disabled while `search_pending` is true; a replacement query owns the window until it commits.
- The loaded window is capped at four 80-row pages. `ListState::splice` is the only logical-index adjustment for front/tail eviction.
- Total count, window start offset, next cursor, and selected visible index are separate state; no cross-entity synchronous update may be introduced in render/callback paths.

### 4. Validation & Error Matrix
- Generation/query/category/scope mismatch -> discard without changing rows, selection, loading, or error state.
- Append/prepend offset no longer matches the current window -> discard as stale.
- Worker/database failure -> preserve committed rows for append/prepend; replace shows a typed query error.
- Native callback failure -> record bounded diagnostics and keep the platform event thread alive where possible.

### 5. Good/Base/Bad Cases
- Good: search replacement blocks scroll paging, then commits a fresh first page and resets the window.
- Base: moving forward evicts the front; moving backward prepends and evicts the tail.
- Bad: append a new-query page using the previous cursor, or treat `QuerySuperseded` as an empty/error result.

### 6. Tests Required
- Worker test proves queued obsolete queries are skipped before SQLite.
- UI test covers empty-selection keyboard behavior, source contract for bounded append/prepend, and generation/category/scope rejection.
- Desktop full suite plus packaged scroll/search/delete soak must measure retained rows, queue depth, and private memory.

### 7. Wrong vs Correct
#### Wrong
```rust
if panel.has_more_entries && !panel.page_request_pending {
    panel.load_next_page(cx);
}
```
#### Correct
```rust
if panel.has_more_entries && !panel.page_request_pending && !panel.search_pending {
    panel.load_next_page(cx);
}
```
(To be filled by the team)
