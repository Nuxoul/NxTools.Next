# Packaged Windows application icons

## Contract

The existing Quick Launch result list and 32-DIP image rail remain unchanged. Kit 0.8.0 `component::Icon::new(MaterialIcon).size(px(...))` supplies the static fallback; existing GPUI `img(path).object_fit(Contain)` renders the result-owned native PNG. System uses the existing Dashboard application glyph, not ProgressActivity. This is leaf content within the original result ListItem, with no Toolbar adaptation, custom control or Spinner.

AppsFolder targets validated by InstalledApplicationQuery's exact `shell:AppsFolder\` prefix and <=128 ASCII `family!application` grammar gain shell-icon capability only. File preview, native Shell menus, reveal, path copy and elevation remain unavailable. Activation stays the original ordinary Shell Open. Unknown names/packages are not filtered by an allowlist.

## Native ownership

Original `ExtractShellIcon {target,size}` -> original bounded STA (5-second logical wait/max8 actual workers) -> real resolver -> existing IconCache -> narrow ShellItemImageFactory -> SHCreateItemFromParsingName with IShellItemImageFactory IID -> GetImage(requested SIZE, ICONONLY=4) -> HBITMAP conversion/freeze -> original PNG publication and validation. The raw native leaf alone is injectable; resolver, cache, factory cleanup, conversion and PNG encoding remain real in fixtures. Every returned COM pointer and HBITMAP is released on success, HRESULT failure, retry and cancellation. Bounded E_PENDING retries stay inside the same STA operation.

Namespace cache keys keep the full exact target plus size, a distinct selector/type, zero filesystem timestamp and no GetFullPath, environment expansion, filesystem probes or extension/type fallback. Existing cache limits128memory/2048disk and Rust96LRU, per-key coalescing, PNG validation, cancellation and timeout ownership remain unchanged. Existing platform correlation/start/result diagnostics and fixed shell_icon failure categories carry HRESULTs without target logging.

## Lifecycle invariants

Visible-range prefetch remains the only ordinary-list admission owner. Duplicate resource requests coalesce, replies bind by result ID, and memory hits defer panel writes until the next application turn, including a leased panel callback. Failure retains the same static semantic fallback and original bounded retry policy. No new popup, preview, focus, placement, selection, activation or history path is introduced. Primary activating popup and one passive preview retain their original constructors/show/hide owners.

## Regression boundaries

- Capability assertion becomes positive for packaged icons while all file-only negatives remain.
- Actual QuickLaunchApplication/Panel visible-range callback emits unchanged target/32; duplicates coalesce; successful PNG binds only requesting IDs; unknown identities differ; cache-hit delivery is deferred under a leased panel; failures keep a static semantic fallback and do not issue surface commands.
- Native fixtures use injected raw factory facts and GUID-owned HBITMAP/cache files only. Real resolver/Extract/factory exercise exact namespace/IID/SIZE/ICONONLY, PNG output/cache hits and distinct unknown identities, invalid shapes, HRESULT preservation, COM/HBITMAP cleanup and cancellation. No live AppsFolder COM or Shell operations.

## Verification ownership

Parent serializes all Cargo and owns combined spec/verification updates. This slice may build C# and execute only the exact effects-reviewed fixture selection after list-only review. Installed AppsFolder extraction, activation, mixed-DPI/native appearance and user visual acceptance remain unrun.

## Controlled verification

| Exact command | Result |
| --- | --- |
| `dotnet build .\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore` | PASS, 0 warnings / 0 errors. |
| `dotnet run --project .\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list "packaged application icon fixture"` | Three reviewed Fixture entries; selected=3, run=0. |
| `dotnet run --project .\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures "packaged application icon fixture"` | PASS, selected=3, run=3, pass=3, skip=0, fail=0, not_run=0. |
| `rustfmt --edition 2024 --config skip_children=true .\rust\apps\nxtools-desktop\src\quick_launch\installed_applications.rs` | PASS; syntax/formatting only, not Rust type checking or GPUI execution. |

The scoped tracked-file `git diff --check` passed. The build and fixture selection have no observed blockers. Rust compilation, focused `windows_apps_` GPUI tests, full suites, shared contract gates and installed/native/visual acceptance remain pending with the parent. The existing Finder quality contract's file-only icon exclusion at `.trellis/spec/nxtools-finder/backend/quality-guidelines.md:144` needs the parent's combined icon-capability amendment. Shared specs and the pre-existing blacklist picker changes remain untouched by this slice.
