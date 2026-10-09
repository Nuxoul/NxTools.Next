# Core quality

Evidence: `repo:rust/crates/nxtools-core/src/lib.rs#tests`, `repo:Cargo.toml`.

Required tests cover serde default/legacy compatibility, validated network settings, normalize idempotence, mixed-case/path blacklist basename, independent feature scopes, panel clamps, stale-section merge and corrupt/missing write failure. Use temp paths, never real config. Instance hints are advisory: verify actual lock before acting on stale HWND hint. Don't infer process identity from stored hint alone. Keep guard lifetime observable.

`cargo test -p nxtools-core --lib`; scoped strict Clippy/check/fmt follow [verification](../../shared/verification.md) memory environment. No broad allows or arbitrary refactoring. Cross-owner migration/config tests live in Desktop and must remain with local tests. [Shared engineering](../../shared/engineering.md).
