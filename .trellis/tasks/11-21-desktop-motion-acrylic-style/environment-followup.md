# Environment inheritance

## Boundary

Stage four uses the existing Extensions manager and owned 744×430 TestWindow. Package management → 来源 → 编辑运行环境 uses official pointer controls, saves an override independently from trust/enable, and reopens the same package environment to restore inheritance. Configuration uses a temporary synthetic package and config.

## Reference and released API

- `docs/reference-code/winshun.md` and `.trellis/tasks/10-15-extension-runtime-settings/{design,code-spec}.md` define override → default selection and separate Save/Cancel/trust.
- `D:/Code/WinShun-main/src/plugin/Programs.cpp:95–170`: `find` checks an override before discovery; `setOverride` invalidates discovery; `launchFor` keeps EXE package-local. The existing Next runtime-settings contract owns the interpreter editor.
- Kit/Base/Component0.10.0 with default pre0.3.12: `Form::child(Field)`, `Field::child`, outlined `Button::with_size(Size::Medium)` and `Input` retain released32DIP geometry. `Toolbar.content` owns the heading; action buttons remain outlined outside `Toolbar.child`.
- `extensions_settings.rs::detail_content` uses official Sheet → TabBar → Settings/SettingPage/SettingGroup/SettingItem. Runtime controls belong to 来源 route4. `view_button` schedules the existing owner callback on the next application turn.
- Component `root.rs::open_sheet_at/close_sheet` owns focus and removal; `sheet.rs:257–268` owns finite horizontal ingress. `extension-detail-settings-viewport` is a layout container, while the official Button semantic snapshot supplies the action frame and paint visibility.

## Invariants

- Draft changes stay in the existing view callback. Deferred Save uses `ExtensionApplication::save_runtime_override` → original commit/Core transaction; only successful commits reconcile and close. Save does not grant trust or enable a package.
- Inheritance restores the captured default, clears retired probes/results and submits `None` through the original override owner. Cancel does not write; validation failure preserves the draft and original disk bytes.
- Save/Cancel preserves the package and returns to 来源 route4. The original Sheet/Root retains focus, close and occlusion ownership.
- Pointer tests match the official Button's exact final element ID and role, visible/non-disabled state, and full vertical containment in the real viewport/window clip. They click the center of the horizontally painted intersection during Sheet ingress. Scroll positions use the clipped viewport center and reverse for targets above it.
- The original24-step reachability budget, five-second commit timeout and persistence assertions remain. Immediate draft assertions verify pointer dispatch before persistence is tested. Runtime/config diagnostics remain with their original owners.

## Verification

`cargo test --offline --locked -p nxtools-desktop --bin nxtools-desktop main_window::extensions_page::environment:: -- --test-threads=1`: **6 passed,0 failed**. Rust verification uses one build job, incremental disabled and256codegen units.

Coverage includes actual pointer Save/Cancel/reopen, override persistence and inheritance restoration, source-route return, unchanged trust/enable/defaults, discovery, stale browse results and correlated bounded probes. TestWindow results are separate from native/physical/visual acceptance.
