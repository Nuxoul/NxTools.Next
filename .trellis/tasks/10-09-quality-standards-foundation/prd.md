# Source-backed engineering and UI standards

## Goal
Replace generic/contradictory guidance with accurate, discoverable, enforceable standards suitable for a mature compact Windows utility project. This child creates standards and quality-check support, not a product redesign.

## Context
Parent `10-09-project-quality-and-core-polish` owns scope/schedule. Twelve Rust packages exist in root Cargo.toml; C# PlatformHost and browser extension have real independent native/web boundaries. Spec indexes still contain templates; seven recent quality scenarios must survive. `docs/ui-remediation-plan.md:75-117`, later approved staged design:3-25 and current metrics:5-50 disagree on numeric tiers. Prior SOL review report records unresolved tests/live gates.

## Requirements
- S1: inventory first-party owners/spec topics/tests/check scripts and establish one authority/navigation map. Existing bootstrap task is related, not erased or falsely completed.
- S2: fill applicable coding topics from actual modules: typed interfaces/state ownership, error taxonomy, async/generation/cancellation, persistence/config, bounded work, diagnostics/privacy, tests and dependencies. Remove or mark non-applicable topics accurately; no template or invented contract.
- S3: write source-backed native C#/GPUI and extension guidance where actual layers require it. Share common contracts once; keep module-specific call paths/assertions local.
- S4: publish UI conflict/resolution table and normative roles/state/keyboard/focus/DPI/motion/accessibility guidance. Later approved design governs target; current smaller controls/special geometry are observed differences or justified narrow exceptions, not implicit authority. Product token migration belongs to existing Foundation stage.
- S5: add or adapt bounded static/spec checks only if they provide real signal; route documented Cargo/C#/Node/architecture verification commands. Existing violations appear in a gap ledger rather than false passes or blanket allowlists.
- S6: preserve historical references and recent executable contracts, unrelated dirty changes, architecture and security restrictions.

## Acceptance criteria
- Every first-party ownership area is routed through an accurate index; package guidance points at real files/symbols/tests, not generic framework prose.
- Applicable specs have no template placeholders/empty boilerplate; non-applicable database/etc. sections are removed or explicitly explained.
- Triggered cross-layer/storage/IPC contracts retain scope, signatures, fields/env, error matrix, cases, tests and wrong/correct examples.
- UI target/current/exception conflicts are explicit and traceable; no numerical authority remains ambiguous for Foundation implementers.
- Check commands match real paths/flags and clearly separate source, compiled, interaction and release evidence. New checks are exercised with a bad-case fixture if introduced.
- No product geometry/default/domain changes, dependency addition, unsafe deployment action, commit or archive.

## Out of scope
Mass refactoring to satisfy newly invented rules; product code/style migration; enforcing arbitrary module size or timing budgets; new tooling framework/dependencies; rewriting Trellis runtime; automatic GitHub/CI/deployment setup. Tooling changes must be small validation scripts scoped to these standards.

## Decisions
Use repo evidence and already approved desktop direction, not popularity claims. Maintain Rust state/layout, C# native and GPUI rendering ownership. Documentation-only work does not require pending protected-service acceptance to pass, but must describe it honestly. Planning ready only after the parent's final review approval.