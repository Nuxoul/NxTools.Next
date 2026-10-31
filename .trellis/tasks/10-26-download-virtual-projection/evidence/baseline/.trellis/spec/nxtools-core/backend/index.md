# Core backend

Owner: settings/defaults/paths/instance and logging guards, not feature engines or Windows UI. Evidence: `repo:rust/crates/nxtools-core/src/lib.rs#AppConfig`, `repo:rust/crates/nxtools-core/Cargo.toml`.

## Pre-Development Checklist
Read [shared architecture](../../shared/architecture.md), [engineering](../../shared/engineering.md), owning reference/task and the topics below. Preserve serde/default/legacy compatibility and storage-field ownership.

| Topic | Scope |
| --- | --- |
| [Structure](directory-structure.md) | Core boundaries and dependency direction |
| [Persistence](database-guidelines.md) | JSON/config locks, no SQL |
| [Errors](error-handling.md) | Validation/context and unsafe raw diagnostics |
| [Diagnostics](logging-guidelines.md) | Nonblocking guard and payload policy |
| [Quality](quality-guidelines.md) | Defaults/normalize/merge/instance tests |

## Quality Check
Follow [verification](../../shared/verification.md); `cargo test -p nxtools-core --lib`, scoped Clippy/check/fmt when code changes. Docs-only use spec-contracts. Test stale/missing/corrupt config, mixed-case blacklist and instance hints. No product/service launch. [Standards entry](../../index.md).
