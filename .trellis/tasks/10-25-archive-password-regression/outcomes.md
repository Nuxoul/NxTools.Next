# Archive password unlock regression — bounded closeout

## Outcome
The single previously disclosed Archive failure is resolved as a **test-fixture compatibility defect**, not a missing production button. Only `rust/apps/nxtools-desktop/src/archive/application_tests.rs` changed. Production application.rs/password.rs/browser.rs exactly match this turn's opening byte snapshots. Preserve current canonical **Phase F — Explorer integration and genuine native drag** and all broader Archive UI/native acceptance obligations; this does not complete those phases.

## Root cause and correction
The approved current UI intentionally shows unlock only when original read metadata establishes encrypted:true. The old password test called the ordinary drag-read fixture (encrypted:false); the filename encrypted.zip cannot establish encryption. It also expected historical32DIP, while later approved icon-over-label Toolbar.content uses official Medium outlined Buttons adapted to64DIP.

The original plain helper still supplies Some(false). A narrow nullable encrypted parameter in the shared test read completion supports Some(false)/None/Some(true) without changing original Accepted/Negotiated/Batch/credit/Terminal/source/deferred installation. Both password tests now supply Some(true). The original interaction regression checks actual named visible/enabled official Button bounds,64DIP/current viewport, real pointer down/up, original masked Dialog/Confirm/private reply, deferred authorization, no automatic OLE and closed-ticket rejection. No safety assertion or production visibility gate removed.

New production_password_unlock_visibility_uses_actual_read_encryption_metadata sends all three states through the original application transport/install. Ordinary and unknown have no visible unlock Button; encrypted has exactly one actual official named Button. Reading/rendering emits no authorization/drag command. Fresh per-case windows retire through the original owner. No real archive/native dialog/OLE/secret/user state access.

## Executed checks
Evidence root is this session scratch, files prefixed archive-password-closeout-.

| Check | Actual result |
| --- | --- |
| Original single regression before edits |0PASS/1FAIL/0ignored,0.17s,exit101: official unlock Button |
| production_password_ after fix |6PASS/0FAIL/0ignored,2.53s,exit0 |
| archive::application::tests:: |16PASS/0FAIL/0ignored,2.03s,exit0 |
| Desktop --tests strict Clippy -D warnings |PASS,55.94s,exit0 |
| cargo check --workspace --tests |PASS,25.07s,exit0 |
| One full Desktop run |1193PASS/3FAIL/0ignored,1196total,207.97s,exit101 |

All6password tests pass in that full run, including the original failing Dialog/ticket/deferred-grant case and the new nullable-encryption visibility case. The latest full run is still NOT green. Remaining3failures are:
- main_window::download_page::kit_migration_tests::sort_keyboard_actual_headers_have_names_tab_focus_and_original_column_bounds
- main_window::download_page::kit_migration_tests::sort_keyboard_enter_space_match_pointer_reverse_once_and_preserve_task_identity
- main_window::download_page::kit_migration_tests::sort_keyboard_persists_both_fields_and_reopened_main_restores_direction

Each fails at download_page/kit_migration_tests.rs:940, actual semantic sortable header Button name/created. These files were not edited or diagnosed by this task. No full rerun/claim that earlier tests did not fail; previous1159/1 is retained historical evidence. Concurrent additions increased total count, not coverage authored entirely by this task.

## Preservation and boundaries
Opening copies/initial.diff under archive-password-closeout-baseline; exact turn-owned diff archive-password-closeout-owned.diff. Production3file byte equality verified after integrated run. Shared Cargo preflight and required jobs1/incremental0/codegen256 before each launched command; no duplicate process, peer pause request or process termination. Scoped rustfmt/diff pass. No DEBUG probes or warning suppression added. Native/C#codec/password/STA/OLE owners unchanged; no redundant native operations.

No package/install/binary replacement/service/Registry/COM/Explorer/userwindow/product launch/commit/push/delegation/archive. Physical password/focus/IME/DPI/reader/visual/installed acceptance unperformed; no user-visible release fix inferred from a fixture correction. Remaining current integration blocker is the3Downloads tests above, plus previously declared native/user acceptance and capability gaps outside this bounded task.

## Final static checks
Spec-contracts114documents/14owners, component catalog79historical/current0.8.0authority, exact GPUI two-file release provenance, pinned editor57file provenance, DesktopShell and sharedUI lifecycle source gates:6/6PASS. Scoped rustfmt--check and gitdiff--check PASS. No compiled/fullsuite rerun after the one integrated result above. These checks are documentation/source evidence only, not native/user acceptance.

## Subsequent Downloads integration verification
The1193/3 run above remains historical. User requested next task; current existing-owner sortable-header repair was verified without Rust edits by this session. Download-page15/15, strictDesktoptestsClippy/workspace PASS; ONE fullDesktop1196PASS/0FAIL/0ignored includes all6password and3sort tests PASS. Concurrent2line test-only multiselection strengthening separately reverified3/3/currentlint/stablehash; not a frozen whole-worktree/native/visual/release acceptance. [Latest closeout](../10-25-download-sort-closeout/outcomes.md). No current executed test blocker remains; original installed/native/user limitations unchanged.
