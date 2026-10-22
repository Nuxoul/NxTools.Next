# Encrypted ZIP/7z/RAR extraction passwords — scoped evidence

2026-10-21. **Implemented; scoped checks PASS; independent acceptance pending.** Phase A — Engine
contracts and safety foundation remains in progress, NOT fullB2/PhaseA/A–F completion.
[Verified reference/source seam](research/extract-password-sources.md),
[previous open/test independent PASS](password-operation-acceptance.md),
[standalone stream conflict independent PASS](stream-conflict-acceptance.md).
Only [current checkpoint](phase-a-checkpoint.md) owns current status.

## Preparation boundary, classification and cost

| Format | Existing no-output preparation / error restriction | Successful extraction cost |
| --- | --- | --- |
|ZIP (supported ZipCrypto/AES)|Engine open only reads names/flags,not payload password. Existing TEST streams all records/authentication/CRC/size/limits; Required prompts; key/auth/data mismatch remains AuthenticationOrDataError,not guessed WrongPassword.|One complete read-only TEST plus one ordinary extraction payload decode. No third decode.|
|7z|Existing TEST parses encrypted headers and validates all encrypted blocks with no nativeoutput token. MaybeBadPassword/auth-data remains existing ambiguous terminal; Required prompts.|One complete TEST plus one original extraction decode,including original full-solid-integrity-before-publication.|
|RAR4/5|Existing RAR_TEST/null destinations uses original callback actualbytes/nativeintegrity. Explicit ERAR_BAD_PASSWORD24 alone is WrongPassword; encrypted BAD_DATA remains AuthenticationOrDataError.|One full TEST plus original extraction's metadata-only list and one payloaddecode. Metadata pass is not a third full decode.|

Same worker_password finite loop now runs read-only preparation for original extract. It constructs
WorkerRequest::open then sets test/path/codepage/limits/private password,not an extract clone with
write options. Original extract request/destination remains untouched,retaining supported options.
At most3preparation attempts (initial+2manual replies); failed attempts can consume a prefix or full
payload under original budgets/Job/timeout. Only PasswordRequired/explicitWrongPassword loops;
ambiguousauth/data/corruption/resource/native failures terminate without guessing text.

After success emit fixed extracting state and call original execute_attempt EXACTLY ONCE with the
ORIGINAL OperationControl/native ID accepted by runner. Every extraction result returns directly,
even PasswordRequired/WrongPassword/damage/conflict/cancel/commit uncertainty. No republishing,
rollback/deletion/merging or automatic return to password stage. Fresh prep attemptIDs share original
cancel/lifetime and use logical correlation/tickets; originalsource/Job/helper/slot remain held through
wait. Native output never uses an unaccepted preparation ID. Output/read/risk/budget contracts stay.

Private schema2 password_interaction admits extract; new password_extract_v1 negotiates that scope.
Open/test capability/behavior retained; ArchiveRead v1/v2 remains password-free/read-only; create and
nonZIP/7z/RAR extraction opt-in refuse. No selection interface,public command/UI/FIFO/framework.
C#only transports typed reply/recognizes fixedstate/capability; no C#password retry or outputtransaction
changes. Secrets remain private stdin and SecretPassword/Zeroizing; temporary requestpassword copies
zeroed after preparation/extract,not argv/settings/cache/log/metadata/terminal. C#managedString is not
claimed locked/GC-zeroized memory. Diagnostics reuse correlation/state/attempt/error/elapsed,not names.

## Real red → green and exact native scenarios

New `ArchiveExtractPasswordTests.ActualEncryptedExtraction` executed against original runner before
production edits: `encrypted extraction password admission` failed. Exact LIST1/RUN1/PASS0/SKIP0/
FAIL1/not_run0,exit1. C#isolatedapphost first0warnings/errors, matchingworker next. Original real seam,
not a mock: same fixture now green through original runner/Job/actualworker/native publication.

Nine small scenarios: ZIP,7z,RAR4,RAR5,explicitRAR5wrong_then_correct,waitcancel,ZIPconflict,
correctpassword+damagedRAR5,ZIPafter_commit_cancel.
- Each format missing->realpending->correctpassword->exactnative bytes/hash. Pending has no new
  output member/partial/write/publication,source/helper held and originalslot active.
- RAR5wrong->explicitWrongPassword->fresh ticket/attempt; oldticket delivered for originalRust
  rejection,then correctreply. Still zero output before preparation success,finite3attempts.
- TypedCancel during wait retains oldtarget/foreignpartial,zeroextraction/commits/writes and real
  exited/drain/cleanup/slot0/exclusive source+helper reopen.
- Original ZIP AutoRename survives password phase: oldpayload retained,new numbered payload exact.
  Codepage936,finite entries/entry/output quotas,MOTWfalse and original options retained.
- Correct password+documented RAR5damage yields AuthenticationOrDataError,no thirdprompt or actual
  extraction/invalidpublication. Existing ambiguousformat errors reused unchanged.
- Actual native ZIPfirstpublication triggers cancellation: originalcommitted1 retained and
  cancelled_with_committed_output,one extracting event,no repeatedpasswordstage/extraction.

All scenarios assert oneworker,uniqueprepIDs distinct from original,original business/native correlation,
0or1actual extracting,truthfulnativecounts/state/ownedcleanup and no secrets in frames/logs. No source
recycle/delete. Current runner eagerly prepares output root as before; zero-publication promise is
not a new global directory-rollback contract.

Independent inputs unchanged: existing independent-ae2-256.zip literal UTF8+bytes0..255×4 plus empty
member/directory (2regularfiles,1050payloadbytes); published encrypted.7z has2files2247+1417bytes with
retained independent7Zip24.09 SHA256 oracles. RAR4/5encrypted .gitignore literal `target\nCargo.lock\n`
18bytes. Original fixture producer/attribution/hashes in upstream README and prior ZIP evidence;
no new custom encoder,generator rerun,download,upgrade or allstrength/method crossmatrix.

## Executed final checks

All Cargo: jobs1/incremental0/RUSTFLAGS=-C codegen-units=256,offline,serial,machine lane checked.
C#isolatedapphost first, matchingisolatedworker rebuilt last after lint/fmt before nativeexecution.

| Check | Actual final result |
| --- | --- |
|dotnet build PlatformHost.Tests --no-restore|exit0,0warnings/errors|
|4smallRusttargets --features isolated-codecs --offline -- --test-threads=1|27PASS/0fail/ignored: interaction4,transaction5,worker_cancel1,worker_wire17|
|Exact existing aes_damage_framing_crc_budgets_and_operation_failures_never_publish_invalid_bytes|1PASS/4filtered,exit0;existing ambiguity/integrity assertions unchanged|
|StrictClippy isolated/default --tests --examples --offline -- -D warnings|both exit0|
|archive scoped fmt --check /matchingisolatedworker build|both exit0|
|Exact C#LIST→same-setRUN|selected5/run5/pass5/skip0/fail0/not_run0,exits0|
|Spec/context/fixtureselector/gitdiff--check|PASS; document/source gates,not UI/nativeRelease acceptance|

Exact C#catalog order:
1. archive password fixture actual wait retry cancel
2. archive password fixture actual encrypted extraction
3. archive product fixture actual open test privacy
4. archive overwrite fixture actual worker integrity and commit lifecycle
5. archive codec actual Rust password budget corruption cancel fixture

Existing representatives retain read-only open/test password finiteattempt/cancel/secret behavior,
productreadprivacy,ambiguouscodec errors and pre/in/postcommit outcome truth. No100k/longpath/largefile/
performance or fullconflict/risk/readmissionmatrix. Explicit worker/ZIP/codecfixtures/Python configured
before LIST/RUN,noSKIP treatedPASS. No globalworkspace build/test/format claim.

## Preservation and stop

Scratch `C:\Users\Administrator\.pi-desktop\scratch\949da288-1ef9-46b3-a698-90f543613cf5` retains
extract-password-opening-hashes/status, red/green/final logs, extract-password-verify.ps1,
command-results/native-results and closingassertions/cleanup.132historical taskfiles match opening
hashes; priorcheckpoint byte-identical snapshot retained. Only narrow coordination/admission/wire/
diagnostic/test/spec/navigation/currentcheckpoint changes; originalcodec/conflict/publication and
historical reports/results/acceptances/ledger/Failed/Stopped facts unchanged. Existingnative-contracts
35497byte context warning retained. No broad reviewer/independent acceptance claim.

No newpasswordmanager/cache/guessing/persistence/infinite retry,defensiveframework/runtime/fallback,
risk/budgetapproval/UI/FIFO/PhaseB/UNC/sourceRecycle/delete/system/dependencychange/formalapp,
package/install/sign/commit/push. Stop for independent scoped acceptance; do not append tests/tools
or start another feature.
