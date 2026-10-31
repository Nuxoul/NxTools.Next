# Phase F release-chain / isolated staging — independent acceptance

Date2026-10-23. **NOT ACCEPTED: F-RC01 and F-RC02 confirmed.** Existing controlled tests and actual
staging identity/signature checks PASS at their stated boundaries; they do not cover these failures.
Current status belongs only to [checkpoint](phase-a-checkpoint.md). [Implementation outcomes](phase-f-release-chain-outcomes.md)
and all earlier acceptance reports remain immutable. No production source repaired by acceptance.

## Confirmed findings

### F-RC01 — temporary staging private key is outside its cleanup try/finally

`scripts/package.ps1:40-47` creates TEST-ONLY-private.pem, writes test trust and the outside-package
label before entering the main try at131–132 / finally238–242. A failure in that setup interval
leaves the test private key behind, contradicting the promised success/failure cleanup boundary.

Independent probe supplies a fresh scratch output whose immediate parent does not exist. It passes
staging parameter validation, generates a TEST key, then label WriteAllText throws
DirectoryNotFoundException. Actual: expected_prebuild_failure=true, remaining_temporary_private_keys=1,
output_created=false. No Cargo/service/product operation reached. Probe removed only its own exact
TEST-ONLY-private.pem afterward; cleanup.json confirms0 remaining. No production key read or changed.

Evidence: parent scratch `stage-key-failure-d8bce9f0fb2147ca8e0f3f8b5f8241de/{result.json,cleanup.json,run.log}`;
runner `staging-key-cleanup-probe.ps1`. Fix the existing owner so every step after key creation is
covered by cleanup (including trust/label/preflight failures); add a prebuild failure regression.
Do not solve only this specific missing-parent input while other setup exceptions still bypass finally.

### F-RC02 — durable installer cleanup recovery is unreachable on a new installer attempt

`installer/nxtools-next.iss:92-135` PrepareToInstall always invokes fresh suspension, without
-Finalize/-Committed. `installer/close-owned-processes.ps1:340-347` refuses any existing journal.
The recovery-capable finalize path is only used after PrepareToInstall succeeds (iss138–165,184–196).
After a commit cleanup/journal-delete failure, restarting the installer therefore cannot reach the
otherwise valid cleanup retry and repeatedly fails with the retained journal. The user-facing message
atiss130–132 says retry, but that retry cannot advance. One bounded read-only installer review found
this; the parent traced and independently reproduced it, with no second review loop.

LibraryOnly probe: suspend a synthetic12-member old set, publish the complete new set, inject failure
only in Remove-UpgradeJournal after commit. Actual durable_phase=committed, complete_new_set=true;
calling the fresh Suspend entry used by new PrepareToInstall returns
`Previous owned-set journal requires recovery; refusing new transaction.` Explicit manual
Complete-UpgradeBrowserHost -Committed clears the journal. No real installer/process/service/Registry/
COM/Explorer was invoked. Evidence: parent scratch `installer-reentry-745c9c49bae546558e314c5a66d9ed77/result.json`,
runner `installer-reentry-probe.ps1`.

Connect bounded existing-journal recovery to the real installer start/preparation owner before a new
transaction. Respect validated phases and original commit decision: postcommit only revalidate new
set/resume cleanup, never roll back; precommit recovery uses existing compensation. Never delete or
ignore an unknown journal, or require manual journal deletion. Add a regression exercising two
installer-entry invocations, not merely calling Complete manually inside a library test.

## Source inspection / positive evidence

Read approved amendment/owner guidelines, package/build/publish/identity generators, explicit project
compile inputs, worker resolver/runner leases, signed manifest allowlist, protected installer file
approval/deployment, Desktop redirect and supplied tests/reference. Original owners remain in use:
actual Rust output → actualDesktopSHA → helper4runtimefiles → PlatformHost archiveapproval → signedset.
No hash cycle or competing signer/updater. New archive-member maps reject missing/null/empty/malformed
hashes; production worker uses fixed sibling resolution and retained approval before original Job
admission. Current default product Registry Apply remains disabled; packaging is not COM registration.

Old enrolled stable bootstrap compatibility remains explicitly unproved, not a new discovered defect
and not solved by staging. No authority was inferred to replace bootstrap, rotate trust or install.

## Independently executed checks

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\release-chain-independent-20261023-104715`.
Runner `release-chain-independent.ps1`, raw logs/commands.jsonl; stage copied into owned stage-copy,
original final stage left unchanged. No repeat Release build/staging/signing was necessary.

- Fresh no-incremental Tests/PlatformHost/helper build PASS0warnings/errors.
- First apphost invocation: LIST30, **RUN8/PASS7/FAIL1/NOT_RUN22/SKIP0**. Actual-stage test expected
  production ForPackage owner but tests.exe intentionally selects the explicit fixture-only branch.
  Preserve run30.log, do not call it product rejection or fullPASS.
- Corrected invocation `dotnet Tests.dll`: **LIST30=RUN30=PASS30**,skip/fail/not_run0; names/order equal.
  Same compiled tests, no source edit. Actual staged assembly metadata, original helper identity,
  approved worker/helper set and production ForPackage owner verified without launching their entrypoints.
- Installer LibraryOnly regression **632assertions PASS**. This validates direct functions, not
  a second Inno start (F-RC02 remains).
- Exact original Desktop protected-redirect test **1PASS**,1028filtered; production Desktop strict
  Clippy(full dependencies) and scoped main.rs rustfmt PASS. Shared Cargo lane checked, jobs1 /
  incremental0 / codegen-units256; no fullDesktop/codec/split/edit/100k/performance tests.
- archive-release-chain unsafe invocation rejects, package-memory-policy, selector, exact-file rollback,
  browser-signing-isolation, shell/spike, installer-update-lifecycle, GPUI/editor provenance, spec/task
  and diff gates PASS. These static gates did not detect F-RC01/F-RC02.
- Independent Python RSA-PSS/SHA256 signature verification PASS:77signedfiles/112860404bytes,
  79inventoryfiles including sidecars; exact final inventory hashes match. TESTsequence1/publickeySHA
  d44dad102642abc70ef2889f02275f6db101a75365bedb1ef7daa337d2776b38; not production trusted/installable.
  Final stage's own TEST private key absent. Helper runtime members/deps/runtimeconfig intact.
- `independent-stage-verification.json` records actual file hashes/counts and raw test-name equality.
  Local verifier initially stopped on case-sensitive Not/NOT label check, then UTF16/CRLF log parsing;
  only scratch parsing corrected; no test/build rerun or delivered file edit needed for those checks.

## Preservation / stop

679source/test/task baselinefiles unchanged before records. Sequence21 **78/78protected files** unchanged,
no extras; installerSHA8F371044ADA651DD01F86DBD5AA8A6D77BF15CDD49AAD3E5A4CCE1EF5C22A713.
ProductionGenerated2files unchanged. Existing production sequence ledger directly hash-checked,
sequence21 unchanged; private PEM was not read.54historicaloutcomes/acceptance hashes independently
compared with implementation opening baseline, unchanged. Task82implement/79check PASS with existing
context warnings(native38035/archive49080bytes); spec109documents/14owners.

Stop for two scoped repairs/regressions, not a broad redesign or additional review loop. Real install,
protected deployment, service/indexer release gate, Registry/COM/Shell/Explorer/physical/UI acceptance,
production signing/sequence issuance and user-installer generation NOT RUN/unauthorized. Existing
sequence21 user package remains available and unchanged. P208production drag remains disconnected;
earlierA–E/JOB07/SAFE08/native/user debts retained. No commit/push/PhaseF/firstrelease completion.
