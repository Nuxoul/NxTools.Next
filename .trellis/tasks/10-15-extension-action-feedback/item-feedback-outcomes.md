# Item-local Clipboard feedback — source handoff

## Current authority and delivered change
The user explicitly rejects processing feedback above Clipboard history. [item-feedback.md](item-feedback.md) supersedes that placement in ux-refinement.md; the earlier rejected galleries and outcomes remain historical, not visual acceptance.

- The context-menu invocation captures the immutable source history item ID in ClipboardOrigin. ClipboardPanel retains bounded item-keyed projections, not a panel-top banner. Feedback is a lower content slot INSIDE the original official ListItem; original text/metadata remain.
- The compact slot has a readable status/indicator row and a separate action row. Pending work uses released Spinner::new().small() only when visible and motion is allowed. Unknown/reduced preference and terminal states use static official icons; no fake percentage, height animation, new timer or frame logging. No completion fade was implemented.
- Official outlined text actions retain default32px; result paging is deliberately Small24 and icon-only dismiss XSmall24. Actions are not children of Toolbar, so no forced ghost conversion. Full action name is available through the existing official Tooltip on the status row; Details remains explicit. Existing Default Light/Dark/CJK fonts are unchanged.
- Cancel, View Result, result paging, Details and dismiss stay within the source item and stop pointer/click propagation. No automatic report Dialog/window, copy or paste. Multiple source items are independent; a newer invocation for the same item may replace that projection.
- Original ListState::remeasure_items handles expansion/collapse without replacing the list controller. Stable IDs retain the feedback through selection/reordering/filtering; a filtered source does not move its feedback to another row. Exact-origin worker result navigation, pending/terminal/effects/trust/runtime/native-menu owners remain unchanged.
- No C#/IPC/native API, engine, store or example-package changes belong to this item-local slice. Existing low-frequency manual/result diagnostics are reused; no payload or animation logging.

## Behavioral red/green and resolved findings
1. Initial placement regression failed against the old panel-top feedback: feedback y93 versus the source item y247. The current-frame regression now checks feedback inside the second actual history item and below the first item, with no automatic Dialog.
2. Visibility lookup from child render caused a real RefCell already mutably borrowed panic while the virtual list measured rows. The final implementation uses prepaint bounds intersected with the content mask/viewport and deferred notify only on visibility changes, not ListState reads during measurement.
3. Added long action-name /64 host-result /Running-Cancelling-Succeeded control-containment checks at300x440. Although controls initially fit, the new status-width regression failed at31px. Status now has its own upper row, with >=200px width in that constrained fixture; buttons occupy the lower row. Pending/cancelling partial results are explicitly labelled as still processing/cancelling, not falsely completed.
4. Actual click/frame regressions verify: per-item cancellation does not cancel the other source or start paste; dismiss leaves the other item intact; selection/reorder/filter/restore retain source identity; visible pending work requests animation frames; live reduced motion and terminal rows stop repeating frames after settling.
5. Test authoring ElementId signed-ID mismatch and strict-Clippy unwrap failures were corrected; these are not claimed as behavioral-red evidence. The pre-existing production-only unused ClipboardApplication::extension_adapter warning remains unsuppressed.

## Final checks actually performed
All Cargo commands were serialized with Rust1.96, CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256.

| Check | Result / scratch log |
| --- | --- |
| Full Desktop after final presentation change |759 passed,0 failed/ignored,89.99s; item-feedback-handoff-desktop.log |
| Final expanded item interaction after test-only unwrap->expect correction |1 passed,758 filtered,0.87s; item-feedback-handoff-interaction.log |
| Strict Desktop tests Clippy -D warnings |PASS; item-feedback-handoff-strict.log |
| Workspace --tests check |PASS; item-feedback-handoff-workspace.log |
| fmt --all -- --check |PASS; item-feedback-handoff-fmt.log |
| Final Debug preview build |PASS, existing unused-adapter warning only; item-feedback-handoff-preview-build.log |
| Clipboard/UI-state/preview-isolation/Desktop-shell lifecycle source gates |4 PASS; item-feedback-handoff-gates.log |
| Local spec/catalog contracts |94 documents/13 owners and79 pages/78 locked mappings PASS; documentation/source consistency only |

The full suite preceded only the two test-only unwrap->expect substitutions and formatting; subsequent strict/workspace/focused tests cover that exact final test source. No product behavior changed after the full suite.

## Current actual rendering artifacts
[Gallery](../../../artifacts/clipboard-item-feedback-ui/index.html) and evidence.json contain exactly4 owned synthetic GPUI PNGs: Default Light/Dark x running/saved,300x440,96DPI. The processed source is the second synthetic history record, not a panel-top region. Captures were regenerated from the final Debug presentation, checked against owned PID/HWND/client bounds and nonblank sample colors77–85; item-feedback-handoff-capture.log. PNG hashes/dimensions and scoped UTF8/whitespace/JSON/context links are checked at handoff.

**Direct viewing remains blocked:** Browser screenshot returned "Browser view did not become visible for the calling session. Switch to that session and retry." These images were generated and identity/nonblank checked, not directly visually inspected. Static PNGs cannot prove animation; actual frame tests above are separate. No aesthetic/user acceptance is claimed.

## Remaining limits and non-actions
No normal product startup, real plugin/user Clipboard/history/credential operation, installation, packaging, deployment, service mutation, commit/push or archive. Foreign-process installed native menu, physical gestures, mixed DPI/workarea edges, live Windows motion toggle and reader/full-product acceptance remain unperformed. Sequence9 installer remains unchanged and does not contain the later native-menu or item-local refinements. No new package authority is implied. This is a source/compiled handoff ready for user review, not user visual acceptance.

## Subsequent package authorization
After clarification that feedback is appended below original content within the enlarged item, the user requested packaging the current implementation to inspect. [Sequence10 package](release-package-sequence10.md) now includes this and the native-menu/contextual slice; it supersedes the preceding no-packaging/unchanged-sequence9 checkpoint. No further layout change or automatic installation occurred. User visual acceptance remains pending.
