# W2–W5 source closeout — 2026-10-14

## Outcome
W2–W5 are source-delivered in canonical order, including the user-approved W4 true-time prerequisite. W6–W10/plugin work remains deferred. See code-spec.md for executable contracts and usage.md for user behavior/data migration.

- W2 bounded no-result homophone matching with explicit label/shared highlights, ordinary-result/rank/filter preservation. Ordinary local sources complete before indexed fallback permission; local hits suppress the expensive approximate phase without suppressing ordinary indexed files.
- W3 default-off per-kind candidate retention; launcher opts in and Finder owns unchanged final rank. Directory completeness regression with900competing files passes.
- W4 explicit launcher recent mode, narrow view-only noise suppression/diversity, true MFT/USN last-write, mapped base+delta persistence and current-generation budget failure. Default/file-dialog prefix retains ordinary syntax and terminates normally.
- W5 rejects retained old rows while pending and revalidates deferred primary/action-mode execution. Input change, close/reopen, mode/level/action replacement reject stale Shell actions; current actions still dispatch normally.

## One bounded review and fixes
The completed directed review found three concrete defects; all were reproduced and fixed, with no second broad review.
1. Deferred action-mode execution lacked current ownership checks. Real GPUI regression failed `input must cancel the deferred Shell action`, then passed all five invalidation cases and unchanged current dispatch.
2. Indexed homophones ran even when a local ordinary command existed. Indexer permission and actual launcher bridge regressions both went red; explicit default-off permission plus local-first coordination made both green.
3. Raw recent prefix hijacked file-dialog requests and could strand pending state on budget error. Same over-budget synthetic snapshot failed the default-request regression; explicit default-off recent mode at both search entries made ordinary terminal batches green. Actual file-dialog batch owner regression confirms current completion clears waiting; stale completion does not.

Two test-authoring compile issues (qualified Message variant and TestAppContext borrow) were corrected before behavioral evidence; they are not counted as red behavior tests. A later mapped-cache regression omitted the new explicit mode; fixed only that fixture option, retained all assertions, and reran both persistence tests successfully.

## Final verification
All Cargo commands used Rust1.96.0 direct-toolchain PATH, jobs1/incremental0/codegen256, serialized execution and full-operation timeout. Logs are in session scratch under `w2w5-close-*` / `w[245]-review-*`.

| Check | Actual final result |
| --- | --- |
| Desktop full suite |684 passed,0failed,0ignored |
| Finder full suites |80 passed (35+10+4+5+16+4+6),0failed |
| Protocol full library |44 passed,0failed |
| Indexer isolated contracts |28 passed: W1=1,W2=1,W3=2,W4=14,R8=5,base/delta reopen=2,USN structure/cursor/budget=3 |
| Scoped strict Clippy with tests |PASS, Indexer/Finder/Desktop/Protocol |
| Workspace test-target compilation |PASS `cargo check --workspace --tests` |
| Formatting |PASS `cargo fmt --all -- --check` |
| Architecture gates |PASS QuickLaunch,Indexer,FileDialog lifecycle and FileDialog visual/source gates |

Final document/spec check passed:87documents/12owners; scoped git diff --check and accidental edit/debug-marker checks passed after final records. Gates inspect source/contracts, not installed behavior or measured visual approval.

## Tradeoffs and remaining acceptance
No W2–W5 real latency/performance gain is claimed. W3 can execute two kind passes; permitted W2 can add<=8variants with kind passes. W4 streams cached rows with bounded candidate/classification resources, but initial/rebuild per-record OpenFileById metadata cost is unmeasured. Production MFT scanner still lacks external cancellation token; injected cooperative seam checks do not certify interruption of native scans.

Old unchanged caches retain unknown timestamps until a legitimate rebuild; no automatic reconnect backfill/scan was added. True NTFS races/ACL/reparse behavior, LocalSystem idleCPU/restart/persisted base+delta/search and protected runtime hashes remain native release gates. Run only after separate authorization and a matching package.

No package, install, service restart, real volume scan, user history/config mutation, commit or push was performed for this closeout. The sequence2 installer delivered earlier does NOT contain W2–W5 or subsequent UI/Explorer fixes. Installed pointer/keyboard/IME/DPI/visual acceptance remains separate. Earlier Explorer success/clipboard resize/first-preview latency issues are not certified by these tests.
