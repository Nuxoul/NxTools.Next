# Transparent edge source-over composition

## Verified upstream fix

Zed [PR65571](https://github.com/zed-industries/zed/pull/65571), merged commit [e18de65b75901f8720ff9c9320bb013395de54e5](https://github.com/zed-industries/zed/commit/e18de65b75901f8720ff9c9320bb013395de54e5), dated2026-10-14, corrects translucent-window alpha composition. The maintainer extended the fix to DirectX and wgpu. The Windows diff changes only DestBlendAlpha from ONE to INV_SRC_ALPHA in create_blend_state and create_blend_state_for_path_sprite. RGB factors, shaders and path rasterization are unchanged. It adds no draw pass or shader work.

The checked-in gpui-pre-windows0.3.9 snapshot93d7456 contains both original ONE assignments. The real flow is DirectXRenderer::new -> DirectXRenderPipelines::new -> create_blend_state for quads/shadows/sprites -> draw_quads/OMSetBlendState -> premultiplied-alpha composition swap chain. The existing path-rasterization blend already uses INV_SRC_ALPHA. Correct accumulated alpha is SA + DA*(1-SA), not SA + DA. A rounded fill and separate antialiased outline over a transparent shadow can otherwise accumulate excess alpha, increasing dark edge contrast on white.

Microsoft's [Composite effect](https://learn.microsoft.com/en-us/windows/win32/direct2d/composite) documents source-over O=S+(1-SA)*D; [premultiplied alpha](https://learn.microsoft.com/en-us/windows/apps/develop/win2d/premultiplied-alpha) distinguishes straight input colors from premultiplied render-target storage. These support the exact upstream correction, not a wholesale pipeline change.

## Other reviewed approaches

- [Warp rounded-rectangle rendering](https://www.warp.dev/blog/how-to-draw-styled-rectangles-using-the-gpu-and-metal) uses distance fields and smooth transition at both border interfaces. Current GPUI already has signed-distance antialiasing, so simply adding smoothstep is not an established fix here.
- [SDF pixel-width analysis](https://blog.pkh.me/p/44-perfecting-anti-aliasing-on-signed-distance-functions.html) derives derivative-based coverage and discusses linear versus smooth ramps. It is a fallback if correct composition leaves unacceptable spatial aliasing, not a reason to broaden the edge before fixing alpha.
- [Zed glass-content PR58833](https://github.com/zed-industries/zed/pull/58833) preserves destination alpha for content drawn on a system glass backdrop. That is a different surface policy from transparent outer window corners; it is not backported.
- [Sonora's public fork](https://github.com/sonorahq/gpui) exposes corner_smoothing/squircle and layer filters. Changing the corner curve or adopting that fork does not establish a fix for this locked renderer and is outside the isolated experiment.

## Isolated validation boundary

The user approves a narrow renderer experiment after online research. The production source/dependency graph stays unchanged. A session-temporary copy of the current Windows backend retains the existing no-activate patch and receives exactly the two upstream alpha assignments. Cargo uses an explicit temporary patch override only for the standalone example/backend tests. The ordinary example executable is preserved across the experimental build; the candidate runs from its separate target/debug/preview-source-over directory through the same PID/title-qualified C# frame helper.

A native backend test calls the original device/blend-state constructors and reads ID3D11BlendState::GetDesc for both affected pipelines. It must reject the original additive destination alpha and accept source-over. Existing example layout/input/close tests remain independent. Actual white-backdrop captures compare the same620x60/radius8/frame/type/shadow without shader, palette, geometry, native-region or global input changes.

## Validation

The original native GetDesc test rejects ONE(2); the exact upstream backport passes that native regression and both original example tests. Debug candidate build succeeds. Actual Windows11/100% white-backdrop output retains the same620x60/radius8geometry but lightens the dark corner fringe. The [corner comparison](source-over-corner-comparison.png) is nearest-neighbor enlarged from actual screen captures, not a browser recreation. Candidate native Edit value confirms submitted synthetic text; normal close and ordinary executable byte restoration pass. The result corrects alpha accumulation without claiming that every low-DPI spatial staircase is gone.

## Limits

Upstream and mathematical agreement do not certify the local visual result. Native screenshots, user acceptance and Win10 remain separate. This experiment does not authorize a product backport, dependency upgrade, package, installation or installed-runtime replacement.
