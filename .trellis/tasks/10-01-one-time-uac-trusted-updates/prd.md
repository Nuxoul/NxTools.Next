# Investigate One-Time UAC and Trusted Silent Updates

## Goal

Eliminate repeated UAC prompts during ordinary NxTools updates while keeping `ProgramData` service binaries and the LocalSystem indexer protected from untrusted modification. Target behavior: one explicit authorization for initial trusted deployment; subsequent authenticated updates of the same approved installation apply without another interactive elevation prompt. If an update cannot be authenticated or safely applied, fail closed and ask the user rather than weakening integrity checks.

This task investigates and designs the update trust/deployment path. Product behavior changes require review of the final design and separate explicit approval.

## Confirmed Facts

- The Inno Setup installer declares `PrivilegesRequired=lowest` (`installer/nxtools-next.iss:12-27`); it is not configured to require elevation for ordinary installation.
- The desktop has no `requireAdministrator` manifest in the source tree. It redirects to the protected desktop when Desktop, PlatformHost, and Indexer hashes match (`rust/apps/nxtools-desktop/src/main.rs:483-565, 590-628`).
- Quick Launch file-index preparation sends `EnsureIndexerService` (`rust/apps/nxtools-desktop/src/quick_launch.rs:419-459`). PlatformHost `EnsureAsync` returns without elevation only if `IsInstalledAt(indexerPath)` is true and `TryStartService` succeeds; otherwise it launches itself with `Verb="runas"` and `--install-indexer-service` (`csharp/NxTools.PlatformHost/Windows/IndexerServiceInstaller.cs:46-104`).
- `IsInstalledAt` checks service registration, protected files, registered Indexer/Desktop/PlatformHost hashes, equality of current package and protected files, registered protected desktop path, and SCM properties (`IndexerServiceInstaller.cs:272-337`). A changed package hash makes the old protected tuple fail validation until an elevated deployment refreshes it.
- Elevated deployment validates package siblings and release-manifest hashes, stops the service and only exact protected-path processes, deploys the protected binary set, updates service registration/hash values, and starts the service (`IndexerServiceInstaller.cs:106-223`).
- Prior project discussion states the intended behavior as one UAC prompt per new package hash, not on every ordinary launch. The desired improvement is to remove repeated prompts during routine trusted updates, not to disable UAC globally or run the UI as administrator.
- A read-only local audit found three NxTools uninstall entries: two under the same legacy user install directory and one isolated 0.7.0 verification directory. Start Menu/Desktop shortcuts target more than one user install path; HKCU autostart targets the protected ProgramData desktop. This creates a credible multiple-install-path explanation to test, but does not alone prove the user's prompt frequency.
- On this machine, the protected ProgramData Desktop, PlatformHost, and Indexer hashes matched their registered service hash values, but all differed from the currently inspected Release package. The service was registered LocalSystem, automatic start, and running. Launching that different package and requesting index setup would fail the complete-package hash gate and request protected deployment authorization.
- The checked-in installer is `PrivilegesRequired=lowest`; inspected installer and protected executables report Authenticode `NotSigned`. Current authorization therefore relies on UAC consent, fixed protected paths, trusted package siblings/manifest hashes, and an atomic registered hash tuple—not publisher Authenticode verification.
- Local PlatformHost logs sampled for 2026-09-28 through 2026-09-30 contained no `indexer_service.ensure_failed` or `indexer_service.installed` events. The current registry/service snapshot cannot establish when or how many UAC prompts the user saw.
- Read-only comparison found no equivalent protected silent-update mechanism in `D:\Code\Throne-dev` or `D:\Code\satelite-proxy-main`. Throne keeps ordinary install/start standard-user and only elevates an explicitly configured whole-app admin mode; Satelite elevates only the Windows TUN core/helper via `ShellExecuteExW("runas")`, not application updates. Full evidence is in `research/reference-uac-comparison.md`.

## Requirements

- Trace all update/launch sources: installer upgrade, manual package replacement, any in-app/browser download or update path, Start Menu/Desktop shortcuts, HKCU startup, protected-copy relaunch, Quick Launch `EnsureIndexerService`, and service recovery/reinstall.
- Produce a payload-free timeline showing which executable path initiated `EnsureIndexerService`, whether `IsInstalledAt` passed, whether `TryStartService` passed, whether `runas` launched, installer parent/exit result, resulting protected hashes, and restarted process identity. Never log credentials, trust secrets, or clipboard payloads.
- Explain whether repeated prompts are due to one prompt per changed package, multiple active/stale install paths, a service start failure despite matching hashes, or a deployment/registration mismatch. Support the conclusion with measured hashes and registration values, not assumptions.
- Implement the selected trust direction in design only: **one-time user-consented pinned update public key plus a protected LocalSystem service broker**. The standard-user updater submits a signed complete-release manifest; the broker verifies and publishes the protected tuple.
- Define the signed manifest, product/version identity, Desktop/PlatformHost/Indexer/BrowserHost hashes, key enrollment/rotation/recovery, monotonic anti-downgrade state, atomic staging/publish/rollback, and failure UX.
- Preserve the atomic release rule: protected Desktop, PlatformHost, and Indexer remain one versioned set; startup may use the protected copy only when the complete approved set matches. Do not weaken path, hash, peer, ACL, or protected-directory validation to suppress UAC.
- Keep ordinary Desktop/GPUI/PlatformHost launch at standard-user integrity. Only initial protected-service installation or a verified protected update may elevate.
- Do not implement the update mechanism until the user reviews and approves the final design.

## Out of Scope

- Disabling UAC, setting global compatibility flags, embedding `requireAdministrator` in the desktop, or launching the whole UI elevated.
- Allowing arbitrary user-writable executables/paths or user-controlled requests to overwrite protected binaries.
- Updating only one member of the protected Desktop/PlatformHost/Indexer set.
- Modifying the installed service, shortcuts, registry, protected files, or ACLs during this investigation.
- Signing binaries with an unavailable/unapproved certificate or inventing a trust root without user approval.

## Acceptance Criteria

- [ ] Every observed UAC launch path is identified and classified as initial deployment, changed package, service recovery, or unexpected repeat.
- [ ] A read-only audit records source/protected hashes for Desktop, PlatformHost, Indexer, service registration values, install/uninstall entries, shortcuts, and startup route; redact user-specific paths in shared reports.
- [ ] The report determines whether the user's “every update” experience is expected once per changed package or an unintended repeat; missing historical updater evidence is explicit.
- [ ] At least two update designs are compared, with the pinned-public-key service broker selected as the current recommendation and its atomic rollout/rollback/recovery behavior documented.
- [ ] The recommendation enables silent updates only for authenticated/authorized complete package sets and fails closed on tampering, downgrade, partial deployment, or missing trust material.
- [ ] No product files, service registration, ACLs, shortcuts, or protected binaries change before user approval of the design.
- [ ] Focused tests/static checks for any later implementation are specified; this investigation itself makes no product-code changes.

## Decisions and Deferred Items

- Target behavior is “authorize initial trusted deployment once; no repeated interactive UAC for authenticated routine updates,” not “never elevate for any reason.”
- A service start failure also falls through to the elevated installer path; distinguish this from hash mismatch using logs/controlled evidence.
- Selected trust model: **one-time user-consented pinned update public key + protected LocalSystem service broker**. The broker, not the standard-user updater, verifies and publishes signed complete release tuples. Authenticode is deferred as a future publisher-identity enhancement, not the current implementation dependency.
- The signing public key, manifest format, key-rotation/recovery procedure, and release pipeline are still implementation inputs to define; the private signing key must never enter the repository or client package.
- Separate service/update UAC from user-invoked “Run as administrator” and elevated-directory helper prompts.
- Local state proves the current package/protected hash mismatch and multiple launch paths, but does not prove which route produced every reported prompt. Exact repeat frequency and historical update route remain evidence gaps.

## Approved implementation continuation

The user confirmed one authorization per updated package, not repeated authorization for
the same installed release, then approved implementation after the reference comparison.
The historical investigation-only restrictions above describe the earlier phase. Current
approved source scope is recorded in [implementation-approved.md](implementation-approved.md).
Live enrollment/service changes, production signing-key provisioning and installed native
acceptance remain separate; approval to implement is not proof that they were executed.
