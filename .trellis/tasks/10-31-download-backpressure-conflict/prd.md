# Download backpressure and destination conflict

## Scope
Fix H1 and M2 and deliver their user-visible U1/U2 behavior from the 2026-10-31 quality audit and UX proposal.

## Requirements
- H1: primary download events never block callers; retain bounded capacity and secondary subscriber semantics. Lost state is recovered through the existing generation/sequence-aware SnapshotRequired event.
- U1: one asynchronous batch at a time; official loading/disabled actions; existing NotificationCenter reports counts and offers failed-only selection. Cancel and file-deleting batches of at least 20 tasks require one quantity-labelled confirmation; list-only removal does not.
- M2: final publication and browser Blob import never replace an existing destination, retain temporary bytes on conflict, and reserve names for uncommitted tasks that can run again.
- U2: the user-approved 2026-11-01 action set is automatic numbered-name retry and Cancel. The existing 440x196 failed task window names the destination conflict and filename and preserves deferred callbacks/window lifecycle; existing destination bytes remain unchanged.
- Define bounded correlated lifecycle diagnostics without URLs, headers, cookies, names, paths or per-target batch logs.

## Acceptance
Regression tests demonstrate red then green at engine command, final publication/Blob and production GPUI/TestApp boundaries. Verify UI turn responsiveness, list/snapshot consistency, duplicate admission, partial-result selection, confirmation threshold and Light/Dark fixed-window action bounds. Run the requested download, engine_contract, download-ipc, filtered desktop, strict Clippy, formatting and diff checks; report actual counts and any unrelated failures.

## Constraints
Preserve all pre-existing dirty changes. No commit, push, package, install, product launch, external network or real user data. No vendor/dependency/palette changes or unrelated audit/UX work. Temporary synthetic files and loopback fixtures only. Every Cargo process uses the workstation single-job/nonincremental/codegen-units policy.
