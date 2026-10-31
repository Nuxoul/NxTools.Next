# Download default Enter / schedule Popover repair (#1 only)

## Scope
Repair audit item #1 only. The remaining twelve audit items are background, not execution authority. Preserve all pre-existing dirty changes. No commit, package, install, product/service launch, user configuration/database or recursive delegation.

## Behavior gap and owner
`NewDownloadView::new` intercepts Enter at the window before official focused controls. It can call `submit(StartNow)` while the schedule draft is open, or preempt a focused Button/Select. Keep `begin_probe` / `submit` as the only business owners; default Enter belongs only to the existing URL/directory/file-name single-line inputs when no child layer or IME owns the gesture.

Expected files: `downloads/new_download_view.rs`, directly associated `new_download_keyboard_tests.rs`, the existing Enter source gate and schedule lifecycle fixture in `downloads/tests.rs`, the append-only #1 reference entry in `docs/reference-code/download.md`, and this task's evidence. The date-dependent fixture scrolls its target into the existing viewport before clicking; it does not change production calendar behavior. No shared action helper, engine, protocol, native adapter, layout, theme or other audit-item repair.

## Component selection
Retain released Kit 0.8.0 Form/Field + Input, official Small Button footer, official Popover 304x200 and Small Select hour/minute. Existing parent composition, primary/outline/ghost variants and native client budgets are unchanged; no Toolbar.child adaptation or substitute controls. Existing custom calendar is not redesigned.

## Invariants / acceptance
- An active schedule/category layer and Select Enter cannot probe/start or submit unconfirmed/stale schedule draft.
- Official Button/Select/Popover remain the unique keyboard owner for their focus; Enter/Space invokes only the intended action. Clear/Confirm/outside/Escape retain draft/commit/close policy.
- Outer input default Enter still probes or submits through original owners. Modifiers, composing/character-preferred input, held/repeated down, busy and native confirmation gates do not mis-submit.
- Cross-owner effects remain deferred; no new logging, sensitive payloads, polling or native ownership. Existing probe/submission correlated diagnostics remain authoritative.
- Regression dispatches actual production keyboard events; direct submit calls are not keyboard evidence.

## Verification plan
Add production GPUI keyboard tests before the fix and run a focused red command. Apply the smallest scoped handler change and rerun focused tests, existing download interaction tests, rustfmt/static gates and constrained compiled checks when available. Every Rust invocation uses jobs=1, incremental=0, codegen-units=256; never duplicate Cargo. Source/compiled/synthetic/native/physical/DPI/user acceptance are reported separately.

## Execution status
Delivered for parent review; not accepted. Keyboard8/8, Downloads168/168, strictClippy/workspacecheck/fmt and scoped static gates pass. Single final fullDesktop1158passed/2failed in unrelated Archive/Extensions owners; recorded, not repaired or called green. No physicalIME/native/DPI/visual/user acceptance. Exact own changes, all failed runs and limits: outcomes.md. Stop here; no other audit item without a new single-item task.
