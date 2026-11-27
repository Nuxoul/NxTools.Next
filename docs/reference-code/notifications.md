# Notification UI reference index

Last verified: 2026-07-20.

Current framework authority: [Kit0.10.0/defaultpre0.3.12](gpui-kit-0100.md), with duration/header-action in `vendor/gpui-component-0100/src/notification.rs` and live noactivate in `vendor/gpui-pre-windows-0312/src/events.rs`. Earlier versioned source checks and outcomes below remain dated evidence; official clock/pause/stack/exit, explicit Copy, passive staging and deferred owner rules remain unchanged.

## Zed reference

Root: `D:\Code\zed-main`

| File | Verified responsibility |
| --- | --- |
| `crates\workspace\src\notifications.rs` | Replaces notifications by stable ID and schedules auto-dismiss. |
| `crates\workspace\src\toast_layer.rs` | Renders a dedicated overlay, isolates pointer input and owns dismiss timing. |
| `crates\notifications\src\status_toast.rs` | Compact surface, optional status icon/action and explicit dismiss affordance. |

Verified rendering details:

- status is communicated by a small leading icon instead of a fixed-width text badge;
- the dismiss affordance stays in the header action area and does not participate in message layout;
- an optional action is rendered after the message and owns its click handler;
- content remains compact, while the toast surface expands only when secondary information exists.
- Zed's toast layer applies a one-shot entrance animation and keeps dismiss timing in the owning
  lifecycle object; NxTools.Next extends that lifecycle with a short exit phase so removal never
  cuts the surface off abruptly.

## NxTools.Next contract

- transient operation state must not be appended to settings descriptions or change settings-row
  height;
- a reusable Rust `NotificationCenter` owns notification identity and lifecycle; feature modules
  publish typed progress, success, information or error notifications;
- GPUI renders a top-right overlay above every main-window module; C# has no notification UI role;
- publishing the same ID replaces the previous state so progress becomes success/error in place;
- persistent progress never schedules frames, completed notifications auto-dismiss, and every item
  has a manual close action;
- the visible stack is bounded to three newest items and stale dismiss timers may not remove a newer
  replacement with the same ID.
- completed file operations carry structured key details instead of burying paths in prose;
- database backup/compaction notifications expose the actual backup directory and an `打开目录`
  action through the existing Rust-to-C# shell command boundary;
- the close button is anchored to the notification's top-right corner and never shifts with title,
  message or detail length.
- appearance and disappearance are separate one-shot transitions keyed by notification generation;
  neither transition repeats or requests frames after its short duration ends.

## Current official lifecycle and user amendment — 2026-10-20
The preceding2026-07top-right/customlifecycle observations are historical, superseded by `.trellis/tasks/10-16-menu-notification-refinement/official-notification-integration.md`: releasedKit0.7.1 Notification/NotificationList/ToastStack own actualcard/stack/clock/wholegroupremainingpause/hoverclose/transition; existingNotificationCenter ownsbounded3businessrecords/correlation/source/actions only. Rust screenhost->C#NotificationWindowPlacement.Validate/Prepare->sourceworkarea/DPI/passivestyle/exactclientreadback->hiddenexplicitdraw->deferredreveal is the realnativeboundary. Visiblegrowth/shrink useskeep_visible,not hide/reveal; max360x400logical. No rawWindowsUI in Rust.
Rechecked actualZed `crates/notifications/src/status_toast.rs::render` (compactnaturalcontent) and `crates/workspace/src/toast_layer.rs::show_toast/start_dismiss_timer` (singleowner10seconddefault),behaviorreferenceonly; noNotificationnamedentry found in oldNxToolssource. Officialpublished0.7.1/0.7.3/0.8.0 notification.rs are byte-identical SHA25609010c94024ee6e2580ecf557d97dcf276512a52be8cf298e0936088df0afeca. No publicduration/timeout; NotificationList::push hardcodes5. Archivesverifiedagainstcrates.iochecksums;0.8requiresGPUI0.3.9,notadopted. Detailed currentcheck `.trellis/tasks/10-20-notification-sizing-duration/latest-version-capability.md`.
User rejects54pxcombinedtitle/messagecap and approves minimalcomponentpatch1–60/default6. Newproductionredtests observedexact54pxclipping bothrenderandnativeallocationbeforefix. Projectcontentnowmeasuresnaturaluncompressedbodybeforeboundedscroll; officialactionsremainnonshrinking. Originalgeneration/once-only/source/deferred/noactivate/firstframe/hide/reuse/retiredACKinvariantsremain. `vendor/gpui-component-071/src/notification.rs` addsoptionalDuration/defaultNone/publicbuilder/ToastOptionsforwarding only; originalupstream5sforunconfiguredcallers,officialremainingpause unchanged. GlobalConfigApplyCancel,1–60range,default6,newnotice-onlysnapshot; no existingtimerreset. No dependencyupgrade or liveclipboard/service/installedacceptance implied.

## 2026-10-22 passive notification frame pacing (approved repair prerequisite)
Current source is Kit0.8.0/defaultpre0.3.9; the earlier0.7.1pin and not-adopted0.8 descriptions above are historical. Rechecked Zed toast_layer.rs single lifecycle and current Component notification.rs official one-shot Animation/ToastStack/manager. Next ScreenHost::sync -> open_surface creates hidden/focusfalse PopUp with ..Default; C# PrepareNotificationWindow applies WS_EX_NOACTIVATE/workarea/DPI/clientreadback -> Rust finaldraw/frame -> reveal. Exit stays visible until official removal; visible resize keep_visible unchanged.
Published pre0.3.9 platform.rs::WindowOptions::default sets inactive_frame_interval=Some(33333us); window.rs request-frame callback1813–1862 applies that cap to inactive animation. Next notification open_surface inherits it, unlike Clipboard/QuickLaunch explicitNone. The approved fix uses only the public per-window option, no framework/Toast changes. Add red-capable production-options/scheduler regression before fix; bounded explicit Debug fixture cadence samples are not production frame logs or physical/user acceptance. Owning task: `.trellis/tasks/10-22-notification-frame-pacing/`.

##2026-10-22 bounded native cadence verification
The production-options fix now has compiled35notification regressions and finalDesktop1002green. `tests/native/notification-cadence.ps1 -AuthorizeNative` builds only a scratch relay, starts explicit Debug synthetic source/popup and uses existing C# PrepareNotificationWindowForTest/SetNativeWindowVisibilityForTest, never PlatformHost main/services/formalIPC/userdata. PID+exe+synthetictitle/rootchecks precede nativeoperations. Debugcanvas observes<=512paints/8stage marks and never requestsframes; no production perframelog. Historical33333uscap is explicitDebugcomparisononly.
At actualreported180Hz/96DPI, reproduciblescript sample: legacyentry13frames/median34.131ms,exit9/33.483ms; currententry79/5.484ms,exit39/5.486ms; idle0framesboth. Reducedmotionentry1frame/exit2settlementpaints, idle0. Originalexitcompletionprecedeshide (~276mscurrent); noforegroundchange and exactnativeclientreadback. Fixture must call public App::set_reduce_motion as productionPlatformEventDispatcherdoes; setting only UiMotionPreference missesofficialBasepolicy. Paintcadence is not GPU presentedframe/physicalperceivedsmoothness/userapproval/installedacceptance.

## Approved2026-10-22 hovered-card copy prerequisite
Passive HWND retains keyboard focus in another app, so GPUI keybindings cannot implement hoverCtrlC alone. Reuse existing LowLevelKeyboardMonitor (one hook) plus PostMessage/EventWindow and copy-only WindowsPasteCoordinator, never activation/paste/anotherhook. Actual public-content inset observer experiment failed: it covered278x44content, not336x74card (`density-card-observer.log`); removed without production behavior change. Source-backed border/rem/icon projection matches real official cards for3cards x6positions xcollapsed/expanded xrem14/16/20 in `notification_copy_full_card_geometry_tracks_official_stack_and_rem`. This proves settled geometry only, not current paint/clip/opacity/occlusion or cross-process target freshness.
Actual official-close regression `notification_copy_business_record_is_not_an_exit_start_signal` confirms business record remains non-exiting after close starts until official completion. Component0.8.0 `DismissRequest` (line35), `transition_status`, `begin_close` and `complete_enter` are private; `on_close` runs only in `complete_close` after exit. Base Toast::status is public on a different, unowned primitive and does not expose the styled Notification's state. No public animation-settled signal was found on Window. Within approved no-extra-framework-patch/no-second-clock/no-per-frame-IPC scope, safe exit-start revocation remains unresolved. Notification copy is NOT implemented: no hook consumption, protocol/capability or clipboard writer change. Do not use business items or settled rectangles to silently intercept global Ctrl+C. A narrowly approved official lifecycle observation seam or upstream API is required before resuming this slice; all3geometry/lifecycle prerequisite tests pass, not feature acceptance.

## User cancellation — 2026-10-22
The user explicitly decides not to implement hoverednotificationCtrlC. The prerequisite/API-gap research above is historical only, not an outstanding task or authority to patch Notification/Base/motion. No hook consumption, new protocol/capability, notification-copy queue or native clipboard action was added. OrdinaryCtrlC and official notification lifecycle remain unchanged.

## User-approved2026-10-23 header copy alternative
This does NOT resume the cancelled global hovered Ctrl+C implementation. User instead approves an explicit Copy icon directly left of the original Close. Exact Component0.8.0 Notification::render has no adjacent publicslot; the approved minimal notification.rs extension adds optional header_action(builder->AnyElement), same official hovertoolbar, omitted during Ending, no autohide mutation. Existing card/list/clock/stack/native passive show owners remain.
Current entry chain: presentation.rs creates CopyButton entity in header_action; original XSmall ghost Button/Copy (successCheck) stops click propagation, defers NotificationCenter::request_copy with ID/generation -> bounded caller-ID-correlated PasteText(paste_after_write=false,targets0) -> existing C# WindowsPasteCoordinator.Execute writes and returns before activation/SendInput branch. main.rs binds original bridge; platform_events.rs defers copy result/disconnect back to original center. No real clipboard read/globalhook/secondnotification/feedbacktimer. Copy payload is title/message/first8displayeddetails, <=256KiBUTF8 checked before allocation; oversized is rejected without truncation. Matching native result, timeout and disconnect retire pending; stale replacement result cannot update new card. SuccessCheck lasts until replacement/retirement; fixed errorTooltip allows retry. Fork generic API contains none of NxTools copy business logic. See `.trellis/tasks/10-23-notification-copy-button/` for exact final checks and remote commit.

## Default placement — 2026-11-20
New and missing Core `SystemSettings.notification_position` values use `TopCenter`. Explicit values,
including the former `BottomRight` default, remain authoritative across section saves and reloads.
The official notification owner and source-bound passive native preparation are unchanged.
