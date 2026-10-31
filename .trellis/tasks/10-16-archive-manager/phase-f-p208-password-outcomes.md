# Phase F P208 — password authorization / virtual drag outcomes

Date2026-10-23. **P208 password interaction source hookup pending independent acceptance.** Same
canonical **Phase F — Explorer integration and genuine native drag**, not optional P209, a new phase,
spike or fullP208/PhaseF/first-release PASS. Formal user workflow and delayed decrypt chain are wired.
Affected checks pass, but the ONE full Desktop run is **1100PASS/2FAIL**, with unrelated Extensions /
Notification failures retained below. Do NOT describe the entire integration as green.

Authority and chosen flow were recorded BEFORE code in [existing plan](implement.md#approved-p208-password-authorization--fresh-gesture-amendment--2026-10-23)
and [sole checkpoint](phase-a-checkpoint.md). Current [production contract](phase-f-p208-production-contract.md)
records this extension. [AC01 closure](phase-f-p208-production-repair-acceptance.md) and
[finite tree acceptance](phase-f-p208-tree-acceptance.md) remain CLOSED/accepted, unchanged; no cleanup
or new review of those slices. Historical reports/failure logs remain immutable.

## Formal interaction: unlock first, then a NEW held gesture

1. A locked encrypted selection does not submit start/OLE. Original official Alert explains to click
   **解锁拖出**, enter password, then press/drag again. No silent filtering of encrypted descendants.
2. Original ArchiveBrowser Toolbar.content adds one actual official Medium32DIP outlined Button via
   existing action API; original Dialog/masked Input and original password ticket/reply are reused.
   No competing window/selection/focus owner. Same Kit0.8.0/defaultpre0.3.9; no theme/vendor edits by task.
3. Button retires old drag candidate and any previous grant, registers original-connection prepare,
   then original EntrySession/runner executes a no-output whole-archive **TEST**. UI says this test can
   read/decode the whole archive. No selected output, cache lease, disk password or automatic drag.
4. Original Rust worker_password / InteractionPort owns at most3attempts. Only PasswordRequired and
   explicit WrongPassword can retry; ZIP/7z authentication/data ambiguity remains terminal, not a guess.
   Wrong/cancel/stale-ticket/revoke cannot restart extraction or create an authorization after retirement.
5. Only successful actual worker cleanup publishes the nonsecret opaque grant ID. UI displays short-term
   unlock and asks the user to drag again. Input completion, prepare completion and deferred result
   never enqueue start; no async reply counts as proof the pointer is held.
6. A NEW formal row Down -> held>=4DIP Move freezes full plain/encrypted selection/tree and immediately
   submits ONCE from that still-pressed callback, with actual Window HWND and original read connection.
   All public content entries still have password_interaction=false. No GetData modal/password wait.
7. Descriptor request does no work. Target FileContents(index) resolves the grant inside ORIGINAL
   EntrySession and passes private bytes to ORIGINAL worker/Job/source/output/cache/native IStream.
   Original oneSTA/oneOLE/capture/button/finalcancel/remaining budget/independent stream owners stay.

Same finite32roots/128actualdescriptors/32files/256requests/256KiBcommand/512MiBactual output contract.
Directory descriptor-only and unknown-size semantics are unchanged. Original BrowserModel remains the
sole selection/focus owner; complete immutable subtree expansion does not discard encrypted children.

### Costs and actual scope

Unlock performs ONE additional FULL no-output archive TEST; it can decode all payloads, not just the
selected entries. Failed attempts may decode a prefix/all data before typed failure. Original30sworker,
finite512MiBtest/output/work/metadata/Job limits apply; legal selections can therefore be refused when
whole-archive authorization verification exceeds these bounds. After unlock, each requested selected
file still incurs its normal original materialization/decrypt; it is NOT pre-extracted. No hidden
performance claim or format/password/conflict matrix was added.

Known encrypted **ZIP / SevenZ / Rar4 / Rar5** reuse existing format/crypto support. This round executes
representative AES ZIP mixed with plain files/directories, plus RAR5header-encrypted explicit wrong ->
correct; it does NOT claim every encryption-method cross-product was newly executed. 7z/RAR4 use the
same accepted original password/codec route, with existing capability failures remaining explicit.
Unknown encryption status, unsupported encrypted format/kind, links/unsafe paths/conflicts/limits and
locked encrypted selections reject WHOLE. No password manager, credential store or source recycle.

## Secret ownership, identity and revocation

| Boundary | Actual owner / lifetime |
| --- | --- |
| User editing / Rust private reply | Existing masked Input is cleared on confirm/cancel; PrivatePassword has redacted Debug and Drop clearing. Original bounded authenticated pipe transports private byte reply, not a drag/descriptor secret. GUI/runtime transient copies are not claimed perfectly erasable. |
| Password attempts | Original InteractionPort / worker_password finite loop. SecretPassword uses Zeroizing; temporary request password copies cleared after each attempt. Metadata-read/entry-open success does NOT silently unlock future drag. |
| Pending authorization | Original per-pipe EntrySession captures ONLY last admitted supplied password. Windows CryptProtectMemory SAME_PROCESS/block16 protects byte storage at rest; wrong attempt replacement, failure/cancel/revoke/Stop clear actual backing arrays. No retained ordinary String or derived secret record Debug. |
| Ready grant | <=8 per ORIGINAL pipe-session EntrySession,120s expiry;1s coalesced expiry Timer erases protected memory even without GetData. Rust view uses conservative110s local validity and never treats current reconnected bridge as origin. |
| Identity | Grant ID equals random authorize operationID; bound to session/generation,exact path,codepage,depth,MOTW and full browser snapshot/format/comment/native source/digest/size/mtime. New pipe owns no old map. Same original IPC router checks read-origin connection under lock. |
| GetData use | EntrySession decrypts a temporary byte copy, injects private inherited worker browser_password AFTER ordinary request construction; never retained in Run.Request/public DTO. Private encoded buffers and temporary copies zeroed after delivery. Rust request Drop clears original bytes/string. No InteractionPort/password prompt enabled for this single materialization. |
| Revocation | Original view invalidate on refresh/projection/source invalid/retire/close -> original bound revoke. Typed password Cancel retires local prepare before late result. Source revision failure removes grant. Expiry/revoke/Stop/dispose erase stored secret and cancel related workers/content; active worker slots wait actual cleanup. |
| Target-held media | Existing native IStreams already transferred to the target remain target-owned/readable after grant revoke or drag retirement. New GetData refuses; no forced close or early lease/worker cleanup claim. |

Original Entry command adds strict authorize/revoke without new public secret fields. Authorize has
source context/path but no ordinal/output/lease, passwordinteractiontrue; revoke is control-only with
lease_id==operation_id. Drag adds optional authorization_id, omitted on cancel/plain legacy commands.
Neither descriptor nor public drag contains a password, argv/config/cache secret or long-lived credential.

Original router/receiver has at most8authorization reservations, one coalesced exact revoke each. Start
must dequeue first; ordinary queue pressure cannot discard revoke. Uses SAME original receiver/wake/
fair turns, no new transport/retry tasks. Prepare/reply/revoke reject different connection origin; only
these exact held receipts avoid ordinary10s sweep while original worker cleanup owns retirement.
Revoke ACK is not preparation cleanup. Generic unbound authorize/revoke fail closed. No synchronous
cross-entity view writeback; application/dialog/results stay on deferred turns, including immediate paths.
Diagnostics are fixed correlation/state/count/nativecode/elapsed, no password/path/content/pointerticks.

## Reference / red -> green / bounded review

Read AGENTS, current PRD/design/plan/checkpoint, production/tree/AC01 accepted contracts, PhaseC
closeout/password acceptance, extract-password acceptance and indexed password/extract sources before
implementation. Recorded actual chain in `docs/reference-code/archive-virtual-drag.md`: original masked
Dialog -> private reply -> ArchiveBrowserPassword -> EntrySession/runner -> original InteractionPort;
source acquisition/binding before TEST, private bytes only for later original content. Historical SupaZip
respawn/Test->Open behavior is NOT copied. Existing browser/native layout/STA/capture boundary stays.

Unique evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-f-p208-password-20261023-202345`.
All command attempts/raw logs retained, no overwritten failed log. Cargo checks/waits for shared lane,
jobs1/incremental0/RUSTFLAGS codegen-units256; no unrelated owner killed/duplicate build.

- **01-locked-row-red.log** actual official row:RUN1/PASS0/FAIL1,exit101, specifically missing readable
  unlock-first feedback; no start. SAME `production_password_drag_locked_row_has_unlock_feedback_and_no_start`
  green in04/10 and full run. Other row tests prove success never starts, oldheldcandidate retired,
  fresh gesture once, mixed selection complete, source/refresh/close/disconnect/reconnect late rejection.
- **csharp-password-03-red-run.log** original Entry parser/admission rejects new authorize JSON:
  LIST1/RUN1/PASS0/FAIL1,exit1. After implementation exact wire fixture PASS1 (16-wire-green +39firstpass).
- **06-password-rows.log** retained2PASS/1FAIL fixture issue: MouseUp runs approved ordinary Click and
  collapses multiselection. Test restores full selection via original model BEFORE new gesture; product
  selection behavior unchanged. Same3named tests10PASS3; final full run contains all3PASS.
- Application tests click actual official unlock Button and original Dialog Confirm; password reply and
  deferred grant cause no start. Separate typed Cancel test proves local preparation retired and late
  success/secret cannot restore authority. These final2cases PASS in60 and full run.
- C#39first native set: selected4/run2/pass1/fail1/not_run2,exit1. AES contents/revoke/heldstream succeeded,
  then fixture read an exclusively held cache manifest while EntrySession still alive. Only move disk
  privacy assertion AFTER original owner releases the lock; production lock/cleanup untouched.42targeted
  selected3/run3/pass3/skip0/fail0/not_run0; old failed4 remains FAIL, not rewritten as a clean batch.
- Enhanced AES test later enters the SAME original ArchiveVirtualDragSession.Dispatch/STA with injected
  controlled loop: bad opaqueID rejected beforeSTA; validID enters one loop; descriptorzerooutput, target
  index decrypt, repeats, mixedplainfile, revoke/backingarrayzero/newcontentrefusal/heldstream allPASS in66.
- ONE optional bounded read-only review found no confirmed new defect; not an independent acceptance
  or native/user verification. No second review/cleanup cycle.

Retained local build/fixture failures: format02 caught a bracket error before Cargo; fixed locally.
14 compile lacked InteractiveElement/WindowExt trait imports; fixed.23private-shape missing original
BrowserEntry metadata caps (and explicit extract options supplied),26PASS.27strict found localcollapsible
if plus concurrentunusedimport; local equivalent refactor only,61strict laterPASS.46newcanceltest used
Entity.read withTestAppContext, correctedread_with.52compile failed concurrently changing notification
copy.rs lengths mutability; that task fixed it, not archive. No broad format/password/conflict/100k/
large/performance matrix; historical native Ole32 pre-cancel timeout fixture NOT selected.

## Actual checks / names / counts

Commands with full arguments/exits/logs: `commands.jsonl`; isolated C# command ledger + prefixed logs.

| Check | Actual result |
| --- | --- |
| Formal browser collection /21 | **31PASS**,0FAIL; original plain/tree/unknown/budget/cancel tests plus3password rows |
| Original application collection /22 | **13PASS**,0FAIL before final Cancel case; final full run contains14PASS |
| Final password application filter /60 | **2PASS**,0FAIL |
| Final Protocol library /57 | **75PASS**,0FAIL/ignored/filtered |
| Final IPC library /58 | **41PASS**,0FAIL/ignored/filtered; originalAC01/congestion/cleanup plus authorization reservation |
| Private worker shape /26 | **1PASS**,16filtered; no new ArchiveRead secret/output/prompt authority |
| Strict Archive lib/worker_wire isolated-codecs /33 | PASS,exit0,-Dwarnings |
| Strict Desktop/IPC/Protocol tests /61 | PASS,exit0,-Dwarnings; earlier concurrent Notification compile/lint failures remain in27/32 |
| Workspace --tests /62 | Compilation PASS,exit0; not all workspace test execution |
| Feature trees /35,/36 | Desktop has no archive isolated-codecs; matching worker original features/dependencies |
| Isolated C# host-first then Tests /29,/30 | PASS,0warnings/errors; later test-only fixture/controlled-loop rebuild40/63PASS |
| Actual password fixtures | Four distinct names pass across wire1 +repaired3;66 repeats ONLY enhancedAEScase. Not one clean LIST4=RUN4 batch; original39 remains failed evidence. |
| Original C# compatibility /43,/44 | **LIST11=RUN11=PASS11**,names/order equal,skip/fail/not_run0 |
| Final controlled-loop AES /65,/66 | **LIST1=RUN1=PASS1**,names/order equal,skip/fail/not_run0 |
| Scope format /68 | PASS,affected Rust only,skip_children=true; no global format |
| Ownership/secret/spec/provenance gates | PASS; source/docs only, NOT actual HWND/Explorer/user acceptance |
| ONE full Desktop /67 | **1100PASS/2FAIL**,0ignored/filtered,203.06s,exit101; not a globalPASS |

Password exact distinct names:
1. `archive Phase F virtual password wire no output admission`
2. `archive Phase F virtual password actual ZIP lazy mixed retained stream`
3. `archive Phase F virtual password actual RAR tickets revoke expiry source`
4. `archive Phase F virtual password actual ZIP ambiguity cleanup revoke`

Same filters LIST before each RUN. `native-final-identities.json` checks names/order for repaired3,
compatibility11 and enhancedAES1; first wire PASS belongs to listed failed initial4 and is not a new
standalone invocation. Original compatibility11 names retained in44raw log and identity JSON.

Host-first -> Rust checks/features -> matchingisolatedworker37. Password3 andcompatibility11 each
observe worker SHA `1A47F3BCBF2CADC795BFE885D35BFD4D1966DB16866AC43301F16A61FC6BDDE1` unchanged before/after.
After finalProtocol/strict/workspace convergence worker rebuilt64; enhancedAES1 observes final SHA
`3CDFF2061EAFD2A50E1C1841E294461D2D3EE75222AFDF1A794A585E67537AB4` unchanged before/after. Do not
attribute all earlier cases to the later binary or claim a new whole set with that final hash.

Representative real-native evidence: tiny AESZIP +addedplainmember/tree TEST zerooutputs, actual
cleanup barrier beforegrant, independent literalAESpayload, directorydescriptorzerooutputs/indexed
lazydecrypt/repeatcursor/targetheldstream; RAR5explicitwrong->correct,oldticket/oldrevoke rejection,
allidentityfields,expirywithoutGetData/backingarrayzero,replacementpipecannotreusetoken,actualchanged
native sourcerevision revokes; ZIPwrongauthenticationambiguityterminal,preparationcleanuprevoke race,
materializationrevoke while cleanupblocked,originalslots retained and no late medium. No perfectGUI/
managedruntime memoryerasure or foreignExplorer/COM acceptance claim.

### Full Desktop integration failure retained

ONE full run:715source/project hashes watched; ONLY concurrent `notifications/copy_tests.rs` changed
while running. All96archive testsPASS,includingbrowser31/application14 and5newpassword names; these
collections overlap and are NOT extra disjoint executions. No archive source changed during run.
Two failures outside archive:
- `main_window::extensions_page::tests::w6_manager_pointer_preview_confirm_and_toggle_commit_real_config`:
  extensions_page.rs445 indexoutofbounds(len0,index0).
- `notifications::copy_tests::notification_copy_keyboard_uses_original_button_release_once`:
  copy_tests.rs339 emptycommandqueue.
Neither root cause was diagnosed/fixed here; no single-case rerun or full-suite repeat, no green-hunting.
This remains a genuine failed whole-integration observation plus source drift, not frozen-workspacePASS.

## Preservation, remaining scope and stop

Opening3042tracked+preexistinguntrackedbaseline,not cumulativeHEADdiff. Finalpreservation JSON separates
archiveowned33existingchanges from concurrent25 (numbers refreshed at closure), missing0. Newsource
files: archive_authorization.rs,ArchiveEntrySession.Password.cs,ArchiveVirtualPasswordTests.cs;this
additiveoutcomes report. Unrelated Notifications/assistant/Extensions files never edited or reverted.
**84/84sequence25artifacts unchanged**,noadditions;setupSHA
`1CF5FCCC2ACD62B96A6128018642A87C2D21DA07BED9BC206355E21C663D07E5`.
**66/66historicalarchiveoutcomes/acceptance unchanged**,includingAC01/tree closure and failed evidence.
Manifests/Cargo.lock/installer/scripts/productionGenerated unchanged by this task; no ownvendor/codec/
dependencyversion change. Whole-workspacevendor is NOT unchanged: concurrent Notification task changed
`vendor/gpui-component-080/src/notification.rs` and `NXTOOLS-PATCH.md` during this round. Initial audit
correctly failed on drift; preserved `preservation-initial-vendor-drift.json` and exact observedhashes.
These are separately recorded concurrent deltas, not archive authorization or permission to revert.
Final current release/provenance gates executedPASS, not native/visual acceptance of that other task.

Sequence25doesNOTcontain this source. Installerduplicatepayload deferred to separately authorized
packaging. ProductApplyAuthorized=false; no actual productlaunch/GUIbypass/Explorer/physicalOLE/
Registry/COMregistration/service/bootstrap/recycle/package/sign/install/commit/push/P209or otherfeature.
Actual HWND/capture/button continuity,physicalgesture,externalOLE/foreignCOM/Explorer,DPI/UIA/user
visual acceptance remain NOTRUN/unaccepted. ExistingA–E/JOB07/SAFE08/native/userdebts remain.

**Stop: P208 password interaction source hookup pending independent acceptance.** Formalworkflow,
protectedshorttermgrant anddelayeddecrypt delivered; affectedchecksPASS,wholeDesktop1100/2failure
explicitlyremains. Not independentclosure/fullP208/PhaseF/firstreleasePASS; no further work auto-started.

## Final closure audit

Final baseline3042 =2984unchanged +33ownedchanged +25concurrentchanged,missing0;4ownednew
(three implementation/test files plus this report),9concurrentnew. Artifacts84/history66 unchanged;
two observed concurrent Notification vendor deltas remain explicitly NOTunchanged. No archive edit
to those files or manifest/lock/dependency/installer/scripts/Generated. Task validation82implement/
79check entries and spec110documents/14owners passed with existing32KiBcontext-size warnings.
Final source/secret ownership,scopedRustformat and owned diff checks exit0; Program.cs retains its
existing CRLF-to-LF Git warning. No build/test/full-suite/review repeated after the one1100/2run.
Stop remains P208password sourcependingindependentacceptance,with globalintegrationfailure retained.
