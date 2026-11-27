# NxTools contour patch on gpui-pre 0.3.12

The published gpui-pre 0.3.12 archive has SHA256 `29db89613527079737f53752eab27fb65c9a7dd94979256c65a0e590c9fd34d8`. `LICENSE-APACHE` is retained. `NXTOOLS-UPSTREAM.json` inventories the original published files. The matching framework uses Kit/Base/Component/Assets 0.10.0 with GPUI Fast disabled.

The approved material delta changes only `src/scene.rs` and `src/style.rs`. `UnderBorder = 2` marks the background quad for a visible solid border and carries the actual border widths and color. The companion Windows shader compensates background coverage for the unchanged foreground border without changing the SDF footprint. Dashed and absent borders, child order, masks, shadows and layout retain their existing paths.

Source and provenance gate: `python tests/architecture/gpui-release-contracts.py`. The companion Windows patch documents the rendering and native frame regressions.
