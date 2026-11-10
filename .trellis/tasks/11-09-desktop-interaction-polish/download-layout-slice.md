# Download layout slice

## Scope and reference authority

This slice changes only the keyed progress-window information hierarchy and the Download Settings category-rule header. Rust retains draft/layout state, GPUI renders, and the existing download application plus native window lifecycle remain the only task/settings owners.

Reference review order:

1. AB Download Manager `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/ProgressDownloadPage.kt`: the main progress/actions stay primary while optional part information expands below; `RenderInfo` projects immutable monitor values.
2. AB Download Manager `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/singledownloadpage/BaseSingleDownloadComponent.kt`: downloaded bytes, speed, time left and resume capability derive from the original monitor snapshot.
3. AB Download Manager `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/settings/DesktopSettingsComponent.kt`: destination/category policy remains grouped under the existing settings owner.
4. Legacy NxTools `Views/DownloadProgressWindow.axaml` and `ViewModels/DownloadTaskViewModel.cs`: filename, downloaded bytes, speed, remaining time, explicit actions and same-window task presentation remain product behavior; technical segment data is secondary.
5. Legacy NxTools `Views/SettingsWindow.axaml`: Download settings remain one durable settings section rather than a second category saver.

## Component selection record

| Surface | Released API and source | Decision | Compatibility boundary |
| --- | --- | --- | --- |
| Progress summary | Kit0.8.0 `DescriptionList::horizontal().small().bordered(false)`, `Progress::xsmall()`, and official `Tooltip`; `vendor/gpui-component-080/src/{description_list.rs,progress/progress.rs}` | Retain official compounds and arrange their public elements in the fixed task window. | Original sampler, task snapshot, progress source, actions and footer are unchanged. |
| Technical details | Existing official `Table` inside the original disclosure | Move resume/limit/connection/segment summary into the disclosure above the existing semantic segment table. | One `details_expanded` state, one resize owner and the original440×280 expanded client remain. |
| Category choice | Kit0.8.0 `Select`/`SelectState`/`SelectEvent` and public `SelectItem::render` with `label::Label`; `vendor/gpui-component-080/src/{select.rs,searchable_list/item.rs,label.rs,list/list.rs}` | Adopt a real searchable Select with default Medium32DIP trigger,260DIP menu width,220DIP bounded option viewport below the search field and accessible name `选择分类`. Each public option-content slot gives the official Label a stable category-ID identity and accessible text boundary. | `selected_category` remains the draft owner. Select state mirrors category IDs and emits into that owner; it adds no persistence path. |
| Category header action | Existing official Small secondary outlined Button from `action_button` | Place `分类规则`, official help Tooltip, Select and `+ 新增` in one wrapping header. | Released Settings/Page/Group/Item still own page search/navigation/scroll; original Apply/Cancel owns durable persistence. |

## Production layout contract

- Progress remains440×184 collapsed:34DIP titlebar,106DIP body and44DIP fixed footer. The collapsed body does not scroll.
- The filename uses the14/20 title tier, one tier above12/16 body metrics. It stays one line in progress and retains the bounded full-name Tooltip; failed/completed two-line limits remain.
- Primary content is filename, Server route/time, official progress, downloaded/total bytes, speed and ETA. The title owns status, so status is not repeated in the metric grid.
- The Server row shows host, `直连`/`代理`/`系统策略`, and `{N} ms`.0–100 uses semantic success,101–300 warning and301+ danger. Missing observation has no time element. The Tooltip states that the value is HTTP request-start-to-response-headers duration, not Ping or RTT.
- Peer IP is not read by the renderer and is absent from visible text, Tooltip, accessibility copy and diagnostics.
- Resume support, effective speed limit, active connections, segment count and segment rows appear only in the original expansion. The expansion remains440×280 and the segment table remains the only scroll owner there.
- At ordinary settings width, category heading/help/Select/add share one row. The header permits wrapping when width cannot contain the released controls. Category fields and ordering/deletion controls remain below it.

## Lifecycle and persistence invariants

- One task ID retains one task entity, one HWND, one generation and the original close/reopen/cancel/activation behavior.
- Completion still transforms the keyed task window in place; no new task window or network operation is created by rendering.
- Task-window callbacks continue to update local view state first and defer download-application mutations.
- Category selection edits only the existing settings draft. Category add, rename, reorder and delete resynchronize the Select by stable category ID without a second state owner or saver; InputState change callbacks defer that cross-entity mirror update to the next window turn.
- Apply continues through `DownloadSettingsView::current_draft` and `DownloadApplication::apply_settings`; Cancel/titlebar close discard the draft.

## Focused acceptance filters

- `download_layout_`
- `server_info_`
- `audit06_aux_task_details_last_segment_and_real_toggle_cancel_keep_budget`

The filters cover440×184/440×280 geometry, root14/16/18 rem scaling, long filename Tooltip, fixed footer actions, route/time thresholds, unknown time, no peer-IP projection, real Select popup/scroll/selection, durable Apply ownership and original task-window lifecycle. Compiled, native, DPI, screenshot and user acceptance remain separate evidence levels.
