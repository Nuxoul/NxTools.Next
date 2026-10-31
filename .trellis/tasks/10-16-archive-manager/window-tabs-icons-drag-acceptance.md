# Independent acceptance — icons, captions, progress, preferences, Tree and tabs

2026-10-27. Verdict: **PARTIAL ACCEPTANCE, three P2 password/focus defects remain OPEN**. User supplied the implementation transcript and requested acceptance, not additional feature work. Main independently inspected source/original evidence and ran checks; one bounded readonly reviewer inspected host/caption lifecycle. No product code changed, package/install/real GUI/Explorer/Registry/service operation performed. This report supplements the single phase-a-checkpoint.md; original negative reports remain unchanged.

## Requirement verdicts

| Slice | Independent conclusion |
| --- | --- |
| Approved app/archive icons | PASS source/offline bytes: unchanged approved originals, reproducible derivatives, both actual DebugPE groups/18frames pass. Native installed appearance and package update NOT RUN. |
| Shared captions / duplicate caption | PASS inspected source and synthetic boundary: shared capability-driven official controls, explicit Client decorations for archive windows, main single/double-click original commands. Physical native/DPI/IME/UIA NOT RUN. |
| Compact progress / readable statistics | PASS source/synthetic: binary sizes, shared readable elapsed, terminal uses real committed/verified counts rather than stale processed values; footer accessible at minimum bounds. Native pixels/user perception NOT RUN. Speed/ETA is separate later HZ2research, not required by this original slice. |
| Extract/preferences | PASS accepted-only field merge/cancel-preserves/disk reopen tests; separate custom-target optin and original lock/writer retained. **Actual fresh-process/restart restore NOT RUN**, despite test messages referring to restart; current tests call AppConfig::load_from in the same test process. |
| Tree subtree/root native drag | PASS synthetic pointer -> original command and controlled native worker/descriptor/content chain. Wrapper only changes presentation path, original ordinal/cache path retained; budgets/refusals intact. Actual held mouse/foreign OLE/Explorer NOT RUN; implementation-before behavioral RED missing as already disclosed. |
| Tabs / open reuse | Core same-host/duplicate-read/close/lease/current-source tests PASS, but password-dialog coexistence/retirement/focus is incomplete (WAC01–03 below). Not fully accepted. |

## Confirmed source findings, not newly reproduced failures

All three are source-call-chain findings crosschecked by main after readonly review. Existing tests do not exercise exact scenario; no new failing behavioral test was authored/run in this acceptance. Fixes must start with red-capable regressions, not claim source suspicion as an observed RED.

### WAC01 — P2: pending password never displayed after another dialog closes

`rust/apps/nxtools-desktop/src/archive/password.rs:148` returns when host already has an active Dialog. Ticket stays pending but no generic close/resume hook retries it; password_prompt is retriggered by password event or tab activation only. Example: current loading encrypted source, drag-in decision Dialog is open when password event arrives, then cancel the decision. User sees no password input until reactivating the tab or timeout. `application.rs::handle_drop` provides a real competing Dialog and has no pending-password replay on close. Existing background-password and new-tab tests do not cover overlay-before-password delivery.

Required regression: open actual competing host Dialog -> deliver current password ticket -> close overlay -> same active tab gets password Dialog exactly once; no tab switching, no duplicate prompts, original ticket/generation/secret/timeout rules preserved.

### WAC02 — P2: ordinary entry terminal leaves password Dialog and draft alive

`archive/entry.rs:229` non-preview entry_result removes passwords ticket only; unlike read terminal (`application.rs:865–866`) and preview (`entry.rs:224–226`), it does not call original dialog retirement. `finish_open` clears browser pending state, not password_dialogs/input. If entry worker times out/fails while user is still typing, the orphaned modal remains blocking and can hold its draft; confirmation no longer belongs to a live entry.

Required regression: real ordinary entry pending + displayed password input with sentinel -> deliver failed/timeout terminal -> Dialog removed, draft cleared, dialog/ticket map retired; late confirm dispatches nothing; subsequent entry still works. Close only the matching operation's Dialog, never a newer unrelated modal.

### WAC03 — P2: old dialog close can restore hidden tab focus after new-tab activation

`archive/browser_host.rs:75–82` defers current-view focus but skips while old Dialog exists. `password.rs:70–82` later clears/closes the old modal; official `vendor/gpui-component-080/src/root.rs:288–290` restores its previous focus handle (old A tab). No later B-content focus pass is scheduled. When A has a password modal and external request opens B, visible tab is B but focus may end on hidden A/invalid chain. Existing `application_tests.rs:2319` verifies no old Dialog and retained A ticket, not actual B focus/keyboard operation.

Required regression: A password Dialog -> original open request B -> after all deferred closes, B's original focus is current and keyboard action affects B only; A ticket remains available when returning. Do not query WindowExt before Root, synchronously reenter leased entities, or steal focus through a current/new Dialog.

## Independent executed verification

Evidence directory: `C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\window-slice-acceptance-20261027-215801`.

- Opening baseline591task-related files; SHA256 unchanged through all checks. CumulativeHEADdiff was not treated as round ownership.
- Related Desktop archive + four historic failures + caption filters:219PASS/0FAIL/0ignored,52.80s.
- Core93/93, IPC43/43, Protocol83/83;0ignored.
- Independent full `cargo test -p nxtools-desktop --locked --offline -- --test-threads=1`: **1398unitPASS/0FAIL/0ignored,449.93s**, then **10integrationPASS/0FAIL/0ignored,2.69s**; wrapperexit0. This is a new independent run, NOT rewriting original1393/4history.
- Four-owner Clippy `--tests -- -D warnings` PASS; workspace `check --tests` PASS. EveryCargo used jobs1/incremental0/codegen256 and checked lane; no duplicate build/name-wide kill.
- Production-only strict independently FAILS existing SPACING_12(main_window.rs75), animated(tools/floating_assistant/media.rs62). Globalfmt independently FAILS three existing network files; no suppression/out-of-scope rewrite.
- Fresh isolated C# testhost build0warnings/0errors. Fresh matching isolated-codecs worker buildPASS. Explicit reviewed Tree5+pureiconplan1: exact **LIST6=RUN6=PASS6,SKIP0/FAIL0/not_run0**. Original native HGLOBAL/IStream/Job/cache controlled consumers only, no actual external OLEtarget.
- `generate-approved-icons.py --check`, approved-icons.py with actual DebugPE18frames, Tree3sourcechecks, spec/catalog/release/editor, DesktopShell/DownloadUIlifecycle source gates PASS. Source gates are not physical lifecycle proof.
- Scoped diffcheckPASS; original135321byte checkpoint remains exact suffix. One evidence-aggregation attempt hit CRLF exact-name comparison; normalized read and reran aggregator only, no test rerun/false failure claim.

Machine evidence `acceptance.json`, raw command/logs and source-baseline/drift records are in that directory. No complete native/physical/user visual acceptance was attempted. Original installerror1 is not fixed by these checks; current package has not acquired this round's source. Next action: repair WAC01–03 with exact regressions and one affected retest, then separately approve packaging/native acceptance. Do not reopen all completed codec/history/temp/settings slices.
