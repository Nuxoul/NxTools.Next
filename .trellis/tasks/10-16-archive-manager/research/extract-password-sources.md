# Encrypted extraction password preparation — verified seam

2026-10-21. Phase A only. Read original password-operation sources/independent acceptance and
stream/format conflict independent PASS before changes. Indexed SupaZip nested GUI lib.rs151–169,
main.rs475–504 was rechecked: preserves Extract intent but respawns extraction/default destination;
Test maps to Open. Do NOT copy respawn/intent loss or unguarded retry/truncation. Legacy tracked
ZipArchive/SevenZip/SharpCompress search still only finds unrelated package ZIP. No native archive
UI/layout/focus/show/hide boundary in this slice.

Original worker acquires source ONCE under C#runner/Job before worker_password::execute. Existing
loop uses new OperationControl::attempt sharing cancellation, original request.operation_id for
business correlation and InteractionPort ticket; only Required/explicit WrongPassword retry,max3.
Native PrivateOutput uses control ID,so true extraction MUST use original control/runner ID,not
password attempt ID. C#owns original source/helper/Job/process/timeout/cleanup-bound slot. Secrets
use SecretPassword/Zeroizing,temporary request copy and private wire zeroing,not argv/settings/log.

Each encrypted format has existing no-output TEST path suitable for preparation:
- ZIP engine.open reads names/encryption flags only,not payload key. engine.execute(Test) streams
  all original records through zip_crypto/zip_aes authentication/CRC/size/budget. Missing password
  is PasswordRequired; ZipCrypto/AES verification mismatch is AuthenticationOrDataError,not reliable
  WrongPassword. No string guesses. Full test required to guarantee zero-publication retries.
- 7z run_sevenz(test) parses encrypted headers or checks encrypted blocks then fully decodes with
  token output absent. Wrong/auth/data ambiguity stays existing sevenz_error,not a retry guess.
- RAR4/5 rar::execute(test,None) uses RAR_TEST/null destinations/full callback byte/integrity checks.
  Header password validation may explicitly return ERAR_BAD_PASSWORD24 -> WrongPassword; encrypted
  BAD_DATA -> AuthenticationOrDataError. Existing RAR5header fixture supplies explicit correction.

Narrow implementation: for original extract only, construct existing WorkerRequest::open as
preparation TEST (same path/codepage/limits,private password,no conflict/MOTW/delete/readmission/
write options). Run same finite loop/typed port. On successful preparation call original
execute_attempt EXACTLY ONCE with original request+control/options and retained secret; every
extract result is terminal,never remapped to password loop. Fresh prep attempt IDs emitted on
original logical correlation; one extraction-state event uses original native control ID.
Success cost: one full no-output test + one actual extraction payload decode; RAR extraction's
existing metadata-only first pass remains. No extra third full decode or worker/Job/source reopen.
Readmission/risk are not expanded; no selected-extraction API or ArchiveRead v1/v2 authority.

Tests at user-authorized real seam: original C#runner/Job -> actual matching Rust worker -> native
publication with existing independent AES ZIP/7z/RAR4/5 fixtures/hashes. Pending zero writes/partial/
publish,correct answers/exact bytes,explicitRAR5correction+stale ticket,waitCancel,post-password
conflict policy,correctpassword+damagedpayloadterminal and cancellation after first real ZIP commit.
No real source recycle/delete,UI,codec framework or dependency changes.

Preserved prior source history: [open/test password seam](password-operation-sources.md).
Fresh preparation is read-only by request construction,not a renamed extract clone. Original
request stays operation=extract and retains conflict/MOTW/limits/path/codepage/private options;
temporary password copies are explicitly zeroed after both preparation and terminal extraction.
Only extract negotiates password_extract_v1. The existing password_state gains fixed extracting
state,recognized by C#safe diagnostics; no secret or extra output permission in public protocol.
Failed preparation can decode a prefix or all data before typed failure,still under unchanged
per-attempt/output/work and original Job/time ceiling;max3attempts. No aggregate-work waiver,
extra third full decode or continuation after actual extraction was introduced.
