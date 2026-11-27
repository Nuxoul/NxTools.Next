# Download readiness fixtures — 阶段三：原生材质与展示生命周期

## Boundary

This slice adapts old successful-presentation fixtures to the existing hidden-first-show material contract. It changes no production behavior, shared material runtime, native writer, dependency or vendor source. The approved graph remains Kit/Base/Component/Assets0.10.0 + gpui-pre0.3.12. The canonical stage remains stage three in `implement.md`.

Reference authority: `docs/reference-code/desktop-materials.md` (qualified receipt, original runtime transfer and ordinary readiness owner), `docs/reference-code/download.md` (keyed task windows, passive completion, fixed confirmation, native ACK and source cancellation). No new migrated interaction or native behavior is introduced. Existing layouts, released compounds, visible variants/sizes and parent adaptations remain unchanged; the fixture observes the released Base Root's exact view type rather than substituting controls.

## Supplied regression evidence and classification

Input log: `D:\.pi-desktop\data\scratch\50e0d2fb-748d-49cd-b5fc-3c5a69200c57\material-stage3-owner-17.log`.

The parent run contains359 Download tests:168passed,191failed. The supplied earlier material run has all9 `downloads::material_tests` passing inside108/108 `material_` tests. These are pre-adaptation results, not results from this slice.

| Failed module | Count |
| --- | ---: |
| `downloads::batch::tests` | 1 |
| `downloads::batch_tests` | 3 |
| `downloads::conflict_tests` | 4 |
| `downloads::controlled_handoff_tests` | 9 |
| `downloads::directory_picker_tests` | 18 |
| `downloads::floating_assistant::tests` | 1 |
| `downloads::grouped_completion_tests` | 1 |
| `downloads::new_download_link_tests` | 5 |
| `downloads::polish_tests` | 40 |
| `downloads::polish_tests::intake_tests` | 10 |
| `downloads::server_information_tests` | 6 |
| `downloads::settings_view::background_limit_ui_tests` | 1 |
| `downloads::settings_view::category_extensions::tests` | 2 |
| `downloads::settings_view::settings_task_rem_tests` | 60 |
| `downloads::task_window::layout_tests` | 1 |
| `downloads::tests` | 29 |
| **Total** | **191** |

190failure blocks are consistent with absent material preparation in old success fixtures: removed ordinary/intake/settings handles; missing keyed task/delete registrations; controlled/external probe flows never admitted; missing fixed confirmation; unexpected material-failure notifications; released batch busy ownership; and cancelled unseen shutdown countdowns. Representative log boundaries are367–408,410–580,582–862 and868–1334. This diagnosis does not claim190post-adaptation passes.

The independent failure at log863–866 / `downloads/settings_view.rs:3134` uses silent `InputState::set_value("0")` while expecting the original dirty-input subscription to run. Released Base0.10.0 `src/input/base/state.rs:1285–1315` explicitly suppresses Change for `set_value`; `replace_all:1317–1337` is the eventful edit operation. The test now edits and drains the event before the original validation assertions; production validation and dirty ownership are unchanged.

## Fixture implementation

- `downloads/ordinary_presentation_fixture.rs::confirmed_application` explicitly opts old successful-presentation fixtures in before the original `DownloadApplication::new`.
- One test-only Root observer matches only NewDownloadView, DownloadSettingsView, DownloadScheduleView, DeleteDownloadView, DownloadTaskWindowView, DownloadShutdownCountdownView and DownloadTaskbarOverlayView. Unrelated roots are not prepared.
- Synthetic session/revision facts are explicit. Each matched TestWindow receives an exact nonzero `runtime::set_test_window_handle(window, hwnd, cx)` mapping and a matching command/receipt through `Preparation::confirmed`, then `runtime::track_prepared` transfers that record to the original readiness owner. Ordinary targets use Backdrop; taskbar uses Transparent and a Transparent effective receipt.
- The observer performs no application/entity readback, placement, reveal, activation, clipboard operation or IPC auto-response. It does not replace a pending readiness callback or install a production disconnected/PlatformNone success path.
- Common window, polish and detail fixtures, picker/settings/task/server/conflict fixtures, grouped task completion and external-link fixtures opt in. Existing failure/cancellation/generation/job/probe/clipboard assertions remain.
- Two older automatic-probe success tests now supply existing synthetic native handles and explicitly deliver the correlated fixed-presentation result after the material-gated command appears: `polish_tests::successful_probe_replaces_compact_window_with_fixed_confirmation` and `floating_assistant::tests::floating_add_download_reuses_blank_and_auto_opens_fixed_confirmation`. Native ACK is not inferred from material readiness.
- New `ordinary_presentation_fixture::material_fixture_qualifies_only_download_roots` asserts retained Backdrop/Transparent receipts, distinct HWND identities and no receipt for an unrelated Root.
- `material_tests.rs` remains unchanged and constructs its application directly. All9unprepared/real-IPC request, receipt, stale identity and failure tests retain their original coverage. The added fixture test increases the full Download selection by one.

## Changed paths

All Rust paths below are relative to `rust/apps/nxtools-desktop/src/`:

- `downloads.rs:147–148`: cfg(test) fixture module only.
- `downloads/ordinary_presentation_fixture.rs`: test-only qualified preparation and narrow-root regression.
- `downloads/tests.rs:3784,5312`: common success fixture and typed shutdown fixture opt-in.
- `downloads/polish_tests.rs:19,3669`: success fixtures;604,638–645: explicit fixed-presentation ACK.
- `downloads/detail_request_tests.rs:48`: shared task/batch fixture opt-in.
- `downloads/directory_picker_tests.rs:28`: picker fixture opt-in.
- `downloads/server_information_tests.rs:34`: task fixture opt-in.
- `downloads/conflict_tests.rs:344`: persisted failed-task fixture opt-in.
- `downloads/grouped_completion_tests.rs:313`: original keyed task-window fixture opt-in.
- `downloads/settings_task_rem_tests.rs:35`: settings/task fixture opt-in.
- `downloads/new_download_link_tests.rs:79`: external-link fixture opt-in; Clipboard roots remain excluded.
- `downloads/floating_assistant.rs:269,342–371`: cfg(test) synthetic handles and correlated fixed-presentation ACK.
- `downloads/settings_view.rs:3130–3156`: cfg(test) eventful background-limit edits with application-turn delivery; original assertions retained.

## 验证结果

阶段三当前Kit0.10.0/pre0.3.12图的 `downloads::` 选择共360项通过，0失败、0忽略；日志为 `material-stage3-download-converged.log`。同轮材质109项、原悬浮助手22项通过。下载全组结果早于阶段四表面投影改动，不作为之后整组重跑的结果。

测试继续使用显式准备夹具；原9项未准备/真实IPC材质回归保留。原URL替换测试使用已有 `/partial-old` 合成端点，确认非零且未完成的真实写入后暂停；总量来自共享 `CONTENT.len()`，原暂停状态、SQLite计划/工件、身份、取消、输出和工作线程排空断言保持。另一有限传输在原窗口准备/绘制后恢复同一已排队任务，保留原8秒期限、分段重平衡和几何断言。

固定确认窗口测试显式绑定两个合成HWND，保持operationID，先验证材质后发原展示命令，再交付原ACK并验证替换/退休。冲突通知断言遵循原延迟应用轮；请求头布局通过官方textareaID测量真实复合边界、8DIP重置间隔和120DIP高度，输入实体及编辑断言不变。

Parser-only rustfmt、定向空白检查、当前框架与固定编辑器来源门禁通过。真实Windows位置/焦点/原生材质、实际像素和物理交互独立验收；不自动打包、操作服务、提交或推送。

## 实事件夹具边界

重平衡视图夹具先创建并绘制原窗口，再恢复同一任务。已有 `/range-imbalanced` 合成响应可显式暂停首次 payload；原视图确认 Downloading 后释放。HEAD/范围探测、默认响应、实际分段/重平衡、原事件泵、同一HWND/视图/输入、真实区间、最终8MiB输出及连接排空断言保持。初始状态与部分进度共用原8秒截止时间。暂停默认关闭，server析构释放，单个响应另有10秒退出上限；无生产开关或进度注入。

## 连续事件投影

普通连续状态事件刷新紧凑快照时，只确认实际消费的事件水位，保留当前任务字节/总长兼容的非终态进度；权威刷新、会话变化、缺口和SnapshotRequired仍清空缓存并重建水位。事件投影3项、原真实HTTP重平衡及当前下载整组363项通过，见[投影契约](download-followup.md)。
