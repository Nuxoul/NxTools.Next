# C# PlatformHost native boundary

Evidence: `repo:csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj`, `repo:csharp/NxTools.PlatformHost/Program.cs`, `repo:csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs`. net8.0-windows/win-x64 with WinForms/WPF; one native adapter process, not a GPUI state owner.

## Pre-Development Checklist
Read [authority](../shared/architecture.md), [engineering](../shared/engineering.md), matching indexed reference, [native contracts](native-contracts.md), [diagnostics](diagnostics.md) and [verification](../shared/verification.md). Archive creation/partial-read changes also read [archive creation](archive-creation.md). Preserve source HWND, ID/generation, pixel units, focus/noactivate and STA ownership before changing APIs.
Floating assistant changes also read [its partial native boundary](floating-assistant.md); current owned OLE/source tests do not certify the incomplete GPUI/UI integration.
Archive extraction metadata/MOTW changes also read [extraction metadata](archive-extraction-metadata.md), including held-volume ADS capability, exact partial mutation and metadata-aware cleanup.
Archive source-delete/recycle changes also read [source authorization](archive-source-deletion.md); isolated result tests are not actual Shell recycle acceptance.
Archive product open/test also reads [product read lifecycle](../nxtools-archive/backend/product-read.md), including fixed helper selection, exact negotiated capabilities and independent business metadata credit.
Readv2 explicitly negotiates finite100k budgets via a legacy-compatible bootstrap then one
original-pipe upgrade before source reads. Preserve v1stdout8MiB/defaults and actualcleanup-bound
max2Run reservations; do not globally raise stdout/timeouts. See the same read lifecycle.
Phase F virtual-file spike changes also read [native OLE reference](../../../docs/reference-code/archive-virtual-drag.md),
[spike contract](../../tasks/10-16-archive-manager/phase-f-spike-contract.md) and [pure Shell plan/reference](../../../docs/reference-code/archive-shell.md).
Controlled COM/worker fixtures do not accept real source gestures/Explorer targets. User2026-10-23
authorizes bounded P208 production ArchiveBrowser row hookup: formal `archive_virtual_drag_v1` and
legacy spike alias share the same native session/STA/content owner. Current directory/unknownsize
amendment retains32roots/128descriptors/32files/512MiBactualtotal; directory descriptor-only, unknown
size omits FD_FILESIZE, original worker output capped by remainingtotal. Physical/foreigntarget/DPI/UIA
remain separate NOTRUN acceptance.
P208 password extension reuses original EntrySession authorize/revoke + worker no-output TEST and
InteractionPort; <=8perpipe/120second source-bound protected-memory grants. Explicit originalbrowser
unlock precedes a NEW heldgesture; GetData has no UI/passwordretry. See same production contract.
Formal row/receipt/reserved-cancel/error/native acceptance contract:
[P208 production contract](../../tasks/10-16-archive-manager/phase-f-p208-production-contract.md).
Phase F P206/P207 changes also read the 2026-10-22 section of [archive Shell reference](../../../docs/reference-code/archive-shell.md):
`ArchiveIntegrationService` keeps product Apply gated (`ProductApplyAuthorized=false`), persists observed facts before `applied`,
never deletes key trees; `csharp/NxTools.ShellSelectionHelper` is a separate unregistered x64 COM local server.
The 2026-10-23 release-chain amendment adds its apphost/DLL/deps/runtimeconfig and isolated-codecs worker
to original signed/approved/protected deployment and redirect ownership; source builds keep empty fail-closed
identity defaults. Explicit `package.ps1 -IsolatedStaging` uses fresh scratch output and test-only trust,
never production sequence reservation, service/process operations or installer. Deployment is not registration.
`tests/architecture/archive-phase-f-shell.py` and `archive-release-chain.ps1` guard these source boundaries;
actual scratch identities and fake file compensation are not Registry/Explorer/installed acceptance.

Archive CLI list/test/extract uses [CLI contract](../../tasks/10-16-archive-manager/archive-cli-contract.md).
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveCliHost.cs` is an early demand-only mode over the
original read/job sessions, fixed sibling CLI parent and GUID pipe. No ordinary adapter/GUI/service
initialization. Original helper approval and actual cleanup remain unchanged; see CLI quality gate.
7z extraction readmission also reads [7z safety boundary](../nxtools-archive/backend/sevenz-safety.md).
Only original extract+OutputReadmission opts into a held-source 7z signature check and the existing
finite encoded ceiling; actual output/Job budgets and Rust policy ownership stay unchanged.
RAR4/RAR5 uses its separate [RAR safety contract](../nxtools-archive/backend/rar-safety.md), actual
held-source signature and rarOutputPreflight extract+readmission opt-in. Original family/source/Job/
output ownership remains; C# never decides Risk or new-budget approval/retry.
TAR/stream extraction readmission also reads [its owner contract](../nxtools-archive/backend/tar-stream-safety.md).
Only original extract+OutputReadmission opts into bounded held-signature recognition and the original
max(output,metadata) encoded reservation; no100GiB/global source/output/Job increase.

## Quality Check
Source gates plus C# build and reviewed isolated fixtures. `dotnet build csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj --no-restore`; `dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -- "centered worker admission"` preserves substring filtering for a self-owned HWND fixture. Harness no args or --fixtures selects only TestEffects reviewed fixtures; --list enumerates without executing. Unknown/unmatched/cross-effect filters exit2 rather than silently omit tests. Live/native and RestrictedFixture entries remain available only via --native --authorize-native with explicit filters after separate approval; --native --list needs no execution authorization. Summary distinguishes RUN/PASS/SKIP/FAIL/not_run; missing live environment is typed SKIP, never PASS. scripts/build.ps1 explicitly passes --fixtures and package verification inherits it. See `repo:csharp/NxTools.PlatformHost.Tests/README.md` and [verification](../shared/verification.md). No registry/service/installer/live Shell acceptance implied.
