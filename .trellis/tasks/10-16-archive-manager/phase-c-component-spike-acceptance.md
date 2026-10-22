# Phase C — Archive browser and entry lifecycle: component spike independent acceptance

2026-10-21. **Blocking component spike evidence independently PASS; archive multi-selection
requirement NOT passed.** The proposed archive-details-only fallback is justified for a user
decision, NOT approved or implemented by this acceptance. No Phase A/B/C or A–F completion claim.
Only [checkpoint](phase-a-checkpoint.md) owns current status.

## Reviewed scope and source

Reviewed the supplied handoff, explicit spike-only dependency amendment in implement.md,
[spike evidence](phase-c-component-spike.md), ui-spec selection record, mandatory component rules,
current table/locked index, all519 lines of archive_component_spike.rs and the exact released
DataTable/TableDelegate/TableState/Tree public implementation involved in the claims.

The test module is disconnected from archive engine/config/IPC/jobs and is included only under
cfg(test). Against the implementation's saved main.rs, the exact delta is the two-line cfg(test)
module declaration. No production browser or fallback was added.

The fixture uses actual published components and current-frame row hit bounds, sends modified
pointer/keyboard events, explicitly focuses the table/tree for keys, and inspects both public
state and final Row selected semantics. It does not equate an external set with multi-selection.

## Independently reproduced result

| Boundary | Observed result |
| --- | --- |
| Stock Ctrl/Shift clicks | internal single Row selection, exactly1 selected Row; no additive/range selection |
| Stock Shift+Down / Ctrl+A | still1 selected; logged Shift+Down Row(3) |
| Stock Up/Down and double click | ordinary single-row navigation and DoubleClickedRow work |
| Stock right click | context row1, selection remains row3 |
| Public render_tr with external set and row_selectable(false) | external range IDs{1,2,3}, final semantic selected rows0/internal None; built-in arrows and double-click path absent |
| Official Tree pointer and focused Up/Down | folder-ID-driven table projection updates correctly through deferred owner work |
| Table folder double click / unchanged Tree notify | deferred Tree selection and table projection agree; unchanged ID does not increase sync count |

Read-only source confirms selection()/set_selected_row maintain one row, on_row_left_click ignores
modifiers and returns before double-click when row_selectable is false, and render_tr is followed
by the component's own Role::Row/aria_selected and internal handlers. Source and executed evidence
support the bounded conclusion: the reviewed released public seams do not supply the required
single-owner Ctrl/Shift/keyboard multi-selection with consistent final row semantics. This is NOT
an exhaustive impossibility proof of every custom composition, nor native Windows UIA evidence.
The Tree synchronization result is a small fixture capability proof, not production lifecycle proof.

## Independent checks

Serial/offline Cargo, jobs1/incremental0/RUSTFLAGS=-C codegen-units=256, no competing build before
each Cargo command. Actual results:
- `cargo test -p nxtools-desktop archive_component_spike:: --offline -- --test-threads=1 --nocapture`:
  **3passed,0failed/ignored,871filtered**, all3named tests executed,0.43s.
- Strict Desktop Clippy --tests AND --bins with -D warnings: both exit0.
- `cargo fmt --all -- --check`: exit0.
- ui-state-lifecycle: exit0, source-only.
- spec-contracts: exit0,106docs/14owners; catalog: exit0,79pages/79rows/78locked entries.
- task validation: exit0,82implement/79check, original two oversized-document warnings retained.
- scoped git diff --check and new-test trailing-whitespace check: pass.

The outer first wrapper reported exit1 because its final git diff --no-index retained the expected
"files differ" exit code. All9 verification commands serialized exit0; preservation completed.
A separate non-build validation checked the results, exact main delta and preservation and exited0.
No failed test was hidden or rerun. No full Desktop, codecs, large fixture or performance run occurred.

## Preservation, decision and stop

Evidence: parent scratch ceebe351-d540-4ca3-a460-dadc71f3a7df/phase-c-spike-independent.ps1 and
phase-c-spike-independent/{results.json,component-tests.log,other command logs,baseline.json,
preservation.json,implementation-preservation.json,main-delta.txt}.

Before report edits,408/408 current baseline files remained unchanged. The implementation's403file
baseline independently matches402 unchanged files; only its documented ui-spec addition differs.
Dependencies/manifests/lock/vendor are unchanged. This acceptance adds only this report and updates
the current checkpoint; prior reports and source are not edited.

**Decision:** accept the spike and retain official Tree. The narrow proposal—official ListItem plus
public GPUI virtual-list for archive details rows only, one selection/focus/range owner, no changes
to other DataTable users or dependencies—has sufficient gap evidence to request approval. Approval
must preserve required columns/sort/resize/horizontal scrolling and later production selection/
identity/lifecycle regressions; none of those fallback behaviors was implemented or accepted here.

Wait for explicit user confirmation before fallback/browser implementation. Phase A/B outstanding
conditions and live screenshots/native input/DPI/UIA/user acceptance remain unchanged/NOT RUN.
No GUI calls/preview launch/safety bypass, native worker/system changes, packaging/install/signing,
commit/push or broad Phase C/D/E/F implementation occurred.
