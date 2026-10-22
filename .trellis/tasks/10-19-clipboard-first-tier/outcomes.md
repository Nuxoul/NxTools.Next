# Clipboard first-tier source checkpoint

Date: 2026-10-19. Scope: the four approved first-tier slices, synthetic data only. Implementation and automated verification complete; this is not installed/native/visual/user acceptance. No commit, task archive, product launch, package/install, service change or real clipboard/history operation.

## Implemented in canonical order
1. Derived list thumbnails: independent content-addressed PNG cache, bounded decoding/serialized jobs, no enlargement, original/alpha preserved; corrupted pixel stream rebuild, cache pruning and original purge cleanup. Production list resource cache uses derivative paths, while actions retain originals.
2. Ctrl+1..9 on release: immutable displayed item ID/query generation, held/modifier/IME/focus/editor/dialog/menu/pending guards; existing paste/terminal owner. PageUp/Down reuse selection/scroll/paging with viewport-sized stride.
3. Separate400-Unicode-scalar summary APIs plus full character count, full-body search and unchanged full/archive APIs. NUL-safe SQLite scalar functions. Full-by-ID transactional action/native formats; edit/extension/preview hydration; full-target metadata resolved by worker and revalidated before open. Summary/target/derivative metadata pruned to current bounded list. Stale/deleted/disconnected/query-replaced preparation cannot act.
4. Runtime-only15minute/manual capture pause, explicit resume/retry and pending/applied/failure UI via official Small outlined Buttons/Popover/Label. Independent typed native command, event-owner gate before reads, bounded32requests, post/cancel/stop cleanup, detached confirmation/nonblocking reader. Resume excludes current sequence and does not backfill. Rust in-flight capture guard precedes management deferral and cleans staging. Deadline/reconnect/late-ID guards; no config persistence.

## Verification actually executed
| Gate | Result |
| --- | --- |
| Final thumbnail fixtures | 4 passed/0 failed |
| Actual production panel efficiency fixtures | 10 passed/0 failed |
| Pause application/lifecycle focused set | 3 passed/0 failed (includes the panel fixture) |
| Clipboard full library | 48 passed/0 failed/0 ignored |
| Protocol full library | 51 passed/0 failed/0 ignored |
| IPC full library | 29 passed/0 failed/0 ignored |
| First integrated Desktop run | 819 passed/3 failed; retained as failed evidence |
| Three causal repair regressions | each1passed |
| Repaired integrated Desktop run | 822 passed/0 failed/0 ignored,115.08seconds |
| C# test-project build | 0warnings/0errors |
| Exact six pause fixtures | LIST6,RUN6,PASS6,SKIP0,FAIL0,not_run0 |
| Clipboard C# compatibility union | LIST19,RUN19,PASS19,SKIP0,FAIL0,not_run0 |
| Scoped strict Clippy including tests | pass, -D warnings |
| cargo check --workspace --tests --offline | pass |
| Clipboard lifecycle/UI lifecycle, PlatformHost selection source gates | pass |
| Spec contracts | pass after final contract append,106documents/14owners |
| Scoped rustfmt --check / diff whitespace / new debug-marker scan | pass |

All Cargo used jobs1/incremental0/codegen-units256. Shared builds (Archive and unrelated Wallhaven) repeatedly occupied the lane; commands were deferred or stopped before the next command instead of killing/duplicating them. Logs live in session70da3267 scratch, notably clipboard-final-*.log, clipboard-csharp-*-list/run.log, clipboard-repair-*.log. PowerShell NativeCommandError wrappers around compiler stderr do not override actual exit/test counts.

## Failures and corrective evidence
- Thumbnail small120x80 was enlarged to432x288 by image::thumbnail; conditional no-upscale fixed it. Cache test now also preserves valid PNG headers while truncating IDAT data to ensure full decode catches damage.
- Initial summary tail-target fixture raced the first render's old target request. Fixture waits for that terminal; replace/navigation clears derived targets; effect revalidates active full row.
- First full Desktop: preview used syntheticID7 absent from SQLite and did not await real worker; seeded actual temporary rows and bounded wait fixed it. Cancelled hover now checks generation before reading.
- Source paging assertion still named removed full-query wrapper; updated to summary caller plus actual cursor query branch, retaining four-page/splice assertions.
- Full-item action fixture triggered newly added background target work without scheduler parking. Only summary rows now request full-target metadata; original full-item path remains unchanged. Exact action regression passes; no scheduler/production warning suppression added.
- Early wiring/test compilation failures (trait import/Styled return, test deletion timestamp and window accessor) were corrected and subsequently compiled/tested.
- Tests were written red-capable before initial implementation but not initially run red because lane was occupied; do not describe that as an executed TDD-red baseline.

## Synthetic size evidence, not installed performance
- Text body:350011UTF-8bytes ->1400bytes for400Unicode scalars; tail remains searchable and full hydration exact.
- 3840x2160RGBA original decode:33177600bytes;512x288derivative:589824bytes (56.25x smaller decoded pixel buffer). Original bytes and alpha independently asserted.
- Existing temporary10k/100k query benchmark ran with the48library tests; its JSON timings are retained in clipboard-final-clipboard.log. No reference app/installed capture-latency or memory benchmark was performed. Serialized thumbnail batches remain<=4; no extra image thread pool was added.

## Scope and coordination disclosure
Dirty baseline was preserved; Cargo.lock differs from this task's saved baseline by one Desktop image dependency line, using existing locked image0.25.10. Archive Program.cs resource-probe/timeout registrations and TestEffects entries remain present; Archive implementation was not modified by this task.
Before the user objected, this agent sent a coordination message to the referenced session and started a C# fixer. The user rejected that authorization assumption. The fixer was aborted; its partial disk changes were inspected/completed/verified by this session. After the correction, no further cross-session messaging or delegation occurred. This behavior must not recur without explicit permission.

## Remaining acceptance
No code/test blocker remains at this source checkpoint. Actual visual readability/LightDark/minimum/failure state, physical Ctrl-number/IME/pointer/focus, native listener pause/resume under real copied content, mixedDPI, UIA/reader and installed runtime acceptance have not been performed. Synthetic GPUI rendering and injected C# WndProc operations do not certify these. No release readiness claim or packaging authorization follows.
