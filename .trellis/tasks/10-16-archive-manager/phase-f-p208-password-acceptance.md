# Phase F P208 password authorization — independent acceptance

Date: 2026-10-23. **PASS at the bounded production source / isolated-consumer level.**
No new confirmed actionable in-scope defect found. This is not real Explorer/foreign COM/physical
acceptance, full P208, Phase F or first-release completion. Current state belongs only to
[the checkpoint](phase-a-checkpoint.md). [Implementation outcomes](phase-f-p208-password-outcomes.md),
its failed1100/2 integration observation and prior AC01/tree reports remain immutable. See
[current production contract](phase-f-p208-production-contract.md) for finite capability limits.

## Independently inspected and accepted boundary

Parent traced official unlock action -> original masked Dialog/private reply -> original-connection
IPC authorization reservation -> EntrySession/runner/InteractionPort -> successful actual cleanup ->
opaque grant -> NEW pressed row gesture -> original STA/virtual contents/private worker injection.
One bounded read-only secret/native review found no confirmed defect; no second review/cleanup loop.
Speculative additional coverage suggestions were not treated as release blockers or new scope.

- Unlock first performs an extra whole-archive no-output TEST using the existing finite password
  loop. Cost is real: full archive read/decode under original limits, possibly refusing otherwise small
  selections. Metadata browsing does not automatically authorize drag. No selected content/cache
  lease is produced during preparation.
- Locked encrypted selection gets readable unlock-first feedback and no start. Explicit official
  Medium outlined Button/original Dialog is used. Prepare, password reply and grant completion retire
  old candidate and do NOT enqueue OLE; new Down/held threshold Move submits exactly once. Cross-entity
  dialog/result updates stay deferred. GetData has no password UI/interaction/retry.
- Known encrypted ZIP/SevenZ/Rar4/Rar5 use existing crypto support; real samples here are AES ZIP mixed
  with plaintext/tree and header-encrypted RAR5. Unknown encryption/unsupported kinds/path conflicts/
  unsafe selections reject wholly, not filtered. Same32roots/128descriptors/32files/256requests/
  256KiBcommand/512MiBactualtotal and directory/unknown-size semantics remain.
- C# per-pipe EntrySession stores at most8 short-lived grants,120s expiry,1s coalesced expiry timer;
  Rust local validity is conservative110s. Opaque ID binds exact path/browser/source snapshot/digest/
  size/mtime/session/generation/codepage/depth/MOTW; original connection is checked atomically by IPC.
  No grant is a cross-pipe/general credential. Private secret is not in drag/descriptor/result/argv/
  config/disk/cache or ordinary diagnostic payload.
- ProtectedSecret owns SAME_PROCESS CryptProtectMemory block16 backing arrays. Failed attempts,
  replacement, cancel/revoke/expiry/source-change/Stop/dispose erase storage. Temporary unprotected
  copies/private serialized frames are cleared; Rust request/SecretPassword drop clears owned bytes.
  No claim of perfect GUI/runtime transient-copy erasure.
- Only original Required/explicit WrongPassword retry, at most3attempts; ambiguous authentication/data
  remains terminal. Grant publication waits original worker exit/actual cleanup, not private complete.
- New contents revalidate grant; private bytes inject into original worker after ordinary request
  construction, not Run.Request/public DTO. Single-entry decrypt is noninteractive, bound to original
  source/output/Job/cache, actual remaining budgets and native streams. Revocation cancels owned work
  but cannot force-close already transferred target streams; actual cleanup still owns slot retirement.
- IPC reserves at most8 prepare identities/one exact revoke each, same receiver/fair wake/control
  boundary. Full ordinary queue cannot lose revoke; start precedes revoke, ACK is not prepare cleanup.
  Original-origin replies/revokes reject replacement connection and unbound authorization fails closed.

Representative source coordinates: browser_drag.rs354-503 (prepare/revoke/result), password.rs43-177
(original Dialog/private routing), ipc/archive_authorization.rs39-215 (locked origin/control/terminal),
ArchiveEntrySession.Password.cs17-143 (protected memory/binding/expiry/revoke), ArchiveEntrySession.cs
78-107,126-224 (prepare/private materialize/grant only after cleanup/Stop), original VirtualContent and
ArchiveVirtualDragSession pre-STA grant validation. Worker private browser_password shape is exclusive
with interaction/public password/read authority and is zeroed through the original request lifecycle.

## Independent checks actually executed

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\p208-password-independent-20261023-222412`.
Runner: parent session scratch `p208-password-independent-verify.ps1`. Per-command raw logs,
commands.jsonl, independent-results.json, full-meta.json and C# metadata retain actual results.

| Check | Independent actual result |
| --- | --- |
| Fresh isolated PlatformHost THEN Tests | PASS,0warnings/errors;01-host.log,02-tests.log |
| Formal browser collection |31PASS,0FAIL;03-browser.log |
| Formal application collection |14PASS,0FAIL;04-application.log |
| Protocol library |75PASS,0FAIL/ignored/filtered;05-protocol.log |
| IPC library |41PASS,0FAIL/ignored/filtered;06-ipc.log |
| Exact private worker shape target |1PASS,17filtered,0FAIL;07-worker-shape.log |
| Desktop/Protocol/IPC strict tests Clippy |PASS,exit0,-Dwarnings;08-strict.log |
| Archive lib/worker_wire isolated-codecs strict Clippy |PASS,exit0,-Dwarnings;09-archive-strict.log |
| Workspace --tests |Compilation PASS,exit0;10-workspace.log,not all workspace tests executed |
| Feature trees then matching worker build |PASS;11-13 logs; no own feature/dependency change |
| Initial exact controlled C# selection |LIST15=RUN15;12PASS/3SKIP/0FAIL/not_run0;14-15 logs; skips due to omitted explicit encrypted-fixture directories |
| ONLY skipped password cases after fixture setup |LIST3=RUN3=PASS3,skip0/fail0/not_run0;16-17 logs; names/order equal |
| Final matching worker before/after both sets |`3CDFF2061EAFD2A50E1C1841E294461D2D3EE75222AFDF1A794A585E67537AB4`,identical |
| ONE full Desktop |1102PASS,0FAIL/ignored/filtered,160.38s test time,exit0;18-desktop-full-once.log |
| Archive/password inside full run |96archivePASS,5formalpasswordPASS; browser31/application14 overlap,not disjoint extra runs |
| Full-run source drift |715 Rust/C# source/project files monitored,none changed during run |
| Scoped rustfmt |PASS,17 affected Rust entry/modules,skip_children=true; no global/vendor formatting |
| Source/doc/secret gates |PASS: PhaseF/secret/origin/reserved controls,Shell,GPUI catalog/release/editor provenance,spec110documents/14owners,task,diff |

Initial C# batch is explicitly NOT clean15/15: forgot two required fixture env directories in this
acceptance invocation. Harness correctly reported3SKIP, not failures or passes. Resolve original local
fixtures and run only those3; no rerun of12already passed cases. Original logs retained. LIST/RUN identity
matches in each batch, PASSES plus initial SKIP set reconciled in independent-results.json. All15distinct
selected names now have actual passing evidence using the same final worker, but not one15PASSbatch.

Initial12PASS = password wire/no-output admission + original11 tree/unknown-size/budget/STA/cleanup
compatibility fixtures (exact names in13/14/15 logs and independent-results.json). Supplemental3:
- archive Phase F virtual password actual ZIP lazy mixed retained stream
- archive Phase F virtual password actual RAR tickets revoke expiry source
- archive Phase F virtual password actual ZIP ambiguity cleanup revoke

These exercise AES mixed encrypted/plain/tree through original Dispatch/STA controlled loop, invalid
grant beforeSTA, descriptor zerooutputs, target-request-only real decrypt, independent held streams,
actual cleanup before grant, RAR5 wrong->correct/stale ticket, source/replacement identity, expiry/
protected-array erasure, ZIP ambiguous error and revoke while original cleanup is blocked. No foreign
Explorer target or actual DoDragDrop loop was executed.

Fixture directories: repository rust/crates/nxtools-archive/tests/fixtures and fixtures/upstream,
explicit NXTOOLS_ARCHIVE_ZIP_FIXTURES/CODEC_FIXTURES. Existing attributed samples copied to self-owned
scratch roots,not edited. AESZIP SHA2567884F120C15DE0F36E0676FE1A2EBA01E6045104EC7255D15CAD09140BE99959;
RAR5 SHA256DD47390A7FC73391183B38A4C140B595FDD52AB22AFBDFA4E41DB5023FC55F1B.

Parent read preserved implementation locked-row red (0PASS/1FAIL,missing unlock-first explanation);
no source reversion/new red claim. No product/test edit in this acceptance. Host-first -> checks/features
-> matchingworker -> controlled consumers preserved. Every Cargo checked/waited shared lane and used
jobs1/incremental0/RUSTFLAGS=-C codegen-units=256; no duplicate build or unrelated process termination.
No extra codec/password/conflict cross-matrix,100k/large/performance or historical Ole32 timeout rerun.

## Prior integration failure / preservation

Implementation's ONE1100PASS/2FAIL remains historical FAILED evidence. This independent ONE full run
is1102PASS/0FAIL. Both original failing names individually appear PASS in this full log:
- main_window::extensions_page::tests::w6_manager_pointer_preview_confirm_and_toggle_commit_real_config
- notifications::copy_tests::notification_copy_keyboard_uses_original_button_release_once
No causal repair or flakiness claim, no unrelated source fix, no green-hunting repeat.715monitored
source/project files unchanged during this run; this is not certification of unrelated vendor changes.

Opening3139baseline files before report/checkpoint writes:7concurrentchanges,missing0. They are two
Notification vendor files(notification.rs,NXTOOLS-PATCH.md),two shared spec documents and3Notification
task documents. Recorded hashes/diff in preservation evidence; none edited/reverted by archive
acceptance. Whole-workspace vendor therefore NOTunchanged, even though current provenance gate passes.
Archive implementation/tests/history, IPC/protocol/worker,installer/package/manifests/lock unchanged
by acceptance. **84/84 sequence25 delivery files unchanged**,setup SHA256
`1CF5FCCC2ACD62B96A6128018642A87C2D21DA07BED9BC206355E21C663D07E5`.
Only this additive acceptance report and sole current checkpoint are workspace edits; scratch evidence
separate. Task validation retains existing >32KiB context warnings; no unrelated doc restructuring.

## Stop / remaining native acceptance

P208 password workflow and finite delayed decrypt source/controlled slice accepted; no new confirmed
source blocker. AC01/tree closure retained. No additional cleanup/review cycle required. Real GPUI HWND/
capture/button continuity, physical gestures, real external OLE/foreign-process COM/Explorer receive/
cancel, DPI/UIA/user visuals remain NOT RUN/unaccepted. No GUI-denial bypass, Registry/COM registration,
service/bootstrap/source recycle/product startup/package/sign/install/commit/push/P209. Sequence25 does
NOT contain this source; payload duplication remains deferred to next explicitly authorized packaging.
Existing A-E/JOB07/SAFE08/native/user obligations remain. NOT fullP208/PhaseF/firstrelease PASS.
