# REF-20 — Create UI and completion

Status: **accepted at compiled production TestWindow/TestPlatform boundaries**. All seven new UI/completion selections and the selected-output quota control independently pass; [coordinator acceptance](coordinator-acceptance.md) records the complete run. Physical HWND/IME/DPI/user visual acceptance remains separate.

## Original owners

`CreateDraft` captures a default-false, one-off `self_extracting` choice in the existing immutable `ArchiveJobCreate`. It is not an archive-format enum value or a saved settings field. Enabling generates `.exe`, disabling restores `.zip`, switching away from ZIP clears the choice and generates the selected format extension. Separate-input requests retain the same captured intent and generate individual `.exe` names.

`OperationForm::self_extracting` updates only local draft state in the Checkbox callback. Name InputState synchronization and original revalidation run in the next application turn. The existing InputState/Select entities, Root, scrolling body, fixed footer, validation, busy state, default Enter, close and retirement owners remain in place. Entered passwords remain intact and produce an explicit incompatibility message while SFX is enabled.

The existing `ArchiveApplication::finish_attempt` owns post-create Test. It uses the committed `result_location.target`: ordinary SFX's EXE or a split family's final ZIP. This is a Test request, never an executable launch. Post-Test failure, cancellation or incomplete cleanup retains the original publication/journal/location facts; incomplete cleanup keeps the original running reservation.

## Released component selection

The format/level group remains `Form::vertical().with_size(Size::Small).columns(2)`. Its ZIP-only full-width `Field::new().col_span(2).visible(...).description(...)` contains the released `Checkbox::new("archive-self-extracting").label(...).with_size(Size::Small).checked(...).disabled(...).on_change(...)`. The visible label is `创建自解压 ZIP（EXE）`; the description identifies Windows x64, Store/Deflate, password incompatibility and the complete-prefix split requirement.

Form/Field supply the grid and description slots; Checkbox/Base owns pointer activation, Tab focus and keyboard Click. Kit/pre0.3.9 activates keyboard Click on KeyUp. There is no clickable parent, replacement key dispatcher, painted substitute, vendor patch or new dialog controller. The original Medium32 primary/outline footer remains fixed outside the scrolling body.

## Production boundary coverage

- Real pointer toggles produce `.exe`/`.zip`, retain original entities and freeze an immutable request independent of later toggles.
- Actual Tab reaches the official Checkbox. First Space and complete Enter presses toggle it. Held Enter repeats before release produce one activation and never submit the outer form.
- Actual format Select Enter/Down/Enter changes to7z, clears SFX, updates the existing name input and hides the ZIP-only control. Separate-input requests have individual names and immutable intent.
- Existing password InputStates keep both values; enabled SFX shows the explicit password conflict and disables Create. Disabling SFX restores ordinary encrypted-ZIP eligibility.
- Busy state rejects pointer/keyboard changes. Actual Escape and titlebar pointer close retire the original form without submission.
- Two JobCenter/TestPlatform selections cover ordinary/split committed targets across Test success/failure/cancellation, with both complete and incomplete cleanup:12terminal scenarios. They preserve publication/location/journal facts and forbid automatic executable/member launch or unrequested folder opening.

The completion scenarios use synthesized terminal events on the production queue; they are not additional native publication evidence. Actual Create/Job/native output and recipient execution remain separately qualified in [creation/publication](ref20-creation-publication.md) and [native execution](ref20-native-execution.md).

## Evidence

All selections use nonzero LIST followed by the same-filter RUN, with raw commands, required Cargo environment, source copies, counts and exits retained in session scratch.

| Receipt | Result |
| --- | --- |
| `b7-ref20-desktop-ui-red.results.json` | LIST5/RUN5,0PASS/5FAIL at the absent SFX control |
| `b7-ref20-desktop-ui-completion-green.results.json` | LIST7/RUN7,5PASS/2FAIL; the five successful selections are retained |
| `b7-ref20-desktop-ui-busy-green2.results.json` | Only the remaining close/busy selection:LIST1/RUN1/PASS1 |
| `b7-ref20-desktop-ui-keyboard-green3.results.json` | Only the remaining keyboard selection:LIST1/RUN1/PASS1 |
| `b7-ref20-product-selected-quota-output-only.results.json` | LIST1/RUN1/PASS1 across all four qualified product forms;34-byte member credit and33-byte total credit refuse with no publication and actual release |

Seven unique UI/completion successes are composed5+1+1, not a single seven-PASS run. The initial absence RED precedes implementation; two test functions subsequently align their fixture premises with official Tab/KeyUp and removed-window lifetime. Whole-file identical-byte RED/GREEN is not asserted. Product draft/form/layout bytes are unchanged across the failed-only resumptions; `jobs.rs` requires no completion change.

Component/release/editor/spec source gates and final common-source preservation/quality checks are complete; see [verification](verification.md). REF-20 raw evidence is frozen in4070files excluding its manifest. Physical HWND, IME/DPI, user visual approval, installed/Release and independent acceptance remain separate.

## Scoped paths

Existing product paths: `rust/apps/nxtools-desktop/src/archive/draft.rs`, `forms.rs`, `forms_layout.rs`. Existing `forms_tests.rs` and `job_tests.rs` add only child-module declarations for these tests. New task-owned paths: `forms_ref20_tests.rs`, `jobs_ref20_tests.rs` and this outcome record. C# `ArchiveSfxProductTests.cs` derives the isolated negative quota from the qualified expected bytes; the earlier dual-insufficient-credit ledger remains unchanged. Original B7 baseline/before-copies, REF-19 frozen evidence and B1–B6 records remain authoritative.
