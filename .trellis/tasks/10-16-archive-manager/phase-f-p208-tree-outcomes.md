# Phase F P208 — directory / unknown-size source outcomes

Date2026-10-23. **P208 directory / unknown-size source hookup pending independent acceptance.**
Same canonical **Phase F — Explorer integration and genuine native drag**, not a new phase/spike,
fullP208/PhaseF/first-release PASS. Source/controlled-consumer extension delivered; final Desktop
integration verification is **BLOCKED by concurrent floating-assistant settings compilation**, detailed
below. Do not treat the partial compiled evidence as a final frozen Desktop acceptance.

User authority recorded BEFORE implementation in [existing plan](implement.md#approved-p208-directories--unknown-size-vertical-source-amendment--2026-10-23)
and sole [checkpoint](phase-a-checkpoint.md); [production contract](phase-f-p208-production-contract.md)
is current. [AC01 independent closure](phase-f-p208-production-repair-acceptance.md) stays CLOSED and
unchanged; origin binding is retained, not re-reviewed. All historical outcomes/acceptance are immutable.

## Actual implemented production path

```text
original immutable BrowserModel.entries + same sole selected RowIds
 -> same official row Down candidate / held >=4DIP Move before release
 -> freeze_drag_selection metadata-only complete subtree / safe relative hierarchy / stable ordinals
 -> formal nullable-size/directory descriptors + original context/source/session/codepage/depth/MOTW
 -> SAME original read connection / SAME locked bridge admission / bounded start+exact cancel slot
 -> SAME C# parser / source HWND validation / dedicated STA / single OLE loop
 -> FILEGROUPDESCRIPTORW only, zero materialization; directory FileContents rejected
 -> file full-array lindex -> SAME ArchiveVirtualContent -> SAME EntrySession
 -> original worker/Job/source binding/output validation/cache lease -> verified native IStream
```

### Selection / hierarchy / safety

- Original BrowserModel remains sole selection/focus owner. Private `model_drag.rs` extends its pure
  metadata behavior, not another controller. No disk scan, archive reread or content materialization.
- Explicit directories retain original ordinal; implicit ancestor descriptors carry no ordinal but
  retain original source/snapshot context. Empty and multilevel directories are included. A selected
  subtree includes ALL descendants even when child rows are filtered/not visible.
- Parent/child/file overlap is deduplicated by stable source identity: each immutable entry is visited
  once and directory nodes merge only when exactly representable. Frozen order is deterministic
  case-folded relative path order; file content maps original ordinal, never current sort index.
- Within a nested browser folder, descriptor paths are relative to that folder, preserving selected
  subtree structure. All source path components are validated BEFORE relative rebasing; only one
  legal terminal slash on an actual Directory is allowed. No path sanitation, traversal, links,
  drive/ADS/absolute paths or arbitrary filesystem permissions.
- Whole selection rejects on unsafe components, duplicate/case conflicts, inconsistent ancestor case,
  file-as-parent, duplicate explicit directory identity, links/other kinds, warning safety failures,
  encrypted or unknown encryption, password interaction and bounds. No truncation/partial hidden
  exclusion. Same official Alert communicates fixed readable reason, no new control/theme/layout.
- Official Kit0.8.0 selected/secondary ListItem32DIP, original uniform_list, Tree, Toolbar.content
  Medium outlined actions, focus/click/Ctrl/Shift/doubleclick/keyboard and native geometry unchanged.

### Simple finite contract

| Bound | Contract |
| --- | --- |
| Selected roots | <=32 original selected root IDs before overlap dedup; receiver independently <=32 distinct top-level names |
| Expanded descriptor array | <=128 INCLUDING all actual explicit/implicit ancestor directories and empty directories |
| File descriptors | <=32, so original cache32leases is NOT enlarged |
| Content requests | <=256 files; directory queries/requests rejected BEFORE request counter or content owner |
| Command | <=256KiB, original1MiB pipe unchanged; Rust conservative C# Unicode/HTML receiver-escaping bound before enqueue |
| Known estimates | <=512MiB single/file aggregate before start |
| Actual output | <=512MiB per drag; distinct index charged actual sealed length ONCE; repeat requests do not double-charge |
| Other native owners | Original two worker runs, Job/memory/work/source/output/cache/120s drag/ordinary timeout unchanged |

Directory fields: defaultfalse/omitted `directory`, nullable `size` missing/null means UNKNOWN, NOTzero.
Directory size MUST null. Every descriptor retains nonnull original Entry/source/context; implicit
Directory may omit ordinal ONLY within descriptor parser validation. It cannot be dispatched to
ArchiveEntry materialize; files still require their original ordinal. Both aliases remain protocol1.
All proper path prefixes must be actual SAME-case Directory descriptors; implied parents cannot bypass
128-count. Source/context/path/codepage/depth/MOTW consistency is checked across the entire selection.

### SDK / original native content and actual budget

Verified installed Windows SDK10.0.26100.0 ShlObj_core.h1839/1843/1847/1865-1879/1897,
winnt.h FILE_ATTRIBUTE_DIRECTORY0x10, objidl.h2682-2695/12246-12253. Reference recorded before native
changes in `docs/reference-code/archive-virtual-drag.md`. FILEDESCRIPTORW592bytes: attributes36,
sizeHigh64/Low68,UTF16name72. Directory attributes0x10, files0x80; FD_UNICODE|FD_ATTRIBUTES for both,
FD_FILESIZE ONLY for known files. Descriptor HGLOBAL <=75780bytes. Unknown-size file/Directory do NOT
promise zero. Full descriptor-array lindex includes directories, but directory FileContents returns
index rejection without entering any file materialization. Native IStream Stat reports actual length.

`ArchiveVirtualContent` serializes DISTINCT materializations inside the existing owner until original
actual worker cleanup/lease validation/accounting completes. Before dispatch it passes remaining
actual output allowance to internal narrowing-only `ArchiveEntrySession.Dispatch`; that same original
runner/output gets max_entry_bytes/max_output_bytes lowered BEFORE writes. No wire budget override,
second extraction runtime or post-write-only aggregate gate. Native cache accessor preserves held
identity/hash/native read-only stream and validates optional known estimate; exposes verified actual
length. Repeated requests revalidate identity and return independent cursors without charging twice.
Already transferred media/target-held streams remain readable after original drag/session retirement.

Unknown output, concurrent opens and failed/retained/incomplete cleanup cannot finance additional
outputs beyond remaining allowance. Same original exact lease release and actual worker cleanup own
retirement. When remaining==0, new file materialization refuses even a zero-byte file because original
worker budget requires positive values; no zero-budget bypass added. Zero-byte files work while any
positive budget remains. This bounded limitation is explicit, not a known-size-zero substitution.

BrowserEntry encoded source-family cap stays its ORIGINAL512MiB independently of remaining output;
other operation source limits unchanged. A small selected output from a larger encoded archive is not
rejected merely because output allowance shrank. Original decode-work limits are separate from output.
AC01 immutable original read origin/deferred install/locked generation admission and exact cancellation
remain unchanged. No GPUI drag owner, Extract-then-drag, clipboard, SendInput or second OLE framework.
Diagnostics reuse existing correlation/start/content/terminal/cleanup/count/index/elapsed events only;
no source/name/password/payload/pointer/tick logs, no temporary DEBUG probes.

## Red -> green and single bounded review

Unique evidence:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-f-p208-tree-20261023-181914`.
Per-command Cargo `commands.jsonl`, full raw logs and opening/protected baselines remain. All Cargo
checks/waits for shared lane, jobs1/incremental0/RUSTFLAGS codegen-units256; no owner killed/duplicate build.

1. **Actual formal-row red** `01-formal-tree-red.log`: production ListItem Down/Move with completed
   snapshot entries explicit/implicit/empty tree + unknown file + ordinary file. RUN1/PASS0/FAIL1,exit101,
   formal receiver empty. After freeze/DTO hookup `03-formal-tree-green.log` SAME test PASS1,exit0,
   complete five-node hierarchy/source ordinals/no eager contents. This was executed before subsequent
   final tightening; final complete Desktop view collections remain blocked, not claimed rerun green.
2. **C# wire and descriptor reds** `csharp-tree-red-wire.log`, `csharp-tree-red-descriptors.log`: EACH
   LIST1/RUN1/PASS0/FAIL1,exit1. Old DTO/native descriptor cannot accept null size/Directory. Final exact
   native collections below PASS, including real unknown-size worker/read/stream.
3. ONE bounded read-only review, no repeat review: found receiver missing-parent and32-root bypass.
   `34-review-red.log` executes5matched tests:3PASS/2FAIL,exit101; EXACT new failures
   `tree_missing_parent_descriptor_cannot_bypass_expanded_count` and
   `tree_wire_limits_distinct_roots_independently_of_files`. Original production freeze already emitted
   directories; receivers now independently require actual parents/countroots. Final Protocol74PASS.
   C# `37-csharp-red-run.log` selected2/run1/fail1/not_run1: missing parent accepted; harness stopped
   before source budget test. No unrun case counted as failure/pass.
4. Parent traced original runner max_output also serving encoded source-family cap. Narrowing output
   made a tiny archive fail before3byte file could open. **39-source-budget-red.log** exact LIST1/RUN1/
   FAIL1,exit1. Minimal BrowserEntry source-cap separation, fixed receiver counts/parents and updated
   full-parent fixtures -> **final11 all PASS**. Source-vs-output test uses two3bytefiles under6byteoutput
   allowance while encoded ZIP>6bytes; original source/worker/Job/output chain, no large files.

Honest retained intermediate attempts: `05-protocol.log` failed test-only None type inference, fixed
Option<u64> annotation. `49-model-final-strict.log` nonminimal_bool on this round's model, fixed exact
is_none_or equivalent; `50/51` model tests/strict PASS. C# initial nullable browser exception path,
fixture lambda/discard compile and too-small exploratory fixture budgets are retained in the subagent
logs; exact final cases pass after concrete fixes. No failed log overwritten or relabelled green.
Old worker explorations are explicitly exploratory, NOT final matching-worker evidence.

## Actual execution / counts / integration blocker

| Check | Actual observed result |
| --- | --- |
| Final Protocol library /41 | **74PASS**,0FAIL/ignored/filtered. Includes concurrent protocol additions, not74new archive tests. New compatible nullable/tree/root/parent/command-cap regressions pass. |
| Final IPC library /48 | **40PASS**,0FAIL, original congestion/ACK/incomplete/timeout/origin reservations intact |
| Current production model directly referenced by scratch test crate /50 | **10PASS**,0FAIL; includes overlap/hidden descendant/conflicts/counts/original IDs. Supplemental headless compilation, NOT formal GPUI collection. |
| Same exact production model strict Clippy /51 | PASS,exit0; scratch allow(dead_code) only because other production model APIs have no caller in this tiny supplemental crate; no product warning suppression |
| Protocol/IPC strict tests Clippy /44 | PASS,exit0 |
| Final isolated C# host-first then Tests /42,/43 | PASS,0warnings/0errors |
| Desktop enabled feature tree /21 | No archive isolated-codecs activated; framework graph/provenance unchanged by this task |
| Worker isolated-codecs tree /22 + final matching worker /45 | PASS, original dependencies/codecs, buildexit0 |
| Final exact C# LIST/RUN /46,/47 | **LIST11=RUN11=PASS11**,same names/order; selected11/run11/pass11/skip0/fail0/not_run0 |
| Matching worker SHA256 before/after final11 | `0F6DA80E4C26B74CB90BAABD0D6144E9E107920994AC57673BBC743B069467D4`,identical |
| Scoped rustfmt/source/doc gates | Affected Rust format, PhaseF row/tree/origin/reservedcancel/oneSTA/content ownership, Shell, catalog/release/editor provenance/spec/task/diff; gates are NOT native/user acceptance |
| Final Desktop model/formal drag/browser/application collections | **BLOCKED at compilation** by concurrent floating-assistant settings; do not substitute old binaries or supplemental crate as final view PASS |
| Desktop strict tests Clippy /19 and workspace --tests /20 | **BLOCKED/exit101** by unrelated concurrent code, not pass; no unrelated fix |
| ONE fullDesktop allowance | **NOT STARTED** because workspace/target not compilable. No full-suite retries, no100k/large/performance/codec/password/conflict matrices. |

Final11 exact C# names (raw identity/order checked in `final11-results.json`):
- `archive Phase F virtual tree wire shapes conflicts bounds`
- `archive Phase F virtual tree SDK descriptors directory refusal`
- `archive Phase F virtual tree actual ZIP unknown streams identity`
- `archive Phase F virtual tree actual remaining budget cleanup`
- `archive Phase F virtual tree actual unknown Zstd native stream`
- `archive Phase F virtual tree source versus output budget`
- `archive Phase F virtual formal wire alias source peer rejection`
- `archive Phase F virtual descriptor indexed streams lifetime`
- `archive Phase F virtual STA gesture cancellation responsiveness`
- `archive Phase F virtual actual worker delayed indexed contents`
- `archive Phase F virtual slow worker cancel source close disconnect`

Actual unknown Zstd fixture is independent12byte frame without Content Size: original Browse reports
sizeNone/encryptedfalse, descriptor omits size flag, original worker returns `one`,nativeStat3,repeat
cursor independent and held stream survives retirement. ZIP tree demonstrates directories never request
contents, lindex3->originalordinal4 and lindex4->ordinal1, descriptor zeroleases, exact contents/changed
sealed identity refusal. Budget fixture uses700byte outputs/lowered1300byte allowance with known/
unknownoversize,concurrent,repeat,estimate mismatch,zero remaining; only700bytes written, no failed
medium/partials/manifests/outputs after actual cleanup. Old slow cancel/Escape/sourceclose/disconnect
still waits actual worker cleanup, not ACK. Controlled consumer is NOT foreign-process COM/Explorer.

### Exact concurrent Desktop blocker

`10-model.log` initially E0004 for new FloatingAssistant event in clipboard.rs/quick_launch.rs; those
were later changed by their owner, not this repair. `19-desktop-strict.log` records8 concurrent errors,
`20-workspace.log`6. Bounded retry `27-production-model.log` still fails with5 errors in concurrent:
- `main_window/floating_assistant_tests.rs:2,17`: private settings body/choice_labels;
- `main_window/tools_page/floating_assistant_settings.rs:311`: actions inference;
- same file458: missing Focusable trait;803: settings label type mismatch.
At last observation settings SHA80334945E62085E81DDB57E85FFBC234D2B846C9DD93D794B4CFC812A76B08FE
and testSHA0B80D8D47D1AF1CBE21E37638B5290EAE5E43285EEE9D14350A3368F1CED74CC unchanged since failed run.
No archive source location reported. No broad retry/patch to Clipboard/QuickLaunch/assistant to mask
this blocker. Final new direct-directory-pointer and hidden-encrypted-member production-view test is
written but NOT executed in final compilation; source-only evidence is labelled honestly. Later full
formal view/AC01/application/strict/workspace regression remains required once concurrent work compiles.

## Preservation / unperformed / stop

Opening baseline3018 tracked+preexisting-untracked files, not cumulative git diffHEAD. Final preservation
separates task-owned changes/new model/test/report from concurrent floating-assistant source/docs and
manifest edits. This task made no dependency/version/vendor change; concurrent Desktop Cargo.toml and
Core/Protocol/main/UI changes remain owned by that task and are not reverted or claimed here.
**84/84sequence25artifacts unchanged**, no additions; installer hash remains
`1CF5FCCC2ACD62B96A6128018642A87C2D21DA07BED9BC206355E21C663D07E5`.
**64/64historicalarchiveoutcomes/acceptancereports unchanged**,including AC01CLOSED and prior failures.
Cargo.lock/vendor/installer/scripts/productionGenerated untouched by this task. SourceNOTinsequence25;
installer payload duplication deferred until separately authorized packaging. ProductApplyAuthorized=false.
No product launch, security bypass, package/sign/install, real HWND/physical drop/Explorer/COMregistration/
Registry/service/bootstrap, source recycling, dependency/vendor edit, commit/push or second review.

Password interaction/encrypted/unknown encryption/link/other types remain unsupported; no fullP208claim.
Actual HWND capture/button continuity, physical gesture, real external OLE/foreign-process COM/Explorer,
DPI/UIA/user visuals remain **NOT RUN/unaccepted**. Prior A–E/JOB07/SAFE08/native/user debts remain.

**Stop: P208 directory / unknown-size source hookup pending independent acceptance**, with the explicit
concurrent Desktop compilation blocker and deferred final formal-view/full integration checks above.
No other feature or release work started.

## Final closure audit

Final per-round audit:3018baseline =2977unchanged +24ownedchanged +17concurrentchanged,missing0;
3ownednew(model_drag.rs,ArchiveVirtualTreeTests.cs,this report),19concurrentnew. Concurrent edits
remain untouched. Seq25artifacts84/84/history64/64/lock/vendor/installer/scripts/Generatedunchanged.
Final spec gate110documents/14owners (includes concurrent spec additions), task82implement/79check
entries passed with existing32KiBcontext-injection warnings. Scoped rustfmt and owned diff checkexit0;
Program.cs existing CRLF-to-LF Git warning is not a format rewrite. No new build/fullDesktop/native
operation after final11/model verification. Concurrent Desktop compile blocker remains explicit.
