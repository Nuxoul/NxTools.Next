# Phase F P208 production ArchiveBrowser row contract

Authority2026-10-23: existing implement.md/checkpoint recorded before code. This contract supersedes
historical production-disconnected wording only for the bounded formal source hookup. Historical
spike outcomes/acceptance/failed Ole32 evidence remain immutable. No native/live acceptance implied.

## 1. Scope / Trigger
User holds a formal ArchiveBrowser ordinary row and moves >=4 logical DIP on either axis. Down alone
is candidate only; MouseUp/doubleClick/normal selection never means drag. One BrowserModel owns
selection/focus; original official ListItem32DIP/uniform_list and all toolbar/layout/variants remain.

## 2. Signatures
`PlatformCommand::ArchiveVirtualDrag(Box<ArchiveVirtualDragCommand>)` serializes `archive_virtual_drag`;
C# formal capability `archive_virtual_drag_v1`. Original `ArchiveVirtualDragSpike` wire/capability stay
compatible and dispatch to SAME `ArchiveVirtualDragSession`, not a second native owner.
Start: version1/operationID/sessionID/generation/positive actual sourceHWND/items. Cancel: same operation/
session/generation, hwnd0/itemsNone. Separate request receipt IDs under operation correlation.
Directory/unknownsize amendment2026-10-23: item.size is nullable (missing/null=unknown,NOTzero),
item.directory defaultsfalse/omittedforoldfiles. Directory entry retains source/context; implicitdirectory
ordinalNone is descriptor-only and NEVER dispatched to materialize; files require originalordinal.
Password amendment2026-10-23: optional authorization_id is a source-bound opaque ID, never a secret.
Cancel must omit it; each public content entry keeps password_interaction=false. Original Entry command
adds authorize(noordinal/nooutput/interactiontrue) and revoke(lease_id==operation_id,control-only).

## 3. Contracts
Completed original read hydration installs immutable browser/source context into original view, deferred
through ArchiveApplication. At threshold, current view freezes actual complete selection + original
ordinal/source/snapshot/generation/codepage/depth/MOTW and immediately enqueues from still-pressed move
callback. No await/defer/application entity update before enqueue. Actual product HWND comes from
original Window native resolver; synthetic test override is cfg(test) only. No GPUI on_drag/native API
in Rust, clipboard/SendInput/CF_HDROP/BeginFileDrag substitute, pre-extraction or after-mouseup launch.
AC01 repair2026-10-23: immutable `ArchiveReadOperation::connection_generation()` originates at original
read admission, survives completion/Pending removal, and passes through consume/deferred install/
configure_drag unchanged. Formal `send_archive_virtual_drag(command,receipt_id,origin_connection)`
compares source origin with active connection atomically under original router lock BEFORE pending,
reservation or enqueue; stale origins reject without leaks even when old Disconnected remains queued.
No sampling current connection at configure/move. Generic unbound formal start fails closed; original
legacy isolated spike alias stays compatible. Cancel retains admitted connection, not newer hydration.

Existing C# peer-owned HWND -> capture/input release -> actual button/cancel recheck -> one background
STA COPY OLE remains. Descriptor zero-materialization; indexed contents use original EntrySession ->
worker/Job -> source binding -> output validation -> cache lease -> native IStream. Independent cursors
and target-held streams survive session retirement. Cancel ACK is NOT worker cleanup/terminal; native
slot stays until actual cleanup. Original start receipt alone retires view pending (incomplete cleanup
keeps busy); late cancelled/projection/generation replies do not alter new presentation.
Original PlatformCommandReceiver reserves ONE admitted drag identity/ONE exact cancel envelope on the
same connection, independent of ordinary queue saturation. Start must dequeue before cancel; original
fair cancel/ordinary alternation and coalesced wake remain. Cancel deduplicates; ACK does not release;
original start terminal releases unless fixed virtual_cleanup_incomplete; connection retirement clears.
Only these bounded original receipts avoid ordinary10second sweep; native120second owner stays.
Results and connection changes route to view on a later application turn, avoiding entity lease reentry.

## 4. Validation / Error Matrix
- Plain File/Directory snapshot entries, known or unknownsize, safe paths -> start without grant.
- Known encrypted ZIP/7z/RAR4/5 require explicit originalbrowser `解锁拖出` (officialMediumoutlineButton
  inToolbar.content) -> originalmaskedInput/Dialog/privatepasswordreply -> originalwholearchiveTEST.
  This extra fullread/decode/failedprefixcost is explicit, finite3attemptRequired/explicitWrongPassword
  only; ambiguousAuthenticationOrDataError is terminal. UnlockcreatesNOselectedoutputs/cacheleases.
- Successfulactualworkercleanup publishes opaquegrant; originalpipeEntrySession owns<=8grants/120s,
  protectedSAME_PROCESSmemory atrest,zeroes onrevoke/expiry/close/refresh/sourceinvalid/pipeStop/dispose.
  Binds exactsourcecontext/path/session/generation/codepage/depth/MOTW; nevercrosspipe/generalcredential.
- Prepare/reply/success retireoldcandidate and NEVERstartOLE; newfreshDown/heldMove ismandatory.
  Lockedgesture explainsunlockfirst. GetData NEVERopensmodalUI; originalworker receivesprivatebytes
  under originalsource/output/Job/cachechain,nointeraction/retry. Alreadyreturnedtargetstreamskeepowner.
- <=32selectedroots,<=128expandeddescriptors,<=32files (originalcache32leases),<=256contentrequests,
  <=256KiBserializedcommand and <=512MiBknownsum/actualtotal. Metadata-only immutable snapshot expansion
  includes explicit/implicit/empty/multileveldirectories, stableoriginalidentitydedup of overlaps.
  Receiver independently caps32distincttop-levelroots and requires EVERY proper path prefix to have
  an actual same-case directorydescriptor; impliedancestors cannot bypass128descriptors.
- Link/otherkind/unknown-encryption/unsupportedencryptedformat/locked-encrypted/unsafe/duplicate/case/
  path-prefixconflict/out-of-boundmember -> rejectWHOLEselectionviaoriginalAlert; no partialdelivery.
- SDK592byteFILEDESCRIPTORW,FD_UNICODE|FD_ATTRIBUTES; directoryattribute0x10; directories/unknownfiles
  omitFD_FILESIZE. Fullarraylindex includesdirectories; directoryFileContents rejectswithoutwork.
- Unknownfile actualsealedIStreamStat/contents are truthful; originalEntrySession worker/Job/source/
  output/cache remains. Eachdistinctindex chargesactualbytesonce,repeatindependentcursorsnotdoublecharge.
  Originalworker output is cappedbyremainingdragbudgetBEFOREwrites,knownestimate validatedagainstactual.
  Encoded source-family admission for BrowserEntry retains its original512MiBcap independently of
  remainingoutput; never reuse shrinkingoutputallowance as source-sizepermission. Otheroperationsunchanged.
- Projection/sort/filter/navigation/refresh/source-invalid/retire/close -> cancel candidate and active
  exact operation. Disconnect retires local transport state; original pipe Stop owns actual cleanup.
- Mouse no longer left-pressed before threshold -> discard, never async start.
- Mismatched/late request receipt -> ignore; separate cancel receipt cannot finish original start.
- Native rejection/failure -> fixed readable local message; payload/native errors not routine logs.
- cleanup_complete=false -> explicit blocked state, no retry that hides occupied native slot.

## 5. Good / Base / Bad
Good: Ctrl/Shift selection survives dragging a selected row, original source ordinals freeze before
mouseup, descriptor has zero work until target requests index. Base: one known plain file.
Bad: MouseDown means drag; cross-entity synchronous update; extract then mouseup then drag; silently
exclude directories/encrypted files; cancel receipt clears cleanup slot; target-held stream force-close.

## 6. Tests Required
Actual production ListItem events (not spike view): normal click/doubleclick/no-drag, threshold before
mouseup immediate queue, whole multi-selection, source IDs/HWND injection seam, once-only, retire
before/afterthreshold, terminal/cancelACK/late replies and visible official rejection feedback.
Original read-credit/complete install -> actual row -> CommandResult deferred route regression.
AC01 actual app/read/row regressions: replace after completion before deferred install OR after Down
before enqueue, keep old Disconnected queued, reject old start on new receiver, then fresh read starts
immediately once. Original router asserts pending/reservation empty after rejection, receipt reusable,
old connection cannot cancel a matching new reservation. Operation origin survives terminal removal.
C# formal alias/peer rejection, original descriptor/streams/STA/cancel responsiveness and actual small
ZIP delayed indexed contents/source-change/held-stream/slow cancel tests, exact LIST/RUN/PASS.
Host builds first; matching isolated-codecs worker after feature-unification checks, hashes before/after.
Passwordregressions: lockedfeedback/noOLE; explicitofficialunlock/Dialog; reply/successnoautoStart/
freshgestureonce/mixedencryptedtreecomplete; source/refresh/close/expiry/pipe replacement revocation;
originalticketwrongcorrect/stalecancel/ambiguous terminal,descriptorszerooutput/delayedactualbytes;
protectedbufferzeroprivacy/nodisksecrets/actualcleanupboundgrant/revokerace and originaltreebudgets.

## 7. Wrong vs Correct / evidence boundary
Wrong: ordinary row MouseDown starts; Correct: candidate -> held threshold -> synchronous enqueue.
Wrong: drag current sort index; Correct: frozen original ordinal and versioned source binding.
Wrong: cancel ACK means cleanup; Correct: original native terminal follows actual worker cleanup.
Synthetic GPUI/C# OLE consumers do not prove real HWND capture, physical button continuity, actual
DoDragDrop external target/COM marshaling/Explorer, DPI/UIA/user acceptance. Those remain NOT RUN.
This password slice is not fullP208/PhaseF/firstrelease PASS. TrueHWND/physical/externalOLE/Explorer/
DPI/UIA/useracceptance remain separate; noP209/package/install/Registry/COMregistration/bootstrap.
