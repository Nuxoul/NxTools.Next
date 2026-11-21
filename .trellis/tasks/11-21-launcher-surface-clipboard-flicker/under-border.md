# Background/border coverage compensation

## Isolated test scope

User requests testing the background-compensation proposal. Preserve source-over, restore the original fixed SDF ramp, and target the current opaque620x60/r8/1px solid InputGroup. No product vendor, root manifest/lock, installed runtime, service or Registry changes. The temporary standalone manifest points at the same example source and pins Kit0.8.0/pre0.3.9, overriding temporary core/Windows copies and the existing Component vendor.

## Verified boundary and formula

Core Style::paint paints background, then inset shadows/children, then the border. Window::paint_quad can split a border-only draw into four masked strips; the test does not assume the border stays one primitive. Background draws retain the real border widths/color under an internal UnderBorder marker only for visible solid borders. The foreground border and child order remain unchanged.

At the shader's background-only branch, o is outer coverage and q=k*border_alpha. The background alpha becomes a*o*(1-q)/(1-o*q). The branch returns directly so outer coverage is not multiplied twice. When o=q=1, the branch uses zero background coverage instead of evaluating0/0; the later opaque border covers that pixel. This validates the premultiplied coverage model, not arbitrary straight-RGBA interpolation equivalence.

## GPU oracle

The fixture can disable only its own shadow for diagnostic readback; normal preview retains it. At100%DPI, all256pixels of the four8x8corner regions are compared against one opaque fill/border coverage application. Colors come from actual opaque interior/straight-border pixels. The oracle uses the original outer/inner SDF coverage and checks premultiplied RGB plus alpha with a3byte-channel tolerance. It also requires unchanged non-arc samples, viewport and transparent gutter. A source-over-only baseline must fail this oracle before the compensated candidate is evaluated.

## Results

The source-over-only Dark baseline fails with maximum RGBA channel error61.452/255. The compensated candidate passes all256corner pixels in each theme: maximum0.784Dark and0.641Light, below the3byte-channel tolerance. Straight/interior samples and viewport remain unchanged. Native blend-state1/1, component/input/close2/2, Debug build and strict candidate Clippy pass. Fresh private-lock resolution selects the two temporary0.3.9sources; production Cargo files remain untouched.

The shadow-enabled [full-frame](under-border-comparison.png) and [corner](under-border-corner-comparison.png) comparisons were inspected. The dark outer fringe no longer mixes the popover fill into the border edge; Light remains consistent. The original shadow and geometry remain in normal preview. `-UnderBorder -NoBuild` launches the separate candidate, not normal NxTools.

## Limits

Normal shadow-enabled Dark/Light frames and the real native preview remain separate visual checks. This is not a general fix claim for translucent fills, group opacity, dashed borders, child content covering the edge, other rendering platforms or Win10.
