# OPT-07 — O7-AC01/02/03 directed repair

## Result and scope

**All three directed repairs pass their original-callback regressions and scoped Desktop verification. Independent closeout remains pending.** The [original partial verdict](opt07-independent-acceptance.md) is unchanged; O7-AC01/02/03 remain open for directed independent closure, not self-accepted by this record.

The repair uses the original ArchiveApplication/Jobs, immutable BrowserModel, Root/Dialog, typed bridge and current attempt/ticket/connection owners. Four product files and two existing test files changed: [Find](../../../rust/apps/nxtools-desktop/src/archive/browser_find.rs), [shared row warning policy](../../../rust/apps/nxtools-desktop/src/archive/model.rs), [batch projection/Retry](../../../rust/apps/nxtools-desktop/src/archive/batch_jobs.rs), [password Dialog](../../../rust/apps/nxtools-desktop/src/archive/progress.rs), [application regressions](../../../rust/apps/nxtools-desktop/src/archive/application_tests.rs) and [batch regressions](../../../rust/apps/nxtools-desktop/src/archive/opt07_batch_tests.rs). Original C#/protocol/native rights, scheduler capacity/FIFO/source serialization, resource/deadline limits, dependencies, vendor, palette and fonts retain their existing boundaries.

## Repairs and executable protection

| Finding | Root cause | Final behavior and regression |
| --- | --- | --- |
| O7-AC01 | Eligibility followed destructive browser/Dialog transition; warning Navigate coverage did not protect refusal. | Resolve original row identity, safe path and shared warning policy before closing Find or entering a parent. Both warning Extract/Delete retain the Dialog/query/result, folder/filter/selection/focus/anchor, visible identities and projection work; no form/Job/footer command. Warning Navigate and valid/stale original Extract/Delete remain supported. |
| O7-AC02 | Historical logical Started was used as current-attempt state; deferred presentation could capture the pre-dispatch snapshot. | Clear existing worker_state at Retry generation transition and dispatch. Pump the original queue before publishing the retry snapshot. Reserved current attempts with no worker state are Admitting; dispatch/receipt/old Started cannot become Running. Matching current Started advances the original member; ever_started still supplies historical never-started accounting. |
| O7-AC03 | The checked UI choice was combined with live eligibility as a boolean downgrade, bypassing invalid-consent rejection. | Refuse original Dialog confirmation before constructing/copying the password, clearing input, closing or scheduling a reply when checked consent is ineligible. The same ticket/input/checked Checkbox remains reachable; original Field description explains explicit uncheck. Unchecking permits exactly one ordinary reply with the original password. The final reply_job eligibility/one-use guards remain. |

These are transition/assumption and coverage gaps, not new native authority or secret-storage designs. Protection is at the original footer/Retry/confirmation boundaries, with assertions on the deferred rendered projection as well as owner state. The [browser contract](../../spec/nxtools-desktop/backend/archive-browser-workflows.md) and [batch contract](../../spec/nxtools-desktop/backend/archive-batches.md) record the exact ordering and assertion points.

### Original-callback RED → GREEN

| Boundary | Behavioral RED | Current GREEN |
| --- | --- | --- |
| Find warning Extract and Delete | Both actual footer callbacks fail “refusal retains original Find Dialog”. | Both retain original browse/query state and submit nothing; Find-directed selection 10/10 passes. |
| Retry after an actual initial Started | New dispatch fails with Running instead of Admitting. | Dispatch, successful receipt and stale Started remain Admitting in owner and rendered summary; current Started becomes Running. Batch-directed selection 10/10 passes. |
| Checked password A across sibling B Retry recheck/retirement | Original Continue consumes A's current ticket instead of refusing. | Same ticket/input/checked choice survives; zero rejected commands; explicit uncheck sends one same-password ordinary reply. Final batch-directed selection 11/11 passes. |

Four new test names:
- `archive::application::tests::opt07_ac01_find_warning_extract_preserves_dialog_and_browser`
- `archive::application::tests::opt07_ac01_find_warning_delete_preserves_dialog_and_browser`
- `archive::application::jobs::tests::opt07_batch_tests::opt07_ac02_retried_started_member_waits_for_current_started`
- `archive::application::jobs::tests::opt07_batch_tests::opt07_ac03_checked_password_consent_retirement_keeps_dialog_and_ticket`

## Verification

| Check | Result |
| --- | --- |
| Exact preservation selection | LIST61 matches the original 57 unique directed passing names plus the four new names. Final RUN61/PASS61/FAIL0/ignored0; 39.12s. |
| Strict Desktop all-target Clippy | PASS, offline `cargo clippy -p nxtools-desktop --all-targets -- -D warnings`. |
| Scoped rustfmt | PASS, six changed Rust files, edition2024 with child traversal disabled. |
| Debug Desktop link | PASS, offline original binary target; no worker/package replacement. |
| Executable spec navigation/contracts | PASS, 125 documents/14 workspace owners. |
| Locked Kit/editor provenance | PASS, coherent Kit0.8.0/pre0.3.9/two narrow licensed deltas and pinned editor contract. |
| Archive source boundaries | PASS, CLI16 assertions, tree drag3 tests and Phase F P206/P207 ownership. |

Final preservation uses the original saved C# progress/Create wire exports read-only through `NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE` and `NXTOOLS_OPT03_CREATE_WIRE`. This is transcript replay, not a new native publication. The 61 names are not added to overlapping vertical groups; the historical448 run retains its original result. Prior Protocol105/IPC3/matcher1/CLI15 and C#controlled14/actual-native5 evidence remains attributed to the [independent report](opt07-independent-acceptance.md).

The current 10000-entry preservation assertions retain zero repeated-render catalog checks/45 controls, and Find10000 visits/checks/matches with30 virtual controls. No new baseline speedup, archive/source I/O, exclusive codec CPU, exact peak-memory or physical responsiveness claim is made.

## UI and diagnostics

The same released Kit0.8.0 Dialog/child/DialogFooter, Form/Field public description slot, stable masked InputState and Checkbox::on_click own the decision. Find/footer and batch commands retain direct Medium outlined buttons. No new HWND/controller, theme, layout budget or focus policy is introduced. Cross-entity writes remain deferred; eligibility is read from the original owner before mutation and at final admission. Existing correlated low-frequency lifecycle diagnostics remain; no password/query/path/payload logging or per-render instrumentation is added.

## Evidence and frozen attribution

Session evidence root: `D:\.pi-desktop\data\scratch\a2072588-1f47-4232-be7a-3951a25df615`.

- Repair baseline: `opt07-ac-before/manifest.json`, 105 source/test/document context files. The source delta is exactly the six files listed above.
- Behavioral RED: `opt07-ac01-callback-red.log`, `opt07-ac02-callback-red.log`, `opt07-ac03-red.log`.
- Vertical GREEN: `opt07-ac01-callback-green.log`, `opt07-ac02-presentation-green.log`, `opt07-ac03-callback-green.log`.
- Final names/results: `opt07-ac-selection.json`, `opt07-ac-preservation-list.log`, `opt07-ac-preservation-final.log`, `opt07-ac-verification-index.json`.
- Quality: `opt07-ac-clippy-final.log`, `opt07-ac-rustfmt-final.log`, `opt07-ac-desktop-build.log`, `opt07-ac-static-gates.json` and six `opt07-ac-gate-*.log` files.
- Separate repair freeze: `opt07-ac-delivery`, containing current source context, repair documents, Debug Desktop/test binaries and completed evidence, with `manifest.json` and `consistency.json`. The original `opt07-delivery`, partial report and earlier package verdicts/measurements remain immutable.

## Remaining acceptance

The repair is ready for directed independent closure of O7-AC01/02/03. G-16, Retry's touched G-17, G-20/touched G-26 remain partial until that decision. Overall remains `in_progress`, **Phase E — Format expansion and ZIP editing**. OPT-02/03/04/05/06 independent decisions and closed O5-AC01/02 are preserved; no OPT-08 is introduced.

G-00, OPT-00/01 overall, complete G-04, physical/Release, real G-18/G-28 and G-29's original-object recycling/Create-through-required-post-Test prerequisite remain open. Controlled GPUI callbacks do not certify physical focus/keyboard/DPI, visual approval or a genuine system picker. This repair includes no product GUI/Explorer/system clipboard/interactive picker, user source mutation/recycling/HZ5, service/registry, package/sign/install or commit/push action.
