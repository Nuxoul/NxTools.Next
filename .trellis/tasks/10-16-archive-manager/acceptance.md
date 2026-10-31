# Archive manager — executable acceptance scenarios

Status: **approved A–F first-release test contract; partial Phase A fixtures executed, no complete scenario/phase acceptance**. Existing56 IDs remain stable. Phase ownership is fixed by [implement.md](implement.md); RAR P201 moves to A, split/ZIP P202/P203 stay E and genuine drag P208 stays F. Actual evidence and the native overwrite red are in [checkpoint](phase-a-checkpoint.md). Documentation gates do not satisfy behavior; tests must be red-capable at real boundaries.

Overwrite amendment2026-10-17 supersedes strict target-ID atomic CAS only: validated complete
temporary + immediate target recheck + single atomic path replacement. User accepts recheck-close
to replacement TOCTOU. No delete/truncate/backup-first, no automatic retry after published/unknown
outcome, no power-loss guarantee. Source lease/parents/reparse/integrity/budgets remain mandatory.

## Continuation acceptance amendment — 2026-10-28

[Continuation plan](continuation-plan.md) adds stable requirement IDs UX01–UX05, EDIT01/02,
HZ2–HZ5 and named V-UX/V-PROGRESS/V-DRAG/V-BATCH/V-EDIT/V-CRASH/V-RECLAIM/V-REGRESSION gates.
Original56IDs and past results remain unchanged; new planned cases are NOT executed PASS.
Real Explorer latency/actual target completion differs from fake consumer/materialization;
source-preserving crash tests differ from resumable recovery and VM power loss. HZ5 requires
new consent/native-object authority/output-readback→durable-journal→source-mutation proof,
ordinary/keep mode byte preservation and actual allocation gain; no simple DeleteSource test.
New per-format edit tests preserve existing ZIP record fidelity and precise native commit facts.
No real product/system/destructive source operation is authorized by this planning document.

## Engine and interoperability

| ID | Fixture / operation | Required observable result |
| --- | --- | --- |
| ENG01 | Each advertised writable format: empty dirs, nested Unicode tree, empty file, binary bytes, big streaming source; semantic levels where valid | create→independent reader/list→extract→byte comparison; no absolute input paths; stored root policy correct; TAR no checksum guarantee stated |
| ENG02 | Independently produced read-only ZIP methods plus AES/ZipCrypto,7z coder/encryption/header variants | enumerate/read supported entries; capability-specific unsupported errors; no self-round-trip alone as interoperability proof |
| ENG03 | missing/wrong password, damaged AES/CRC stream; cancelled password prompt; retry with selected subtree/custom destination | typed PasswordRequired/WrongPassword only when certain, otherwise AuthenticationOrDataError; original request preserved; no secret log; no output commit after cancellation |
| ENG04 | disguised extension, compound tar.gz/tar.xz, ZIP-family JAR/APK, random file, single gzip stream | bounded signature probe; correct container; signed package read-only; no full decompression just to fill size column |
| ENG05 | ZIP UTF8 flag, valid/invalid Unicode Path extra, GBK/Big5/Shift-JIS/CP949/CP437, ambiguous short bytes | authoritative Unicode precedence; exact manual decoding; auto uncertainty not fake certainty; collisions rechecked after code-page switch |
| ENG06 | 7z solid/shared blocks; stream unknown size; optional method/time/encryption fields | unknown remains unknown; packed size not copied to every solid entry; time precision maintained; progress never fabricated |
| ENG07 | snapshot loaded then source file replaced/mutated, same display path | operation rejects SourceChanged or performs explicit reload/reconfirm, never applies old ordinals to new bytes |
| ENG08 | 100k-entry ZIP, recorded workstation/cache state; >4GiB source/ZIP64, long paths | measure metadata load/peak RSS/first paint; target1.5s recorded as target not guaranteed; bounded allocation; ZIP64 no silent truncation |
| ENG09 | paused reader, cancellation during chunk, prompt wait, finalization; slow uncooperative codec | synthetic cooperative response target≤200ms and clean owned partials; native/codec call not falsely interrupted; retained worker slot and truthful busy state |

## Safety and durability

| ID | Fixture / operation | Required observable result |
| --- | --- | --- |
| SAFE01 | ../, mixed slash traversal, root/drive-relative/UNC/device path, ADS, NUL, control chars, CON.txt, trailing dot/space | no destination escape or unsafe auto-renaming; list warning/typed reject; no write outside owned temp destination |
| SAFE02 | duplicates, case-folded collisions, file `a` with child `a/b`, Unicode normalization collision | preflight detects collision before writes; same-archive ambiguity not treated as ordinary overwrite prompt |
| SAFE03 | existing parent junction/symlink, destination replaced between check/open/commit, input reparse loop | real native temporary-tree adversarial seam proves no outside writes; lexical `starts_with` test alone insufficient |
| SAFE04 | metadata entry bomb, forged sizes, actual expansion beyond finite budgets, huge dictionary | bound before allocation where possible and during output; typed budget/risk result; no uncontrolled RSS; benign high-ratio data can request finite escalation |
| SAFE05 | write/decode/CRC/flush/access failure before commit; user/other-job partial collision | CREATE_NEW owned sibling only; full integrity+flush before one publication; FailedBeforeCommit preserves old target and cleans only own partial; postcommit/unknown failure explicitly does not claim old-target preservation |
| SAFE06 | overwrite/skip/auto-rename/ask-all; target replaced/edited/missing/appeared/directory/reparse/busy before final recheck | fullID/version recheck relative to held parent; typed refusal/re-prompt before closing target check handle; ordinary atomic path publish then post-ID observation; accepted TOCTOU is documented, no target-ID CAS assertion; native32/5 never trigger generic fallback |
| SAFE07 | cancellation before/during/after commit; post-verification failure/external replacement; multiple owned/unowned partials | precommit cancel removes own partial only and preserves old target; CommitInFlight linearizes cancellation so actual native success stays committed; published/unverified/unknown output never partial-deleted or blindly retried; unchanged new-output optional cleanup only, replacements/external edits retained |
| SAFE08 | create output inside source tree, source changed before Recycle Bin, selected-only extract+delete, partial skip/failure | no self-inclusion; no source deletion unless complete eligible success/test and identity match; per-path recycle failure becomes warning, no permanent fallback |

## Configuration and main workspace

| ID | Fixture / operation | Required observable result |
| --- | --- | --- |
| CFG01 | valid old config missing archive field; valid new section round-trip | defaults only for absent field, normalized persisted values; no whole-cache overwrite |
| CFG02 | stale archive settings save while unrelated section changes | both latest unrelated values and intended archive values survive locked merge |
| CFG03 | malformed JSON/invalid existing section/missing update target/write failure | visible save failure, unchanged old file/live state; no default recreation or false saved/applied message |
| CFG04 | settings search/page switch/remount, limit invalid text, Clear recent/temp | official Settings state controls filtering; stable mounted drafts; no background side effects on render; no active/unowned temp deleted |
| UI01 | sidebar keyboard/pointer and quick actions | approved six-module order and labels; one dispatch; original176 rail/main budgets; Configure page does not launch archive jobs; native picker cancel silent |

## Browser, interaction and UI states

| ID | Fixture / operation | Required observable result |
| --- | --- | --- |
| UI02 | screenshot-like archive with one enclosing folder, implicit dirs and comment | tree root/archive name, directory list, parent row, correct status totals and bounded plain comment; no copied Bandizip skin |
| UI03 | Ctrl/Shift pointer, Shift+arrow, Ctrl+A, right-click selection, sort/filter/scroll | one ID-keyed selection owner; selected semantics/UIA and actual paint agree; context targets immutable IDs; not last clicked index |
| UI04 | Tree expansion/selection plus double-click folder and Backspace | one navigation route; deferred Tree/list sync; no loops or leased-entity reentry; expansion not misused as selection |
| UI05 | Ctrl+O/N/E/T/F,F4,Enter,Escape,Tab/ShiftTab; active text input | action eligibility identical for pointer/keyboard; no editing text shortcuts hijacked; focus restored to live trigger; no duplicate submit |
| UI06 | default/minimum × DefaultLight/Dark; long CJK names/error text; empty/loading/failure/selected/disabled | real official controls/outlined idle header actions, no clipping/unreachable footer; unknown progress not percentage; no error disguised as empty |
| UI07 | window close/F4, same-path reopen, two archives, maximize/restore, mixed monitor | correct per-window ownership/bounds; archive browser activating, not passive; close does not cancel independent jobs; other windows unchanged |

## Lifecycle and native result races

| ID | Fixture / operation | Required observable result |
| --- | --- | --- |
| LIFE01 | immediate worker/cache completion inside a browser callback | next-turn application, no synchronously leased-entity update cycle; exact production fixture |
| LIFE02 | open A→close→open B, old listing/password/picker result arrives | generation/revision/weak-handle checks discard old response; no B mutation/reveal |
| LIFE03 | repeated same-path open while loading/minimized/ready | at most one keyed live browser; restore/focus once; late first open cannot resurrect retired entity |
| LIFE04 | temp entry-open success/failure and active-content warning cancelled | only validated successful extract reaches Shell; no process launch on cancel/failure; MOTW result truthful |
| LIFE05 | child archive open, parent close; external viewer holds file; startup sweep | leased temp retained; only retired inactive expired owned sessions swept; no process kill or arbitrary recursive deletion |
| LIFE06 | refresh fails/source changed/unsupported revision | last-good list visible but stale/read-only; explicit retry, no hidden corruption or stale mutation |

## Forms, jobs and completion

| ID | Fixture / operation | Required observable result |
| --- | --- | --- |
| JOB01 | extract all/selected; Smart/Here/Name; one dir/one file/multiple/empty; compound suffix | exact deterministic output plan, implicit folders counted, selected structure preserved; empty archive no spurious destination |
| JOB02 | Create format switch with password/method/level/solid fields and GZ multi-item error | capability-validated request; semantic levels; unsupported hidden values cannot submit; files recurse with relative names and exclusions |
| JOB03 | conflict or password/risk prompt then Escape/close/cancel/late response | promptID/generation, cancellation wakes worker, single terminal; original source/destination/selection preserved on retry |
| JOB04 | max2 running/max32 queued, simultaneous same-archive mutations and output target | bounded admission, resource-aware serialize, deterministic Busy; no duplicated worker per UI repaint |
| JOB05 | progress1000 updates, pause/resume/cancel, presentation close/background/reopen | latest snapshot≤10Hz, lossless terminal/prompts; close means background; no fake pause guarantee, taskbar cleared on terminal |
| JOB06 | success/warnings/cancel/failure + NotificationCenter actions | once-only meaningful result and eligible actions; warnings not full success; cancellation not error spam; no second notification owner |
| JOB07 | test-after-create failure, source Recycle Bin failure, target commit failure | old archive/source preserved as applicable; recycle result separate from archive success; no destructive cleanup after partial success |

## IPC and startup

| ID | Fixture / operation | Required observable result |
| --- | --- | --- |
| IPC01 | every new Rust/C# payload round-trip, invalid enum/count/path/NUL, capability absent/version mismatch | bounded symmetric decoding, fixed typed failure, no silent incompatible old-payload change |
| IPC02 | immediate native result/queue full/disconnect/timeout and replacement generation | pending installed before dispatch, once-only retirement, no late apply; native timeout not thread interruption |
| IPC03 | picker/save/source dialog accept/cancel/fail, old draft receives late result | selected path alone no create/install; cancel silent; ID+generation match; native error code retained only when present |
| IPC04 | cold archive launch, already running, protected relaunch, silent/background/browser modes | intent handled before summon return; archive UI without unwanted main-window flash; original modes unchanged; authentic current-user peer, not client_name trust |
| IPC05 | duplicate request ID, queue overflow, grouped multi-select vs unrelated near-simultaneous launches | duplicates suppressed/bounded reject; unrelated requests not timer-merged; all actually grouped paths preserved; no argv passwords |

## Extended first-release gates and explicitly optional enhancements

| ID | Fixture / operation | Required observable result |
| --- | --- | --- |
| P201 | **Required A:** independent RAR4/5 plain/encrypted/corrupt fixtures | reviewed read-only adapter, exact capabilities/errors/license; never RAR write |
| P202 | **Required E:** `.7z.001`, `.z01 + .zip`, RAR volumes; first/second/last/missing; ZIP/7z split creation | bounded discovery/recovery; genuine split-format interoperability, not chopping hack |
| P203 | **Required E:** ZIP add/delete/rename/new-folder/comment + error/cancel/crash before commit | metadata/encryption preserved or operation explicitly refused; original intact on failure |
| P204 | Optional explicit temp-edit write-back; original changed/decline/commit failure | prompt/revision verified; user edits retained, no automatic overwrite |
| P205 | Optional image preview; malicious dimensions/huge/stale selection | bounded off-UI decode; current-ID/generation only |
| P206 | pure registry plan + simulated write-then-throw rollback/uninstall mixed ownership | exact previous values/kinds restored, only owned cleanup, no HKLM/UserChoice writes; real apply separately authorized |
| P207 | actual Windows10/11 classic Shell multi-selection | demonstrate delivery/group identity; no promise of Win11 top-level menu; no unrelated-intent coalescing |
| P208 | virtual and materialized multi-item OLE drag, slow materialization, Escape/mouse-up/stale HWND/disconnect | live pressed initiation, one dedicated C# STA loop, source ownership/capture release, bounded lifetime and cancellation; subsequent platform commands/clipboard/hotkeys remain responsive |
| P209 | Optional password-manager store/retrieve/delete/cancel/retry | isolated current-user namespace, no extension secret exposure or plaintext JSON/log/argv |

## Diagnostics and evidence

DIAG01: capture synthetic operation traces through actual worker+native dispatch. Each accepted job has start/meaningful phase and one terminal with matching correlation, elapsed, safe typed error and native code when applicable. Search captured logs for synthetic sentinel password/path/comment strings: none may occur. Rapid byte updates must not cause high-volume logging. Failed diagnostic sink must not change operation result.

For every case record **not run / passed / failed / skipped with reason**, exact command/date/artifacts; optional P204/P205/P209 may remain unimplemented, but no required case may be waived as former P2. Do not pre-tick from this document. Distinguish:
1. source/reference review;
2. compiled unit/integration and isolated production GPUI;
3. individually inspected screenshots;
4. actual authorized pointer/keyboard/DPI/native Shell/UIA;
5. user visual/interaction acceptance;
6. separate Release/install acceptance if requested.

All fixture data belongs to a unique temporary directory; no real archives, settings, credentials, Shell defaults, indexer or service mutation. Browser screenshots use disconnected synthetic preview data. Deterministic malicious archives are generated or attributed, never distributed from an unknown user dataset.
