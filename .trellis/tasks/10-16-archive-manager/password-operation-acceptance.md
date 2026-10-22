# B2 password operation — independent scoped acceptance

2026-10-20. **PASS for the delivered B2 password-only open/test portion.** No concrete in-scope
blocker found. [Implementation evidence](password-operation-evidence.md), [results](password-operation-results.json),
[source boundary](research/password-operation-sources.md). Conflict/apply-all, full B2, ENG03/ENG09,
Phase A — Engine contracts and safety foundation and A–F remain incomplete.

## Reviewed behavior

- Opt-in private schema2 open/test only, after original source acquisition; existing read_contract
  paths remain password-free. No public ArchiveRead DTO/version change or write/extract retry.
- Rust worker owns the small password-only loop using original InteractionPort. Original request
  retains operation/path/codepage/limits; fresh attempt IDs share original cancellation/lifetime.
  One worker/Job/source/helper/slot persists through waiting and all attempts; no respawn framework.
- Only PasswordRequired/explicit WrongPassword requests manual input, at most3actual attempts.
  AuthenticationOrDataError remains terminal. Installed ticket is notified before waiting;
  stale/duplicate responses are ignored by original port. Original cancel/EOF and typednullCancel
  terminate waiting; nextattempt checks original cancellation.
- Typed cancellation maps to outer cancelled on this opt-in path only. Original first failure/
  timeout/native ownership preserved, no new retry of ambiguousauthentication or output rollback.
- Answer travels original private stdin into SecretPassword/Zeroizing. Raw Rust frame buffers,
  temporaryrequestpassword and C#packed/private inputbuffers are zeroed; no argv/config/metadata/
  terminal/log password. C#managedString is not claimed locked/GC-zeroized memory.
- No extra defensive framework, queue, process owner, persistence, public pause/UI or new dependency.

## Independent checks actually executed

Waited for unrelatedDesktopCargo lane to finish, never terminated it. C#isolated apphost built
first; all Cargo commands inherited JOBS1/INCREMENTAL0/RUSTFLAGS=-C codegen-units=256, offline,
serial. Matchingisolatedworker rebuilt after lint checks before actual C#execution. No extra
largeinputs, performance observation or repeated green-seeking runs. No delegate was necessary.

| Check | Actual independent result |
| --- | --- |
| C#Debug testapphost build |exit0,0warnings/errors |
| interaction_contract/worker_wire/worker_cancel_contract |3+12+1=16pass,0fail/ignored,exit0 |
| password_notification_tests |1pass/13filtered,exit0 |
| Exact existing AES ambiguous classification regression |1pass/4filtered,exit0 |
| Rust selected total |18pass,0fail/ignored |
| Strict Clippy Archive isolated/default tests/examples |both exit0,-Dwarnings |
| Archive scoped fmt |exit0 |
| Global fmt --all --check |**exit1**, external Desktop currency_tests.rs80 parse error; NOT globalPASS |
| Matchingisolatedworker build |exit0 |
| C#same exact4fixture LIST/RUN/PASS |selected4/run4/pass4/skip0/fail0/not_run0,exits0 |
| Spec/context/fixtureselector/diff |all exit0 |
| Own result/identity/sourcehash checks |PASS;61baselinefiles unchanged during checks |

New actual C#fixture executes7small RAR5 scenarios: missing->correcttest, missing->correctopen,
explicitWrongPassword->manualcorrect with stale reply, typedcancel, originalcancel, attemptlimit,
correctpassword+damagedpayload ambiguity without prompt. Runtime asserts oneworker, uniqueattempt
IDs/originallogicalcorrelation, original source locked duringpending, helper occupied duringrun,
cleanuptrue/slots0, exclusive source+helper reopen afterterminal and0commits/nooutput. Decrypted
openheader is .gitignore/18bytes; actualtest verifies1file/18bytes. It is not merely a promptmock.
OriginalArchiveRead open/test privacy and strictwire plus codecpassword/budget/corruption/cancel
fixtures pass in SAME4collection. Exactnames compared to LIST and PASS, no environmentSKIP.

The globalfmt failure differs from implementation's historical main.rs283formatdiff: current
independent run saw `rust/apps/nxtools-desktop/src/main_window/currency_tests.rs:80` missingsemicolon
while unrelatedDesktopcurrency work was present. This is outside passwordchanges; no unrelated
file was edited/reformatted. Preserve both observations, do not infer fullworkspace compile/test
success from archivechecks or hide globalfailure. Scoped acceptance is unaffected.

Specs106documents/14owners,context82implement/79checkentries. Existingnative-contracts35497byte
contextwarning retained. No claim of independent103historicalfileopening comparison this turn;
current61reviewedsource/evidencebaseline verified and submittedhistoryclaims left unchanged.

## Evidence / stop

Acceptance scratch C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df:
`password-independent-verify.ps1`, commandledger, per-step logs, sourcebaseline and
`password-independent-assertions.json` retain actualcommands/exits/counts/exactnames/fmtfailure.
No implementation/tests/submittedreports/results edited by acceptance; this additive report only.
Closing0archiveworkers/testhosts,0archiveproduct/resourcepipes,fixedhelperabsent.

No100k,longpathmatrix,1GiB/largeZIP64,sampling,UI,formalDesktop/installedhost,systemconfiguration,
sourceRecycle/delete,packaging/install/sign/commit/push. NativeRecycle remains
native_protection_unproven/NotAttempted. **Password-only slice accepted; stop. B2conflicts/apply-all,
B3risk/publicUI and completePhaseA/A–F remain separate work. Globalfmt remains externally blocked.**
