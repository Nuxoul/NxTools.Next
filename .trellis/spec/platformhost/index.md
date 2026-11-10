# C# PlatformHost native boundary

Evidence: `repo:csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj`, `repo:csharp/NxTools.PlatformHost/Program.cs`, `repo:csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs`. net8.0-windows/win-x64 with WinForms/WPF; one native adapter process, not a GPUI state owner.

## Pre-Development Checklist
Read [authority](../shared/architecture.md), [engineering](../shared/engineering.md), matching indexed reference, [native contracts](native-contracts.md), [diagnostics](diagnostics.md) and [verification](../shared/verification.md). Archive creation/partial-read changes also read [archive creation](archive-creation.md). Preserve source HWND, ID/generation, pixel units, focus/noactivate and STA ownership before changing APIs.
Installer preparation changes also read [process identity and installed acceptance](installer-preparation.md); CIM-null image, native access failure and cancelled elevation are different states.
Floating assistant changes also read [its partial native boundary](floating-assistant.md); current owned OLE/source tests do not certify the incomplete GPUI/UI integration.
Archive extraction metadata/MOTW changes also read [extraction metadata](archive-extraction-metadata.md) and [ordinary output alignment](../../tasks/11-02-archive-workflow-optimization/ordinary-extraction-output.md), including held-volume ADS capture, commit-carried metadata on the live final-name handle and incomplete-output-only cleanup.
Archive source-delete/recycle changes also read [source authorization](archive-source-deletion.md); isolated result tests are not actual Shell recycle acceptance.
HZ5 source-space reclamation reads [its cross-layer/native contract](../nxtools-archive/backend/reclamation.md): separate writable exact-object lease, protected output receipts, append/flush/readback pending authority and64KiB-or-larger NTFS sparse units; ordinary source sharing remains read-only.
Archive product open/test also reads [product read lifecycle](../nxtools-archive/backend/product-read.md), including fixed helper selection, exact negotiated capabilities and independent business metadata credit.
Readv2 explicitly negotiates finite100k budgets via a legacy-compatible bootstrap then one
original-pipe upgrade before source reads. Preserve v1stdout8MiB/defaults and actualcleanup-bound
max2Run reservations; do not globally raise stdout/timeouts. See the same read lifecycle.
Explicit ordinary numbered-file join reads [its cross-layer contract](raw-numbered-join.md): frozen metadata preview, original router/reader and bounded source/output ownership; Cancel ACK is not physical completion.
Phase F virtual-file spike changes also read [native OLE reference](../../../docs/reference-code/archive-virtual-drag.md),
[spike contract](../../tasks/10-16-archive-manager/phase-f-spike-contract.md) and [pure Shell plan/reference](../../../docs/reference-code/archive-shell.md).
Controlled COM/worker fixtures do not accept real source gestures/Explorer targets. User2026-10-23
authorizes bounded P208 production ArchiveBrowser row hookup: formal `archive_virtual_drag_v1` and
legacy spike alias share the same native session/STA/content owner. Complete directory selections
have no separate root/descriptor/file/request-count or512MiB drag cap. Directory is descriptor-only;
unknown size omits FD_FILESIZE. Shared source context fits the real1MiB envelope; original worker
concurrency stays2 while independent leases and streams retain actual cleanup ownership. Materialize
uses existing general extraction resources and actual total-output accounting, not preview quotas.
See [wire and native lifetime contract](../nxtools-protocol/backend/directory-structure.md#scenario-complete-virtual-directory-wire-and-content-lifetime).
Physical/foreign-target/DPI/UIA remain separate NOTRUN acceptance.
P208 password extension reuses original EntrySession authorize/revoke + worker no-output TEST and
InteractionPort; <=8perpipe/120second source-bound protected-memory grants. Explicit originalbrowser
unlock precedes a NEW heldgesture; GetData has no UI/passwordretry. See same production contract.
OPT-04 retained demands follow the [original runtime contract](../nxtools-archive/backend/product-read.md#scenario-drag-scoped-retained-materialization): one frozen plan/activity hold, genuine worker source authority, per-member native commit/Seal, retained Run/global slots and actual receiver/pipe/process cleanup. Preview shares never replace independent encrypted-drag authorization.
Same-stage source/target full-read reuse is archive-edit-only and reads [protected archive observations](archive-read-observation.md): complete live version equality,five fresh stages,chunk cancellation,FILE-only handoff and candidate fingerprint/postpublication/actual cleanup. It does not authorize ordinary extraction output hashing.
Private source/candidate/cache transport and ordinary extraction output read [bounded binary transfer](../nxtools-archive/backend/binary-transfer.md): capability agreement before effects,original Run/input/stdout pumps,acknowledged bounded frames for their existing directions,and `extraction_output_v1` <=32KiB original-pipe records without per-write ACK for ordinary final-name output. First native failure and actual cleanup remain authoritative.
Batch password reuse and source-folder selection read [archive workflows](archive-workflows.md): original session/batch-protected memory, live ticket/retirement, one owner-bound IFileOpenDialog vector and actual modal-exit reservation. Controlled fixtures do not accept real OS picker/keyboard/DPI interaction.
Formal row/receipt/reserved-cancel/error/native acceptance contract:
[P208 production contract](../../tasks/10-16-archive-manager/phase-f-p208-production-contract.md).
Phase F P206/P207 changes read [archive Shell reference](../../../docs/reference-code/archive-shell.md) and
the [explicit product enablement amendment](../../tasks/10-16-archive-manager/integration-enable-outcomes.md).
The historical `ProductApplyAuthorized=false` development restriction is superseded for source:
explicit user Apply/Revert now uses original HKCU plans/transaction, approved Desktop/helper leases,
bounded STA and detached reader route; facts persist before `applied`, no key-tree deletion.
This is not authorization to execute Registry/COM/Shell operations during development. The separate
x64 `csharp/NxTools.ShellSelectionHelper` is not registered by builds, startup, saving or closing UI.
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

Shell icon clarity reads [resolution-aware native extraction](native-contracts.md#scenario-resolution-aware-shell-icons): original bounded STA/cache, native high-resolution pixels, reserved type safety and legacy small-image cache separation. PNG fixtures do not certify installed/mixed-DPI appearance.

Configured preview follows [the frozen preview budget contract](../nxtools-desktop/backend/archive-preview.md):
strict optional options, original-purpose min-output quota and preview-only runner timeout; clear
options before private materialize serialization. Source/working/metadata/spool/slots and actual
lease/output cleanup remain original.
