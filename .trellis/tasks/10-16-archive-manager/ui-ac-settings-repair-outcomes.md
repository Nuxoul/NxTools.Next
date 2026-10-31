# UI-AC01 / UI-AC02 — bounded settings repair outcomes

2026-10-25 user authorizes continuing the concrete UI acceptance repairs. This addresses ONLY
UI-AC01/02 from [negative acceptance](ui-correction-acceptance.md); it does not complete the five UI
goals, omitted settings, image comparison or native/user acceptance. [Sole checkpoint](phase-a-checkpoint.md)
owns current status. Historical negative report/probes remain unchanged. No package/install/system work.

## Repair contract and ownership

UI-AC01: rule textarea entities have a separate last-loaded/saved Vec<String> baseline. Unchanged
editors cannot finance a stale rule write, even when ordinary settings have adopted newer disk rules.
Only explicit Save Rules submits a changed draft. Under the original config lock, a changed draft
against different newer rules refuses overwrite and retains literal draft/disk values with a fixed
conflict diagnostic and actionable error. An official Small outlined `载入最新规则` Button appears only
for conflict; explicit click discards this rule draft and loads current disk rules. It does not write,
apply other preferences or alter task intent. Read failure retains draft. Clean-editor synchronization
is deferred through the original window, and checks baseline/current draft/current settings before
rebuilding stable textarea entities; user changes made before that turn are not discarded.

UI-AC02: original archive Settings and background remembered-default saves now use narrow
`save_settings`, wrapping original AppConfig::update_section. A process-local monotonic order is
allocated while its existing config lock is held, and returned with the merged snapshot ONLY when
save succeeds. No persisted field/schema, Core lock/API, IPC or runtime framework added. Both original
publication routes carry this opaque result; ArchiveApplication rejects order <= last published
before touching app/browser settings. An equal-value newer result still advances order. Browser
writeback remains deferred. This protects the same-process archive save/publish chain, not arbitrary
external process configuration watching or unrelated configuration writers.

Files: archive.rs exports, archive/application.rs save/result/publication owner,
archive/jobs.rs remembered-default adapter, main_window/archive_page.rs editor/persistence adapter;
matching job_tests.rs/archive_page_tests.rs regressions. No browser/decoder/native/selection/drag/
password/queue/accepted-job option changes, no dependency/vendor or unsafe input authority changes.
Original changed-field merge and exact strings/count/grammar/failed-save validation remain.

## Actual red -> green and checks

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\archive-ui-ac-repair-20261025-013448`.
All direct Cargo waited for the shared lane and inherited jobs1/incremental0/codegen256, locked/offline.
Other download builds were waited out, not terminated/overlapped; unrelated files were not repaired.

- 01-red.log: actual three regressions **0PASS/3FAIL**,1156filtered,exit101. Original settings buttons
  show clean rule overwrite and dirty draft overwrite; old publication-boundary result reverts runtime.
- 03-focused.log: **3PASS/0FAIL**,1156filtered,exit0 after repairs. UI-AC02 final regression strengthened
  from ordered publication facts to actual remember_accepted background save and completion callback:
  test-only bounded async channel pauses publication AFTER disk save, foreground commits/publishes
  newer settings through the real save wrapper, then release old result; disk/app/original browser
  retain newer settings. cfg(test) barrier is absent in production; no OS timing assertion.
- 05-scoped.log: **55PASS/0FAIL/0ignored**,1105filtered,20.55s,exit0. Original51 plus four repair cases:
  clean rules after ordinary switch, clean rules directly saved without switch, dirty conflict with
  explicit reload/re-edit/save recovery, actual delayed remember callback. Earlier focused cases
  overlap55 and are not extra unique tests. Original source/selection/lifecycle/forms/remember/rules
  validations remain in that scoped batch; no full Desktop/format/password/CLI matrix.
- 06-core-config.log: **12PASS/0FAIL/0ignored**,73filtered,exit0.
- 08-tests-strict.log: Desktop --tests strict Clippy **PASS**,exit0,-Dwarnings.
- 07-production-strict.log: **FAIL exit101**, ONLY existing unrelated
  tools/floating_assistant/media.rs62 `animated` unused field. No suppression/unrelated fix; not full
  production lint PASS. Earlier negative acceptance's external diagnostics remain immutable history.
- Scoped rustfmt/diff, spec114documents/14owners, component catalog79/78, current GPUI release and
  pinned Markdown provenance checks PASS. Final task/link/history/preservation checks recorded below.

Intermediate incorrect Edit positions briefly produced malformed export/struct syntax, corrected
before compilation; original logs/reviews retained. One Edit unseen anchor reveal retried unchanged;
no failed tool step is counted as a product red. No DEBUG production instrumentation or unsafe calls.

## Remaining scope and preservation

Original five-goal UI correction remains NOT ACCEPTED. Default-app URI, Shell double-click/individual
commands/grouping, recent history/configurable temp-root/retention/child-exit cleanup remain absent
or require their separately identified owner/native decision. Mandatory ZIP verification/source
retention/no-rollback safety unchanged. No image viewer retry, reference matching, physical splitter/
scrollbar gesture, IME/DPI/UIA/Explorer/COM/product installation acceptance was performed here.

Opening source/spec/report hashes saved in baseline.json; source-drift.json distinguishes six owned
Rust files from concurrent downloads/tests.rs,new_download_view.rs and main_window/extensions_settings.rs.
Historical reports remain untouched. Latest scoped code is compiled/tested, not a frozen whole
workspace claim. No artifacts, release ledger, installed/service/updater/protected-state inspection,
package/sign/install, Registry/COM/UNC/recycle/P209/dependency/vendor/elevation/commit/push.

Stop at **UI-AC01/02 bounded repairs implemented and tested, awaiting independent acceptance**;
not whole UI completion. One bounded read-only repair review and final static closeout are recorded
before delivery; no repeated broad review or automatic next debt.

## Final bounded review / documentation checks

One read-only repair review completed: no new confirmed defect in clean/dirty baseline, conflict
recovery or commit-order publication. Reviewer performed no execution; parent55UI/12Core/strict
results above remain actual verification. No second review cycle.
Final spec/task/link/whitespace/history/source-preservation results are retained in final-audit.json
and final-* logs under the evidence root. Test-source changes remain permanent regressions;
the old negative acceptance report and original checkpoint body remain byte-identical history.
