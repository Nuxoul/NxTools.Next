# Implementation and verification

1. Add Core temporary-file regression tests, then implement missing-value migration, explicit-global precedence and rule-update preservation.
2. Add the Clipboard configure field and route the same effective policy through both original feature commands, preserving receipt and operation correlation.
3. Move the official Switch into Global Settings → Application Blacklist; preserve draft/Cancel/Apply and minimum-window table/footer geometry.
4. Extend the existing C# pre-publication gate for both features and exclusive/borderless classification. Keep capture and low-level-hook lifecycles unchanged.
5. Exercise actual GPUI interaction, durable/deferred consumers and native publication fixtures, then run affected compiled/static gates.

## Commands

All Cargo commands inherit `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, and `RUSTFLAGS=-C codegen-units=256`, with one Cargo operation at a time.

- `cargo test -p nxtools-core --lib --test fullscreen_summon`
- `cargo test -p nxtools-protocol --lib`
- `cargo test -p nxtools-desktop --bin nxtools-desktop -- --test-threads=1`
- `cargo clippy -p nxtools-core -p nxtools-protocol -p nxtools-desktop --tests -- -D warnings`
- `cargo check --workspace --tests`
- `cargo fmt --all -- --check`
- `dotnet build csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj --no-restore`
- `dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build -- --fixtures "clipboard configuration preserves" "clipboard capture limits clamp" "Quick Launch diagnostics keep" "Quick Launch suppresses both" "Fullscreen summon" "Quick Launch and clipboard WndProc"`
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/quick-launch-lifecycle.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/clipboard-lifecycle.ps1`
- `python tests/architecture/gpui-release-contracts.py`
- `python tests/architecture/markdown-editor-contracts.py`
- `python tests/architecture/spec-contracts.py`

Compiled fixtures and source gates do not certify physical game hotkeys, DPI/visual acceptance or a packaged release.
