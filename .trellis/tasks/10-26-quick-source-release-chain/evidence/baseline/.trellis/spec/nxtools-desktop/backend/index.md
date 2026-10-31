# Desktop backend and GPUI composition

Evidence: `repo:rust/apps/nxtools-desktop/src/runtime.rs#DesktopRuntime`, `repo:rust/apps/nxtools-desktop/src/main_window.rs`.

## Pre-Development Checklist
Read [architecture](../../shared/architecture.md), [engineering](../../shared/engineering.md), [approved UI/current matrix](../../shared/ui-design.md), matching reference/owning phase plan, then [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). State/layout Rust; rendering GPUI; Windows C#; callbacks defer cross-owner updates.
For UI work, FIRST read [component selection](../../shared/ui-component-selection.md), [current0.8.0 release/API delta](../../../../docs/reference-code/gpui-kit-080.md), then [historicalcatalog](../../../../docs/reference-code/gpui-kit-component-table.md)/[API index](../../../../docs/reference-code/gpui-kit-component-index.md) and exactcurrentreleasedsource. Current Kit0.8.0/defaultpre0.3.9 carries only two approvedpatches. Requestedshape/compound/variants/size/parentadaptations remain; Toolbar.child still forcesghost+compact. Tests/imports do not certifyvisualacceptance.

## Quality Check
Desktop full tests plus production bounds/action/lease/generation regression and affected native gates; strict scoped lint/check/fmt per [verification](../../shared/verification.md). Source token gates do not prove approved palette, accessibility or real lifecycle. Visual/keyboard/pointer/DPI acceptance remains separate, no product launch/service/DOpus without authorization. [Entry](../../index.md).

Current2026-10-22 approved amendment: [unified document/image/spacing/cadence/theme contracts](ui-experience-contracts.md) supersede historical no-editor and built-in-palette-only restrictions only within this explicit slice. Both carried framework patches remain unchanged.

P208 formalrow2026-10-23: `archive/browser_drag.rs` is private state of the original ArchiveBrowser,
not a second selection controller. Down candidate/held>=4DIP move/direct formal platform enqueue;
actual Window HWND, source/snapshot/ordinal/codepage frozen at start. Projection/refresh/source/close
retire/cancel original operation; CommandResult request receipt is deferred through ArchiveApplication.
Official ListItem32DIP/Alert/Toolbar composition retained. Tests exercise actual rows, no GPUI on_drag,
pre-extract/clipboard substitute or self-app/native acceptance bypass.
Directory/unknownsize2026-10-23: original `model_drag.rs` expands immutable BrowserModel.entries
without IO; explicit/implicit/empty directories, parent/child overlap dedup and unknown file sizes
retain original source/ordinal. Whole-selection safety and finite32roots/128descriptors/32files/
256KiBcommand/512MiBknown+actualtotal; no new selection/cache/controller. Original AC01 read-origin
connection and same locked bridge admission unchanged. See owning production contract above.
