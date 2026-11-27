# NxTools contour patch on0.3.9

Published gpui-pre0.3.9, archiveSHA256 `34f61c1cfd36fc9c436af09308c963822ae27a45a84feb21a77ba5c9e051f1c0`, Apache-2.0 LICENSE-APACHE retained. `NXTOOLS-UPSTREAM.json` records the published file inventory. Kit/Base/Component/Assets remain0.8.0; Fast is disabled.

User-approved2026-11-21 production scope changes only `src/scene.rs` and `src/style.rs`. UnderBorder2 marks a background pass carrying the actual solid border widths and alpha. The Windows shader compensates that background for the later unchanged foreground border. Children, content masks, foreground order, layout and SDF sampling footprint remain owned by the original renderer. Dashed and absent borders keep their original path.

Companion Windows0.3.9 patch owns source-over alpha and the UnderBorder shader. Source/lock/provenance gate: `python tests/architecture/gpui-release-contracts.py`. GPU regressions: `cargo test -p gpui-pre-windows --lib source_over_tests -- --test-threads=1`; run with the repository's required low-memory Cargo environment. These gates do not certify native Acrylic, physical input, mixed-DPI or visual acceptance.
