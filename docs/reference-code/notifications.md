# Notification UI reference index

Last verified: 2026-07-20.

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
