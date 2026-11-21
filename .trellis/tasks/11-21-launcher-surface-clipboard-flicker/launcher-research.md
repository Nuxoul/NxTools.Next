# Win10 launcher surface

## Visual target

One620x60DIP search surface,8DIP outer radius, restrained single shadow, one border and original search/IME/result/preview owners. The proposed20/28DIP regular input type replaces the current26/34DIP only after visual agreement; current fonts and neutral palette remain. Icon/input/feedback use the released InputGroup compound and its public control/addon parts, not a custom ordinary Input or another search controller. The browser [visual proposal](../../../docs/ui/quick-launch-win10-proposal.html) is an appearance review artifact, not native GPUI evidence.

The supplied Win10 crop shows an already rounded inner input inside a square dark outer shell. It rejects the current appearance. The cause of the square composed pixels is not established by that crop or the prior STATIC-window region fixture. The local workstation is Windows11build22631, not a Win10 acceptance environment.

## Confirmed platform references

- [Microsoft Windows11 rounding](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners): DWMWA_WINDOW_CORNER_PREFERENCE is a Windows11 system-rounding route, not a Win10 solution. Per-pixel alpha and custom window regions also interact with system-rounding policy.
- [Tauri11321](https://github.com/tauri-apps/tauri/issues/11321): a reported Win10 transparent/custom-rounded window retained the default system shadow; its author resolved that case by disabling the shadow. This is analogous evidence for a shadow test, not proof of the NxTools cause or a Tauri dependency recommendation.
- [Microsoft DirectComposition clipping](https://learn.microsoft.com/en-us/windows/win32/directcomp/clipping): visual/subtree clips support rounded corners, independently from 2D offsets/transforms.
- [IDCompositionRectangleClip](https://learn.microsoft.com/en-us/windows/win32/api/dcomp/nn-dcomp-idcompositionrectangleclip): public per-corner X/Y radii; minimum supported desktop client Windows8, so the API exists on Win10.

## Exact locked implementation boundary

Kit/Base/Component0.8.0 and gpui-pre0.3.9 remain pinned. Component input/group.rs exposes InputGroup::new/input/addon, InputGroupControl from Input, InputGroupAddon and Styled/Sizable; the parent owns the single painted frame and preserves the existing InputState. Current plain Input prefix/suffix also remains an official supported composition.

Component root.rs542-559 fills the Root and decorates it with WindowBorder. window_border.rs19-22 records zero generic radius with rectangular child masks. When DirectComposition is enabled, Windows window.rs513-514 adds WS_EX_NOREDIRECTIONBITMAP and directx_renderer.rs1014-1033 creates the HWND composition target/root visual, assigns the swap chain and commits without a rounded visual clip. The composition swap chain uses premultiplied alpha (1298-1321); the separate HWND swap chain ignores alpha (1324-1351). The actual Win10 path must therefore be recorded before interpreting transparent-window results. Transparent child styling alone does not establish correct native pixels.

The previous corner-radius operation applies SetWindowRgn after client readback and has valid region/GDI tests, but its STATIC HWND fixture does not verify DirectComposition pixels or shadow. Existing C# SetWindowShadow/NativeMethods.SetWindowShadow uses DWM non-client rendering policy to control the system shadow. No new native command is needed merely to compare shadow on/off.

## Chosen implementation order

1. Use an isolated real GPUI launcher window and matching Win10 environment to compare current opaque rendering with the actual requested region, then the existing native shadow-disabled policy. Read back native region and compare pixels on a controlled backdrop; preserve resize/reuse/input/focus.
2. Review the single-surface visual proposal. If accepted, compose it through official InputGroup slots and original InputState. Check compact/results/actions, loading, Light/Dark and DPI without duplicate borders or idle animation.

A self-painted soft shadow needs space outside the visible bar: within a fixed HWND it requires a transparent gutter, or a separately justified native/composition shadow policy. The browser shadow is a visual target, not evidence that extra pixels fit the current native budget. Do not enlarge or shift the product window merely to reproduce CSS shadow; geometry and source/preview anchoring must be explicitly reconciled before implementation.
3. If native region and shadow handling do not constrain the composed pixels, prototype a rounded DirectComposition visual clip at the actual composition owner. The current public GPUI API does not expose that root-visual operation; any framework API/source amendment requires demonstrated native evidence and explicit approval before changing vendor/provenance.

## Limitations

No claim that SetWindowRgn fails generally, or that disabling shadow alone fixes this program. No Win10-native render result, installed replacement, provider/IME/physical acceptance or framework amendment is implied by web research or browser preview. The visual proposal does not change product geometry, typography or framework code.
