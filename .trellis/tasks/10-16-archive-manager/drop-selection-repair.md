# Ordinary extraction after archive drop

## Scope

- Root and Tree directory drags keep the existing receiver-created wrapper folder: the archive root
  uses the archive stem and a single selected directory uses its basename. A file-only selection from
  the right list uses one hidden marker file instead; the receiver reports the marker's parent and
  the ordinary extraction job writes the frozen selected members directly there. Mixed file/directory
  selections retain the wrapper form. The receiver may still rename a directory wrapper on collision;
  direct marker drops do not create a wrapper folder. Extraction starts only after the marker has been
  located and removed.
- The existing held >=4 DIP gesture, source HWND, original read connection, one native STA/COPY OLE loop, capture release, exact cancellation and locator cleanup remain. No archive output is decoded during OLE. Source, code page, browser snapshot, selected ordinals and output prefix are frozen at admission. Retired generation/projection, refresh, close, disconnect and cancellation cannot submit extraction from a late receipt. The ordinary independent job owns password prompts, output policy, conflict decisions, progress, cancellation and cleanup after handoff. The virtual-drag lifetime does not govern it.
- Root keeps ordinary whole-archive extraction. Tree directory selections write directly into the receiver-created directory, including an empty directory without an extra nested basename. File-only right-list selections preserve archive-relative structure below the located parent; source lookup, integrity and freshness still use original member identities. Never silently truncate a selection.

## Owners

- `archive/model_drag.rs`: metadata-only selected expansion and wrapper/output-prefix decision.
- `archive/browser_drag.rs`: immutable pending drop, existing locate command and current receipt handoff.
- `archive/jobs.rs`: ordinary Extract with Here, original settings and source code page.
- `archive_job` Rust/C# contract and original worker planning: optional selected-output prefix, leaving ordinary extraction without a prefix unchanged.
- Native `ArchiveVirtualDragSession::Locate` / `ArchiveDropLocator`: unchanged native placement and marker owners.

Diagnostics retain the drag correlation through `archive.drop_extract.submitted`, recording its ordinary job identity and selected count, never source/member/destination names.

## Component selection

The existing toolbar layout keeps its official components and visual variants. Released Component 0.8.0 `list/list_item.rs::ListItem::{on_mouse_down,on_click}` remains the styled 32 DIP list row; original Tree public render-item composition and root move/up callbacks remain the sole gesture boundary. The obsolete pre-unlock Button/menu item is absent; other toolbar Button variants and parent adaptations are unchanged. Ordinary JobProgressView retains official Progress and existing password/conflict Dialog owners.

## Reference evidence

`docs/reference-code/archive-virtual-drag.md` records Bandizip 7.32 `FUN_140147200` -> `FUN_1401aedd0` -> ordinary selected extraction `FUN_1401481d0` after destination handoff. Legacy NxTools `Views/DownloadCompletedWindow.axaml.cs::DragFileSurface_PointerPressed` -> `StartFileDragAsync` -> `Services/WindowsFileDragService.cs::TryDragFile` supplies held-pointer real-file CF_HDROP and one native OLE loop, without an archive-browser extraction engine. Next `ArchiveVirtualDragSession::Dispatch/Locate/Prepare` owns source-peer validation, capture release and marker disposal before completion. Rust `ArchiveApplication` applies the receipt through the original weak owner and submits extraction on a later application turn.

## Verification

Boundary regressions cover Root, list single/multiple and Tree subdirectories, frozen selection/prefix, encryption without preauthorization, empty directories, complete large selections, exact cancellation and stale receipt rejection. Worker assertions verify selected output paths, original ordinals/source and refusal of an outside-prefix member.

Selected-output collision checks use only participating original records. An unselected unsafe or colliding record cannot grant output authority or reject a safe selected namespace; selecting both colliding records still refuses. SevenZ/RAR intersect the plan's original member ordinals before any name-to-output mapping, including same-name files across blocks and interleaved empty records. Codec metadata, complete required decoding, source freshness and output budgets remain mandatory. Selected directory prefixes follow the native 255-UTF-16 component grammar; UTF-8 remains the transport bound. Explicit directory metadata and codec source-name lookup retain original identities while final output names use the stripped prefix.

## Verification results — 2026-11-12

| Check | Result |
| --- | --- |
| Protocol library tests | 117 passed |
| Archive library tests, isolated-codecs | 106 passed |
| Desktop drag/selection/receipt regressions | 36 passed |
| Controlled C# Job/locate fixtures | LIST9 / RUN9 / PASS9 / SKIP0 / FAIL0 |
| Archive IPC tests | 17 passed |
| C# build | 0 warnings, 0 errors |
| Workspace check, all test targets | Passed; SFX test target has dead-code warnings |
| Archive/protocol library strict Clippy | Passed |
| Full Desktop tests | 1920 passed, 13 failed, 1 ignored |
| Full Desktop/Archive test-target strict lint | Blocked by the diagnostics below |
| Source/spec/vendor/editor/format gates | Passed |

The actual selected-output fixture verifies ZIP, nested7z and RAR source binding, empty selected roots, outside-prefix refusal,86-CJK component prefixes, duplicate ZIP original content, dual-block7z original ordinals and source replacement. Locate fixtures verify receiver-created and receiver-renamed folders, probe fallback, marker/staging cleanup, exact cancellation and the single native slot.

## Known issues outside this slice

Full Desktop failures:
- `archive::application::browser_host::icon_layout_tests::narrow_tree_folder_name_stays_on_one_line_inside_fixed_row`
- `downloads::polish_tests::queue02_headers_have_one_styled_textarea_frame`
- `downloads::polish_tests::queue05_task_uses_official_xsmall_aggregate_progress`
- `downloads::settings_view::background_limit_ui_tests::background_limit_setting_keeps_manual_limit_and_validates_only_when_enabled`
- `downloads::settings_view::settings_task_rem_tests::b4_e2_native_close_retires_held_probe_without_changing_reopened_task`
- `downloads::settings_view::settings_task_rem_tests::b4_ref08_actual_close_and_stale_identity_preserve_reopened_window`
- `downloads::settings_view::settings_task_rem_tests::b4_ref08_actual_first_focused_enter_preserves_writer_and_sqlite`
- `downloads::settings_view::settings_task_rem_tests::b4_ref08_actual_first_focused_space_preserves_writer_and_sqlite`
- `downloads::settings_view::settings_task_rem_tests::b4_ref08_actual_pointer_confirmation_preserves_writer_and_sqlite`
- `downloads::settings_view::settings_task_rem_tests::b4_ref08_actual_refusal_retains_input_retry_and_cancel`
- `downloads::settings_view::settings_task_rem_tests::b4_ref10_actual_paused_window_exposes_written_range_coverage`
- `downloads::settings_view::settings_task_rem_tests::b4_ref10_real_parallel_events_refresh_original_window_through_rebalance`
- `main_window::tests::download_context_menu_omits_actions_that_cannot_run_for_the_selection`

Strict-lint diagnostics:
- `downloads/browser_handoff_impl.rs:436`: nonminimal boolean expression.
- `downloads/settings_task_rem_tests.rs:614`: Option unwrap in a test comparator.
- `nxtools-archive/src/sfx/mod.rs`: unused Kind variants, Error.native and Limits fields in the SFX test target.

## Acceptance limits

The packaged runtime is unchanged. Physical Explorer/Directory Opus gestures, installed Release behavior and mixed-DPI/user acceptance remain unverified. Automated fixtures exercise owned data and controlled receivers; they do not certify those acceptance items.
