# Rounded-edge pixel coverage

## Requested boundary

User still rejects the dark corner after the source-over candidate; the light theme is substantially better. Continue only the isolated preview. Preserve official InputGroup/InputState,620x60visible geometry,8DIP radius,20/28type, theme colors, straight borders, shadow and current native/input owners. Checked-in vendor/manifest/lock and product runtime remain unchanged.

## Reference-backed method

[Perfecting anti-aliasing on signed distance functions](https://blog.pkh.me/p/44-perfecting-anti-aliasing-on-signed-distance-functions.html) derives pixel-width coverage from the screen-space SDF gradient. The L1 norm used by fwidth is abs(ddx(d))+abs(ddy(d)); it varies with edge direction. [Warp's rounded-rectangle article](https://www.warp.dev/blog/how-to-draw-styled-rectangles-using-the-gpu-and-metal) treats both the outside shape edge and the border/background interface.

The locked Windows quad shader uses saturate(0.5-distance) at both interfaces. The isolated candidate uses the rounded-box SDF's analytic normal to obtain the same L1 pixel footprint:1on axis-aligned edges, up to sqrt(2)on diagonal arcs. It keeps the original linear ramp, normalized by that width. Each SDF surface's0.5coverage contour and radius do not move. Border admission and inner/outer coverage use the same width. Analytic normals avoid fragment derivatives after the existing nonuniform early returns. There is no theme-specific palette, blur, extra draw pass, render-target enlargement or sample grid.

The experiment runner retains the upstream source-over correction and injects only the coverage helper and five exact quad-shader sites into its temporary backend copy. Source-over and pixel-coverage candidates have separate target directories and quick-reopen commands.

## Real GPU boundary

Exact public gpui-pre0.3.9 Window::render_to_image forwards the last Scene to WindowsWindow's existing DirectXRenderer::render_to_image. That path renders and maps a staging texture, converting BGRA to RGBA without presenting. The example's one-shot capture runs on a deferred app turn after transparent Root construction, explicitly draws/clears the arena, exports premultiplied bytes plus a correctly composited white image and viewport/DPI metadata, then exits. It does not substitute a browser/CPU-drawn control or require desktop focus/clipboard input.

`tests/graphics/check-gpui-pixel-coverage.py` checks actual100%GPU readbacks at four symmetric arc samples. Their signed distance is0.5147px outside the circle: the fixed-width ramp discards shape coverage, while the direction-aware pixel footprint includes it. The lower/upper alpha bounds derive from that footprint, the existing shadow and the group's fill/border draws; they reject clipping and opaque squared-off corners, not a subjective image score. Straight edge/interior samples remain byte-exact against source-over, with identical viewport/DPI, opaque body and transparent gutter. The original native blend-state and component/input/close tests remain.

## Verification

The original GPU arc sample reads alpha9 and fails; the directional candidate reads70at the two upper corners and75at the lower corners. Both dark/light coverage checks pass, including the unchanged straight/interior samples. Native blend-state1/1, component/input/close2/2, Debug build and strict example Clippy pass. Capture-to-file failure exits unsuccessfully at the directory stage; success is set only after all exports finish. The [full-frame](pixel-coverage-comparison.png) and [corner](pixel-coverage-corner-comparison.png) GPU comparisons were inspected. Native/user visual acceptance remains separate.

## Limits

GPU readback does not certify native DWM first-show behavior, Win10, physical Windows IME, mixed DPI or user visual acceptance. The pixel regression is a100%DPI fixture. Native preview and Light/Dark visual comparison remain separate from the pixel assertions.
