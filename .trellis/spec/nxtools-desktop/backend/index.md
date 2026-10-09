# Desktop backend and GPUI composition

Evidence: `repo:rust/apps/nxtools-desktop/src/runtime.rs#DesktopRuntime`, `repo:rust/apps/nxtools-desktop/src/main_window.rs`.

## Pre-Development Checklist
Read [architecture](../../shared/architecture.md), [engineering](../../shared/engineering.md), [approved UI/current matrix](../../shared/ui-design.md), matching reference/owning phase plan, then [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). State/layout Rust; rendering GPUI; Windows C#; callbacks defer cross-owner updates.

## Quality Check
Desktop full tests plus production bounds/action/lease/generation regression and affected native gates; strict scoped lint/check/fmt per [verification](../../shared/verification.md). Source token gates do not prove approved palette, accessibility or real lifecycle. Visual/keyboard/pointer/DPI acceptance remains separate, no product launch/service/DOpus without authorization. [Entry](../../index.md).
