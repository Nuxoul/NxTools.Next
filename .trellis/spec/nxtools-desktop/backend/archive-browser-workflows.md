# Archive browser, Find and atomic intake

## 1. Scope / Trigger
OPT-07 Up, whole-archive Find, catalog replacement, preview/readiness rendering and New/Add folder input. Read [navigation reference](../../../../docs/reference-code/archive-browser-workflows.md), [picker reference](../../../../docs/reference-code/archive-picker.md) and [batch contract](archive-batches.md) first. Evidence: `repo:rust/apps/nxtools-desktop/src/archive/browser_find.rs`, `repo:rust/apps/nxtools-desktop/src/archive/model.rs`, `repo:rust/apps/nxtools-desktop/src/archive/browser.rs`, `repo:rust/apps/nxtools-desktop/src/archive/forms_intake.rs`. No new archive runtime, selection controller or global cache.

## 2. Signatures
- `BrowserModel::up() -> bool`; `folders() -> &[String]` is a model-local immutable-catalog `OnceCell` projection.
- `ArchiveBrowser::{start_find,invalidate_find,find_action}`; browser-owned `FindView` retains browser generation/snapshot/Find epoch/query generation and original `RowId`.
- `MemberQuery::new(query:&str,wildcard:bool) -> Result<MemberQuery,MemberQueryError>` / `matches(&str)->bool` lives in `repo:rust/crates/nxtools-archive/src/member_query.rs`, reusing locked globset. No archive I/O.
- Original `submit_form_admitted` validates selected Extract's frozen `ArchiveBrowserContext` against the registered WindowSlot, Ready model and original loaded read connection (`preview_connection()`). Modify retains `EditFormOrigin`.
- `valid_source_roots` validates the combined existing/incoming roots; original `accept_sources`, pending picker and asynchronous metadata publication remain sole intake owners.

## 3. Contracts
Up at root returns false without changing filter/selection/focus. Otherwise project parent, select/focus/anchor exactly the departed child and defer nearest reveal under the original snapshot/projection epoch. Replacement/encoding restores the old folder or nearest surviving ancestor, clears old row IDs and preserves initial-open-only single-root entry and committed-edit restoration precedence. Both deferred Tree install and projection publication read the latest same-snapshot model folder/focus, not a captured old folder.

Ctrl+Shift+F/View menu searches the whole loaded immutable catalog; Ctrl+F stays current-folder filtering. Empty query matches all; nonwildcard is case-insensitive full-path substring. Wildcard without separator matches basename; with separator matches full normalized path, using literal separators and case-insensitive globset. Normalize backslash to slash; reject query UTF-8 length>1024 or any control character, and malformed glob, with fixed `MemberQueryError::Invalid`. No query/path Debug or logs.

Keep `RowId::Entry(snapshot,original_ordinal)` and `RowId::Folder(snapshot,path)` for explicit/implicit/empty directories; noncontiguous ordinals are not Vec offsets. Query/results selection does not mutate browser selection until explicit action. Result element ID includes query generation; callbacks compare that generation AND original RowId, so a pressed old row cannot select its replacement at the same index. Deferred actions recheck query generation/selection, Find epoch, browser generation, snapshot, Ready and retirement. Navigate may locate a warning row; Extract/Delete retain original safety/capability checks and original forms. Final submission revalidates original browser/read connection without retargeting to the user's later selection. Native source authority remains final.

Resolve the original identity, safe internal path and warning-action eligibility before `window.close_dialog` or `BrowserModel::enter`. Find and ordinary rows share `model::entry_warning`; explicit directories retain their original projected warning policy. Refused warning Extract/Delete leaves the same Find query/input/result and browser folder/filter/selection/focus/anchor/projection untouched, with zero form/Job/command. Navigate remains allowed for the same warning identity. Never mutate a temporary browse projection to discover eligibility.

Original Root Dialog uses `.child(find)` and `.footer(DialogFooter)`, stable InputState in Form/Field/InputGroup, default-unchecked `Checkbox::on_click`, official 32px ListItem/public uniform_list and bounded viewport. Direct Medium outlined Navigate/Extract/Delete/Close buttons stay in the fixed footer, not scroll content or Toolbar.child. Close/Navigate restores original list focus only for current live Find epoch; old close cannot steal focus from another tab. Cross-entity writes occur next turn, including initial catalog access and cached actions.

Catalog/folder/readiness/preview visibility work belongs to model install or selection/summary/capability/projection/fingerprint transitions. Render reads cached presentation facts in O(1), not full selection/catalog expansion. Final commands still perform authoritative validation. Preserve preview fingerprint/generation, per-consumer leases, Seal-before-Ready, independent encrypted-drag authorization, transferred-stream survival and actual cleanup. Test-only work counts distinguish catalog visits, normalization, folder builds, projections and rendered controls from I/O/worker starts; no production render/frame logs.

Native folder picker follows [PlatformHost ownership](../../platformhost/archive-workflows.md). Register pending before send, one per form/max8/7s, and compare original operation/session/generation/connection after deferred delivery. Known missing `archive_multi_folder_picker_v1` refuses locally before pending/send. Whole New vector including existing roots<=32; Add<=31 external roots plus original archive path. Aggregate source UTF-8<=131072 including existing roots and Add archive path. Reject invalid paths, case-insensitive duplicates/component-ended parent-child overlap and output self-inclusion atomically. `a` and `ab` are not overlap. Recheck merged roots at asynchronous New publication and frozen Add source/folder/snapshot/connection before append/submit. No recursive/render filesystem reads; Remove is draft-only.

## 4. Validation & Error Matrix
| Condition | Result |
| --- | --- |
| Empty / valid query / invalid glob | All / current results / explicit invalid state, no Job/decoder |
| Old query pointer release, refresh, encoding, hidden tab, close or disconnect | No stale selection/action/final submission |
| Valid Find Extract/Delete | One original form/Job, original snapshot and ordinal |
| Warning Navigate / warning Extract or Delete | Locate exact row / refuse before closing Find or changing any browse/projection state; zero submission |
| Root Up / missing replacement folder | No-op / nearest surviving ancestor |
| Incoming individually legal but combined overflow/overlap | Whole refusal; original draft and sources unchanged |
| Cancel/native failure/late picker/old connection | Original retryable form or retired result; no partial append/admission |

## 5. Good/Base/Bad Cases
Good: duplicate filenames at different paths keep distinct noncontiguous ordinals; Find selects one only after Navigate. Base: current-folder Ctrl+F and ordinary New/Add file multiselect/text Paste remain. Bad: use result index as ordinal, reuse old query callback, derive edit readiness every frame, or append the first31 roots from an over-budget vector.

## 6. Tests Required
`browser_find.rs` exercises actual query replacement between pointer down/up, deferred lifecycle retirement and 10000-entry virtual viewport; `browser_tests.rs` covers original identities/directories/warning navigation, Up/folder restoration and repeated-draw catalog work. `application_tests.rs` covers valid/stale Find Extract/Delete final-submit paths; original OPT-03 Add/Step4 refresh remains green. `intake_tests.rs` covers New2/16/32, Add31, combined count/UTF-8/overlap rejection, cancel/error/late connection, Remove/re-add and no Job/source mutation. The shared matcher unit asserts empty/substring/basename/full-path glob, separators/control/length/error cases. Controlled native picker tests are not a real interactive OS multiselect or physical focus/DPI acceptance.
`application_tests.rs::{opt07_ac01_find_warning_extract_preserves_dialog_and_browser,opt07_ac01_find_warning_delete_preserves_dialog_and_browser}` use the actual original application/Root/Find footer from another folder with a nonempty local filter and selected/focused/anchored member. Assert unchanged model/visible identities/projection work, retained Dialog/query/result, zero form/Job and zero footer dispatch; keep the warning Navigate and valid/stale original form tests.

## 7. Wrong vs Correct
Wrong: old row index -> new query result -> current browser selection -> unguarded submit. Correct: query generation + original RowId -> deferred live originating snapshot -> original action/form -> frozen final-submit check -> native source authority. Wrong: render scans all entries. Correct: transition-owned presentation cache plus authoritative final command validation.

## Scenario: explicit incoming Add and transient failure feedback
### 1. Scope / Trigger
The incoming-dialog choice “添加到当前压缩包” directly starts Add. This narrow entry override
does not change deliberate Modify/Paste forms, Open-first or New-from-all.
### 2. Signatures
`application::DropAddTarget { request: ArchiveJobRequest, origin: jobs::EditFormOrigin }`
captures the original `ModifyDraft::request`; `submit_drop_add(request,origin,cx)` uses the
existing queue. `edit_request_current(request,origin,cx)` is shared with final form admission.
### 3. Contracts
Freeze original archive/window/session/generation/snapshot/source/read connection, folder and
the complete source vector at drop. Take the dialog intent once, close it, then defer the
cross-entity work. No hidden form or simulated submission. Install origin/connection before
pumping; recheck active tab and current origin at the deferred choice AND native dispatch.
Keep original conflict/password tickets, cancellation, refresh and new-snapshot result selection.
`incoming_notice` and `notify_job` feed the existing NotificationCenter: operation title,
friendly fixed cause, terminal dedup, original browser target and original screen presenter.
Keep concise status, diagnostics disclosure, cleanup/partial/unknown-publication risks; generic
transient failure prose is not a second progress/form Alert. Only affirmative native
`output_published=false` permits failed Add form recovery; zero committed files is insufficient.
Native journal/publication/commit-state facts determine replacement wording. No new timer.
`archive.drop_add.refused` logs only the browser ID before admission; original Job events carry
the operation ID across IPC/native accepted/start/terminal, elapsed time and native error codes.
### 4. Validation & Error Matrix
Closed/navigated/replaced/reconnected origin -> no native Start; queued stale Add ->
`stale_add_target`. Password/data ambiguity remains ambiguity, not guessed WrongPassword.
Native published/unknown -> retain actual risk; affirmative unpublished Add -> retained draft.
### 5. Good/Base/Bad Cases
Good: rendered explicit Add submits one complete request without a form. Base: deliberate
Add retains picker/source draft. Bad: infer unpublished from `files_committed=0` or retarget a
queued drop to the current tab/folder.
### 6. Tests Required
`repo:rust/apps/nxtools-desktop/src/archive/incoming_add_tests.rs` covers the actual button,
one request/no form, complete sources, six stale bindings, closed origin, queue-time refusal,
retained deliberate draft and one business notice through original hidden prepare/frame/reveal
while the main window is hidden. `incoming_add_native_request_fixture` is explicitly executed
by the effects-reviewed C# incoming fixture with real browse input and the unchanged exported
typed command; its default ignored run is not native acceptance. Preserve original notification,
refresh/result-selection and true-progress tests.
### 7. Wrong vs Correct
Wrong: auto-submit a hidden editor, or stop at NotificationCenter.push. Correct: frozen direct
intent -> original queue/native transaction -> original business notice -> existing passive
screen prepare -> first frame -> reveal acknowledgement.
