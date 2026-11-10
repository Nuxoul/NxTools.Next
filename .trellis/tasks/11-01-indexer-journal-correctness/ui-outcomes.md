# U4 — per-volume maintenance in Index Settings

## Released component selection

Surface: Quick Launch Settings → Index SettingPage → existing volume SettingItem → original40DIP ListItem rows. The original Settings/Page/Group/Item hierarchy, checkbox action, deferred section save, focus, scroll and automatic-selection semantics remain. Popup620×410, native placement and lifecycle are unchanged.

Released Component0.8.0 APIs: `Tag::{info,warning,success}`, `Sizable::with_size(Size::Small)`, `ParentElement::child`, official `Label::new`. Small Tag owns6×2DIPpadding and its semantic background/border. The Label explicitly uses matching official `Theme::global(cx).colors.{info,warning,success}_foreground`,10DIPtype and14DIPline height. Layout-only wrappers supply stable IDs, Label role/aria_label and the project's TestSupportExt for the same drawn text. No Toolbar adaptation, new palette or custom badge.

`volume_scope_row(theme,cx,selector,title,metadata,VolumeScopeRowState,on_click)` groups maintenance, selected and divider state without a new selection owner. Present maintenance shares a title/count/Tag lane plus rebuilding detail inside the original row. The None case keeps the original196DIPmetadata rail and40DIPgeometry. Ready empty volumes show0项.

Labels: 首次建立 / 正在重建 / 就绪. Rebuilding text is 正在重建，搜索结果可能不完整 plus readable typed reason; JournalWrapped displays 日志已覆盖. Long detail/count remains available through the original official tooltip composition. The row retains its single pointer/keyboard dispatch and original deferred save.

Sources: `rust/apps/nxtools-desktop/src/main_window/quick_launch_page.rs` and `rust/apps/nxtools-desktop/src/ui/components/volume_scope.rs`; existing ordinary-rem/polish fixtures use the same row with None maintenance.

## Verification

The production test `main_window::tests::u4_index_settings_renders_per_volume_maintenance_within_minimum_width` compiled before UI implementation and failed at `official per-volume maintenance Tag must be mounted`. The passing test renders real MainWindow Index Settings at920×480 in Light/Dark, asserts mounted three-state Tag and description bounds plus the actual text snapshots, scrolls the original page to a visible row, clicks its original checkbox area and observes exactly one saved C/E volume-selection command.

The test is included in the final200passing Desktop selection:

```powershell
cargo --config 'profile.test.package.nxtools-desktop.debug=0' test -p nxtools-desktop -- quick_launch index_settings volume_scope ordinary_rem_tests --test-threads=1
```

Logs: `indexer-desktop-red.log` and `indexer-final-desktop.log` under session scratch. The aggregate also retains automatic/explicit volume selection, keyboard action and ordinary-rem layout coverage. Strict Clippy and component/release/editor/lifecycle source gates pass. Native DPI/accessibility and user visual acceptance remain separate.
