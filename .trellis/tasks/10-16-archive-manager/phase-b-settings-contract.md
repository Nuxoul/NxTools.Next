# Phase B — settings implementation boundary

2026-10-21 user amendment in implement.md governs this slice; Phase A remains incomplete.

## Smallest change and ownership
No archive navigation/config/settings surface exists. Add Core ArchiveSettings plus locked
update_archive_settings; one retained Desktop ArchiveSettingsView owns its immutable config
projection/local exclude InputState and failures; MainWindow owns the six keyed navigation handles.
No ArchiveApplication/job runtime, new generic config framework, history database or native service.

Existing engine options (destination/conflict/codepage/create format/semantic level/test/root stripping/
exclusions/MOTW) are saveable presets. Visible copy explicitly says operations/browser are not yet
connected. Browser/recent/columns/open-entry settings, cleanup/resource controls and Shell mutations
are not fake switches. Six approved pages remain; unavailable pages explain their actual state.
No path setting exists in design§5: destination is Ask/Smart/Here/Name, not a fixed directory.
Therefore no new picker/Protocol/C# change is needed; open/save/source pickers remain with C/D forms.
Cancel/stale archive-picker tests are not claimed for a nonexistent control.

## Source verification and invariants
Reference note read before product sources. Rechecked tracked legacy archive terms: packaging/FFmpeg
ZIP/image/video compression only, no general archive settings/window/native lifecycle found.
Read Bandizip public SimpChinese.lang IDD_SETTING_EXTRACT and IDD_SETTING_ARCHIVE; no proprietary
code/icons reused. Design§5 owns Next defaults. Existing typed directory/STA picker remains unchanged.
Production creation/encoding/native foundation stays unchanged.

Layout: existing main1060x650/min920x480,176 module rail; Settings160px initial rail/range160–208,
flexible official page scroll and fixed local failure outside the scroller. Six pages retain
the approved order; six module focus handles are independent. Leave/re-enter retains view,
InputState and saved values; official Settings reinitializes to General after module unmount.
No parallel page controller, new native placement/focus policy or render-time writes.
All cross-entity setters defer one application turn using weak original view handles. Failed saves
retain previous live settings and retryable input, corrupt/missing targets are not recreated.

## Component selection (locked0.7.1)
- Settings::new/pages/sidebar_width/sidebar_size_range/with_group_variant; SettingPage::new/groups/
  resettable(false); SettingGroup::new/title/items; SettingItem::new/keywords/layout; SettingField::
  render and scrollable_dropdown. Sources vendor/gpui-component-071/src/setting/{settings,page,
  group,item}.rs and fields/{mod,bool,dropdown,string}.rs reviewed. Official search/selection/list
  scrolling only; no mirrored selected page or private API. Default Medium controls; Normal groups.
- Switch::new/checked/accessibility_label/on_change in per-field public render slots, requested bool
  copied then deferred; released default grammar, no painted substitute or parent duplicate click.
- Input::new on stable view-owned InputState for explicit-save exclusion editor, one32px official
  frame; Button::new/outline/default Medium32 for explicit save. No Toolbar parent transformation.
  The final exclusion Input edits a JSON string array losslessly; explicit Save validates it.
  Core reuses the already locked globset0.4.19/default-features=false parser, no version upgrade.
- Sidebar retains existing FocusedModuleButton public focus adaptation. New Google Material Symbols
  folder_zip asset, Apache2 attribution in existing third-party notice.
- Alert::error is persistent local save failure; no transient success banner. Unavailable features
  use readonly SettingField content, not enabled/disabled pretend controls or fake Apply.

## Focused verification
Before implementation add production red-capable tests for navigation order/identity, actual saved
Switch result on disk, failure+retained prior value, official search no-write and minimum scroll.
Core covers missing-field defaults, changed roundtrip, stale other-section merge, malformed/missing
refusal. Compile/test/lint/fmt runs serial with required memory variables. C# safe affected picker
contracts are LISTed then identical filters RUN; no codec/native/full matrix rerun. Isolated existing
--ui-preview foundation entry renders this actual MainWindow/page with scratch config and no runtime;
default/minimum Light/Dark pointer/keyboard observations are reported separately from user acceptance.
