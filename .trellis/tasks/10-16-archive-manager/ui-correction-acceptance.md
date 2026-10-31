# Compression settings / archive browser UI correction — independent acceptance

2026-10-25. **Overall NOT ACCEPTED.** The five requested UI goals are not fully delivered or visually
accepted. Two concrete P2 settings-state defects were independently confirmed below. Existing51scoped
UI tests and12Core tests pass, but do not cover these interleavings or certify reference appearance.
Only [the checkpoint](phase-a-checkpoint.md) owns current status; prior reports and submitted delivery
history remain unchanged. This acceptance makes no product fix or phase/release completion claim.

## Confirmed findings

### UI-AC01 — untouched exclusion editors overwrite newer persisted rules (P2)

Owners: [settings save](../../../rust/apps/nxtools-desktop/src/main_window/archive_page.rs),
`ArchiveSettingsView::save` lines105–116 and exclusion Save callback lines284–294.
An open Settings view holds the original textarea rows. Another owner saves newer exclude_patterns;
saving an unrelated switch correctly merges disk and replaces self.settings with the newer rules,
but leaves those textareas unchanged. Clicking Save Exclusion Rules without editing then collects
old rows and writes them over the newer rules. This is silent loss of a newer setting, not a malformed
config or cross-section lock failure.

Independent production-view repro: seed defaults; mount original MainWindow/Settings; use original
AppConfig::update_section to represent the create-form's newer rules `["new-form-rule.tmp"]`; click
actual `archive-open-in-nxtools`; assert merged view settings already contain newer rules; navigate
original compression page and click actual `archive-exclusions-save` with no row edits. Result:
expected newer rules, actual `["Thumbs.db","desktop.ini"]`. Actual test FAIL, not a speculative review.
The first disk update injects only the other owner's already committed facts; actual original settings
buttons, persistence and editor collection reproduce the overwrite.

Required bounded repair: distinguish clean editor baseline from intentional dirty rule drafts. Adopt
newer merged rules in clean editors, and preserve/resolve dirty draft conflicts without silent stale
replacement. Add this exact original-owner regression. Do not replace atomic section persistence or
reintroduce JSON input; retain literal spaces/semicolon/newline and failed draft behavior.

### UI-AC02 — a delayed remember-save result rolls runtime preferences back (P2)

Owners: [background remember completion](../../../rust/apps/nxtools-desktop/src/archive/jobs.rs)
`remember_accepted` lines75–108 and [settings publication](../../../rust/apps/nxtools-desktop/src/archive/application.rs)
`apply_saved_settings` lines130–146. Background update returns full snapshot A. If foreground settings
commit/publish newer B before A's UI completion runs, A is unconditionally published; there is no
freshness/revision check. Runtime defaults and browser preferences revert to A while disk retains B.
Deferral prevents entity reentry, but does not establish save-result ordering.

Independent controlled publication-boundary repro: original AppConfig updates commit A then B;
original app.apply_saved_settings publishes B; deliver A at the exact function used by remember
completion. Disk remains B; app.settings changes show_tree=false -> true and single_click_open=true
-> false. Test FAIL. **This injects the delayed completion order, not an end-to-end scheduler race
reproduction.** Current async call chain demonstrates why that order is possible; evidence is not
claimed as actual user/OS timing or a disk rollback.

Required bounded repair: publish only still-current settings results at the owning save/publication
boundary; retain deferred cross-entity updates and changed-field merge. Add a deterministic regression
at remember-completion/foreground-save interleaving, preserving newer disk and runtime/browser state.
No generic settings runtime, speculative retry loop or whole-app revision redesign is requested.

## Five-goal disposition

| Goal | Independently substantiated | Still not accepted |
| --- | --- | --- |
| Settings | Real persisted browser/completion preferences; row-wise rules; original changed-field saves and ordinary validation/failure tests | UI-AC01/02; missing default-app entry, Shell action/per-command/group config, recent history and configurable temp lifecycle; reference completeness/visuals |
| File icons | Name column mounts16DIP folder/file/parent icons using existing assets, no archive-content/Shell IO | Reference-specific appearance, actual asset rendering/readability and user approval; generic file icon is not every extension's Windows icon |
| Tree/split | Depth/disclosure/folder slots, deferred reveal and fixed/flexible split geometry at960x640/720x480; real tests pass | Long-name visual result and physical splitter/keyboard/native UI acceptance |
| Icon toolbar | Official icon-over-label Buttons/DropdownButton, menu separation, h_auto parent; bounds/theme/minimum/disabled assertions pass | Original PNG reference match, physical action/menu/keyboard integration and user visual approval |
| Scroll axes | Original horizontal/vertical handles restricted to their axes; actual simulation covers middle/top/bottom/header/bar areas and explicit horizontal input | Physical mouse/trackpad and horizontal scrollbar thumb drag NOT RUN; tests that directly set offsets are not drag evidence |

Mandatory precommit ZIP modification verification remains, not a disabling setting. Source recycle,
rollback/share-delete/locked-file permission options remain unopened. These are correctly reported
safety limits, not authorization to silently waive the requested settings gaps. `ms-settings:` is
rejected by existing Shell allowlist; any new URI route needs a narrow native decision, no fake UI.

## Independent execution and evidence levels

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\archive-ui-independent-20261025-010008`.
Runner `ui-acceptance-run.ps1` in session scratch, commands.jsonl and raw logs retain actual exits.
Every Cargo waited for one shared lane and inherited jobs1/incremental0/codegen256, locked/offline.
Unrelated new_download_keyboard build was waited out, not terminated or overlapped.

- Fresh source build and original same51scoped UI tests: **51PASS/0FAIL/0ignored**,1102filtered,
  17.91s execution, exit0 (01-ui-scoped.log). Names equal submitted51, not older binary reuse.
- Core archive configuration: **12PASS/0FAIL/0ignored**,73filtered,exit0 (02-core-config.log).
- Core strict Clippy tests:PASS. Desktop strict Clippy tests:**FAIL exit101**,four external diagnostics:
  downloads/new_download_keyboard_tests.rs3 unused import; new_download_view.rs592–593 two
  nonminimal_bool diagnostics; main_window/extensions_settings.rs704 collapsible_if. None in the15
  archive UI files. These observed diagnostics differ from submitted historical three; no global
  strict PASS or unrelated repair/allow. Concurrent owners may change them later; no green hunting.
- One bounded read-only reviewer found UI-AC01/02; parent inspected and ran the two bounded probes.
  **0PASS/2FAIL/0ignored**,1153filtered,exit101 (06-two-findings-probe.log). Probe2 is fact-injected
  publication ordering as labeled above, not a full async schedule. No repeated review cycle.
- Both temporary probes were appended only to original test modules to access real owners, copied
  to scratch `ac01-probe-source.rs`/`ac02-probe-source.rs`, executed, then removed with exact Edit.
  Final two test hashes match original submitted bytes; all15submitted source hashes match.
  No product source fix or retained project-test change was made during this acceptance. Normal
  target test executable may still contain the failed probes until a later build; not a delivered app.
- Scoped rustfmt/diff, spec114documents/14owners, historical catalog79/78,current GPUI release and
  pinned Markdown provenance gates PASS; task82implement/79check with existing size warnings retained.

Historical submitted red-ui-6.log was read:2PASS/2FAIL, actual horizontal-64->0 and menu/toolbar overlap.
Final locks/h_auto are public original-owner adaptations, not vendor edits/global input swallowing.
Current independent tests substantiate the scoped fixes, not every visual/gesture state.

**Visual blocker remains:** seven original PNG files are retained; static reference page loads images,
but this acceptance's single screenshot attempt again timed out. No inspectable current-product
capture/reference comparison. Do not infer visual PASS from loaded images, OCR, imports or geometry;
no honest visual diagnostic score can be assigned. No self-app GUI restriction bypass or formal
product/installed-window interaction was attempted. Native/DPI/IME/UIA/user acceptance remains NOT RUN.

## Preservation and stop

Opening2158scoped source/spec/history files captured; before documentation writes six concurrent
nonarchive paths changed: .trellis/spec/nxtools-extensions/backend/quality-guidelines.md,
.trellis/spec/platformhost/native-contracts.md, .trellis/spec/nxtools-protocol/backend/directory-structure.md,
downloads/new_download_keyboard_tests.rs, downloads/new_download_view.rs, clipboard_worker.rs (Rust
paths under rust/apps/nxtools-desktop/src). All retained; no cumulativeHEADdiff used as exact round
attribution or frozen-workspace claim. All15owned submitted source hashes match after probe restoration.
Final documentation/preservation audit records actual missing/drift/links and retains checkpoint body.

Acceptance writes only this additive report, sole checkpoint and scratch. No package/install, real
Registry/COM/Shell apply, service/updater/protected deployment, UNC/recycle/P209/dependency/vendor,
commit/push or full Desktop/password/codec/CLI/resource matrix. **STOP: partial source/compiled fixes
substantiated, overall UI correction NOT ACCEPTED pending UI-AC01/02 repairs, declared settings gaps,
visual reference comparison and applicable interaction acceptance.**
