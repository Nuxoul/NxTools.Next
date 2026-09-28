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

## Scenario: Foreground-independent Directory Opus source recognition

### 1. Scope / Trigger

- Trigger: changes to PlatformHost Directory Opus/third-party source recognition, file-dialog foreground handling, or Auto Quick Switch dirty-state consumption.
- Purpose: prevent a delayed source capture from being replaced by the file dialog's foreground HWND and prevent a stale source from causing duplicate navigation.

### 2. Signatures

- `FileDialogRecognitionRetry.TryBeginAttempt(foregroundWindowHandle, out windowHandle)` remains foreground-gated for file-dialog recognition.
- `FileDialogRecognitionRetry.TryBeginPendingAttempt(out windowHandle)` is reserved for passive retries against a previously saved source HWND.
- `WindowsPlatformAdapter.EventWindow.RetryDirectorySourceRecognition()` captures the saved HWND, not the current foreground HWND.

### 3. Contracts

- When a third-party source capture is delayed, clear the previous Auto Quick Switch dirty source before scheduling the new bounded attempt.
- A passive source retry may read the saved lister while the file dialog is foreground, but must not activate, focus, navigate, send `WM_COPYDATA`, or send Rust IPC during recognition.
- On source success, publish one source and immediately re-consume the currently known file dialog if it is still foreground.
- On destroy, supersession, timer failure, or budget exhaustion, clear the pending source and never publish the retired HWND.
- Recognition diagnostics may include HWND, attempt count, elapsed time, terminal reason, and native error code; never include paths, titles, or payload data.

### 4. Validation & Error Matrix

- Current foreground differs from pending source HWND -> probe the pending HWND through `TryBeginPendingAttempt`; do not replace the target.
- Pending source HWND is destroyed -> terminate with `target_destroyed` and clear dirty state.
- Capture remains unavailable until the retry budget expires -> terminate with `attempt_budget_exhausted` and clear dirty state.
- File dialog returns before source success -> source success must yield one candidate for that same dialog, not zero or two candidates.
- HWND `0` -> never report a pending retry.

### 5. Good / Base / Bad Cases

- Good: DOpus capture settles after the file dialog returns; the saved lister is read passively and the dialog navigates once.
- Base: DOpus capture succeeds while DOpus remains foreground; the existing source-to-dialog path is unchanged.
- Bad: return to the dialog consumes an old source, then the delayed DOpus source is consumed again.

### 6. Tests Required

- Regression asserts a known dialog plus delayed source capture produces exactly one candidate after the pending HWND is probed in the background.
- Retry test asserts supersession, budget exhaustion, and `HasPendingAttempt(0) == false`.
- Architecture gate asserts the EventWindow caller uses `TryBeginPendingAttempt`, saved-HWND capture, stale-source clearing, dialog re-consumption, and destroy termination.
- Full PlatformHost suite and interactive DOpus/file-dialog soak are required when the environment provides DOpus.

### 7. Wrong vs Correct

#### Wrong

```csharp
var foreground = _foregroundContextProvider.Capture();
_directorySourceRecognitionRetry.TryBeginAttempt(
    foreground.WindowHandle,
    out var windowHandle);
var attempt = DirectoryNavigationAdapter.TryCaptureLocalAttempt(foreground);
```

#### Correct

```csharp
var foreground = _foregroundContextProvider.Capture();
_directorySourceRecognitionRetry.TryBeginPendingAttempt(out var windowHandle);
var target = _foregroundContextProvider.Capture(new IntPtr(windowHandle));
var attempt = DirectoryNavigationAdapter.TryCaptureLocalAttempt(target);
```
