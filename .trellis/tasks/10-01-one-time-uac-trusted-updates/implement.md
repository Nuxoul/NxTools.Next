# One-Time UAC / Trusted Silent Update Investigation Plan

> Investigation and design only. Do not edit installer behavior, service registration, manifests, ACLs, shortcuts, registry values, or protected binaries before the user approves the reviewed design.

## 1. Establish the user-visible trigger

- [ ] Distinguish initial installation, first launch after updating, later normal launches, OS login/autostart, launching from each shortcut, Quick Launch “EnsureIndexerService”, service recovery, and explicit “Run as administrator” actions.
- [ ] Ask for one consent/UAC screenshot only if the prompt's publisher/app name cannot be obtained from local audit artifacts; redact account and machine-identifying data.
- [ ] Record the exact package version/source and the launch target that produced each prompt.

## 2. Read-only install/update inventory

- [ ] Inventory all NxTools uninstall records, Start Menu/Desktop/Startup shortcuts, HKCU Run target, current/running executable paths, parent PIDs, and installer version metadata.
- [ ] Hash user install, selected Debug/Release package, and protected ProgramData Desktop/PlatformHost/Indexer/BrowserHost files.
- [ ] Read service ImagePath, ObjectName, Start, security descriptor, registered hashes, and registered desktop path without changing registry/service state.
- [ ] Check Authenticode status and available signing/update metadata; do not expose certificate secrets.
- [ ] Compare the inventory against `IsInstalledAt` condition-by-condition and report exactly which predicate would trigger UAC for each install root.

## 3. Trace every elevation edge

- [ ] Trace installer `PrivilegesRequired=lowest` and `[Run]` behavior.
- [ ] Trace startup protected-copy hash gate and restart chain.
- [ ] Trace Quick Launch file-index setup -> `EnsureIndexerService` -> `IsInstalledAt`/`TryStartService` -> `runas`.
- [ ] Distinguish unrelated user-requested Run As Administrator and elevated-directory helper flows from updater/service UAC.
- [ ] Correlate safe logs for ensure result, installer process parent/exit, service state, protected relaunch and restart; do not add payload-bearing telemetry.

## 4. Design and threat-model comparison

- [ ] Compare at least: current hash-only prompt-on-package-change, Authenticode publisher-signed service-mediated update, and one-time pinned-key service broker.
- [ ] Assess user-writable package replacement, TOCTOU, compromised updater, partial set, downgrade/replay, certificate/key rotation, service ACL, recovery after interrupted copy, and account/service boundary.
- [ ] Select a recommendation that provides silent routine updates only for authenticated complete release tuples while retaining UAC for initial trust enrollment and failing closed on verification failure.
- [ ] Define atomic staging/verify/publish/rollback and user-facing invalid-update behavior.

## 5. Evidence and user review

- [ ] Produce a concise matrix: trigger, launch path, source/protected hash predicates, service state, UAC process/parent, result, and whether prompt is expected or repeated unexpectedly.
- [ ] State if current machine evidence indicates multiple stale install roots or a service/hash mismatch; do not infer from a generic UAC appearance alone.
- [ ] Present the threat model, alternatives, recommended design, operational requirements, and any needed user-owned certificate/key decision.
- [ ] Stop for explicit user approval before creating implementation tasks or changing product behavior.

## 6. Validation/cleanup for investigation

- [ ] Confirm no service was stopped/reconfigured and no protected binary, registry value, ACL, shortcut, or installer setting was modified.
- [ ] Preserve existing worktree changes and the prior clipboard/UI task records.
- [ ] Validate task context manifests and scoped documentation diffs; do not commit as part of the investigation plan.
