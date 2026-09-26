# P1 Snapshot/Detail Layering Implementation Plan

1. Read current command, message, desktop, CLI, IPC, and TaskDetail contracts; pin the existing list fields and compatibility behavior.
2. Add a deterministic snapshot benchmark/contract fixture for 1,000 and 10,000 tasks. Record baseline JSON bytes, construction time, SQLite query time, and desktop projection/refresh time without changing the public snapshot shape.
3. Review the measurements. If the duplicate full projection is materially expensive, add the smallest typed summary projection and preserve an optional/default legacy decode path.
4. Migrate `command_snapshot`, desktop initialization/refresh/event-gap reconstruction, and CLI list/show to the compact path. Keep selected-task detail on `TaskDetail`.
5. Add contract tests for summary round-trip, list filtering/sorting equivalence, event-gap reconstruction, detail-only reads, and legacy snapshot decoding.
6. Re-run the 1,000/10,000 benchmark and compare payload/query/refresh metrics. Record the before/after result in the reference plan.
7. Run targeted download/message/IPC/CLI/desktop tests, then the full related crate tests, `cargo check`, `cargo fmt --all -- --check`, and scoped `git diff --check`.
8. Perform one review pass focused on protocol compatibility, list behavior, stale event recovery, SQLite query scope, and whether the change has proven benefit rather than structural alignment.
9. Update the reference plan's Section 8 implementation state and validation matrix. Commit only this task's files; leave unrelated dirty worktree changes untouched.
