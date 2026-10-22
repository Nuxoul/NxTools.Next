# Official Notification0.7.1 capability verification — no integration

## Authority and boundary
User explicitly requests verification BEFORE integration. This slice only exercises published official components in a standalone session-scratch Cargo project. No product notification/controller/window/IPC/config code, framework/vendor/registry source, dependency versions or release package are changed. No proposal here authorizes integration or packaging. Dialog is modal UI; Notification is the relevant component. Website currently differs from locked product version; all compile/runtime conclusions below are for gpui-kit0.7.1 / gpui-component0.7.1 / gpui-base0.7.1 / GPUI0.3.8.

Harness: session scratch `official-notification-capability/`, exact pinned Kit plus test-support, raw-window-handle and serde_json only for owned native fixture. Same already-approved Windows backend patch path as product, read-only. Cargo target reused workspace target for compiled dependencies, but no product source linked. Original workspace Cargo.lock copied into scratch only. Test data are synthetic, no real configuration/history/plugin/runtime/service activity.

## Source/API inventory actually inspected
- Published gpui-component0.7.1/src/notification.rs: Notification public builders174–389; official styled card402–532; NotificationSettings537–569; NotificationList713–878; clock/close905–1005; official ToastStack render1008–1073.
- gpui-component0.7.1/src/window_ext.rs::push_notification/notifications/remove_notification1 and root.rs::WindowState/notification_layer; tested both standalone NotificationList and recommended Root+WindowExt route.
- gpui-base0.7.1/src/toast.rs: public root re-exports in Base lib.rs200–203, ToastMotion, ToastOptions.timeout, ToastManager::{push,get,visible,advance,dismiss}, ToastStack::{new,item,motion,placement,focus_handle}, ToastStackState::is_expanded; original code not edited.
- GPUI0.3.8 elements/animation.rs: card animation uses real scheduler Instant::now while Base spring/lifecycle uses executor.now. A virtual-clock-only animation test can leave cards in their entry position; this test-harness issue must not be reported as unsupported official hover/layout.

## Capability matrix
| Requirement | Actual result / public seam | Classification |
| --- | --- | --- |
| Three simultaneous distinct notifications |19-test harness retains3 entities; official Root stack exposes302/319/336px layers with14px peek; real pointer moves expand all to336px with separated geometry | Supported and exercised |
| Same operation progress -> terminal update | Notification::id1<T>(operationKey) replaces only matching entity, other2 retained; default Notification::new generates distinctUUIDs. Public Entity update also keeps one mounted entity | Supported; caller must choose operation identity |
| Hover pauses remaining countdown, leave resumes remainder |400ms entry +2s of fixed5s then10s hover retains; leave2.6s retains, another1s removes incl200ms exit, not fresh5s; public mouse events, not direct field mutation | Supported and exercised |
| Scope of pause | Hover/focus expansion pauses the entire NotificationList;2timed notices both survive7s hover and both expire after leave | List-wide behavior; differs from strictly per-card pause |
| Configurable1..60s ordinary dwell | High-level NotificationList::push hardcodes5s atnotification.rs872; Notification and NotificationSettings expose no duration field/builder. `.duration(Duration::from_secs(6))` compile probe returnsE0599. Public Base ToastOptions.timeout and ToastManager tested with1/6/60s +remaining pause | High-level not configurable out of box; official Base supports it. Requires deliberate public composition/adapter or upstream API, not claimed drop-in |
| Persistent progress / action | autohide(false) retains. `.action(...)` automatically setsautohidefalse; `.action(...).autohide(true)` explicitly restores fixed5s and expires | Supported, ordering/default semantics verified |
| Two real action Buttons plus close | `.action` slot returns oneButton. `.content` public slot contains2officialButtons; actual clicks invoke both once, no unintended dismiss; official close Button actual pointer click removes once and on_close firesonce | Supported via public content slot; not two built-in action slots |
| Complete official card frame | Built-in Toast styled border/backing wraps message/content/action/close; measured actual card and close hit bounds; no replacement Alert/GroupBox host in recommended tests | Structure/interaction verified; user visual acceptance not performed |
| Close always visible | Official close is invisible until group hover (notification.rs460–461); no dedicated public always-show-close builder found. Custom content can provide a button but that is an adaptation, not built-in behavior | Default differs from current always-visible-close request |
| Six positions | Actual official cardbounds in480x500: topY12/bottomY413 for75px card; leftX12/centerX72/rightX132 for336px width | Supported relative to containing window; screen work-area positioning remains native host responsibility |
| Independent nonactivating transparent popup | Standalone official Root/WindowExt with3notices +existing production C# native policy: prepare->correct-size hidden scene->show, activefalse, foreground unchanged, outer==client360x376/96DPI, no native border bits | Supported in owned native fixture; no existing product notification renderer/controller used |
| Content-size-responsive native popup, no large transparent hit area | Native fixture intentionally fixed360x376. It does not verify shrink/grow during official collapse/expand, pointer traversal across resized edges or mixed-DPI source changes | Not yet verified; public stack measurement +host sizing integration must be evaluated before migration |
| Stale replaced entity close | Hold oldNotification Entity, push sameID replacement, invoke old.dismiss; replacement survives and entity identity retained | Supported; retired subscription protects this seam |
| Original-source action +stale business generation | Public contentButton callback defers to captured original sourceWindow, not popup; caller-owned current-generation cell rejects later stale click | Public callback supports adapter; actual business generation/session/action claiming remains application responsibility |
| At-most-once business side effects | Component dispatch callbacks normally; application must claim current ID+generation/action before doing feature work | Not supplied by component; do not equate UIcallback with idempotent domain operation |
| Bounded retained storage | Base manager.visible(3) returns3 whilemanager.len() remains32 after32persistent pushes. High-level max_items is display limit; no hard admission capacity found | Application hard retention/admission bound required |
| Reduce-motion / idle | Mechanical input/placement tests use official reduce-motion policy; separate normal-motion test waits500ms real time then validates settled card. NotificationList source schedules50ms clock while timed or transitioning, stops for empty/persistent atrest; no production frame logs added | Policy/API exercised; no performance budget/reader/installed visual certification |

## Performed test commands and results
All Rust commands serialized Rust1.96, CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256. Offline dependencies, manifest is scratch only.

`cargo test --offline --lib --manifest-path <scratch>/official-notification-capability/Cargo.toml -- --test-threads=1 --nocapture`
- Exact final:19passed/0failed/0ignored,3.24s, final-tests.log. Source retained src/lib.rs and src/recommended.rs; no tests injected into product.
- Initial compile errors were harness private-module import/static-selector lifetime and HWND inherent-method ambiguity, corrected to public re-exports/constantselectors/HasWindowHandle explicit invocation, not official capability failures.
- Early virtual-only normal-animation tests failed because real-time entry had not elapsed. Kept first/second/root/recommended logs; corrected harness uses reduce-motion for mechanical tests plus a separate real500ms entry wait. All original bounds/click/hover assertions pass; no library patch or weakened assertion.
- One deliberately falsifiable stale-dismiss hypothesis was wrong: observed replacement survives. Corrected test to require survival and reran integrated19, no false defect reported.

`cargo check --offline --bin duration_probe --manifest-path <scratch>/official-notification-capability/Cargo.toml`
- Expected compile-negativeE0599: Notification has no duration method. duration-probe.log; source inventory confirms fixed5s and no alternate duration in NotificationSettings. Kept as explicit separate negative probe, not a passing main-test target.

## Real native fixture
`<scratch>/official-notification-capability/run-native.ps1`
- Owned fixture directory `official-notification-native-cf8e8fa4a5834882838df2b21a57bc38`.
- Native prepare_notification_window and set_window_visibility both success; readycards3, popupactivefalse, foregroundunchangedtrue. SourceHWND790830/popup594234/PID23548; actual native clientx2200,y1024,width360,height376/workarea2560x1400/96DPI. Outer==client, style2483027968.
- Official collapsed cardbounds336x76 atx12,y288;302x76 atx29,y260;320x76 atx20,y274, all inside popup. Generated full-window capture, not direct visual inspection/user approval.
- C# helper invokes existing production adapter methods on fixture-owned HWNDs only, no normal PlatformHost process/service/deployment. Exact fixture process killed on exit. Does not prove physical native hover/click, resize on expansion, mixed-DPI, multiplemonitors, reader or installed behavior.

## Conclusions / remaining decisions
The official component provides the core stack/hover/remaining-time behavior and can render inside a passive screen-hosted HWND; lack of those capabilities cannot justify the previous custom replacement. It is also not a fully drop-in match for every existing requirement: high-level fixed5s, list-wide pause, single built-in action slot/default hover-onlyclose, display-vs-storage limit, app business guards and native dynamic sizing must be explicitly resolved.

This report does NOT pick an integration architecture, patch upstream, add a second manager, change the current product, or authorize a package. User decides after reviewing this evidence. Current sequence15 remains unchanged and still contains the previously diagnosed broad operation-ID replacement problem; this capability experiment does not fix it.
