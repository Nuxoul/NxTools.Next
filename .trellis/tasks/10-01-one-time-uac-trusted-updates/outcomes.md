# Trusted update implementation outcome

## Delivered source behavior

The user-approved one-time trust mechanism is implemented in source, not merely moved to installation time. A separately protected NxToolsUpdateV1 bootstrap verifies RSA-PSS/SHA256 complete release manifests with its pinned public key, stages from locked caller-readable source handles, deploys through the existing exact-owned indexer deployment boundary without runas, validates all deployed files/SCM tuple, and commits the monotonic sequence last. Same signed identity is idempotent; invalid key/signature/tamper/partial/traversal/reparse/replay rejects without UAC fallback. Initial legacy migration retains one explicit authorization; ordinary UI remains standard-user.

Prepared transactions retain a signed prior snapshot and recover after failure/restart. Broker and deployed Indexer/PlatformHost are separate so self-stop cannot strand the updater. Cross-process deployment ownership prevents a surviving child racing recovery. Stable bootstrap/key rotation/maintenance are not silently performed.

Publishing requires an explicit private key outside client/source/package and an increasing sequence. Public SPKI is embedded before publish, final package is signed afterward. Publisher sequence is reserved before Cargo under a key-adjacent durable ledger. No configured key -> early failure before Cargo/process stop/package overwrite. Explicit LegacyUnsigned is not a trusted release and cannot update enrolled installations.

Installer includes signed sidecars, prepares enrolled update before original-user launch, blocks launch/returns exit9 on failure, and preserves/restores exact prior BrowserHost backup until trusted success.

## Regression / verification actually performed

- Real EnsureCoreAsync routing regression ran red: enrolled synthetic newer tuple called InstallAsync/UAC path (selected2/run1/fail1). After integration it passes and rejection never falls back to UAC.
- Final C# build: zero warnings/errors; `trusted-update-verified-build.log`.
- Final audited fixture suite: selected223/run223/pass223/skip0/fail0/not_run0; `trusted-update-verified-safe223.log`.
- Focused signed release/routing/transaction/enrollment subset:17/17; temporary RSA keys/artifacts only.
- Production Inno callback/replacement fixtures:10/10, including success, failure, exact-owned process, trusted success/failure, normal rollback, rollback-after-file, commit failure, trusted-failure restoring exact old BrowserHost. `trusted-update-installers-final10.log`.
- Existing owned-process recovery gate18/18.
- Publisher sequence reserve/repeat/downgrade/prior-output floor:5 isolated assertions.
- Same-handle exact file recovery:2 assertions (wrong hash preserves file; right hash deletes only target).
- Indexer lifecycle source gate and safe-test-selection gate passed.
- Signer delegate reported production-verifier scratch harness6/6, isolated helper Release build0warnings/errors, actual PowerShell5 execution of all3 sign/trust/key scripts with temporary keys, no production key/registration operations.
- Missing-key package preflight executed and rejected before build/mutation.
- Scope is C#/installer/scripts/docs; no Rust source change or Cargo verification was needed or run.

## One bounded review and fixes

The optional review found premature BrowserHost backup deletion, incomplete enrollment treated as complete, unobserved service-loop failure, and missing publisher sequence monotonicity. Fixed by delayed backup commit plus exact-file rollback, durable pinned enrollment resume, observed loop failure with SCM restart, and publisher-side reserve ledger. Retested concrete boundaries; no second broad review.

## Remaining release prerequisites (not performed)

No production signing key is configured (`NXTOOLS_RELEASE_SIGNING_KEY` absent; generated default trust empty). No real key created/read, no signed product package built, no formal installer executed, no UAC accepted, no live service/registry/ACL/shortcut changed, no commit/push.

The source fixture result is not installed acceptance. Require a disposable VM/separately approved native test for initial enrollment and partial-enrollment SCM retry, medium-user signed upgrade without UAC, reboot/crash recovery with surviving child, bootstrap service readiness/stop/restart, current protected runtime hashes, installer first launch and failure behavior, and LocalSystem Indexer restart/base+delta/search/idle CPU. Bootstrap repair/key rotation/downgrade/uninstall cleanup require explicit maintenance policy; no claim those operations are implemented or accepted. Existing uncommitted user changes remain preserved.

Publisher instructions: [release-operations.md](release-operations.md). Reference limits: [research/listary-update-uac-comparison.md](research/listary-update-uac-comparison.md). Current approval: [implementation-approved.md](implementation-approved.md).
