# Trusted release operations

## What changes for the user

A legacy unsigned installation needs one explicit administrator authorization to install the independent `NxToolsUpdateV1` LocalSystem bootstrap and pin its embedded public release key. Ordinary Desktop/GPUI/PlatformHost stay standard-user. Subsequent signed complete releases are verified and deployed through this bootstrap without `runas`; a signature failure or downgrade is a rejected update, not another UAC prompt to bypass verification.

The stable bootstrap is outside the replaceable service tuple. Routine release updates do not replace it. Bootstrap security updates, key rotation, enrollment repair and deliberate downgrade need a separately designed explicit maintenance operation; no silent trust replacement is implemented.

## Publisher setup (not performed on production keys here)

Use an existing restricted directory outside the repository and package artifacts. Create a key only when choosing to provision the publisher identity:

```powershell
.\scripts\new-release-signing-key.ps1 -KeyPath 'C:\private-release-keys\nxtools-release.pem'
```

This example directory must already exist. The command never overwrites a key. Protect and back up the private PEM; losing it means losing the ability to sign routine updates under the enrolled identity. No PEM or private material belongs in Git, browser extension, installer or ProgramData.

Build an authenticated release with that same key and a strictly increasing signed sequence:

```powershell
.\scripts\package.ps1 -SigningKeyPath 'C:\private-release-keys\nxtools-release.pem' -ReleaseSequence 1
```

Each later release uses a larger sequence. The existing display version `0.7.0` is not an anti-replay counter. Public SPKI is embedded before the C# publish; final package content is signed after all binaries/assets are assembled. Manifest and signature are `NxTools.release.json`/`NxTools.release.sig`. Signing failure aborts packaging; missing key/sequence fails before Cargo or product operations. `NXTOOLS_RELEASE_SIGNING_KEY` can supply only the explicit private PEM path.

The publisher sequence is reserved under a locked durable ledger beside the private PEM
(`.release-sequence.json`) before building. Reusing/decreasing it or the previous package's
sequence fails preflight. Failed builds may leave sequence gaps; do not delete/reset this
ledger to reuse an identity. Keep the ledger with the key across publishing machines.

`-LegacyUnsigned` deliberately creates the former non-trusted package. It does not provide UAC-free updates and cannot update an enrolled trusted installation through the broker. Do not use it to label this feature complete.

## Installation and updates

The installer includes signed sidecars and invokes `--prepare-trusted-update` before offering a normal original-user launch. If no bootstrap exists, first application service preparation enrolls it through the existing explicit UAC boundary. Once enrolled, valid later updates are prepared by the resident service during installer finalization. Manual replacement remains supported by the application's existing EnsureIndexerService call; it uses the same broker for signed newer releases.

The broker accepts bounded local-package requests only, verifies its pinned key and complete package before interpreting paths, rejects reparse points, copies from locked source handles opened under the caller identity, then deploys from protected staging. The destination is fixed. It does not accept arbitrary commands or caller-selected installation destinations.

A durable transaction retains an independently verified prior package. Deployment failure rolls back before returning failure; restart replays an uncommitted journal. Sequence commits only after the deployment and complete protected-file validation succeed. Client disconnect after preparation does not interrupt mutation halfway. A protected kernel deployment gate prevents a surviving old child and recovery child from writing concurrently.

## Diagnostics / boundaries

Events: `indexer_service.deployment` routes enrollment versus trusted update; `trusted_update.broker` and `trusted_update.transaction` record accepted/prepared/deployment/commit/recovery/reject/elapsed/native outcome using one correlation ID. No PEM, package path or payload in routine logs. Installer callback uses fixed started/completed/failed events and blocks auto-launch on failure.

Source fixtures use GUID-owned temporary keys/packages; no real LocalSystem service or installed environment is touched. Before distribution, require disposable-VM acceptance for initial enrollment, medium-integrity signed upgrade, first-launch behavior, reboot recovery with an in-flight child, SCM start/restart/stop, denied file access, exact protected runtime hash and indexer restart/search/idle CPU. Real key provisioning and this privileged acceptance have not occurred in this source-only task.

Enrollment recovery uses a protected enrollment.json binding public key, bootstrap hash
and initial signed release. Interrupted enrollment is not treated as completed; only
the same identity can resume. Existing completed trust cannot be replaced through retry.
Installer BrowserHost backup is committed only after trusted update success; failure
restores the old host after exact-handle verification of the newly installed file.

## Explicit same-key bootstrap maintenance (2026-10-23)

The user-approved single installer (sequence25 onward) performs explained same-release migration.
Before suspending/replacing application files it reads the fixed updater registration/hash/key and
protocol marker. Absent/compatible updater skips migration; mismatched identity rejects. A legacy
updater prompts for explicit administrator approval once; the elevated preparation process invokes
the original `--maintain-trusted-bootstrap` then continues ordinary file preparation. Cancel/failure
blocks replacement. The former two-package workflow is superseded; no manual maintenance sequencing.
Sequence26 removes the duplicate migration ZIP: Inno ExtractTemporaryFiles('{app}\*') reuses the
normal-install compressed resources with relative directories intact. The compiled PlatformHost SHA
is checked before maintenance execution; the original maintenance verifier still authenticates the
complete package using the same publisher key, fixed service scope and anti-downgrade floor.

`TrustedBootstrapMaintenance` verifies the enrolled key/current bootstrap/service identity and a
strictly newer signed package; preserves Releases/state.json and snapshots; journals old/new exact
hashes before replacement. Original update service is stopped before taking the original deployment
gate. A shared maintenance admission lock prevents new business updates during ready/commit/cleanup.
New service readiness is an authenticated read-only pipe frame, not SCM Running alone. Before commit,
failure restores old bootstrap/enrollment/registry hash; committed cleanup never rolls back business
state and can resume. Failed/incomplete recovery retains its journal. The Windows adapter fixes
service/path scope and never installs a missing updater. Runtime maintenance is separately authorized.

New package admission remains strict. `VerifyRecoverySnapshot` accepts historical required-member
sets ONLY after authenticating the signed manifest and matching durable sequence/releaseID/manifestSHA.
Such a recovery VerifiedRelease cannot be used as a new update request. Original transaction restores
the old package using its own verified deployment child, then exact-hash removes only failed-new archive
members absent from the old signed set; foreign bytes reject cleanup and preserve recovery evidence.
Source fixtures and offline real-snapshot checks do not certify actual SCM/admin/installed acceptance.

## Explicit missing-root enrollment repair (2026-11-06)

User-approved repair distinguishes a genuinely new machine from a surviving approved service
registration whose entire Updater tree is missing. The normal installer offers one explained
administrator recovery only for the latter, using `--repair-missing-trusted-bootstrap <GUID>`
from the complete same-publisher signed payload. Fixed path, LocalSystem, pinned key and valid
registered bootstrap hash are checked before the service is stopped, and again under the original
deployment gate. The original protected enrollment initialization/register/start/validation path
is reused without its ordinary-mode already-registered early return. Actual file, completion
record and compatible protocol marker must agree before normal installation proceeds.

This is not a key rotation, downgrade, arbitrary damaged-tree repair or permission to delete
remaining records. Existing roots, unknown ownership and access denial still refuse. Diagnostics
now preserve detection stage and actual process launch/exit before early failure. No real service
or installed-product recovery is authorized by the implementation fixtures themselves.
