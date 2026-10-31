# Archive Shell / Explorer reference note

Status: **2026-10-23 release-chain source and isolated test-key staging added** to the existing P206/P207 source. Helper remains unregistered; ProductApplyAuthorized=false. No real Registry, COM activation, Explorer, installation, service change or production release issuance. Historical source-round limitations below describe their dates.

## Sources inspected before the product seam

| Source | Exact finding | Boundary retained |
| --- | --- | --- |
| `D:\Code\Archiver-main\gui\src\shell.rs:56-107,116-138,653-675` | Pure Windows key generation writes a classic `SubCommands` cascade below `HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell\Archiver`, with leaf command lines using quoted `%1`. Its unregister deletes whole roots and its tests cover layout, not mixed ownership or grouped selection. | Reuse only the pure-plan shape. Do not copy whole-root deletion or infer grouped multi-selection. |
| `D:\Code\Archiver-main\installer\archiver.iss:81-114` | A ProgID, `shell\open\command`, and per-extension `OpenWithProgids` values populate Open With without writing `UserChoice`. | Next uses an owned ProgID/value plan only; setting a default remains a Windows Settings/user action. |
| `csharp/NxTools.PlatformHost/Windows/BrowserRegistrationTransaction.cs:13-42` | Existing compensation captures every target before the first write, records an attempted write before calling `Write`, and restores every attempted step in reverse. | P206 preserves that ordering through a fake-store-only archive transaction; no Registry-backed store was added. |
| `rust/apps/nxtools-desktop/src/archive_launch.rs:61-175` | Product ingress already accepts `--archive-open`, `--archive-extract {smart|here|name|dialog}`, `--archive-compress`, and `--archive-test`; it bounds/validates the resulting intent. | P206 command strings use existing ingress and remain explicitly one `%1` path per classic invocation. |
| Windows SDK `10.0.26100.0`, `ShObjIdl_core.idl:416-444` / `.h:2836-2981` | `IExecuteCommand`, IID `7F9185B0-CB92-43C5-80A9-92277A4F7B54`, receives invocation settings then `Execute()`; it has no path-array argument. | A grouped handler cannot obtain selection from `IExecuteCommand` alone. |
| Windows SDK `10.0.26100.0`, `ShObjIdl_core.idl:3487-3559` / `.h:10974-11152,11255-11349` | `IObjectWithSelection`, IID `1C9CD5BB-98E9-4491-A60F-31AACC72B83C`, supplies an `IShellItemArray`; the array IID is `B63EA76D-1F85-456F-A19C-48159EFA858B` and exposes `GetCount`/`GetItemAt`. | Exact grouped delivery requires one COM object implementing both `IExecuteCommand` and `IObjectWithSelection`, retaining the supplied array through `Execute`. |
| Windows SDK `10.0.26100.0`, `ShObjIdl_core.h:8826-8879` | `IShellItem::GetDisplayName(SIGDN_FILESYSPATH)` is the filesystem-path extraction boundary. | A handler must reject non-filesystem/failed items, preserve all selected items and never invent a path from display text. |
| Windows SDK `10.0.26100.0`, `Shlwapi.h:1705` | Association query vocabulary includes `ASSOCSTR_DELEGATEEXECUTE` as the CLSID of `DelegateExecute`. | The native selection handler needs COM activation registration; a command-line-only key is insufficient evidence. |

SDK/source inspection is not live Windows 10/11 Explorer acceptance. No installed COM sample or real Shell invocation was run in this authorization.

## P206 approved pure HKCU plan

Implemented in `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveIntegrationPlan.cs`:

- Default intent is disabled: classic cascade off, Open With off, no extensions and therefore no writes.
- The plan is limited to paths under `HKCU\Software\Classes` (represented as relative key paths; there is deliberately no Registry API/store implementation).
- Open With uses owned ProgID `NxTools.Archive`, owned display/icon/open-command values, and only the named `NxTools.Archive` value under each `.<ext>\OpenWithProgids` key.
- Classic menus use owned root `SystemFileAssociations\.<ext>\shell\NxTools.Archive`, an empty `SubCommands`, and owned open/smart/here/name/test leaves. Every leaf calls the existing startup parser with quoted `%1`; this is deliberately per-file delivery.
- The plan rejects duplicate targets, paths outside current-user Classes and any `UserChoice` path. It contains no CLSID/COM or Windows 11 top-level menu claim.
- Saved intent/revision is immutable desired state. `AppliedFacts` remains null on the plan. A successful store transaction separately returns observed applied facts containing the exact planned value plus its pre-state.
- Apply captures all target values/kinds and missing owned key ancestry before any write. It marks the current target attempted before `Write`; write-then-throw compensates that target and every earlier attempted target in reverse. Rollback failures aggregate.
- Uninstall/revert is value-exact and compare-before-restore: only a current value exactly equal in kind/data to the value this application wrote is restored/deleted. A foreign value or a user-modified formerly-owned value is preserved and reported. Cleanup roots are advisory to the eventual Registry store for deleting only empty keys proved created by that write; whole shared roots are never an uninstall primitive.

A real Registry-backed store, `SHChangeNotify`, command routing and settings UI registration are intentionally absent; parent integration owns those seams after separate authority.

## P207 grouped selection decision

### Classic command line is insufficient

The implemented P206 leaf is `"NxTools.exe" <archive action> "%1"`. The inspected Archiver reference has the same per-item shape and explicitly records one process per selected file. There is no verified group identity in `%1`, and no timer may combine near-simultaneous launches: unrelated Explorer actions would be indistinguishable and IPC05/P207 would be violated. `MultiSelectModel` is not used because no inspected SDK contract or isolated invocation proved that it gives this product one complete selection.

### Exact handler contract required for reliable grouping

A future grouped classic verb needs one native COM class with one CLSID that:

1. implements `IUnknown`, `IExecuteCommand` and `IObjectWithSelection` with the SDK IIDs above;
2. receives and retains the `IShellItemArray` in `SetSelection`, then during `Execute` reads bounded `GetCount`/`GetItemAt` entries and each `IShellItem.GetDisplayName(SIGDN_FILESYSPATH)`;
3. rejects null, non-filesystem, over-limit or partially unreadable selections as one invocation (no dropped items and no display-name guessing);
4. emits exactly one existing Desktop archive intent with one new correlation/request ID and the ordered path array; there is no timer/coalescing owner;
5. performs no codec work in Explorer/COM callbacks and preserves current Rust-owned validation, bounded intent admission and diagnostics;
6. has deterministic COM object/array lifetime and one terminal result; COM callback failure cannot be relabeled as successful delivery.

### Exact deployment/registration authority needed

The narrow recommended deployment is a dedicated signed x64 **out-of-process native COM local server** shipped beside the protected NxTools tuple, not managed code loaded into Explorer and not an in-process extension DLL. It must own `CoInitializeEx`, a class factory/`CoRegisterClassObject` lifetime, and activation only long enough to capture/forward one bounded invocation.

Current-user registration still mutates the Shell and is not authorized here. It requires, at minimum:

- `HKCU\Software\Classes\CLSID\{product CLSID}\LocalServer32` default value = the exact quoted helper executable path;
- each owned classic verb command key to use `DelegateExecute` = `{product CLSID}` instead of a `%1` command line for grouped actions;
- exact owned-value pre-capture, compensation and compare-before-uninstall equivalent to P206;
- packaged/protected helper identity, architecture, signing/hash and atomic desktop/PlatformHost/helper release ownership;
- explicit real Windows 10/11 classic-menu tests proving one `SetSelection` array/one `Execute`, ordering, filesystem rejection, cancellation/failure, uninstall, Explorer restart/cache behavior and no unrelated-intent union.

HKCU COM/class registration normally needs current-user write rights rather than administrator rights, but it remains actual Registry/Explorer mutation and a new packaged native executable/CLSID surface. **Implementation stops here until the user grants authority for the COM helper, its packaged deployment, current-user CLSID/DelegateExecute registration and live Shell acceptance.**

## 2026-10-22 formal source round (authorized P206 service + P207 helper source)

Newly verified/added entry files and call chains (source-backed, isolated tests only):

- **P206 explicit chain.** `rust/apps/nxtools-desktop/src/main_window/archive_page.rs` (集成 page:
  official Switch/Checkbox/Button) → `archive::shell_integration::request` → existing
  `ArchiveApplication` platform owner → `PlatformCommand::ArchiveShellIntegration`
  (`rust/crates/nxtools-protocol/src/archive_shell_integration.rs`) → C# `PlatformPipeClient`
  `archive_shell_integration` → `WindowsPlatformAdapter.ArchiveShellIntegration` →
  `ArchiveIntegrationService.Execute` → existing `ArchiveIntegrationPlan`/`ArchiveIntegrationTransaction`
  → `WindowsArchiveIntegrationStore` (HKCU Software\Classes only). Correlated result returns on a later
  GPUI turn; Disconnected retires pending requests as not-delivered.
- **Desired vs observed.** Rust `ArchiveSettings.shell_integration` saves desired intent plus a
  monotonic `desired_revision` (serde default off for legacy sections, unknown fields rejected). The
  native `FileArchiveIntegrationLedger` (`%LOCALAPPDATA%\NxTools\NxTools.Next\data`) persists observed
  applied facts atomically BEFORE `applied` is reported; a ledger failure compensates the writes.
  Apply over unreverted facts returns `revert_required`; revert keeps the ledger on partial failure.
- **Gate.** `ArchiveIntegrationService.ProductApplyAuthorized = false`: product Apply/Revert return
  `disabled/apply_not_authorized` before any capture/write; tests construct only fake stores.
- **Real adapter invariants.** Validates `Software\Classes\` prefix, target-within-owned-root and no
  `UserChoice`; restores exact prior value/kind; deletes one exact value; removes only captured-missing
  owned keys that are now empty (deepest first); never `DeleteSubKeyTree`; SHChangeNotify(ASSOCCHANGED)
  is an advisory hint whose failure cannot change the observed result.
- **P207 helper.** New `csharp/NxTools.ShellSelectionHelper` (WinExe, x64, net8.0-windows):
  `ArchiveSelectionCommand` implements `IExecuteCommand` + `IObjectWithSelection` with exact SDK vtable
  order; `ShellSelectionReader` uses `GetCount`/`GetItemAt`/`GetDisplayName(SIGDN_FILESYSPATH=0x80058000)`
  and rejects the whole selection on null/empty/count/item/non-filesystem/device/relative/over-256/
  32KiB/256KiB failures. `Execute` is once-only, freezes the selection, forwards one ordered array, then
  retires the array and its lifetime slot. Open/Test require exactly one item (no truncation).
- **Forwarding.** `DesktopIntentForwarder` verifies the exact sibling `NxTools.exe` (no reparse,
  release SHA-256; empty expected hash = unpackaged → `IdentityRejected`) and runs it once with existing
  `--archive-*` vocabulary and argv paths. The desktop's existing startup parser, single-instance check,
  `forward_running_archive` peer-authenticated pipe, `MAX_PENDING` queue and request ID own admission;
  no new IPC/scheduler. Exit 0 = forwarded; 2 = rejected; process alive at the bound = new cold owner
  (helper never kills it); other = unavailable.
- **Lifetime.** `LocalServer.Run` only for `-Embedding`/`/Embedding`: STA `CoInitializeEx`,
  `CoRegisterClassObject(REGCLS_MULTIPLEUSE|SUSPENDED)` per verb CLSID, `CoResumeClassObjects`, message
  pump until objects+locks reach zero (30s first-activation idle, 2min hard bound), revoke, uninitialize.
  Bounded 8 live objects; shutdown cancels pending forwards. Diagnostics: correlation, accepted/started/
  terminal, verb, count, failure kind, native HRESULT, elapsed — never paths.
- **Future registration plan (unapplied).** `ArchiveSelectionHandlerPlan` produces HKCU
  `CLSID\{..}\LocalServer32` = quoted exact helper path and per-extension cascade verbs with
  `DelegateExecute={CLSID}` (no `%1`), reusing the P206 transaction. No product path invokes it.

Still required before any live claim (separate authority): packaged/signed helper and its manifest hash,
protected-tuple/release membership decision, real HKCU apply/revert, CLSID/DelegateExecute registration,
Windows 10/11 Explorer invocation (one SetSelection array/one Execute, ordering, cancellation, restart).

## 2026-10-23 lifecycle repairs (F-AC01–03)

- Helper COM identity is explicit (`SelectionCommandIdentity`): final IUnknown::Release of a never-executed
  command retires it (selection + lifetime slot) synchronously; Execute still retires once.
- `LocalServer.PumpUntilIdle` treats a held `LockServer(true)` as activation; idle exit needs objects AND locks at zero.
- P206 ledger-failure compensation attempts every value in reverse, reporting incomplete truthfully.
  Details/evidence: `.trellis/tasks/10-16-archive-manager/phase-f-shell-repair-outcomes.md`.

## 2026-10-23 release-chain and isolated staging amendment

Newly verified current owners (not a new publishing system):

- `scripts/package.ps1` / `build.ps1` build Rust products then the original `nxtools-archive-worker`
  with `isolated-codecs`; `publish-release-hosts.ps1` runs actual Desktop SHA generation, helper publish,
  complete approval generation, PlatformHost publish, in that order. Explicit external MSBuild generated
  inputs keep staging separate from repository production trust/approval. Desktop never embeds helper
  or PlatformHost output hashes, so no circular build dependency.
- `write-shell-selection-identity.ps1` -> helper `Generated.DesktopReleaseIdentity` -> existing
  `Program.ExpectedDesktopSha256` -> `DesktopIdentity.Verify`: actual sibling Desktop hash only;
  empty source-build default and stale/wrong/missing Desktop still refuse. COM selection/refcount/pump
  and ordered once-only forwarding are unchanged.
- `write-release-binary-manifest.ps1` -> `Generated.ArchiveReleaseManifest` maps worker plus helper
  apphost/DLL/deps/runtimeconfig; original `ArchiveReadHelper.Acquire` verifies under retained source
  namespace lease before original runner Job admission. No PATH/environment production authority.
- `TrustedReleaseManifest.RequiredPaths` authenticates all five new members. Original
  `IndexerServiceInstaller` stable-handle source approval/deploy/readback and registered archive hashes,
  original broker all-file verification and `TrustedUpdateTransaction` full snapshots/last-good recovery
  remain the deployment chain. Rust `protected_package_matches_release` compares all five new siblings
  in addition to the existing Desktop/PlatformHost/Indexer identity, refusing partial/mixed redirect.
- `installer/nxtools-next.iss` adds five exact file entries; original `close-owned-processes.ps1`
  suspend/finalize now journals twelve owned root files including signed sidecars, captures all old/new
  hashes before mutation, records each attempt, compensates all reverse attempts and preserves foreign
  files. Exact same-handle deletion remains required; partial recovery retains journal/failure.
- Explicit `-IsolatedStaging` requires fresh scratch output, SkipVerification/SkipInstaller and no
  supplied production key/sequence/LegacyUnsigned. Original key tool creates only temporary test trust;
  original signer signs final actual files. Production generated trust/approval and sequence21 outputs
  remain untouched. Outside-package labels mark this NOT installable/production-trusted; test key deleted.

Installed stable bootstrap compatibility/maintenance, real signed distribution, installation, service,
Registry/COM activation/Explorer and P208 physical gesture remain separate, NOT RUN. Staging does not
prove a currently enrolled old bootstrap can accept newly allowed paths. See current release outcomes
and sole checkpoint; historical acceptance reports are not rewritten.

## F-RC01/F-RC02 bounded repair — 2026-10-23

`package.ps1` now claims its exact fresh TEST key inside the original main try/finally, before creation;
trust/label/preflight exceptions cannot escape cleanup. No production key/ledger writer is used in tests.
Inno PrepareToInstall -> default helper `Prepare-UpgradeBrowserHost` validates an existing schema2
journal and original file facts -> original committed cleanup OR precommit reverse compensation ->
complete readback/journal retirement -> original fresh Suspend. New manifest never grants old-delete
authority. Failed preparation does not set UpgradePreparationReady and cannot Finalize an old journal.
LibraryOnly two-entry regressions are not real installation acceptance. Current protected user package
is sequence22; its files/production Generated/ledger and historical reports remain untouched in repair.

## Directed association/menu settings completion (2026-10-25)

User authorizes source/isolated completion only, ProductApplyAuthorized remains false. Re-read
Bandizip public UTF-8 IDD_SETTING_ASSOC/SHELL resources and Archiver shell.rs windows_keys56–107:
default-app entry, double-click open/smart/name, per-command choices and classic cascade are real
reference vocabulary. Raw PNGs remain uninspected after earlier viewer failure; no visual match.
Current helper entry is ShellSelectionHost.cs HelperIdentity/IntentArguments and ShellSelectionCore.cs,
not historical individual filenames. Existing COM selection owns one complete array; menu layout
does not change that contract. Classic flat verbs are separate owned HKCU shell children, not
Windows11 IExplorerCommand/top-level-menu support. Never delete shared roots.

Boundary: fixed empty-data OpenDefaultAppsSettings -> C# fixed ms-settings:defaultapps operation,
not a general URI allowance. Saved double-click open/smart/name config changes the planned owned
ProgID open command only after explicit authorized Apply. Smart/name route to existing extract
form through smart-dialog/name-dialog modes and retain destination/risk confirmation before Start.
Selected seven stable commands: open,test,extract_smart,extract_here,extract_name,extract_dialog,
create. Default missing fields preserve legacy five commands/cascade/open; null/unknown/duplicate
inputs fail closed. Original save lock/order, deferred entities, native Apply/Revert transaction,
compensation and helper identity/selection owners remain. Official Settings public field slots,
Checkbox/Small outline Button/dropdown; no browser/tree/drag/vendor/dependency changes.

Verified implementation chain: `ArchiveSettingsView::save_integration` -> original Core locked
changed-field save/order -> `ArchiveApplication::request_integration` -> strict protocol/parser ->
`ArchiveIntegrationService` -> original helper approval lease -> selected-only plan/transaction.
Fixed settings opener uses the existing detached command reader path and injected no-argument
operation; Shell acceptance is not Settings UI readiness. Final option/owner/consumer/test table and
native/visual limitations: [shell settings outcomes](../../.trellis/tasks/10-16-archive-manager/shell-settings-outcomes.md).
Executable cross-layer contract: [native contracts](../../.trellis/spec/platformhost/native-contracts.md#scenario-archive-association-and-classic-menu-settings-2026-10-25).

## Installer31 process-close code5 — actual updater role evidence (2026-10-27)

Actual preparation JSONL reports process_close/Win32Exception5/scriptline121, before any file
preparation. Readonly SCM identifies the CIM-null Host PID as NxToolsUpdateV1 at the fixed
Updater/Bootstrap/NxTools.PlatformHost.exe --trusted-update-service, LocalSystem, registered
bootstrap hash equal actual file. Original ordinary Stop-UpgradeTargets iterates this filename
and throws before distinguishing the resident updater from desktop hosts. Ordinary upgrades must
NEVER terminate this independent service; original TrustedUpdatePeer already uses SCM_CONNECT1/
SERVICE_QUERY_STATUS4/QueryServiceStatusEx Running4/ownprocess0x10/PID instead of opening SYSTEM.
Reuse that readonly boundary plus fixed registration/account/plainpath/hash for exclusion ONLY;
other unreadable same-name processes retain failclosed5. Actual lifecycle repair/tests recorded in
the original archive task, no permission for live service stop/preparation/install or key/ledger reset.

## Installer32 process-close native31 — exited-object race (2026-10-28)

Actual user32preparation reports process_close/MethodInvocationException->Win32Exception31 at
original Stop call, before file_preparation. QueryFullProcessImageName runs BEFORE exit check in
the delivered native helper. Owned child exited with retained process handle reproduces31 on
both x86/x64; original Inno PrepareToInstall with stale owned discovery returns helper1/setup7,
fixed helper completes same isolated installation. Historical actual PID state was not retained,
so this is a matching proved trigger, not proof every older code1 had the same cause.
Use the SAME acquired native handle: signalled before query -> harmless false; query/terminate
failure -> preserve error then recheck signalled handle, only observed exit is harmless. Live
unknown-image/accessdenied31/5 and wait failure remain failclosed; no blanket code31 suppression,
PID/name guessing, unrelated process stop or journal/file mutation. Native failure Data contains
fixed native_api open/query_image/terminate/wait for payload-safe production diagnostics.
Formal gate installer-process-exit-race.ps1 covers actual owned live/exited children plus controlled
API races; package.ps1 runs x86/x64 and real Inno exited-process scenario before compilation.

## Installer38 CIM-null process identity boundary (2026-10-30)

Actual correlation20261030104807-is-KYQR3HU6JV-tmp retains process_close/native5/scriptline199,
before file_preparation. The supplied path is a shortened timestamp; actual diagnostic directory
includes the Inno temporary suffix. No PID or elevation fact was retained, so the historical
offending process cannot be identified from this log alone. Current readonly x86/x64 empty-target
Stop-UpgradeTargets correctly excludes approved NxToolsUpdateV1; the prior updater repair is present.
The existing exit-race regression covered only stale candidates WITH an image path. Retained exited
owned child + actual discovery body with CIM-null image reproduces this exact line/native5 on both
architectures: no native exit check is reached. The native query boundary must open only SYNCHRONIZE
plus PROCESS_QUERY_LIMITED_INFORMATION, accept only signalled/gone objects as absent, otherwise read
the image and retain same-handle exit/error checks. Existing Stop still reacquires/reverifies exact
image before termination. Unknown live access failures remain errors; updater exclusion remains SCM
proved and independent. Add bounded process_id/fixed process_name/native_api and elevated fact to
the original correlation diagnostics, without paths/commandline/raw errors. This proved reachable
trigger is not evidence that the unrecorded user process had exited; actual installed acceptance
still requires the original preparation path. No blanket5 suppression or journal deletion.

## Installer41 native image query while closing a running tuple (2026-10-30)

Actual correlation20261030160936-is-LYS9I7BVR4-tmp reports elevated=true/process_bits32,
process_close/native5/native_api=query_image at Stop-UpgradeTargets's native Stop call, not
CIM-null discovery or missing elevation. Stop's query-failure branch checks signalled state only
with WaitForSingleObject(handle,0); a terminating object may lose query access before signalling.
The existing regression assumed exit was already signalled immediately after query failure.
Original Stop body with query5 + zero-timeout258 + bounded-wait0 fails that regression; live denied
and wait failures remain mandatory negatives. Reuse original5000ms bound only after5/31, on the
same acquired handle; return absent only for actual signalled exit, otherwise retain the original
query/terminate native error. Do not terminate unknown images or suppress5 by number. Move original
PID/fixed-process-name failure attribution around the full candidate Stop call so this branch is
observable. Prior real38installation had no running desktop and does not establish running-tuple
upgrade acceptance; require a separately authorized actual running-product upgrade before closing
this new report. Original user41log lacks PID, so the historical object cannot be identified later.
