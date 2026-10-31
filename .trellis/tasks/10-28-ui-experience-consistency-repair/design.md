# Design and boundary

Rust retains existing state/layout; GPUI Kit/Base/Component0.8.0/defaultpre0.3.9 renders; C# retains native HWND placement. No vendor delta. Source seams: QuickLaunchPanel::run_search -> apply_search_update; FileDialogSearchPanel fixed header/source -> original PlaceFileDialogSearchWindow; MainWindow::render_downloads row selection/current detail scope; NewDownloadView original validation/notification helpers; shared settings_toggle and existing Settings custom/typed field slots.

Component record: attached search original InputGroup/Input/addons/default32DIP plus Kbd; fixed36DIP header/source rails with explicit border budget. Download original virtual row and PopupMenu/PopupMenuItem; explicit inspector action reuses current scope, not a new modal/window. Official Switch Medium36x20, parent Settings size must not silently change custom controls. Original NotificationCenter/official toast stack is sole operation feedback owner. Routine static copy removed/shortened, relevant safety decisions remain.

Tests at the audited seams: nonempty pending publication through production panel/current query; attached root/header/source/Kbd containment at actual rendered bounds; real row click/context and original detail request; invalid new-download submission -> original notification/fakeIPC zero effect; real Switch semantic/geometry. Each vertical slice writes and runs red before implementation. Source/static limits and actual/native user acceptance remain distinct.

Reference prerequisites: download note -> ABDM DownloadList/TableDownloadItem then legacy MainWindow selection/context/lifecycle; Quick Launch note -> Listary managed SearchMode/SearchWindowBase and SwiftList anchored search where needed. No proprietary Listary native engine source claimed.

Hotkey-reset addition: existing Settings custom slot keeps original recorder/status. Add official
Button outline/Medium32 labeled 重置 to the shortcut's left, conditional on semantic key+modifiers
vs Core default (display text is not identity). Shared hotkey field accepts optional official
reset control; Clipboard keeps its original separate recorder. Original applications admit only
fixed default reset without recording; original pending Configure*/CommandResult/persistence and
failure paths remain. Cross-entity callbacks deferred; no native code/vendor/schema change.
Tests first reproduce missing actual reset controls, then cover same-row left placement, default
absence, busy/recording no dispatch, exactly one configure, ACK-only durable save and rejected ACK.
