# One-Time UAC and Trusted Update Design Investigation

## Decision boundary

This child investigates and proposes a design; it does not change UAC behavior. The security invariant is that only an authenticated, explicitly authorized, complete release set may replace binaries in the fixed protected service directory or alter the LocalSystem service registration.

Ordinary Desktop, GPUI, and PlatformHost launch remain standard-user. UAC is restricted to initial service installation or an approved update operation. Never solve repeated prompts by removing `runas`, relaxing `IsInstalledAt`, trusting a writable directory, or elevating the desktop process.

## Current deployment/check flow

```text
user/update entry point
  -> user-writable install/package binaries
  -> NxTools.exe launch and protected-copy hash gate
  -> Quick Launch file-index preparation sends EnsureIndexerService
  -> PlatformHost IndexerServiceInstaller.EnsureAsync
      -> IsInstalledAt(current package)
          -> true + TryStartService succeeds: no UAC
          -> false OR service start fails: ProcessStartInfo Verb=runas
  -> elevated PlatformHost validates package siblings/manifests
  -> stops service and exact protected-path processes
  -> deploys Indexer + PlatformHost + Desktop (+ BrowserHost) atomically enough to validate
  -> writes service configuration/hash registry values
  -> starts LocalSystem service
  -> current desktop restarts from protected path
```

Investigation must identify whether repeated prompts arise because the user launches one of several installed copies, because any one protected-set hash/registration field mismatches after update, because the service cannot start, or because the updater does not coordinate the update with the protected service set.

## Selected Trust Model: Pinned Update Public Key + Protected Service Broker

The recommended design is a one-time user-consented public-key trust anchor enrolled during initial protected deployment. A standard-user updater never writes `ProgramData` directly; it submits a signed, versioned complete-release manifest and staged package to the already-installed LocalSystem service broker.

The broker must verify:

- the pinned public key and signature over the manifest;
- product identity, complete Desktop/PlatformHost/Indexer tuple, required BrowserHost/assets, and per-file hashes;
- monotonic release version/anti-replay state and downgrade policy;
- fixed protected paths, no reparse/junction substitution, and bounded package size/count;
- staged files before stopping the service or replacing the last-known-good tuple.

Only after all checks pass may the broker atomically publish the complete tuple, update the protected version/trust metadata, restart the service, and return a correlated success. Failed verification, partial copy, stale signature, downgrade, missing key, or service restart failure must preserve the previous tuple and return an actionable non-UAC error.

The private signing key stays outside the client repository/package and is used only by the release pipeline. Key rotation requires an explicit signed rotation record and rollback plan; it is not inferred from a user-writable config or registry value.

Authenticode remains a future optional improvement for Windows publisher display, but it is not required for the first trusted-update design because no approved Authenticode certificate is currently available.

## Required design properties

- Atomic version tuple: Desktop, PlatformHost, Indexer, and required adjacent assets carry one versioned manifest; no partial tuple is accepted at startup.
- Trusted input: authenticate the package before elevation-free deployment; a registry hash that the same unprivileged updater can rewrite is not a trust anchor.
- Protected writer: only the already-installed elevated service/broker writes ProgramData binaries, registry service values, and ACLs.
- Path safety: fixed protected root; reject reparse points, junction substitution, path traversal, alternate filenames, and user-controlled destinations.
- Replay/downgrade: monotonic signed release version/counter or equivalent anti-rollback record.
- Transaction/recovery: stage and verify all files, stop only exact approved binaries/service, publish the complete tuple, update trust/version metadata last, restart service, and recover/rollback after power loss or failure.
- Consent UX: one clear initial UAC for installing the service/trust anchor; routine valid package updates show no UAC; invalid/untrusted updates fail closed with actionable guidance.
- Diagnostics: correlation ID across updater/broker/service, lifecycle outcomes and native codes only; no tokens, secrets, clipboard content, or user paths in routine events.

## Evidence required before implementation

- Current app install roots and hashes; all NxTools shortcut/startup command targets; uninstall entries and versions.
- Protected Desktop/PlatformHost/Indexer/BrowserHost hashes, ACLs, manifest hashes, and service ImagePath/ObjectName/Start/security descriptor/registered package values.
- Exact standard-user launch path and whether the desktop redirects to ProgramData before/after `EnsureIndexerService`.
- PlatformHost `indexer_service.ensure_failed`, `indexer_service.installed`, service start errors, installer process parent/exit, and protected relaunch outcomes, with user-specific values redacted.
- Release pipeline capabilities: available certificate/secret store, signing policy, offline verification and recovery support.

## Rollout / rollback considerations

- Do not add a silent updater until initial trust enrollment and rollback/recovery are proven on a disposable test VM or isolated test service.
- Keep the current UAC path as a fail-closed fallback during staged rollout; never silently accept unsigned binaries if signing is the chosen model.
- A failed update must retain or restore the last-known-good complete protected tuple and service startup path.
- User approval is required for certificate/key provisioning, update protocol/IPC changes, trust rotation, and any change to the frequency/scope of elevation prompts.
