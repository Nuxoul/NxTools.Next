# Component choices and retained boundaries

| Surface | Released0.7.1 API / variant | Parent adaptation / preservation |
| --- | --- | --- |
| Trust runtime recovery | Button.outline.with_size(Medium), Toolbar.content | Real32pxboundedButtons; no Toolbar.child ghost adaptation; existing deferred callback and pendingdisable unchanged |
| Empty manual activity | Empty/EmptyHeader/EmptyTitle/EmptyDescription | p3inside existingboundedDialog scroll; original noautomaticexecution copy, close/Escape retained |
| Shortcut guidance | Kbd::new(Keystroke), Label.text_xs | Renderonly helper/no keybindings/focus; wrap hints in QuickFindGeneral; Clipboard52px two-linefooter at300wide; actual240wide regression; defaultCtrlG52pxslot borrows4px from path, ordinaryresult48pxunchanged |
| Extension capabilities | DescriptionList.columns1.bordered(false), GroupBox and ListItem | Existing Settings page owns scroll; all source/trust/hash/permission/handler values preserved, lastitem reachable744x430 |
| Download readonly fields | DescriptionList.horizontal.columns1.small.bordered(false), DescriptionItem | Headerlabel100px, ordinary60px; bounded width/longcopywrap, exact values/colors/masking preserved |
| Download timeline | Marker.alignment(Start).with_variant(Border), MarkerContent | Two slots timestamp64px+elasticmessage; original order/formatter/scroll; static no repeated frames after wheelwork drained |
| Probe waiting | Spinner.xsmall.color(theme.accent) | Only actual probing+motionallowed,12px; static fallback unknown/reduced/idle/handoff; compact176/footer preserved |
| FFmpeg progress | Progress.xsmall.value(existingpercent) |4px; onlyknownpositiveDownloadingtotal/validpercent; no syntheticphasepercentbar; originalbytes/status/cancel/footer |

## Exact released source
Published registry gpui-component-0.7.1/src/toolbar.rs::ToolbarItem::sized/content;button/button.rs::prepare_for_toolbar/outline/border_color; kbd.rs::Kbd/RenderOnce;empty.rs::namedslots;description_list.rs::DescriptionList/DescriptionItem;marker.rs::Marker/MarkerContent;spinner.rs::Spinner;progress/progress.rs::Progress. No guessed newerexports or dependency changes.

## Read reference sources
- docs/reference-code/clipboard.md -> D:\Code\NxTools\Views\ClipboardHistoryPopupWindow.axaml.cs constructor61–167/PrepareForPopupShow206–259/KeyDown321–369; base Avalonia Window not supplied. No native/style/focus lifecycle change.
- docs/reference-code/download.md -> D:\Code\ab-download-manager-master\desktop\app\src\main\kotlin\com\abdownloadmanager\desktop\pages\singleDownloadPage\ProgressDownloadPage.kt:231–295,529–578;pages\addDownload\single\AddDownloadPage.kt:510–525;pages\singleDownloadPage\ShowDownloadDialogs.kt:52–80,163–200 -> desktop\app-utils\src\main\kotlin\com\abdownloadmanager\desktop\window\custom\CustomWindow.kt:343–399.
- Then D:\Code\NxTools\Views\MainWindow.axaml1243–1350;AddDownloadWindow.axaml237–253/AddDownloadWindow.axaml.cs20–39,73–109 -> Services\WindowAnimationService.cs13–43;VideoSettingsWindow.axaml192–193/ViewModels\MainWindowVideoViewModel.cs316–370/Services\FfmpegInstallerService.cs64–87. Production C#finalbounds and sourcegeneration/hiddenhandoff are unchanged.

## Intentionally deferred
InputGroup,ClipboardDialog,DataTable,Table,Calendar,NumberInput,Select,Command andcomplexresultrows remain candidates requiring their own preservation/geometry evidence. Existing official Input+prefix isvalid. Tools neutral subordinate status isnot automatically replaced with colorfulTag. This record doesnot certify visual/user/native acceptance; seeoutcomes.md.
