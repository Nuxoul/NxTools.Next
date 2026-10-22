# Listary and supplied references: update authorization

## Outcome

The user confirmed the exact symptom: one UAC prompt on the first NxTools launch after each update; later launches of that same release do not prompt. This is the current protected-release deployment policy, not evidence that ordinary Desktop execution requires elevation.

Reference inspection does not establish that Listary performs every protected update without UAC. Do not present its updater setting as proof of that behavior. No product code, installed service, registry, ACL, shortcut or reference binary was changed in this review. No installer/update/product was executed and no update feed was requested.

## Listary: verified managed boundaries

Installed root: `D:\Program Files\Listary`. The inspected UI reports file version `6.3.5.94`; its adjacent `AutoUpdater.NET.dll` reports `1.7.6.0`. These are observed file metadata, not guaranteed release compatibility or authenticity.

| Entry | Verified behavior |
| --- | --- |
| `.diagnostics/listary-managed/Listary/Core/ListaryApp.cs:121` | Calls the managed updater's `CheckOnStartUp`. |
| `.diagnostics/listary-managed/Listary/Core/AutoUpdate/AutoUpdater.cs:16-21` | Sets `AutoUpdaterDotNET.AutoUpdater.RunUpdateAsAdmin = false`, then subscribes to update-check results. |
| Same file, `CheckOnStartUp:25-31` | Delays ten minutes before checking; checks when enabled or beta, and not while a foreground application is fullscreen. |
| Same file, `Check:35-42`, `method_1:68-99` | Starts the configured feed check and displays the update form when a newer version is available. |
| Adjacent `AutoUpdater.NET.dll`, `AutoUpdaterDotNET/UpdateForm.cs::UpdateForm` download handler | Calls `AutoUpdater.DownloadUpdate`, separate from ordinary application initialization. |
| Adjacent DLL, `AutoUpdaterDotNET/DownloadUpdateDialog.cs:72-173` | Downloads to a temporary file; compares a checksum only when supplied; resolves installer arguments; builds a Shell launch for the downloaded artifact. `Verb = "runas"` is assigned only when `RunUpdateAsAdmin` is true. |
| `.diagnostics/listary-managed/Listary/Core/Class139.cs::method_4:116-133` | Waits up to fifteen seconds for a registered service to be Running. |
| Same file, `method_0:18-52` | Connects through `IListaryService`; otherwise starts an adjacent helper, then verifies the service's reported version. No `runas` assignment appears in this inspected method. Obfuscated string values and the helper's own native/manifest behavior were not resolved. |

Read-only SCM query confirmed `ListaryServiceV2` is Running, automatic, LocalSystem, with image `D:\Program Files\Listary\Listary.Service.exe`. This establishes privilege separation, not an updater broker.

The adjacent updater DLL was decompiled read-only with the existing `dnSpy.Console.exe` into session scratch `listary-uac-autoupdater`; command exit was 0. It and the existing managed output were inspected, not executed or copied into product code.

### What this does not prove

- `RunUpdateAsAdmin = false` prevents the updater library from explicitly adding the `runas` verb. Windows may still elevate a downloaded installer according to its manifest or installation requirements.
- No current Listary installer, update-feed payload, installation transaction, signed release-manifest protocol, protected self-update broker or rollback implementation was inspected.
- Waiting for a service and checking its reported version is not equivalent to NxTools' complete release hash and peer-authentication contract.
- Listary's native index engine source was not available. Managed inspection is not native implementation evidence.
- The updater's optional checksum is not by itself an independently authenticated update signing key; do not transplant it as a privileged trust root.

## SwiftList: real source, but not a silent-update trust model

Root: `D:\Code\SwiftList-main`.

- `App/Services/ServiceManagement/AppStartupServiceBootstrapper.cs:8-34`: first ping the service; then try an existing service without elevation; only then attempt installation.
- `App/Services/ServiceManagement/ServiceInstallManager.cs:80-106`: the method called by `SilentInstall` still launches `--install` with `Verb = "runas"`. Its name does not mean UAC-free installation.
- Same file, `IsInstalledAtCurrentPath:114-135`: compares registered executable path to the current service path. This is not NxTools' protected complete-release integrity validation.
- Same file, `TryStartWithoutElevation:156-184`: invokes `sc.exe start` without Shell execution, with a bounded wait; accepts already-running as success.
- `Installer/installer.iss:62`: requires admin installation. `PrepareToInstall:210-216` stops the service before replacing files. `[Run]:100-102` launches the UI with `runasoriginaluser`.
- This installer does not, in the inspected post-install callback, establish the proposed signed silent-update broker. Its pre-copy name-wide `taskkill` commands are unsuitable for NxTools' exact-owned process policy.

Adoptable principles: explicit installation versus normal startup; original-user UI launch; bounded reuse/start of an existing service. Do not adopt path-only approval, accept old-release availability as current-release approval, or copy process-name-wide termination.

## WinShun: explicitly unsuitable privilege policy

Root: `D:\Code\WinShun-main`.

- `resources/app.manifest:21-29` explicitly requests `requireAdministrator` for the whole application to use NTFS APIs.
- `installer/WinShun.iss:40` requires admin; `[Run]:107` uses `runascurrentuser`.
- `src/app/Updater.cpp::check/finishCheck` checks GitHub release metadata asynchronously. `openDownloadPage:183-188` opens a release page, not an authenticated protected deployment.

This is not an example of standard-user UI plus UAC-free protected updates. NxTools must not adopt whole-application elevation or replace its LocalSystem indexer with this policy.

## Old NxTools and previous comparisons

`D:\Code\NxTools\app.manifest` contains no requested-execution-level admin declaration. The inspected legacy installer script did not expose a service/trusted updater path. This is not evidence of a protected silent-update solution.

The earlier [Throne/Satelite comparison](reference-uac-comparison.md) remains scoped evidence: ordinary standard-user launch and operation-specific elevation, not authenticated replacement of NxTools' protected release set.

## NxTools: root cause and two distinct outcomes

Current source owners:

- `repo:installer/nxtools-next.iss`: `PrivilegesRequired=lowest`; files are installed into `{app}`. The protected service deployment is deferred to the application. Cleanup may separately request elevation on access denial.
- `repo:csharp/NxTools.PlatformHost/Windows/IndexerServiceInstaller.cs#DecideDeployment`: a healthy existing tuple with different Desktop/PlatformHost/Indexer hashes produces `Upgrade`.
- Same owner, `EnsureCoreAsync`: `Upgrade` calls `InstallAsync`, after recording `upgrade_authorization_requested`.
- Same owner, `NativeDeploymentPlatform.InstallAsync`: launches PlatformHost with `Verb = "runas"` and `--install-indexer-service`.
- `repo:csharp/NxTools.PlatformHost.Tests/IndexerUpgradeDecisionTests.cs`: current fixture contract explicitly expects authorization for every mismatching complete release identity.

### A. No extra authorization on first launch after installing an update

Coordinate protected deployment during the installer/update transaction; if that operation needs authorization, request it there. Verify/publish the complete protected release before reporting update success; then launch UI as the original user. This improves launch behavior but does **not** mean one initial authorization forever. Do not quietly change every per-user install into an admin install, assume a child UAC grant elevates its parent installer, or weaken tuple validation.

### B. One initial trust enrollment; no UAC for subsequent authenticated routine updates

Retain the existing [selected design](../design.md): a protected update executor verifies a signed complete-release manifest against a previously enrolled key, then stages and publishes the release with rollback. This remains a design recommendation, not a confirmed Listary mechanism or an implemented NxTools capability.

Before implementation, specify signing-key ownership/provisioning, monotonic release identity independent of the repeatedly used `0.7.0` display version, broker isolation/IPC authorization, safe self-update handoff (the executing indexer cannot stop itself and finish replacement), power-loss recovery, key rotation, downgrade policy and migration from existing unsigned installations. A LocalSystem endpoint accepting arbitrary user-selected unsigned binaries is not an acceptable shortcut.

Required acceptance: same-release launch without elevation; trusted newer complete release without UAC after enrollment; tampered/partial/replayed release rejection; old tuple remains usable after cancel/failure/crash; exact-owned shutdown; original-user UI; correlated bounded diagnostics. Prove privileged deployment/self-update in a disposable VM or separately authorized isolated service, not the user's installed service.

## Verification and remaining boundary

Source paths/call chains and live service metadata were read. No compiled or real native behavior acceptance is implied. The user authorized reference research, not product implementation, signing-key enrollment or live service changes. The task remains investigation/design; implementation needs approval and the operational signing inputs above.
