# OPT-03 — O3-AC01/02/03 focused repair

## Result and scope

**All three directed findings pass their affected automated boundaries.** O3-AC01 is repaired in the existing edit-form origin. O3-AC02/03 now reach schema-valid business terminal handling and assert the required lifecycle facts. The original [independent acceptance](opt03-independent-acceptance.md) remains the historical partial verdict; this record supplies focused repair evidence, not a new independent whole-package acceptance.

Parent remains **Phase E — Format expansion and ZIP editing**. G-21/G-22 scoped independent acceptance, OPT-02 acceptance and previous native/worker evidence are retained. Task remains `in_progress`; G-00, OPT-00/01 overall closeout, complete G-04 and physical/Release acceptance remain open. OPT-04 through OPT-07 are not started.

## O3-AC01 — original edit-form connection

`jobs.rs::EditFormOrigin { browser: (window_id, generation, snapshot), connection }` is stored in the original `Jobs.edit_forms` map. `open_form` captures the current connection while creating an edit form and refuses creation without one. `submit_form_admitted` and `clipboard_requests.rs::intake_target_current` require that original connection in addition to the existing source/path/snapshot/folder guards. Picker and Paste retain their own pending connection checks, including deferred delivery. The logical job's browser refresh origin remains the existing tuple; no wire field, controller, scheduler or native authority is added.

Regression entry: original incoming-drop dialog → actual Add action → original form → real submit/Paste/file-picker/folder-picker controls. `PlatformServerEndpoint::open_connection_for_contract_test` replaces the actual command router; its old Disconnected event remains queued throughout assertions. Source/browser identity and the draft list remain unchanged.

- Before repair, actual submit and Paste reached the replacement connection; the same-connection control passed (`opt03-ac01-red.log`:1pass/2fail).
- After repair, old drafts emit no input command, no ArchiveJob Start and no logical job; their pending/busy state settles without processing the old disconnect. Same-connection submit/Paste/both pickers each dispatch once. The complete intake subset passes8/8 (`opt03-ac01-green.log`).

The existing Form/Field, stable Inputs, Medium action/footer buttons and Small outlined source Remove are unchanged. All cross-entity callbacks still defer through original owners. No physical GUI acceptance is claimed.

## O3-AC02 — valid no-refresh business terminals

`opt03_add_selection_tests.rs::add_case` validates each business event before delivery. AllSkip, precommit failure, legacy-unknown publication, `CommitOutcomeUnknown`, cleanup-incomplete publication and precommit cancellation omit `add_result` and use the released commit-state vocabulary. Through original IPC/application notification handling, each matching terminal must be installed exactly before browser assertions run. The original snapshot, folder, selected ID and empty result summary remain; no BrowserRead is sent.

Stale attempt/generation events remain a separate identity-isolation case. Contradictory published-result data is separately named `opt03_ac02_malformed_add_result_is_rejected_before_terminal_install`: validation fails, no terminal is installed and no refresh occurs. Confirmed published late cancellation remains the distinct positive refresh-without-success-selection case.

Adding the validity assertion exposed the earlier allSkip fixture before it reached business handling (`opt03-ac02-validity-red.log`). The corrected Add selection group passes6/6, including positive results, overflow, stale contexts, valid no-refresh terminals, malformed rejection and late cancellation. No production publication or refresh logic changed for this finding.

## O3-AC03 — Test-side cleanup failure

`opt03_result_tests.rs::post_test_case` consumes actual C# Create exports via required `NXTOOLS_OPT03_CREATE_WIRE`: ordinary ZIP, split ZIP `.zip` and split7z `.001`. It validates the Create event, observes the original Test Start, then validates and delivers the matching Test terminal. The cleanup-false branch covers all three primaries and success/failure/cancel outcome shapes (9 combinations).

Assertions retain Create committed/verified/skipped counts, commit state, journal, confirmed publication and native location while preserving Test outcome/code. Final cleanup stays false, saved Create facts merge once, and the original running reservation remains occupied. Original Locate/Retry calls emit no Shell command, source observation or new job attempt; no recycle/continuation grant is available. Existing cleanup-true and Create-side failure/disconnect tests remain.

The result group passes4/4 (`opt03-ac02-ac03-green.log`). This adds Test-side lifecycle coverage; it is not a new native cleanup-failure execution or a production terminal-merge fix.

## Executed verification

Groups overlap and are not additive. All Rust calls used jobs1/incremental0/codegen-units256 in the same invocation and one Cargo lane.

| Check | Result |
| --- | --- |
| Intake lifecycle |8/8 |
| Add selection/terminal handling |6/6 |
| Native-export post-Test handling |4/4, including9 cleanup-false combinations |
| Original Desktop Job suite |111/111 |
| OPT03 entry selection |22/22 |
| Existing Step4 edit/input/refresh selection |11/11 |
| Desktop all-target strict Clippy |PASS, `-D warnings` |
| Scoped rustfmt |PASS for the five changed Rust files |

Primary logs in this session scratch: `opt03-ac01-{red,green}.log`, `opt03-ac02-validity-red.log`, `opt03-ac02-ac03-green.log`, `opt03-ac-integrated.log`, `opt03-ac-clippy.log`. Actual Create exports are the existing C# fixture outputs; absence fails and never creates synthetic publication facts. Shared AC progress tests consume their existing actual C# progress exports.

Product code changes are limited to `jobs.rs` and `clipboard_requests.rs`; test changes are `intake_tests.rs`, `opt03_add_selection_tests.rs` and `opt03_result_tests.rs`. C#, protocol DTOs, worker, writers, naming planner, original queue and native publication mechanism are unchanged. Prior independent native/worker results remain attributed to that report, not counted as new runs.

## Limitations

The historical independent whole-package closeout verdict is not rewritten. Physical clipboard/picker/Explorer/product GUI, DPI/IME/accessibility/visual and Release acceptance remain separate. No packaging/signing/install, user archive mutation, source recycling/HZ5, service/registry operation, commit or push occurred. The stop point remains OPT-03.
