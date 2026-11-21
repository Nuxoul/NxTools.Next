# Verification

## Clipboard

The two production frame regressions were observed rejecting the original presentation: held statistics replaced the metadata width172->60logical pixels; pending state changed the status width29->39. Those are TestWindow measurements, not Win10 pixel measurements.

After the repair:

- Clipboard page migration/settings group:17pass/0fail/0ignored, including the two real Switch/held-worker frame regressions and snapshot-state projection.
- Original pointer/Space requested-value/persistence test:1pass.
- Original settled-surface motion test:1pass.
- Total19unique related Desktop tests passed. Both motion preference modes run inside the new frame cases.
- Counts use temporary SQLite and the original worker; native configuration uses fake transport and matching receipts. No live clipboard, global input, product launch, Registry/service mutation, package/install or installed replacement.

Commands use one Cargo process with CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256:

```powershell
cargo test --locked -p nxtools-desktop --bin nxtools-desktop main_window::clipboard_page::kit_migration_tests:: -- --test-threads=1
cargo test --locked -p nxtools-desktop --bin nxtools-desktop feature_total_switches_persist_one_requested_value_per_pointer_and_key -- --test-threads=1
cargo test --locked -p nxtools-desktop --bin nxtools-desktop window_feedback_settled_surface_does_not_reenter_on_unrelated_refresh -- --test-threads=1
```

## Search research and browser proposal

Microsoft Win11 rounding, DirectComposition clipping/RectangleClip requirements and the Tauri Win10 shadow issue were searched and extracted. Exact locked InputGroup, Root/WindowBorder and both Windows swap-chain paths were read. The host is Windows11build22631, so no local Win10 acceptance is claimed.

Browser proposal loaded the existing Sarasa UI SC Regular asset. Actual computed style:620x60 panel,8px radius,1px outer border,0px inner input border,20/28px type and no horizontal overflow. Deep/light theme switching and input-value behavior were checked; the dark screenshot was inspected. Browser rendering does not certify native HWND/compositor clipping, shadow, IME or DPI.

## Source checks

Spec navigation/contracts, official component catalog, coherent Kit0.8.0/pre0.3.9 provenance and pinned editor contracts passed. Task JSON/JSONL parsed; scoped rustfmt and whitespace checks passed. No framework source, dependency or production search styling changed in this task.

## Standalone GPUI example

`scripts/preview-quick-launch.ps1 -TestOnly` passes2/2tests,0failed/ignored. The same example Render runs through real Base Root and official InputGroup/InputState. Assertions cover620x60contour/all8DIPcorners, opaque current-theme fill/no opaque external fill, control containment, addon pointer focus, stable entity/text through F2 and Escape window retirement. The opaque-fill assertion was observed rejecting the released group's translucent backing before the public popover-background adaptation.

`-BuildOnly` passes; `cargo clippy --locked -p nxtools-desktop --example quick_launch_surface -- -D warnings` passes. C# preview helper compilation and PowerShell syntax pass. Required single-job/nonincremental/codegen256variables were set for every Rust run. Only the example was built, not a package or normal runtime.

Real Windows11build22631/100% preview client readback is652x92, including16DIPshadow gutter around the620x60bar. Existing GPUI composition remains active with WS_EX_NOREDIRECTIONBITMAP. The runner's C# helper matches the new PID/exact preview title, applies the original SetWindowShadow(false) policy and reads DWMWA_NCRENDERING_ENABLED=false. Native text entry and F2 switched to Light while retaining GPUI123. Actual white-backdrop [dark](preview-dark-white.png)/[light](preview-light-white.png) captures were inspected. The preview exited through normal window close; the separate self-owned backdrop was closed normally. These operations did not target the installed NxTools/PlatformHost/service.

User accepts the overall shape but reports white-background stair-stepping. The [nearest-neighbor detail](preview-corner-detail.png) and sampled RGB show intermediate edge coverage; exact Windows shaders.hlsl::quad_fragment uses the fixed0.5SDFthreshold. The remaining edge quality is not marked repaired. No native region is applied, and no vendor file is changed. The [run guide](../../../docs/ui/quick-launch-gpui-preview.md) records commands and limits.

Spec navigation, component catalog, coherent GPUI provenance and pinned editor gates pass for this example slice. The original Clipboard test totals and unrelated product Clippy findings above remain distinct.

## Upstream source-over candidate

Online research confirmed merged ZedPR65571/e18de65b and its exact two Windows DestBlendAlpha assignments. Microsoft source-over/premultiplied-alpha documentation supports the same formula. The [research](source-over-research.md) records sources, active factory/pipeline/swap-chain flow and excluded glass/SDF alternatives.

The native regression was observed RED with actual ID3D11BlendState::GetDesc returning ONE(2), where source-over requires INV_SRC_ALPHA(6). The temporary backend with the two upstream changes passes1/1native regression,0failed/ignored; the original same-Render example tests pass2/2,0failed/ignored. Candidate Debug build passes. The production vendor/manifest/lock are unchanged; the ordinary preview executable was restored and verified against its baseline by direct bounded byte comparison.

Actual Windows11/100% candidate client remains652x92 and DWM non-client rendering reads disabled/native result0. The [white-backdrop capture](source-over-dark-white.png) and [nearest-neighbor corner comparison](source-over-corner-comparison.png) were inspected. Same silhouette, font, palette and shadow; the dark edge fringe is lighter. One corner sample changes RGB192/185/186 to208/201/202. These pixels demonstrate composition improvement, not complete elimination of spatial aliasing. Candidate native Edit value confirmed the submitted synthetic text. F2/text/entity retention is covered by the two component tests; no additional physical-keyboard/IME claim is made.

Candidate and backdrop normal-close checks pass. `scripts/preview-quick-launch-source-over.ps1 -NoBuild` opens the separate candidate without Cargo; first build runs the native and component regressions. Source reconstruction remains temporary, with only the two upstream runtime assignments and a cfg(test)-only module seam. No shader, version, production adapter, package/install, installed-runtime, service or Registry change.

## Direction-aware coverage candidate

The user rejects the remaining dark corner. The [pixel-footprint method](pixel-coverage.md) retains source-over and changes only the temporary quad-shader arc coverage. The official InputGroup and original geometry/type/palette/shadow remain. No checked-in vendor or dependency change.

The example's public Window::render_to_image path exports actual Windows GPU frames, including raw premultiplied data and a correctly composited white image. The fixed-ramp baseline fails the arc fixture with alpha9 at(17,18). The directional candidate produces70at both upper samples and75at both lower samples, inside the footprint-derived source-over bounds. Both dark and light pixel checks pass: straight edge/interior samples are byte-exact, viewport/DPI identical, body opaque and outer gutter transparent. Coverage upper bounds also reject filled/squared-off corners.

The native blend-state regression passes1/1 and component/input/close regressions pass2/2; candidate Debug build and strict example Clippy pass. Actual GPU captures succeed for both themes and both backends. A capture-to-file negative fixture reaches stage=directory and exits unsuccessfully; success is published only after all readback/export outputs complete. No stale capture success is accepted.

[Full-frame comparison](pixel-coverage-comparison.png), [corner enlargement](pixel-coverage-corner-comparison.png), [dark GPU frame](pixel-coverage-dark-gpu.png), [light GPU frame](pixel-coverage-light-gpu.png) were inspected. Curved-edge transitions are softer and the dark corner's abrupt point is reduced; this is not user visual or Win10 acceptance. `-PixelCoverage -NoBuild` opens the separate native candidate through the original preview C# frame helper.

## Under-border compensation

Temporary core0.3.9 and Windows0.3.9 overrides are selected through a standalone manifest, sharing the actual example source. The original SDF ramp and source-over remain; no directional widening is combined. The background-only marker retains border data, uses the derived precompensation and a defined fully-covered branch, then returns without applying outer coverage again. Border-strip masks and child order remain.

The source-over-only opaque/no-shadow baseline fails the full-corner oracle: maximum RGBA channel error61.452/255. The candidate passes all256corner pixels in both themes with maximum error0.784Dark/0.641Light, below3/255 tolerance. Non-arc samples, viewport/DPI and transparent gutter checks pass. Normal shadow-enabled [full-frame](under-border-comparison.png), [corner](under-border-corner-comparison.png), [Dark](under-border-dark-gpu.png) and [Light](under-border-light-gpu.png) GPU captures were inspected; the outer dark-fill fringe is removed in this scene.

Native blend-state1/1, component/input/close2/2, Debug build and strict candidate Clippy pass. Fresh private-lock bootstrap selects only the intended pinned temporary core/Windows sources; root Cargo files and checked-in vendor are unchanged. The [scope](under-border.md) excludes generic translucent/opacity/child/dashed/platform certification. `-UnderBorder -NoBuild` uses the separate native preview executable and original C# frame helper, with the original shadow.

## Limitations and existing findings

Win10's real GPUI square-shell cause and final native pixels remain unproved. Native region/shadow/render-path A/B validation and user visual acceptance are still required. The single-surface proposal is not implemented in product code; a production framework amendment requires explicit approval.

The Clipboard repair establishes statistics/text-layout stability, not actual native glyph opacity across every compositor. Pending/confirmed/error text remains semantically distinct. Desktop production strict Clippy still reports the same five existing findings in About emblem, Extensions activity, Markdown history/settings/mod; no new diagnostic or warning suppression was introduced. No commit was performed.
