# Archive browser icon/layout repair — isolated presentation slice

## Scope and prerequisites

2026-10-28 user correction: the latest browser screenshot effect is rejected. That screenshot was not supplied in this delegated turn; no screenshot inspection, toolbar-overflow finding or visual acceptance is claimed. This is stabilization of the original archive-browser presentation, not a new phase or HZ1 implementation. Latest checkpoint, original PRD/design/implement/ui-spec, approved-icon-selection, current AGENTS/shared/Desktop UI contracts and exact locked Component0.8.0 Tab/TabBar/Tree/ListItem plus Base0.8.0 Tree and pre0.3.9 img sources were read before changes. Historical0.7.1 ledger pins do not override current0.8.0.

Round-start copies of browser.rs/browser_host.rs/titlebar.rs/assets.rs are retained in session scratch `archive-icon-round-start/`; pre-existing user changes remain intact. No checkpoint/shared reference edits are owned by this slice.

## Component selection / requested shape / invariants (before implementation)

- One original34DIP client caption and unchanged native/minimize/maximize/close ownership. Evidence: current WindowCaption drag region contains only text (titlebar.rs209), so a browser-only public prefix composition is needed to display the approved blue application image; native PE/window/taskbar blue ownership stays unchanged. Caption image: fixed20DIP square, complete aspect-preserving canvas, nonshrinking,8DIP text gap, existing12DIP caption inset.
- Official `component::tab::{TabBar,Tab}` default Tab variant, Medium32DIP; keep label metadata, official label ellipsis,220DIP max width, original menu/scroll/selection and XSmall ghost20DIP close Button. Public prefix/suffix only: fixed16DIP archive image;12DIP symmetric outer inset and8DIP icon/text/close gaps. Released Tab has a4DIP outer gap but12DIP INNER label padding, while prefix/suffix sit OUTSIDE that padding (tab.rs859–900,1048–1063). Compensate that parent padding only inside public slot layout: prefix margin-left12/right-8, suffix left-8/right12. Negative margins consume exactly8 of the otherwise duplicated label padding; image/text/close never overlap. No repaint/replacement/vendor change. TabBar still supplies variant/size/max_width and owns overflow menu/reveal/wheel. Exact Base0.8.0 tabs.rs15–21 explicitly has no built-in tab roving/arrow keyboard owner; this slice preserves that existing limitation rather than inventing a keyboard controller. The original real close Button Tab/Space-release behavior is tested.
- Official `Tree::new(... -> ListItem)` with original32DIP rows and official selection/hover paint. Tree icon slot20DIP root/directories (root visible17.5x15DIP);16DIP arrow,16DIP per-depth indent and4DIP inter-slot gaps. Keep independent arrow/icon/label lanes, original ListItem12DIP horizontal inset, fixed nonshrinking image containers and explicit Contain. Preserve held/native drag callbacks on icon/text and root wrapper, Tree outer click/expand and deferred navigation; right list/marquee untouched. At960x640 measured root icon(x32,y228.5,20x20), directory icon(x48,y260.5,20x20); root text(x56,y225.5,132x26), center aligned at238.5. The surrounding ListItem type/line-height is unchanged.
- Original ArchiveApplication/browser-host activation, password.rs WAC focus, preview/read/entry/source/lease/cleanup and next-turn cross-entity boundaries unchanged. No new behavior, logs, timers, native commands, controllers or titlebars. Existing diagnostic correlation owners suffice for presentation-only composition.

## Alpha/canvas investigation (unchanged resources)

Approved originals both1254x1254 RGBA:

| Asset | nonzero-alpha bounds (exclusive right/bottom) | alpha>=8 bounds | alpha>=128 bounds |
| --- | --- | --- | --- |
| Blue application original | (97,122,1206,1143) | (105,124,1149,1138) | (106,125,1148,1137) |
| Orange archive original | (47,0,1224,1254) | (95,163,1160,1089) | (95,164,1159,1088) |
| Existing blue32 canvas | (2,2,30,29) | same | same |
| Existing archive32 canvas | (2,4,30,28) | same | same |
| Existing blue40 canvas | (2,3,37,37) | same | same |
| Existing archive40 canvas | (2,5,37,35) | same | same |

The generator already discards alpha<8 speckles, crops content and fits7/8 occupancy proportionally. Thus original archive nonzero bounds misleadingly touch canvas edges, but existing derivatives already have genuine transparent margins. No resource regeneration/repainting/cropping/stretching is needed. Tabs retain32sourcepx/16DIP canvas ->14x12DIP visible. Caption and Tree now consume the EXISTING40px derivatives via two additive embedded aliases, so20DIP at200% has40sourcepx (no raster upsampling). Root visible17.5x15DIP; blue caption visible17.5x17DIP. At100/125/150/200%, Tab canvases16/20/24/32devicepx and20DIP canvases20/25/30/40devicepx preserve aspect and full alpha bounds. These are resource/layout measurements, not physical-DPI or aesthetic acceptance.

## RED / GREEN evidence

Command for every compiled run: `cargo test -p nxtools-desktop --bin nxtools-desktop icon_layout_ -- --test-threads=1 --nocapture`. Each call first checked `Get-Process cargo,rustc` and set same-command `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`; timeout21600seconds. No parent Cargo overlapped.

| Scratch log (all beneath current session scratch) | Result / interpretation |
| --- | --- |
| `archive-icon-red.log`, `archive-icon-red-behavior.log` | Initial fixture selector lifetime/type compile errors; NOT behavior RED. Corrected test helper for locked debug_bounds static-lifetime API. |
| `archive-icon-red-compiled.log` | Actual compiled behavior RED, exit101: RUN3/PASS0/FAIL3/ignored0. Official Tab artwork starts1DIP from tab edge instead of13; official Tree root24x24 instead of20x20; sole client caption missing approved blue slot. This preceded product fixes. |
| `archive-icon-green-initial.log` | Compilation blocked by concurrent parent HZ1 draft (`space_requests.rs` PlatformCommand import; `forms_space.rs` Div/Stateful return). Not this slice's defect/behavior RED; parent files not edited. |
| `archive-icon-green-expanded.log` | RUN6/PASS4/FAIL2. Test-matrix exact13DIP assertion observed12.8 at125% due device-edge rounding; corrected to<=one device pixel tolerance. Wheel fixture sent pixel/trackpad delta, while released vertical_wheel remaps Lines only; changed fixture to genuine mouse-wheel Lines. No product workaround. |
| `archive-icon-green-matrix.log` | RUN6/PASS6/FAIL0/ignored0. |
| `archive-icon-green-menu-drag.log` | RUN7/PASS7/FAIL0/ignored0. Original menu selection/keyboard close and held Tree icon/text/root descriptors pass. Parent HZ1 unused-method warning was visible here, not suppressed. |
| `archive-icon-green-painted-tree.log` | RUN8/PASS8/FAIL0/ignored0. Added actual approved-image atlas-paint assertion and arrow/indent/expand/collapse regression. |
| `archive-icon-green-final.log` | FINAL after40px alias integration and scoped formatting: exit0, RUN8/PASS8/FAIL0/ignored0,1420filtered,11.41seconds; build3m00. No warning in this command. |

Final actual official-boundary coverage: single long CJK tab and seven tabs (including same basename/different canonical path),2themes x3logical clients(720x480,960x640,1440x900) x4synthetic display scales(1,1.25,1.5,2) x2selected positions =96 checks. Actual Tab default Medium height32, max220,16square artwork and20square close remain allocated and center-aligned; label-only ellipsis remains official. For short.zip at960x640, tab=(0,34,154,32), image=(13,42,16,16), close=(121,40,20,20):12DIP outer breathing room after each1DIP official border,8DIP text/close gap. Compared original tab146wide/imagex1/closex125. Zero-width TEST-ONLY public Tab content marker observes official label trailing geometry without replacing its render; original label metadata feeds full overflow menu copy.

Decoded approved pixels are supplied to production img through a public test-only ImageCache because TestApp intentionally has an empty AssetSource. Forced actual draws assert atlas entries for orange32/40 and blue40, not only empty icon boxes. Asset test independently checks original SHA256/canvas/nonzero and>=8 bounds and32/40 consumption. Cached native directory PNG and physical GPU/screenshots are not claimed inspected; ordinary directory fallback plus20DIP fixed container/Contain code path and original deferred icon-result owner remain intact.

## Files and static/resource validation

- `rust/apps/nxtools-desktop/src/archive/browser_host.rs:14–16,118–169,191–223`: new tests registration, public-slot inset correction/nonshrinking16DIP Contain, browser-only20DIP blue caption prefix; nonshrinking official TabBar. activate/add/remove and deferred WAC lifecycle unchanged.
- `rust/apps/nxtools-desktop/src/archive/browser.rs:2622–2651`: only Tree icon presentation,20DIP fixed slot and whole/aspect-preserving img root/directory/fallback. Arrow/indent/label/drag/native/marquee code unchanged.
- `rust/apps/nxtools-desktop/src/ui/components/titlebar.rs:134,174–190,231`: additive optional presentation prefix; old controlled_titlebar defaultsNone, every old caller/native/control owner unchanged. Evidence-backed necessity: original209 contained text only.
- `rust/apps/nxtools-desktop/src/assets.rs:125–129,153–154`: register pre-existing40px derivatives without changing old aliases/resources/generator/original PNGs.
- `rust/apps/nxtools-desktop/src/archive/browser_icon_layout_tests.rs:12,94,112,130,211,305,413,498,569`: actual render/pixel/scale/menu/close/drag/disclosure regressions, synthetic fixtures only.

`archive-icon-static-final.log`: reproducibility PASS25resources unchanged; `python tests/architecture/approved-icons.py` PASS9; actual freshly compiled test PE `python tests/architecture/approved-icons.py --desktop-pe target/debug/deps/nxtools_desktop-da7215c5e26ccf8d.exe` PASS18decoded frames/group1blue/group2orange. This is an actual PE resource check, NOT a new production/package executable. gpui-release-contracts.py, markdown-editor-contracts.py, spec-contracts.py PASS; scoped rustfmt browser_host/newtests/assets and git diff--check PASS. Shared titlebar's existing dense caption_control formatting was not mass-formatted; parent owns final broader lint/fmt/check/regression run.

## Limits / handoff

No current screenshot supplied/inspected; rejected effect remains unaccepted by user. No real product/native HWND, Explorer/OLE target, physical100/125/150/200 or mixed-monitor DPI, IME/UIA/provider or visual/user acceptance. Synthetic scale events and atlas allocation do not establish those. No packaging/install/Registry/service/vendor/dependency/generator/asset bytes/commit changes. No HZ1 protocol/forms/jobs/native-capacity edits. Full affected regressions, strict Clippy and workspace check remain parent's final integration work, not claimed executed here. This slice's Cargo has finished; no owned cargo/rustc remains. At11:55 final process check, parent had already started its separate `cargo test ... --locked --offline hz1` (cargoPID26068/rustc26416,started11:54:20); do not misreport globalzero or interrupt that parent build.
