# OPT-07 — O7-AC01/02/03 directed independent acceptance

## Decision

**Close O7-AC01, O7-AC02 and O7-AC03. Accept OPT-07 — Required batch and browser workflow delivery at its authorized automated/owned-native boundary.** The three original interaction defects now have correctly ordered production fixes, effective original-callback RED evidence and fresh independent passing regressions. No remaining directed-repair blocker was confirmed.

The [original partial verdict](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt07-independent-acceptance.md) and [repair verification](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt07-ac01-ac03-repair.md) retain their historical attribution. This report supplies the current independent decision, supplementing the original positive batch/native/picker/Retry/Locate/Find/preview evidence rather than rewriting it. Prior OPT-02/03/04/05/06 acceptance and closed O5-AC01/02 remain intact.

Scope: four Desktop product files and two existing test files changed relative to the105-file repair baseline: `browser_find.rs`, `model.rs`, `batch_jobs.rs`, `progress.rs`, `application_tests.rs` and `opt07_batch_tests.rs`. HEAD remains `0940eec23286c1a750b3aff4284f32e76da68164`. This acceptance changed documentation only; product/test source and unrelated user changes were preserved. Overall remains `in_progress`, **Phase E — Format expansion and ZIP editing**.

## O7-AC01 — closed

[Find action admission](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser_find.rs:184) resolves the original row/snapshot identity, safe internal path and warning eligibility before closing the Dialog or entering the result's parent. It reuses [entry_warning](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/model.rs:77), the existing warning-code policy shared with ordinary row projection. Explicit-directory warning qualification follows the same projected record policy; Navigate retains access to warning rows. No temporary browser projection is needed to discover action eligibility.

The two [actual application/footer regressions](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/application_tests.rs:3507) start from another folder with a nonempty local filter and selected/focused/anchored member. After warning Extract or Delete they require the Find Dialog/query/selected result, browser folder/filter/selection/focus/anchor, visible original IDs and projection work to remain unchanged. They also require zero forms, Jobs and footer commands. Setup traffic is retired before the click; absence of an application owner is not used to manufacture zero dispatch.

The retained RED log contains two compiled behavioral failures at “refusal retains original Find Dialog”. Both cases now pass independently, together with original warning Navigate, valid/stale Extract/Delete form submission, query replacement, lifecycle and virtual-list tests.

## O7-AC02 — closed

[Batch member projection](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch_jobs.rs:139) uses the current attempt's existing `worker_state` for Admitting versus Running. [Retry generation transition](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch_jobs.rs:605) clears that state; the original queue is pumped before [publishing the refreshed view](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch_jobs.rs:619), so deferred presentation no longer captures its predecessor's state. Queue-not-running remains Queued. `ever_started` continues to represent logical history for never-started statistics.

The [original Retry regression](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/opt07_batch_tests.rs:543) delivers an actual initial Started, safe failure and the original Retry button/fresh native recheck. It requires exactly one new operation/generation. Direct owner and rendered original summary remain Admitting after dispatch, successful receipt and an old Started; only current Started makes both Running. Historical never-started remains false and aggregate counts stay correct.

The retained compiled RED reports Running instead of Admitting immediately after new dispatch. The independent current execution passes all four transition points and rendered summary assertions. No new startup flag, scheduler or changed native admission authority is introduced.

## O7-AC03 — closed

The [original Dialog confirmation](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/progress.rs:703) retains the actual checked reuse choice. If that choice has become ineligible, confirmation returns false before constructing the password, clearing InputState, closing the Dialog or scheduling a reply. The [same Checkbox and public Field description](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/progress.rs:649) remain reachable while checked, allowing explicit uncheck. The final `reply_job` eligibility and once-only ticket guards remain unchanged.

The [actual Dialog regression](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/opt07_batch_tests.rs:680) holds B's original Retry recheck pending while A receives its original password ticket and the user enters a password/checks reuse. B's valid receipt retires reuse. Confirming A must retain the same prompt, input and checked choice, with zero commands. Explicit uncheck then permits exactly one ordinary reply carrying the exact original password and ticket; replay emits nothing.

The retained compiled RED fails because A's prompt becomes None after confirmation. The current independent run passes refusal, retained Dialog/input/choice, explicit opt-out, exact password bytes and once-only reply assertions. Protection uses the existing official Form/Field/Input/Checkbox/Dialog APIs, with no new secret store, HWND, focus owner, native permission or diagnostic payload.

## Independent verification

| Check | Observed result |
| --- | --- |
| Exact Desktop selection |LIST61/RUN61/PASS61/FAIL0/ignored0;39.43s. The set is precisely the original57 unique preservation names plus four new regressions. |
| Original RED evidence |Four compiled original-callback failures: Find Extract/Delete2, Retry1, checked-consent retirement1; each matches the original symptom. |
| Strict Desktop all-target Clippy |PASS, offline `-D warnings`. |
| Scoped rustfmt |PASS for the six changed files, edition2024, `skip_children=true`. |
| Debug Desktop link |PASS for the original binary target. No product application was launched. |
| Source/document contracts |PASS:spec125documents/14owners; Kit catalog/release, pinned editor, archive CLI16, tree drag3 and Phase F ownership. Source/provenance scope. |
| Repair scope/freeze audit |Exact six-file Rust source/test delta;142 repair files match current origins before acceptance-document updates:67source context,2Debug,39documents,30evidence,4inputs. |
| Historical continuity |Original partial report/prior independent reports and measured data match the repair baseline; original787-file delivery remains available and is not overwritten. |

Commands used workspace toolchain1.96.0, offline mode, one Cargo lane, scratch TEMP/TMP and `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256` in the same invocation, with full-operation timeouts. Exact LIST preceded identical named execution. The six-file delta was reviewed directly against the saved repair baseline; no additional broad review or full448-case rerun is represented.

Evidence root: `D:\.pi-desktop\data\scratch\aab2f188-352d-4f1c-b7a3-5375fe0df2d8`. Independent list/run/lint/format/build/gate logs use `opt07-ac-independent-*`; the exact name set is `opt07-ac-independent-names.json`. Reconciliation is `opt07-ac-independent-evidence-audit.json` from `audit_opt07_ac_acceptance.py`. Saved repair REDs/source/artifacts remain in the separate `opt07-ac-before` and `opt07-ac-delivery` roots identified by the repair record.

The earlier independent Protocol105/IPC3/matcher1/CLI15, five-package lint and C#controlled14/actual-native5 results retain their original scope and are not new runs in this directed acceptance. C#/worker/protocol/native code was not part of the six-file repair. Existing native source/secret/publication/cleanup guarantees remain covered by that retained evidence; no repeated native matrix or new performance claim is added. Projection/virtual-control preservation asserts current work bounds, not source-I/O or physical responsiveness.

## Accepted boundary and remaining gates

The original O7-AC01/02 Standards conditions and O7-AC03 Spec condition are closed. G-16, touched Retry G-17 and G-20/touched G-26 join the retained automated/owned-native acceptance. Original Stop, Add/post-Test, location receipt, query identity/final submission, Up, render bounds and preview preservation remain green at their stated boundaries.

**Scoped OPT-07 acceptance is not overall task completion.** G-18 physical GUI/Explorer/focus/keyboard/visual/DPI, real G-28 system multiselect, G-29 original-object production recycle and Create-through-required-post-Test source continuity, G-00, OPT-00/01 overall, complete G-04, overall Phase E and Release remain open. Controlled GPUI callbacks are not physical or user visual acceptance. Source recycling remains unavailable; no path-based reauthorization or permission expansion is introduced. No OPT-08 is added.

No product GUI/Explorer/system clipboard/interactive system picker, user archive/source mutation/recycling/HZ5, service/registry, packaging/signing/install, reference-program execution, commit or push occurred. No remaining in-scope directed-repair blocker was confirmed.
