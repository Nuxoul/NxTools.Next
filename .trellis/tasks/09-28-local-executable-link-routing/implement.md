# Implementation Plan

## Ordered Steps

1. Add a red pure detector/route test for an existing `Throne.exe`-named file and an existing directory; add web, bare-domain, nonexistent, copy-only, and no-payload-diagnostic cases.
2. Extend the detector result with the typed local/web kind and keep all existing recognition limits and copy values.
3. Add the panel route helper and wire the click callback so local files send `SelectInExplorer`, directories send `Open`, and web links retain browser settings.
4. Preserve close-before-copy/echo suppression and replace raw link/browser diagnostics with bounded metadata.
5. Run the focused Rust tests, then the existing PlatformHost shell tests and desktop tests that exercise clipboard/link settings.
6. Run the manual Windows smoke test using the supplied executable path and confirm Explorer selects the file; verify a directory and HTTPS URL still route as designed.
7. Review the diff for accidental DOpus or shell-protocol changes; remove temporary probes.

## Expected Files

- `rust/apps/nxtools-desktop/src/tools/link_detector.rs`
- `rust/apps/nxtools-desktop/src/tools/link_panel.rs`
- `rust/apps/nxtools-desktop/src/clipboard.rs` if dispatch helpers or diagnostics require it
- `rust/apps/nxtools-desktop/src/tools/mod.rs` only if the typed helper needs re-exporting
- `rust/apps/nxtools-desktop/src/main_window/tests.rs` only for a settings/production interaction gate
- `csharp/NxTools.PlatformHost.Tests/Program.cs` only for a missing existing shell regression

## Validation

```powershell
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "0"
$env:RUSTFLAGS = "-C codegen-units=256"
cargo test -p nxtools-desktop --bin nxtools-desktop <focused-link-detector-and-panel-tests>
dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore <focused-shell-tests>
```

Then run the relevant full desktop/PlatformHost test commands from the parent plan. The manual Windows result is required because the final `/select` behavior is native Explorer UI.

## Rollback Point

The child is independently reversible by reverting the target-kind field and panel route helper; it does not alter the wire contract or C# ShellTarget.

## Validation evidence (2026-10-02)

- Re-read `docs/reference-code/clipboard.md` and the parent task's PRD/design/ordered plan. The legacy reference contains no equivalent link-detector floating panel; the existing NxTools.Next detector and typed PlatformHost `ShellTarget` boundary remain authoritative.
- The detector accepts only existing drive/UNC paths and classifies from filesystem kind. The shared production/test helper preserves the original local target and maps non-copy-only local files to `OpenShellTarget(SelectInExplorer)` and directories to `Open`; Web targets cannot be sent through that shell branch or synthesized into an `https://` URL.
- Copy-only is the persisted default and intentionally takes precedence for every target type. With copy-only disabled, only Web targets consult the opt-in browser-auto-open setting; local targets use the typed Shell actions. The row still closes before copy/echo bookkeeping and route dispatch; diagnostics omit target payloads.
- Focused Rust `link` tests: 11 passed, 0 failed. Full desktop suite: 427 passed, 0 failed. `cargo fmt --check --all`, desktop clippy (`-D warnings`), UI-token and download-window lifecycle architecture gates passed.
- Existing PlatformHost `ShellTarget` policy test passed; no native shell or IPC change was made. Trellis context validation passed with pre-existing manifest-size/code-path warnings. Scoped `git diff --check` passed.
- No DOpus logic or unrelated application behavior was changed.

## Still required

- Manual Windows smoke with the requested existing `.exe`, a local directory and an HTTPS URL remains outstanding. The Explorer `/select` operation was not invoked in automated tests.
- Keep the task `in_progress` until the manual Windows check and repository commit/archive workflow are completed. No package, installation, deployment or process restart was performed.

## Follow-up verification (2026-10-02)

- Re-ran the temporary existing-`Throne.exe`/directory detector fixture: passed. Local file/directory Shell route and Web/copy-only route tests passed. Existing PlatformHost ShellTarget policy test passed, including Explorer-selection policy coverage.
- Full desktop binary: 427 passed; `cargo fmt --check --all`, desktop clippy (`-D warnings`), UI token gate, download lifecycle gate and scoped task context validation passed.
- The real Explorer `/select` action was not invoked; manual smoke for the requested executable, directory and HTTPS URL remains outstanding. No DOpus behavior was changed or tested.
- Task remains `in_progress`; no package/install/deployment/process restart/commit/archive was performed.

## User validation installer (2026-10-03)

- Installer: `artifacts/installer-validation-20261003/NxTools.Next-validation-20261003-setup.exe`, SHA-256 `503BC888ACF0868E2CD08073B86B11515776C5C81D7D829C20401A52BE565B6F`. `INSTALL.md` requests Explorer smoke for a harmless existing `.exe`, a directory and HTTPS URL.
- Installer was not run; Explorer `/select` remains unverified. Cancel any UAC prompt; do not change startup/tray settings or install/migrate the service. No DOpus action or process/service restart occurred.
- Task remains `in_progress` pending manual smoke and repository closeout.
