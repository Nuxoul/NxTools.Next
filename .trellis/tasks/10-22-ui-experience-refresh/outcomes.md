# Six-item implementation outcome — 2026-10-22

## Outcome
Implemented the exact approved snapshot `.pi/plan/统一-markdown-组件与六项体验改进-20261022-1342.md`, plus the separately approved three stable-Rust cull.rs substitutions. No delegation, contact with other sessions, commit, archival, package, installation, service operation, formal Runtime/PlatformHost launch, or real clipboard/history/config mutation was performed. The unrelated dirty tree and concurrent Archive work were preserved.

1. **Notification pacing:** actual production options now explicitly use inactive_frame_interval=None. Official Toast remains the animation/timeout/hover/close owner; native prepare/readback/final-frame/reveal and exit-before-hide remain. Added bounded Debug paint sampling and an explicitly authorized scratch relay.
2. **Unified Markdown:** pinned Apache-2.0 editor v0.1.0 at31608fa74bdbeca79d9901bf2daa48ea7ecf1ecc across Clipboard list/full preview/explicit edit and Quick Launch text/Markdown preview. Passive mode does not take input/focus. Original payload, safe budgets and source mode remain independent. No-edit/cancel preserve raw bytes; rich save may normalize with notice. Save identity protects duplicate, stale/reopened and newer drafts; failure keeps draft.
3. **Image modes:** Auto(default), Fit, Panorama, Light, Mini, Compact; real work-area and DPI-aware geometry, only overflow-axis scroll, Fit reset, short-window-edge800/500/300devicepx caps. Existing single passive HWND and staged generations retained.
4. **Quick Launch gaps:** default Separated4DIP and optional Continuous0, external to the40DIP hit target; same result/recent/action owners. Durable field-only setting, no indexing/native configuration.
5. **Probe latency:** reproduced1.0030359s slow-HEAD wait. A usable response now collects complementary metadata for350ms, preserving timely HEAD-only filename and conservative resume. An additional red caught empty200HEAD discarding necessary Range metadata; corrected by usable-metadata admission. Slow manifests and actual HTTP-stage cancellation covered. Production network routing unchanged.
6. **Graphite themes:** new project light/dark configurations registered/applied through public Kit APIs; complete state/semantic palette, retained official optional themes, fonts/radius/geometry and source-icon hue. Design comparison precedes actual GPUI capture review.

## Compiled verification
Logs are under session scratch `70da3267-15fc-4e10-b17c-2d9c0bf62e48`.

| Check | Result | Log |
| --- | --- | --- |
| Desktop full serial suite | **1002 passed,0 failed** | experience-final-desktop.log |
| Editor full library | **234 passed,0 failed** | experience-final-editor.log |
| Core library | **73 passed,0 failed** | experience-core.log / experience-postfmt-core.log |
| Network actual local route/snapshot tests | **3 passed,0 failed** | experience-network.log |
| Download library | **117 passed,0 failed** | experience-final-download.log |
| Download engine serial contract | **103 passed,0 failed** | experience-final-download.log |
| HTTP contract | **26 passed,0 failed** | experience-final-download.log |
| Resolution contract | **15 passed,0 failed** | experience-final-download.log |
| Other Download contract binaries | all passed | experience-final-download.log |
| Strict affected project Clippy --tests --no-deps -D warnings | passed | experience-final-clippy.log |
| cargo check --workspace --tests | passed | experience-workspace.log |
| Debug fixture build | passed, no warnings | experience-debug-fixture-build.log |

Focused evidence: Clipboard UI201, QuickLaunch153, notifications35, Markdown14, theme11+optional1, image modes5, row spacing2. Final scoped formatting passed; rustfmt introduced trailing punctuation as well as whitespace, so post-format ClipboardUI201/QuickLaunch153/Core73 were actually rerun and passed rather than claiming byte identity. All Cargo commands inherited jobs1/incremental0/codegen-units256 and checked occupancy; occupied attempts75 did not run tests.

### Genuine lint limitation
Unrestricted dependency-inclusive strict Clippy stops at **unchanged upstream** `vendor/gpui-kit-markdown-editor-010/src/serialize.rs:215`, `clippy::nonminimal_bool`. That file is outside the approved patch scope. It was not modified and no allow/suppression was added. Strict project-package --no-deps lint passes. This is not an entirely green dependency-inclusive lint claim.

## Isolated native and screenshot evidence
- `tests/native/notification-cadence.ps1 -AuthorizeNative` executed the checked-in scratch-only relay successfully. Evidence directory: `notification-cadence-c61da9a8615342309e0cfd3d617e52b6` in scratch. Reported180Hz/96DPI:
  - legacy entry13paints, median34.131ms; exit9paints/33.483ms;
  - current entry79paints, median5.484ms; exit39paints/5.486ms;
  - idle paints0 in both; current hide follows official exit (~275.77ms from dismiss);
  - reduced-motion entry1paint, exit2settlement paints, idle0;
  - native client readback successful and foreground unchanged.
- These are paint-cadence observations, not GPU-present timing or perceived smoothness certification. Initial reduced fixture omitted App::set_reduce_motion; that fixture bug was corrected, then the final samples above were collected.
- Exact existing owned native Quick and Clipboard acceptance filters each passed2/2, skip0: passive WM_MOUSEACTIVATE3, ordinary primary1, native client dimensions/readback, synthetic primary Edit/Button UIA and unchanged foreground at96DPI. This does not certify the complete Markdown accessibility tree or physical pointer handling.
-16 owned GPUI PrintWindow captures at96DPI: [actual captures](../../../docs/audits/2026-10-22-ui-experience-captures/index.html). AI inspected real dark/light results, actions and Markdown title/list/table/code surfaces through the browser screenshot. Continuous/separated result PNG hashes differ. [Earlier design-only state board](../../../docs/ui/graphite-comparison.html) is not a GPUI render.

## Static/source checks
Passed spec109documents/14owners, historicalcatalog79/78consistency, GPUI release source/upstream/active-tree provenance, editor57-file/2approved-delta provenance, notification options and both112-role theme schema/contrast gates. Enabled Windows tree contains Kit0.8/pre0.3.9 and no Fast/syntect/all-language feature. Eight lifecycle gates passed with explicit UTF-8: Clipboard, ClipboardUI, UIState, DesktopShell, Motion, QuickLaunch, PreviewIsolation, DownloadUI; token-boundary and UI-consistency gates also passed. Two source assertions were migrated from obsolete Textarea-in-mod/old text-layout tokens to actual unified document/source/save-identity boundaries, retaining fixed shell/native/action checks.

## Remaining acceptance/reproduction limits
- No physical keyboard/pointer, real Windows IME,125/150% or mixed-monitor DPI, full screen-reader, long soak, user visual approval or installed/package acceptance.
- Native image-mode/physical scroll matrix is not certified by pure geometry and synthetic GPUI tests.
- Direct/System(injected)/Manual tests prove local client branch behavior, not Windows registry/PAC discovery or real external proxy hits. No system settings were inspected or changed.
- User supplied no specific slow external resource. The reproduced local wait is repaired; external resource discovery/resolver/proxy latency remains unproven and needs a redacted reproducible sample if still observed.
- Source/compiled completion is not deployment. No automatic archive or commit.
