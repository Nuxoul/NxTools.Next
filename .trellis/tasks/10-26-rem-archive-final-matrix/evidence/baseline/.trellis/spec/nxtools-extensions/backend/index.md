# Extensions backend

Owner: host-neutral package identity, validation, registry, trust/settings, one-shot runtime and versioned invocation/events/result contracts. Evidence: `repo:rust/crates/nxtools-extensions/src/lib.rs`, `repo:rust/crates/nxtools-extensions/src/runtime.rs`. Desktop feature Adapters and C# native supervision retain their owners. W6 plus joint W7–W9 isolated source acceptance is recorded in `.trellis/tasks/10-14-winshun-w6-w9/outcomes.md`; no installed/Release acceptance implied.

## Pre-Development Checklist
Read [architecture](../../shared/architecture.md), [engineering](../../shared/engineering.md), approved `.trellis/tasks/10-14-winshun-w6-w9/` design/implementation and [reference index](../../../../docs/reference-code/winshun.md). Then [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [diagnostics](logging-guidelines.md), [quality](quality-guidelines.md). Rust owns extension state; C# Windows APIs; GPUI rendering. No dependency cycle through Core/Finder/Download.

## Quality Check
Actual import/registry/config/Finder/GPUI/native fixture seams; extension tests plus affected owners, scoped strict lint/workspace test compilation/fmt/source/spec per [verification](../../shared/verification.md). No installed scripts/native user state/package/service operations implied. See owning task evidence for delivered versus planned methods.
