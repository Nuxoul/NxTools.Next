# Installed trusted-update peer failure — 2026-10-15

## 1. Scope / Trigger
User reported Setup Runtime error(at15:956), trusted update incomplete/no new launch on sequence4. Diagnose/fix existing native trust client and regenerate same-key package; no real deployment/service/ACL/registry/shortcut mutation or UAC fallback.

## 2. Signatures / reference
Installer PrepareTrustedUpdate -> PlatformHost --prepare-trusted-update -> RequestAsync validates enrollment/package then named-pipe peer before sending request. New `TrustedUpdatePeer.Verify(SafePipeHandle,Func<ServiceIdentity>?)`, `ServiceIdentity{ProcessId,State,Type}`; production readSCM uses SC_MANAGER_CONNECT1/SERVICE_QUERY_STATUS4/QueryServiceStatusEx, closes SafeHandles. RequestAsync adds optional IPlatformLog; Program and Ensure caller forward original log. Read prior Listary/SwiftList/WinShun reference note first; none establishes equivalent authenticated native updater implementation, no reference binary executed.

## 3. Contracts / actual evidence
Protected registry ImagePath/LocalSystem/bootstrapHash and product-child ACL/reparse validation remain prerequisite. Connected endpoint GetNamedPipeServerProcessId must equal SCM queried own-process(Type0x10),Running4,nonzero PID for fixed NxToolsUpdateV1. Never open SYSTEM process or relax ACL. Package signature/locked files/anti-replay/postdeployment hashes unchanged, wrong peer/state/type/missing service fail closed before write. Clients log fixed stages/correlation/nativeCode/elapsed, not path/frame/errorMessage; failure no auto-launch/UAC bypass.

Read-only original command scratch trusted-readonly-diagnose: IsEnrolled and ValidateEnrollmentPASS; pipe connected/PIDreadPASS; OpenProcess0x1000failed native5, image could not read, exit1. No deployment request sent. This is a deterministic real failure at the actual pre-request identity boundary, not proof of a later deployment failure. Installed sequence4 had BrowserHost restored to old bytes after failed finalization; that mismatch is rollback aftermath, not original signature diagnosis. SYSTEM log denied; no elevation to read it.

## 4. Validation / Error Matrix
Valid approved Running ownservicePID=pipePID -> proceed; OpenProcess access unavailable -> no longer dependency; wrongPID/zeroPID/stopped/sharedservice/invalidpipe/SCM query failure -> reject before update send; wrong registration/bootstrap hash/package/signature -> original reject; incomplete deployment -> original rollback/no new launch. Old source omitted client failure diagnostics; new fixed stages distinguish actual boundary while observing result only.

## 5. Good / Base / Bad
Good: ordinary medium-user reads authoritativeSCM PID and connects authenticated approved service without openingSYSTEMprocess. Base: protected bootstrap/key remain existing sequence2 identity, newer signed package upgrades replaceabletuple only. Bad: run installer/UI admin to sidesteppeercheck, trustpipe name alone, remove signature, alter service ACL, infer BrowserHost rollback mismatch is original cause.

## 6. Tests / verification
New self-owned actual namedpipe fixture exercises production verifier plus injectedSCM; old extracted image check failed Updater pipepeer mismatch, then green; wrongPID/state/type/zero/closedpipe reject. Registered reviewed fixture only, no live service mutation. Debug/ReleaseC#build0warnings/errors,245/245safe full fixtures. Actual original system service rechecked using repaired Debug and Release method: IsEnrolled/ValidateEnrollment/productionpipepeerPASS, deployment_request_sent=false. No full RequestAsync/update transaction sent, no install acceptance claimed. Installer ten safe callback/BrowserHost rollback scenarios allpassed.

## 7. Wrong vs Correct / delivery
Wrong: pipePID -> OpenProcessSYSTEM -> AccessDenied5 -> misleading generic signature/maintenance error. Correct: protected registration/bootstrapverify -> pipePID == SCM fixedRunning ownservicePID -> signed bounded request. Client failures emit fixed phase/native error without paths; no extra privilege.

Sequence5 repair installer rebuilt same persistent key:18,059,469bytes,2026-10-15T09:15:20.6647811+08:00,SHA256AA48A025703154DBF070D5A105CAFA19A98D1DDFD5F4B6ADF6A893C84823A53C,IDe174b017-5ca7-4c63-ab9f-081cce2a094e. IndependentRSA-PSS/all38hash-size/embeddedkey/3approvalsPASS;5binaries/buildoutputs/30browserfiles/3licenses/no privatekeyPASS. FourRustoutputsunchanged; PlatformHostchanged. AuthenticodeNotSigned; existingRustunusedmethodwarning retained. Commitheadroom30.55GiB>=6,serializedrequiredCargoenv. Sequence4backup in scratch, not reused. Current services Running samePID5432/5540; no agent install/deploy. Full user reinstallation/tuple/newerupdate acceptance remains pending.

Logs: trusted-install-peer-diagnose.log, trusted-peer-red.log, trusted-peer-green.log, trusted-peer-original-green.log, trusted-peer-release-live-readonly.log, trusted-peer-safe-full.log, trusted-peer-release-safe.log, release-seq5-peerfix-package/status/verified/binaries under session scratch. No temporary DEBUG instrumentation or new package authority bypass. Current artifacts/installer evidence supersedesseq4.
