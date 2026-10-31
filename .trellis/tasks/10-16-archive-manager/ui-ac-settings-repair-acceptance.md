# UI-AC01 / UI-AC02 — independent repair acceptance

2026-10-25. **UI-AC01 CLOSED; UI-AC02 CLOSED** at the submitted source and isolated production-view
boundaries. **两项设置缺陷已关闭，整体UI整改仍未完成。** Only
[the checkpoint](phase-a-checkpoint.md) owns current status. The original
[negative acceptance](ui-correction-acceptance.md) and [repair outcomes](ui-ac-settings-repair-outcomes.md)
remain byte-identical; this report does not rewrite either or accept the overall five-goal UI.

## Independent source findings

- **UI-AC01:** [ArchiveSettingsView](../../../rust/apps/nxtools-desktop/src/main_window/archive_page.rs)
  retains an editor baseline separate from its merged settings snapshot. An unchanged draft is not
  submitted, whether Save Rules follows an ordinary switch or is invoked directly. Changed rules
  compare baseline/latest/draft inside the original config lock. Conflicts leave external rules and
  literal local draft intact, show the actual error and official Small outlined `载入最新规则` action.
  Its original deferred callback explicitly loads current rules without saving other preferences;
  only that action discards a conflicting draft. Clean-editor deferred refresh rechecks baseline,
  current editor contents and merged settings before replacement. No new confirmed in-scope defect.
- **UI-AC02:** [save_settings / publish_saved_settings](../../../rust/apps/nxtools-desktop/src/archive/application.rs)
  wraps the original Core update_section. Core takes the config lock before mutation and retains it
  through normalize/save_to; the process-local order is obtained within that mutation callback, not
  at request launch. Failed saves return no SavedArchiveSettings (`?` precedes construction); gaps
  in the sequence do not publish failed results. The private opaque result is used by Settings and
  [remember_accepted](../../../rust/apps/nxtools-desktop/src/archive/jobs.rs). Publication rejects
  order <= last published before changing app/browser state; equal-value newer saves still advance
  the watermark. Cross-entity publication remains deferred. No Core schema/lock/API, accepted-job
  options, IPC, C#, selection/drag, security, dependency or vendor changes were made here.

## Actual independent execution

Built/checked the current source with `cargo test -p nxtools-desktop --locked --offline --no-run
--message-format=json`; Cargo exit0 returned the current test artifact (cache hit, not claimed as
forced rebuild). Independently enumerated names before selecting tests. Every Cargo command checked
shared channel/process state and inherited jobs1/incremental0/`-C codegen-units=256`; no pause,
reservation or termination of other sessions/processes was requested.

| Exact repair test | Result |
| --- | --- |
| main_window::archive_page_tests::ui_ac01_untouched_rules_preserve_newer_form_rules | PASS |
| main_window::archive_page_tests::ui_ac01_clean_rule_save_without_other_switch_adopts_newer_rules | PASS |
| main_window::archive_page_tests::ui_ac01_dirty_rules_conflict_preserves_draft_and_external_rules | PASS |
| archive::application::jobs::tests::ui_ac02_late_remember_snapshot_preserves_newer_settings | PASS |

Four exact tests: **4PASS/0FAIL/0ignored**,1188filtered,1.94s,exit0. UI-AC02 uses real background
remember_accepted save, the cfg(test) bounded async barrier after disk save, a newer foreground
save/publication, then old callback release. Disk, application defaults and original browser retain
new values. This is deterministic scheduler control, not an arbitrary physical/OS timing claim or
comparison-helper substitute. UI-AC01 uses the real Settings buttons and explicit reload/re-edit/save.

Original relevant compression UI collection, actual selected/ran names: **55PASS/0FAIL/0ignored**,
1137filtered,21.06s,exit0. The four repair tests overlap these55; not59 unique cases. Existing literal
spaces/semicolon/newline, add/delete, invalid rules, persistence failure/draft retention, configuration
merge/reopen, browser and deferred-publication checks remain green. No new test kind/probe added.
Core archive configuration: **12PASS/0FAIL/0ignored**,73filtered,0.14s,exit0.

Strict Core tests Clippy and strict Desktop --tests Clippy: **PASS**,exit0,`-D warnings`.
Strict Desktop production Clippy: **FAIL exit101**, ONLY existing unrelated
`tools/floating_assistant/media.rs:62` unused `DecodedMedia::animated`. No unrelated fix, allow/expect,
repeated green-hunting run, or claim of full production strict PASS.
Scoped rustfmt/diff, spec114documents/14owners, catalog79/78, GPUI release and pinned Markdown
provenance: PASS. Task82implement/79check PASS with8retained context-size warnings.

## Evidence / preservation / non-acceptance

Evidence root: `C:\Users\Administrator\.pi-desktop\scratch\b83f39e9-16ef-4c5d-b436-4c52a8fda443\ui-ac-independent-20261025-075531`.
`commands.jsonl`, Cargo artifact JSONL/hash, exact selected names, raw01–10logs and final audit are
retained there. Executed artifact SHA256:
`E12D62B5666AEE49DE58EFF398E23E69DCB526FC46C064693B69C981D1317A76`.
Shared target was later replaced by another build; later target/hash is not this run's evidence.

Opening608source/spec/task/reference files were hashed before acceptance writes. All six submitted
repair files and the four permanent regression sources remained unchanged. Pre-document drift in
archive/application_tests.rs (other retained tests), download_page/kit_migration_tests.rs and
reference-code/download.md was concurrent and preserved; no frozen-workspace or cumulativeHEADdiff
attribution. This acceptance writes only this additive report, the sole checkpoint and scratch.
Final audit checks original negative/outcome reports, retained checkpoint history, links, names,
format/diff and opening-baseline drift. No source/test/product fix or further reviewer was needed.

NOT newly completed: Windows default-app entry; Shell double-click/per-command/group settings;
recent archive history; configurable temp root/retention/exit cleanup; original-image match;
physical splitter/horizontal thumb drag; DPI/IME/UIA/Explorer/user acceptance. No image tool retry,
formal product launch, full Desktop/password/format/CLI/resource matrix, package/install/sign,
service/updater/protected-directory/release-ledger query, Registry/COM/UNC/source recycle, commit/push.

**STOP: UI-AC01／02修复独立验收完成。两项设置缺陷已关闭，整体UI整改仍未完成。**
