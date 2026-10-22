# Contextual UX refinement and example packages — 2026-10-15

## Delivered source behavior
Authorization: [ux-refinement.md](ux-refinement.md). This follows installed sequence9 feedback; earlier rejected galleries remain rejected. The existing signed sequence9 installer was not rebuilt or overwritten in this turn.

### Clipboard menu and actions
- Production Clipboard context menu is now a genuinely **OS-rendered popup outside the source HWND**, not a larger in-window layer or overflow-visible workaround. It uses the same source menu projection and original Clipboard callbacks; C# owns Windows menu APIs, Rust owns IDs/tree/callbacks/generations. Official Kit0.7.1 NativeMenu capability was reviewed, but its direct Rust Windows API boundary was not adopted against project ownership. This narrow native-menu surface is deliberately system-styled, **not claimed to be styled GPUI PopupMenu**.
- Call chain: history_row -> project_menu::MenuTrigger -> native_context_menu::open -> ShowContextMenuCommand -> existing PlatformPipeClient typed parser/peer PID check -> WindowsPlatformAdapter -> NativeContextMenuCoordinator dedicated STA -> worker-owned WS_POPUP tool HWND owned by source root -> CreatePopupMenu/AppendMenuW/TrackPopupMenuEx -> one correlated ContextMenuResult -> deferred original source-window callback.
- One native menu globally; admission rejects busy without queueing. Source HWND/PID/visible root and current interactive session checked, source PID bound to actual pipe-server PID. Root128/total256/depth4/label256 UTF16/total32768 limits; positive unique leaf IDs, no callback for disabled branches, no labels/content in diagnostics.
- Exact session cancel/disconnect/deadline retain native slot until actual menu/HWND cleanup;120s native lifetime,125s Rust presentation watchdog. OS handles cascade placement, screen edges and native menu navigation. Source foreground/pointer hide is guarded only while its menu is pending. Hide retires/cancels that menu; outside dismissal closes source, cancellation/Escape keeps it. Trigger disappearance and old/duplicate/unknown results do not dispatch; business callback is deferred until the native menu session is retired.
- Formal production routes native only. Disconnected GPUI fixtures retain the historical in-window project-menu fallback for deterministic action/keyboard testing; those tests do **not** prove real external pixels. Source gate and test name now distinguish that fallback from current production policy.
- “扩展处理” is a cascading submenu listing actual enabled/trusted manual actions; selecting a leaf invokes the original runtime directly. The old modal chooser and its Clipboard enum path were removed. No guessed input/automatic trust/automatic execution.

### Compact feedback and catalog
- Clipboard manual action starts a compact **same-panel** feedback region above history, with action/status and explicit Cancel/View Result/Details/dismiss as applicable. It does not automatically open a report Dialog or a new result HWND. Dismissing feedback is presentation-only; cancellation remains per-run and retains pending until runtime terminal.
- Host-confirmed derived IDs remain the only result evidence; explicit navigation still uses original worker/current panel/session/query guards and never copies/pastes/replays. Late commits remain observable. Multiple results use bounded paging.
- Explicit details/recent-activity drill-in still use official Dialog, but content height is natural with a maximum, not fixed360px. Removed the repeated “结果与关闭说明” Accordion. Plugin-provided explanation remains labelled; outcome unknown/cancel is not rollback.
- Catalog retains official ListItem/Icon/Label/Tag/outlined Button cells and two-column layout, but removes252px minimum/full-height/flex filler. Short content no longer creates forced empty areas. Header outlines/official keyboard roving and original Settings pages/drafts/trust/import/runtime/events/chains remain.
- Downloads keeps official PopupMenu. Current normal artifact/transfer/queue operations precede an “扩展动作” submenu; destructive actions follow it. Concrete leaf executes directly; service failure is explicit, not another chooser. Existing task selection/current capabilities and engine/native owners remain.

## Regression findings and fixes
1. Actual render red before sizing changes: short catalog252px and short report360px violated compact-content assertions. Same tests green after removal of forced dimensions.
2. Native-menu callback regression red: disabled parent did not disable child callback projection. Fixed ancestor eligibility propagation through the entire wire tree; fake selected disabled leaf cannot dispatch.
3. Real owned HWND acceptance initially failed by timeout. Temporary DEBUG-NCM probes proved native #32768 popup already exceeded source bounds, but Tracking stayed false because TPM_NONOTIFY did not deliver the assumed WM_ENTERMENULOOP notification. Exact cancellation therefore ignored EndMenu. Fixed Tracking ownership at the actual TrackPopupMenuEx boundary; removed dependence on the notification and all temporary probes.
4. Posted HWND keyboard messages did not reliably drive the system menu cascade; test now uses **UI Automation ExpandCollapsePattern on the self-owned native menu**. This is real provider-driven cascade proof, not physical keyboard acceptance. Final clean test proves overflow, a second menu HWND, source still visible, wrong-cancel ignored and exact cancel cleanup.
5. Authoring/import/borrow/fixture-method errors and redundant-let lint findings were fixed but are not claimed as behavioral-red evidence. No warning suppression or dependency upgrade.

## Examples delivered
Three standalone Python3.10+ stdlib JSONL packages, Chinese instructions, strict bounded protocol helper and tests. Generated deterministic ZIPs are in [artifacts/extension-examples](../../../artifacts/extension-examples/):

| ZIP | Manual/resolve behavior | Bytes | SHA256 |
| --- | --- | --- | --- |
| github-public-proxy.zip | Public GitHub release/archive/raw URL prefix resolver; explicit third_party_opt_in=false default, configurable HTTPS prefix, no HTTP/credential access or forwarding |21217|f98a76dabdd0bc2fe0dee90f45c77d2e4485e9e85e3c4ec4d3cfa303f0f11494|
| download-link-list.zip | Copy selected tasks' original URLs as lines or Markdown,1–32 UUIDs, complete validation then one common.copy; no task changes |17365|0cdc050a114701651e20ffcecf4c260fffa31d6801afbd796d1a1ddc9e91d9fd|
| clipboard-text-tools.zip | Ordered duplicate-line removal, strict bounded JSON formatting, conservative percent decode; one create_derived only if changed/nonblank, original retained, no copy/paste |17482|317ddc9a9b33d1df2e45f31bb0d1538bb0f83edc168d99f1008693e57d80e8f4|

Source/how to run: [examples README](../../../docs/extensions/examples/README.md). Upstream `hunshcn/gh-proxy` README was fetched/read: documents prefixing public download URLs and warns demo overload. `https://ghproxy.net/` is a configurable example only; no domestic availability, speed, safety or uptime claim. Mirrors can see URLs/content; independently verify upstream signatures/checksums. Conservative URL/credential validation cannot prove remote public visibility or trusted DNS/redirects. No external download/proxy request was made. Download-list URLs can still contain query secrets; share explicitly and inspect first. Whole-user trust is not a sandbox.

## Verification actually performed
All Cargo commands serialized with Rust1.96/jobs1/incremental0/codegen-units256. Final scratch logs:

| Check | Result | Log |
| --- | --- | --- |
| Full Desktop | **759/759**,0failed/ignored,95.80s |ux-desktop-final.log|
| Protocol library, including new context-menu tree/result/cancel strict roundtrip | **47/47** |ux-protocol-final.log|
| Actual three example directory/ZIP previews, immutable stage, disabled/untrusted defaults and trust-required enablement | **1/1** integration (covers3packages/6handlers) |ux-example-import.log|
| Python package tests | **29/29** (10+9+10) |extension-examples-REPORT.md/direct logs|
| Python aggregate/fake-host/deterministic ZIP tests | **5/5**, includes rerun of29 package tests |ux-examples-parent-tests.log|
| Strict Clippy Desktop/Protocol/Extensions --tests -D warnings | PASS |ux-strict-final.log|
| Workspace --tests check | PASS |ux-workspace-final.log|
| fmt all --check | PASS |ux-fmt-final.log|
| Debug production preview build | PASS; original unused ClipboardApplication::extension_adapter warning only |ux-preview-final.log|
| C# build |0warnings/0errors |ux-csharp-build-final.log|
| Full safe C# fixtures | **252selected/run/pass**,0skip/fail |ux-csharp-fixtures-full.log|
| Explicit owned native popup test | **1selected/run/pass**,0skip/fail, clean probes removed |ux-native-owned-final.log|
| Clipboard/UI-state/preview-isolation/Desktop-shell/Download lifecycle gates |5PASS; source only |ux-gates-final.log|

Final spec/catalog/scoped diff/UTF8/JSON/artifact identity checks are recorded in ux-handoff-final.log. Existing Clipboard44 first-slice suite is historical, not a new full Clipboard run in this turn. No unrestricted real-volume tests or native/service release gate implied.

## Captures and remaining genuine blocker
- New gallery: [artifacts/extension-contextual-ux-ui/index.html](../../../artifacts/extension-contextual-ux-ui/index.html).10 actual owned96DPI Debug GPUI PNGs: Light/Dark1060x650 catalog,920x480 catalog/detail,300x440 inline pending/saved feedback. Scratch config/SQLite/history and synthetic run records, disconnected bridges/no real plugin execution. Exact previewPID/HWND/client size/hash checked; final sampled colors71–173 (nonblank only).
- **Direct visual inspection NOT completed**: BrowserPreview requested the gallery, screenshot tool again returned “Browser view did not become visible for the calling session. Switch to that session and retry.” Do not call generated screenshots inspected or visually accepted. Native overflow test is independent geometry/lifecycle/provider evidence, not aesthetic certification.
- Real foreign-process GPUI -> PlatformHost installed menu interaction, physical mouse/keyboard/outside-click timing, mixed-DPI/work-area edge placement, full screen reader/product-tree and real trusted plugin acceptance are not certified by the same-process owned HWND fixture. No user visual acceptance claimed and no numerical10/10 score invented.

## Non-actions / release boundary
No release packaging/signing/install/deployment/service restart, user plugin import/trust/enable/run, real clipboard/history/credential mutation, network download, commit/push/archive. sequence9 installer SHA remains `25562C761F6A3B68B0E7DD911F23FC474AC8146F223E58BE2F1BA45470B6D89C`; it does not contain this later refinement. Native fixture only showed its own temporary windows. W10 remains deferred. Task in review pending visual/installed acceptance.
