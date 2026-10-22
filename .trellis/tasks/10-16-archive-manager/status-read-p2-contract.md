# Status-read P2 — bounded observation-tool correction

Canonical **Phase A — Engine contracts and safety foundation**, not complete. Scope: Python
control.py status reads, necessary driver_loop.py/run.py call sites and small deterministic tests.
No product protocol/budget/30s/ACK/cancel/codec/Job/source/helper/slot/writer change or new runtime.
No100k/1GiB/largeZIP64run, no broad review, package/install/commit/push/user/system operations.

## Authority and evidence boundary

[Independent acceptance](product-100k-acceptance.md) remains immutable: actual primary product
100kcorrectness PASS(100000exact/391batches/originalsuccess+confirmedcleanup), but its ONLY full
observation remains Failed/Stopped due status PermissionError during follow-up. Same-router reuse
was NOT completed/asserted. Exclusive-handle probe reproduces the reader failure, not the exact
contender/cause of that independent attempt. No new tool test can retroactively make it PASS.

## Verified read/write chain and errors

Python driver_loop.drive polls read_status each turn; Sampler.discover polls it on existing50ms
schedule (or main fallback when sampler thread is not alive). execute performs final status read
before verdict. All must use one reader/retry/error rule and bounded per-run diagnostics.
Rust ProbeStatus.update serializes <=16384bytes under its state mutex, writes/closes status.tmp,
then std::fs::rename to status.json; writefailure records raw_os_error and requests owned stop.
C#test Control.Publish serializes bounded status, FileStream(status.tmp,Create,Write,ShareNone),
flush/close then File.Move(overwrite). Its existing cap65536differs from reader16384; no widening
here. Original publication/stop/cleanup owners are not modified by this Python-only correction.

Actual tiny file probe: Python pathlib.open('rb') under Win32share0 retained handle gives
PermissionError/errno13 with winerrorNone; direct CreateFileW returns native32. ctypes.WinError
maps native5(AccessDenied),32(SharingViolation),33(LockViolation) ALL to PermissionError/errno13.
Thus neither PermissionError nor errno13 identifies a transient sharing race. Native2/3 map to
FileNotFoundError/errno2. Unknown/absent nativecode is not permission to retry.

## Minimal behavior contract

- Windows bounded reader uses CreateFileW GENERIC_READ/OPEN_EXISTING with SHARE_READ|WRITE|DELETE;
  preserves exact nativecode at failing open/read/close boundary, reads at mostLIMIT+1 and always
  closes its exact handle. It grants delete sharing, but does NOT promise overwrite while target
  handles are open: real MoveFileEx(overwrite) still returned5under that condition and succeeded
  after close. This is not authority to retry/ignore native5 or change writer failure policy.
- Retry ONLY native32/33, max4attempts per read,10ms between attempts (max3delays), identical main/
  sampler/final behavior. No background thread/retry task, no path probing to guess another code.
  Native5,unknowncode/errno13 alone, otherI/O and invalid state fail immediately. Exhaustion rethrows
  real failure and triggers existing ownedstop path; resource PASS predicate is not weakened.
- Native2missingfile is unavailable {}, not a previous state/terminal/cleanup proof; missingparent
  and othererrors fail. Wrongrun/schema/type/oversize/corruptJSON/identitylimit fail closed.
- Record bounded fixed diagnostics (caller/stage/category/exceptiontype/errno/nativecode, attempt
  counts/recovery/exhaustion totals), max16detailrows; no path/rawJSON/payload/errorprose copies,
  no per-poll/sample logging. Shared short stats lock never spans I/O/retry delay. Preserve errors
  even when later recovery succeeds. Finalledger retains these facts independently of verdict.
- No last-good return/cache. Main clears currentstatus before read. Final read must be fresh;
  failed/missing finalstatus cannot fallback to old observedCleaned or claim confirmedcleanup.
  Earlier observations remain raw evidence only, not current status/terminal authority.

## Vertical regression/verification

User-approved seam: actual read_status and drive with real tiny status file/Win32exclusivehandle.
First red uses unchanged reader: release competinghandle ONLY at retry scheduling; driver must
not requeststop after a recoveredread. Actual red2tests:1error+1failure/exit1 before implementation.
Then cover sampler samepolicy, heldhandle exhaustion/nativecode, genuineAccessDenied/no-code no
retry,32/33recovery, errorbounds, replacementsharing, lastgood invalidation and finalverdict cannot
PASS stalecleanup. Mocks replace only controlledfailure/scheduling/countervariables.
Run fullPython tool suite/AST, spec-contracts/taskcontext/selector/diff/whitespace/conflict/probes,
record exact counts/exits/historyhashes and update unique currentcheckpoint. No Rust/C#changes
planned, so no compiled/nativecodec tests or resourcesmoke needed. A future largeobservation/
sameconnectionreuse acceptance requires separate authorization, not this repair'sgreenunitresult.

## Final observed test scope

Python-only code change; no Rust/C#source or publisher change. Real exclusivehandle/native32
reader/driver recovery tests red2(1error1failure)/exit1 -> green2/exit0; full directed14/14 and
Python tool42/42pass/exit0. AccessDenied5tested with a real directory/file-open boundary, no ACL
mutation. Native33read error injection controls only I/O failure; errno13without nativecode fails.
One replacement-while-open test initially assumed delete-sharing implied MoveFileExsuccess;
small actual Win32probe disproved that. Regression now asserts compatible DELETEaccess while
readhandlelive, actual handleclose and successful publisher replacement/newread afterward.
No real collaboration smoke is needed beyond deterministic small native file boundaries; RUN0.
No archive100k/1GiB/largeZIP64 or compiled verification this correction. Historical independent
Failed/Stopped and unverified follow-up reuse remain unchanged, pending separate authorization.
