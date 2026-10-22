# C# PlatformHost native boundary

Evidence: `repo:csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj`, `repo:csharp/NxTools.PlatformHost/Program.cs`, `repo:csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs`. net8.0-windows/win-x64 with WinForms/WPF; one native adapter process, not a GPUI state owner.

## Pre-Development Checklist
Read [authority](../shared/architecture.md), [engineering](../shared/engineering.md), matching indexed reference, [native contracts](native-contracts.md), [diagnostics](diagnostics.md) and [verification](../shared/verification.md). Archive creation/partial-read changes also read [archive creation](archive-creation.md). Preserve source HWND, ID/generation, pixel units, focus/noactivate and STA ownership before changing APIs.
Archive extraction metadata/MOTW changes also read [extraction metadata](archive-extraction-metadata.md), including held-volume ADS capability, exact partial mutation and metadata-aware cleanup.
Archive source-delete/recycle changes also read [source authorization](archive-source-deletion.md); isolated result tests are not actual Shell recycle acceptance.
Archive product open/test also reads [product read lifecycle](../nxtools-archive/backend/product-read.md), including fixed helper selection, exact negotiated capabilities and independent business metadata credit.
Readv2 explicitly negotiates finite100k budgets via a legacy-compatible bootstrap then one
original-pipe upgrade before source reads. Preserve v1stdout8MiB/defaults and actualcleanup-bound
max2Run reservations; do not globally raise stdout/timeouts. See the same read lifecycle.

## Quality Check
Source gates plus C# build and reviewed isolated fixtures. `dotnet build csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj --no-restore`; `dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -- "centered worker admission"` preserves substring filtering for a self-owned HWND fixture. Harness no args or --fixtures selects only TestEffects reviewed fixtures; --list enumerates without executing. Unknown/unmatched/cross-effect filters exit2 rather than silently omit tests. Live/native and RestrictedFixture entries remain available only via --native --authorize-native with explicit filters after separate approval; --native --list needs no execution authorization. Summary distinguishes RUN/PASS/SKIP/FAIL/not_run; missing live environment is typed SKIP, never PASS. scripts/build.ps1 explicitly passes --fixtures and package verification inherits it. See `repo:csharp/NxTools.PlatformHost.Tests/README.md` and [verification](../shared/verification.md). No registry/service/installer/live Shell acceptance implied.
