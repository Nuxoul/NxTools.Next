# Whole-project responsibility refactor outcomes

## Outcome
Completed the user-requested whole-project source review and targeted behavior-preserving refactor. Coverage includes12 Rust owner families, Desktop controllers/UI composition, C# PlatformHost/tests, browser extension, installer and five build/deployment scripts. This is a major-family interface/call-chain/ownership review, not an every-line correctness or security certification. Existing broad dirty work preserved; no reset, commit/push, packaging/install, real service/registry/Shell/browser/clipboard/history or unrestricted disk scan performed.

## Implemented organization and simplification
- Core: `paths.rs` owns discovery/config location, `instance.rs` owns process lock/advisory window hints, `logging.rs` owns nonblocking init/process-scoped retention. Root exports unchanged; settings/defaults/normalization/locked atomic section updates retained in lib.rs.
- Clipboard: private `categories.rs` and `query.rs` extend the same ClipboardHistory.33 functions relocated with exact implementation comparison; one connection and capture/retention/native formats/assets/archive/storage ordering unchanged.43 tests retained.
- Download: private engine/browser, engine/queue, engine/projection implementations registered and originals removed.42 functions relocated;40 identical bodies,2 snapshot filters equivalent after shared predicate expansion.147 retained bodies and41 original engine-file test names preserved. Production engine entry reduced from4942 to3162 lines; total still includes inline tests. This does not claim all remaining length is unnecessary.
- Desktop: artifact_actions.rs groups correlated file-target/request/result/timeout/disconnect, drag and delete-confirmation lifecycle on same DownloadApplication; repeated failure copy consolidated. LinkPanelSession now has one GPUI window identity instead of two; native HWND/view/timer/generation unchanged. Existing reuse regression extended through timer-close/new-capture and explicit-close/forget/new-capture.
- IPC: one CommandRouter::enqueue owns pending-before-send/queue rollback for normal and summon entry points. Caller-specific duplicate/error rules and generations retained.
- BrowserHost: one bounded diagnostic JSON metadata decode replaces independent request-ID/type decodes. Typed request parse/error precedence unchanged; new malformed/missing/wrong-type/empty/oversize/fallback regression.
- C#: removed the pure failure-forwarding method, calling existing failure executor directly. Obsolete initial-authorization predicate/helper-only test removed in earlier slice; actual protected deployment decision tests retained.
- Extension: removed unreachable preemptive owner/classifier/dispatch and unused background Blob-reader wrapper; retained passive header/resource capture, accepted-before-cancel, Native Messaging/Blob leases and content/page compatibility.
- Architecture source gates changed only their input file lists to real owners; assertions preserved. Owner structure specs updated.

No dependency, general-purpose Manager/Factory/Strategy, second state owner, public method/schema change or lint suppression added. File relocation is identified separately from actual dead-code removal/deduplication.

## Executed verification
All runs use current source after integration. Rust jobs1/incremental0/codegen-units256 with verified locked1.96.0 toolchain command-local PATH; no system toolchain mutation.

| Verification | Actual result |
|---|---|
| Complete Desktop tests |650/650,0failed/ignored |
| Core lib |44/44 |
| Clipboard lib |43/43 |
| IPC lib |24/24 |
| Download lib |110/110 |
| BrowserHost lib |31/31,including new metadata fixture |
| Download engine_contract |103/103 serial |
| Runtime speed cap contract |8/8 |
| Finder lib/integration |68/68 |
| CLI / Download IPC / Network / Protocol lib |10/6/2/42 allpassed |
| C# build and exact safe fixtures |0warnings/errors;13RUN/13PASS,0skips |
| Extension node tests / background syntax |28/28,0skips;syntax exit0 |
| Strict affected-owner Clippy with tests |exit0, -D warnings |
| Workspace test compilation |cargo check --workspace --tests exit0 |
| Formatting |cargo fmt --all -- --check exit0 |
| Architecture |5scripts passed:browser-native-messaging,download-ui-lifecycle,clipboard-lifecycle,clipboard-ui-lifecycle,ui-state-lifecycle |
| Spec contracts / whitespace |87docs/12owners passed;git diff --check exit0 |

Recoverable integration errors were fixed: moved chrono traits needed explicit test imports, removed leftover unused Core/engine imports, and changed new Link test to observe actual windows rather than cross-module private fields. Initial download source gate could not see relocated artifact lifecycle; owner file list corrected without deleting its checks. Final integrated Desktop and strict/workspace checks passed. No ignored tests or relaxed behavior assertions used.

Scratch logs: whole-refactor-core-ipc.log,whole-refactor-clipboard.log,whole-refactor-rust-behavior-2.log,whole-refactor-desktop.log,whole-refactor-download-integration.log,whole-refactor-quality-2.log,whole-refactor-consumers.log,whole-refactor-csharp.log,whole-refactor-extension.log,whole-refactor-source-gates-final.log. Exact relocation evidence and references are in clipboard-outcomes.md,download-engine-outcomes.md,desktop-artifact-outcomes.md,extension-outcomes.md. Whole-project findings/retained complexity/follow-ups:whole-project-review.md.

## Remaining limits and independently recorded issues
No blocker remains for this targeted source refactor. Real installed/native/physical/DPI/reader and service-release acceptance remain separate and were not executed. Do not claim refreshed Debug/release binaries or visual acceptance: tests compile test binaries; no product package was generated.

Useful native STA/OLE/auth/transaction/queue/mmap/cancellation complexity was retained. Larger transport/cache/signing-identity seams and CLI output deduplication were recorded, not mechanically forced into this slice. Source-backed pre-existing extension alias eviction and Indexer persistence-error visibility issues require separate behavior-fix regressions. Unsafe unfiltered C# build-wrapper and signing-key test isolation concerns are documented; those entry points were not run. These findings are not hidden and are not claimed fixed by this behavior-preserving work.
