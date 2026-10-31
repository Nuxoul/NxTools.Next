# Floating assistant technical design

Status: implementation approved2026-10-23; current **Phase D — Integrated verification and handoff**. Settings compilation blocker resolved; current compiled/owned-native results and remaining limits are in [implementation checkpoint](implementation-checkpoint.md). Evidence: [legacy reference](../../../docs/reference-code/assistant.md), [current integration research](research/current-next.md), [native prerequisite checkpoint](phase-a-checkpoint.md).

## Smallest behavior gap and change boundary
Other Tools has Link Detection and Currency tiles but no independently owned floating ball; the current Windows rendering stack accepts file paths only and has no animated assistant scheduler. Add one optional feature composed from existing owners, not a new download/search/clipboard engine.

Rust owns typed section persistence, one `FloatingAssistantApplication`, logical bounds/edge policy, media generation/playback, action classification and operation state. GPUI owns the image/ball/progress presentation and normal configuration controls. C# owns HWND styles/position/readback, one native incoming-input/OLE boundary and existing native image-copy operation. Downloads/Quick Launch/main window retain their current owners and receive deferred typed intents.

### Expected files and why
- New `rust/crates/nxtools-core/src/floating_assistant.rs` and tests, plus `lib.rs`: serde-default dedicated `floating_assistant` section, enums/defaults/normalize and section-owned atomic save.
- New `rust/apps/nxtools-desktop/src/tools/floating_assistant/{mod,application,media,playback,actions,view,tests}.rs` (split by actual owner, not mandatory file count); `tools.rs`/module entry as appropriate: one controller/view and bounded media pipeline.
- `main.rs`, `runtime.rs`, `platform_events.rs`, `main_window.rs`: composition/weak bindings/event routing/main restore reuse; preserve dirty changes through uniquely anchored additions only.
- `main_window/tools_page.rs`, new `tools_page/floating_assistant_settings.rs`, owning main-window tests: tile and keyed460x320 feature settings.
- `downloads.rs` and its existing prefill module: narrow immutable assistant aggregate snapshot and normal prefilled Add Download adapter. No engine/store algorithm edits.
- `quick_launch.rs` and `quick_launch/panel.rs`: internal summon-with-query through original generation/activation/IME path; normal hotkey summon remains empty.
- New `rust/crates/nxtools-protocol/src/floating_assistant.rs`, `lib.rs`, C# parser/pipe/adapter/new Windows coordinator and fixture tests: source-authenticated bounded commands/events, native input receiver and cleanup.
- Desktop Cargo manifest/lock only as required to enable same pinned image0.25.10 GIF/JPEG/BMP/WebP codec features locally; no version/framework upgrade. Existing dependency license/provenance rules apply.
- New architecture/lifecycle gate and reference/spec/task evidence. No generated release manifests, browser extension, indexer, installer or unrelated feature changes.

## UI component selection record
Requested shape: Other Tools tile, separate Configure, compact scrollable460x320 configuration; native floating ball is a custom media/business surface, not an ordinary control replacement. Existing theme/fonts and Kit0.8.0/defaultpre0.3.9 remain.

| Surface | Exact released API / source | Decision, variant, size and parent adaptation |
| --- | --- | --- |
| Catalog | `component::list::ListItem`, `Label`, `Icon`, `button::Button`; current `tools_page.rs` and `currency_settings.rs` | Adopt180x160 official tile composition. Original focused tile owner toggles enable; Configure uses actual Small outline Button and stops propagation. No DataTable/Card/Grid invention. |
| Configuration compound | `component::setting::{Settings,SettingPage,SettingGroup,SettingItem,SettingField}` inspected in `vendor/gpui-component-080/src/setting/{settings,page,group,item}.rs` and `fields/{mod,bool,dropdown,number,element}.rs` | Evaluated first. This is one compact feature surface, not multipage application settings; use the existing approved feature shell + actual Form/Field composition. Do not embed whole custom pages inside one SettingItem or add private selection mirrors. |
| Field layout | `component::form::{Form,Field}`; `form/form.rs::{horizontal,vertical,label_width,children}`, `field.rs::{label_fn,items_center}` | Adopt Form Small for compact row gaps; use existing `compact_tool_field`264DIP label/help and remaining168DIP lane. One outer shell scroll owner, no nested page navigation/card wall. |
| Booleans | `component::switch::Switch` plus existing feature activation row | Medium/default released geometry, controlled requested bool once; callback changes local setting presentation then defers controller persistence. No parent duplicate toggle. |
| Four action selectors | `component::select::{Select,SelectState,SelectEvent}` | Medium/default32DIP, simple short lists, stable entity per field, Confirm only; Escape/cancel does not write. Original owner consumes one request next-turn. No forced Toolbar transformations. |
| Numeric fields | `component::input::{NumberInput,InputState,NumberInputEvent}` | Default32DIP; stable input, explicit integer ranges and one stepping owner. Empty/intermediate typing retained, valid committed value persisted; invalid input shows local validation, never silently saves malformed text. |
| Choose/clear/default actions | `component::button::{Button,ButtonVariants}` | Actual Small outlined buttons outside Toolbar.child; original C# file-picker boundary. Buttons remain recognizable at rest. No extra header commands. |
| Help/status/errors | Existing official Tooltip/Alert/Notification ingress | Optional descriptions adjacent to titles; errors/unsupported source and native state stay visible. Transient operation outcomes use existing notification center; no new toast manager. |
| Ball/media/progress | Public GPUI image/canvas/path APIs; optional actual official ProgressCircle for standard ring, after public render review | Custom image/ball is feature content. No hand-painted substitute for ordinary Button/Switch/Input. Root transparent per-window only; theme is not recolored globally. |

Visual language: inherit active Kit theme colors (`background`, `foreground`, `muted_foreground`, `border`, `primary`) and existing YaHei UI font. Do not invent palette hexes. One memorable element is the user-selected floating image; settings remain quiet and functional. No gradients/decorative animation/new fonts.

```text
Other Tools: [link tile] [currency tile] [floating window tile]

460x320 configuration:
┌ 悬浮窗                              × ┐
│ 启用悬浮窗                    状态 开关│
│ 外观                                 │
│ 默认球 / 当前图片       选择图片  恢复 │
│ 尺寸                           [数字]│
│ 帧率                           [数字]│
│ 下载速度联动                   [开关]│
│ 窗口行为 / 点击 / 拖入 …         ↕   │
└──────────────────────────────────────┘
```
No native budget inflation to fit every field at once. Scroll to bottom must expose image-drop/click-through controls; long paths/errors stay bounded.

## Configuration and defaults (proposed for final approval)
`FloatingAssistantSettings`: enabled=false, custom image absent -> built-in ball; size20..300/default72 logical pixels; frame_rate1..60/default24; synchronize_download_speed=false; base_opacity10..100/default48, hover_opacity10..100/default96; edge_snap_distance4..120/default24; auto_hide_at_edge=true; hide_in_fullscreen=true; position_locked=false; click_through=false; native topmost=true. Persist position with monitor/DPI context; recover inside current work area when monitor is missing. A temporary edge-hidden position must not overwrite the durable revealed anchor.

Action defaults: left=OpenMain, right=AddDownload, text=Automatic, image=Automatic. Display strings exactly requested: `无 / 打开主窗口 / 添加下载`; text `无 / 自动判断 / 添加到下载 / 速启搜索`; image `无 / 自动判断 / 添加到下载 / 写入剪切板`.

One locked update changes only this section, reloads latest disk and publishes after durable success. Concurrent unrelated section writes survive. Save failure preserves last-good live/disk state; persisted desired versus native-applied status are distinct if Windows apply fails. Retain current immediate per-control tool-save policy; configuration close is presentation-only. Built-in/default-image reset changes image selection only, not all preferences.

## Window and input lifecycle
States: Disabled -> PreparingHidden -> Prepared -> FrameReady -> Visible; optional EdgeHidden/FullscreenHidden; Retiring/Failed are explicit. One renderer HWND/controller generation; one internal native input receiver is an implementation detail, not a second independently visible feature or competing renderer.

- Construct renderer hidden/nonactivating/non-taskbar. Install pending phase before dispatch. C# validates pipe-peer desktop PID, both source/renderer handles, process/session and dimensions. Native preparation computes final device pixels and readbacks while hidden.
- GPUI renders final-size scene after leases; explicit public draw then next-turn reveal, following existing passive-preview discipline. No timeout-as-ready, show-first or competing GPUI resize.
- Receiver feasibility is proven only by owned synthetic HWND/COM/OLE fixtures; actual GPUI opacity/physical browser/Shell/mixed-DPI and user acceptance remain unverified. One C# STA/pump owns admitted OLE input. No cross-process WndProc injection, arbitrary callback pointers, global hooks, GPUI vendor change or cross-process reparent.
- Receiver must exactly track renderer lifetime/geometry/shape/visibility/Z-order/DPI. No full-screen transparent catcher; zero-alpha empty areas cannot intercept unintended input. Pass-through disables native receiver hit testing and native renderer interception together. Noactivate remains even when click-through is false.
- C# supplies fixed monitor-work-area/DPI/foreground/fullscreen snapshots; Rust decides clamp/snap/hide. Native drag computes/updates physical placement locally to avoid per-move IPC; send coalesced terminal bounds only. Rust persists acknowledged final anchor on release, never logs pointer ticks.
- Left drag versus click shares one native gesture owner with immutable session. Threshold uses system-scaled movement and no click after capture loss/drag; position lock disables moving, not chosen click actions. Right release dispatches chosen action exactly once instead of legacy context menu. Disable/configuration remains in Other Tools rather than adding a second native overlay button-input path.
- Click-through means no left/right click, drag, wheel or incoming drop; settings remains accessible from main Other Tools. Enabling it reveals an edge-hidden ball first so it cannot become permanently unrecoverable at an invisible edge. Native flags/result must be read back; config persistence alone is not successful pass-through.
- Wire edge_hidden defaults false. Prepare and ordinary apply fully clamp; only explicit subsequent edge-hidden placement may retain8device pixels. Changed monitor/work-area/DPI/fullscreen signatures are delivery-aware at the existing bounded observer cadence; Rust reveals/reapplies before continuing.
- Disable/hide cancels input/drop admission and animation; reuse renderer safely. Pipe disconnect/source destroy/retire hides receiver, revokes OLE, releases staged files and prevents old callbacks revealing/reusing replacement HWND. Explicit application exit disposes both.
- One bounded native foreground observer/session determines fullscreen state, reusing existing native event ownership when possible. No per-frame Windows probes or downloads polling.

## Media and frame schedule
Use existing pinned image crate with explicitly enabled codecs. File picker accepts PNG/JPG/JPEG/BMP/WebP/GIF; reject remote asset URLs and unsupported/corrupt data explicitly. Decode outside render with generation cancellation. Proposed budgets: source<=16MiB, dimension<=4096 per axis, decoded source<=16M pixels, GIF<=512frames and<=64MiB retained output. Downscale retained frames to max300logical pixels at bounded supported scale; prevent overflow before allocations. Test GIF disposal/transparency/dependent frames with real fixtures. No all-frames PNG re-encode loop or render-time disk/decode.

Frame rate is an explicit1..60 maximum display rate; retain GIF frame timing (source zero-delay normalized safely), do not silently impose legacy6FPS. A single monotonic playback timeline and next-frame task select the currently due frame; skip elapsed frames rather than queue catch-up work. Display remains capped60 even under4x. Static/single-frame assets have no playback timer.

Confirmed speed mapping: `factor=clamp(total_downloading_bytes_per_second/1048576,0.25,4.0)` for positive speed. Currently Downloading tasks with zero aggregate speed pause the GIF. No currently Downloading tasks, or sync disabled, uses1x baseline. Queued/probing/finalizing/paused/completed state never contributes stale transfer speed. Snapshot comes from existing DownloadApplication live guarded state; one immutable projection is published next-turn at bounded/coalesced cadence, not a new engine event subscription/tick log. Keep weighted progress/count separate from speed's narrower downloading filter. Paused/fullscreen/edge-hidden/disabled media schedule no frames. OS reduced-motion/unknown policy freezes nonessential animation without changing download state; retain chosen preference so it resumes when allowed.

## Drop classification and action reuse
One `drop_id`/session/event, accepted once. DragEnter only queries bounded formats; do not decode arbitrary data or perform network/clipboard writes until Drop. Snapshot native IDataObject data while valid; bounded fixed ownership and cleanup.

Accepted sources: Unicode text; HTTP(S) URI formats; bounded CF_HTML image-source metadata; PNG/DIBV5/bitmap image; local image files. Non-image files/directories are unsupported in this scope. Recognized image representation wins over generic alt text; prefer actual image URL from typed URL/HTML image source, not a surrounding link URL or arbitrary local-path string. No OCR, HTML execution, script interpretation or implicit network metadata fetch. `blob:`, `data:`, `file:` are not direct downloader URLs. Multiple candidate links use the existing bounded normal download handoff policy; unsupported/mixed formats produce a clear result, not repeated actions.

- Text None: no dispatch. Automatic: valid downloadable HTTP(S) -> normal Add Download prefill/probe; otherwise -> Quick Launch initial search text. Add to Download: only valid source, otherwise explain why. Search: whole text into original search owner, no automatic result execution.
- Image None: no dispatch. Automatic: valid remote image URL -> normal Add Download; local/bitmap with no remote source -> copy image. Add to Download without supported source URL: explicit unsupported reason. Copy Image: use supplied bounded actual image/local file through existing native clipboard image-copy operation, no implicit download merely to copy URL-only payload; missing image bytes is an honest failure.
- Add Download always requires the existing confirmation; never calls raw submit or silently starts tasks. Click Add Download opens the ordinary blank form.
- Main action uses normal restore/show/activate lifecycle. Search uses internal `show_with_query`, preserving original target/cursor/native activate/IME/query generation; no new global hotkey or wire extension needed solely for internal prefill.
- All cross-entity calls are deferred. Recheck session/generation/enabled at dispatch; reject late events. Dedupe prevents browser-provided multiple representations from writing Clipboard and opening Download together.
- One pending action bounds all effects. Disable/click-through/retry cancels deferred/preparation work and releases native drop staging; an already admitted native copy retains normalized image through actual result/deadline, because cancellation cannot retract Windows work. Retired results suppress UI notices. Main show must succeed before correlated activation.

## Protocol, bounds and diagnostics
Versioned feature DTO module under existing outer protocol1. Commands conceptually Prepare/Update/Visibility/Retire; events include terminal click/drop/move and native lifecycle state. Carry session UUID, generation, source HWND, renderer HWND, sequence/correlation; C# matches peer PID instead of trusting user-supplied PID. Distinct phase IDs share original operation correlation. Retire admission ordered before replacement; accepted is not applied.

Keep outer1MiB IPC frame. Text<=64KiB, HTML<=256KiB, source URL<=8192bytes, one image drop operation, bounded staged image<=16MiB under an owned per-operation temporary directory. Image bytes never inflate IPC base64; use validated managed staging token/path with length/type and cleanup, not arbitrary file deletion authority. Only C# captures Windows formats/clipboard. Proposed bounded1active+1queued decode/drop jobs; reject busy and clean exact owned payloads, no unlimited thread replacement. Concrete wire limits are executable shared contracts before implementation.

Events: `floating_assistant.settings`(accepted/committed/failed), `.window`(prepare/ready/visible/hidden/retired/failed), `.asset`(load/start/ready/rejected/failed), `.action`(accepted/routed/terminal), native `.drop`(accepted/rejected/terminal). One correlation across Rust/IPC/C# plus session/generation/phase/elapsed/native error at owning boundary. Fixed reason/kind/bytecount/framecount only; no URLs, image/path/clipboard/text/window titles, per-frame/pointer/download-byte logs. Existing bounded/nonblocking sinks and notification ingress; diagnostics failure never changes operation result.

## Risk, rollout and rollback
Primary risk is native receiving-layer feasibility/transparent input geometry. It is an implementation prerequisite explicitly deferred to a safe owned experiment, not a license to reduce requested behavior or patch the framework. If it fails, stop at this boundary and seek a bounded architecture decision with evidence. Same pinned image codec-feature expansion is the only planned dependency change.

Default feature off permits additive rollout. Disabling retires native/media work without affecting downloads/clipboard/search. No package/install/user data/service operations are part of implementation approval. Rollback removes only task-owned changes after reviewing dirty hunks; never git reset unrelated work. Preserve user configuration section if a temporary implementation is retired; do not silently delete data.
