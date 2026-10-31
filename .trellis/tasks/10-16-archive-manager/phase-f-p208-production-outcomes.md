# Phase F — P208 production ArchiveBrowser row hookup outcomes

Date2026-10-23. **P208 production source hookup pending independent acceptance.** Same canonical
**Phase F — Explorer integration and genuine native drag**; not fullP208/PhaseF/first-release PASS.
User authority recorded in [existing plan](implement.md#approved-p208-production-row-source-hookup--2026-10-23)
and [sole checkpoint](phase-a-checkpoint.md) BEFORE implementation. Executable boundary:
[production contract](phase-f-p208-production-contract.md). Historical spike/repair/negative acceptance
and failure logs remain unchanged. No installer/release-chain follow-up or new backend research.

## Formal production entry and actual supported scope

**Formal ArchiveBrowser rows are now connected in production source**, not just cfg(test) spike:

```text
original completed ArchiveRead context/source identity
 -> deferred ArchiveApplication.consume install + configure_drag
 -> actual official ListItem LeftDown candidate (not drag)
 -> original browser root held MouseMove >=4 logical DIP
 -> original BrowserModel full selection freeze
 -> actual Window HWND + snapshot/source/ordinal/generation/codepage/depth/MOTW
 -> immediate original PlatformBridge enqueue before MouseUp (no await/defer/other entity update)
 -> formal archive_virtual_drag / original pipe bounded admission
 -> SAME ArchiveVirtualDragSession dedicated background STA
 -> original source HWND peer/capture/input/button/final cancel checks -> one COPY DoDragDrop
 -> target FileContents(index) -> original EntrySession/worker/Job/source/output/cache/native IStream
```

Current supported source slice: **1..32 known-size regular files, encrypted==false, aggregate<=512MiB**,
safe unique descriptor names, original finite256 content requests. Multi-selection is never truncated.
Dragging an already selected row preserves the complete original selection; dragging an unselected row
uses the same BrowserModel selection API. Ordinary Click/Ctrl/Shift/keyboard/doubleclick/right-click/
scroll retain original selection/focus/action ownership. Doubleclick does not arm drag. Movement without
left held, before threshold, or after source/projection retirement cannot start.

Directory/link/other type, unknown size, encrypted/unknown-encryption/password-interaction selection,
unsafe names, duplicate names and exceeded item/byte bounds reject WHOLE selection with readable
original official Alert. No hidden password prompts, silent exclusion, Extract-then-drag, clipboard,
SendInput, real-path BeginFileDrag substitute, after-mouseup materialization or second GPUI drag owner.

These refusals are bounded current capability, **not removal of full P208 requirements**. Remaining
full-P208 capability work includes directory/tree virtual delivery, unknown-size representation and
password interaction/other beyond-spike selections, with their own safe lifecycle/native acceptance.

## Reference and component review before interaction edits

Read AGENTS/applicable specs, PRD/design/canonical plan, checkpoint, spike contract/independent
acceptance, F-AC repair acceptance, current component selection/catalog/index/0.8.0 API and exact
`list/list_item.rs`/Alert source. Reviewed ABDM first:
- `CompletedDownloadPage.kt:106–139`: Compose dragAndDropSource detects gestures; empty onClick is
  not evidence that every ordinary row MouseDown should begin a drag.
- `DownloadItemListDataFlavor.kt:14–39`: completed-file paths, not virtual archive contents.
Then legacy NxTools `DownloadCompletedWindow.axaml.cs:78–97,119–143` dedicated pressed drag surface and
`WindowsFileDragService.cs:29–61` one OLE loop; historical Avalonia fallback NOT adopted.
Current NativeFileDragService/Coordinator original input attachment, source-thread capture release,
actual button state and dedicated STA/dispatcher boundaries were traced and preserved.
Newly verified entry/call-chain invariants were recorded in `docs/reference-code/archive-virtual-drag.md`.

Layout stays the approved details-row form: official Kit0.8.0 selected/secondary ListItem32DIP plus
existing uniform_list/BrowserModel. Toolbar.content Medium outlined actions, Tree, status, palette,
fonts/native geometry remain unchanged. Original official warning Alert renders rejection; a plain
nonpainting semantic/test-support wrapper gives explicit label/positive-bounds evidence, not a fake
control or replacement theme. No dependency/vendor/backend/version change.

## Source owners changed

| Owner | Change / invariant |
| --- | --- |
| `archive/browser.rs` + private `browser_drag.rs` | View-local candidate/pending state, threshold/click suppression only after drag recognized, whole current selection validation/freeze, actual HWND, direct enqueue. Existing BrowserModel remains sole selection owner. |
| Browser projection/install/failure/refresh/retire/release/native-close | Discard candidate and exact-cancel active operation. No stale selection or source silently reused; cancel ACK does not clear original pending. |
| `archive/application.rs` | Completed original source context/platform/MOTW/depth hydration; deferred request-receipt result and disconnect routing. No synchronous application update from leased row callback. |
| `nxtools-protocol` | Additive formal `ArchiveVirtualDrag`, old `ArchiveVirtualDragSpike` compatible; shared exact version1 DTO/validator and symmetric formal/legacy roundtrip. |
| C# parser/capabilities/pipe | Formal `archive_virtual_drag` / capability `_v1` plus legacy alias -> SAME parser, synchronous bounded admission before detached receipt wait, SAME original session/STA/EntrySession. No new native controller/framework. |
| `nxtools-ipc/archive_virtual_drag.rs` / original router | One admitted drag retains one exact cancel independent of ordinary queue pressure; original receiver start-before-cancel/fair alternation/coalesced wake. Same connection generation; dedup exact cancel. Only held drag receipt IDs bypass ordinary10s timeout; native120s owner unchanged. |
| Original content/dataobject/entry/worker/cache | **Unchanged**. Descriptor does zero work; indexed request enters original worker; repeated streams have independent cursors and STGMEDIUM ownership; target-held streams remain readable after session retirement; actual cleanup owns slot retirement. |

Start request receipt ID is distinct from operation correlation. View matches original start receipt;
cancel has separate receipt and is not terminal. Late canceled/projection/generation replies cannot
change newer presentation. Explicit virtual_cleanup_incomplete keeps both view and original route
busy rather than pretending cleanup. Disconnect retires original connection; C# pipe Stop cancels and
drains actual owned workers. Native success already committed to a target is not retroactively undone
by a late cancel ACK during cleanup; no stronger native-drop cancellation claim was added.

Diagnostics preserve one drag operation correlation through Rust/IPC/C#: submitted/cancel_requested/
finished and original accepted/started/content_requested/content_ready/terminal/cleanup events, counts/
elapsed/fixed native result only. No names/paths/password/content, pointer/frame/tick logging.

## Red → green and bounded review

1. **Actual production row red** `02-production-row-red.log`: one selected test,0PASS/1FAIL,exit101;
   official row held movement left queue empty before mouseup. After formal listeners/state, same test
   PASS; no new spike view. OrdinaryDown/subthreshold emitted nothing, selected0+2 froze complete order,
   source/context/codepage/generation and synthetic HWND dependency917; no eager materialize command.
2. **Formal C# parser red** `csharp-red-parser-run.log`: LIST1/RUN1/FAIL1,exit1, formal name rejected.
   Final alias/peer/correlation/bounds fixture PASS; legacy compatibility uses same native admission.
3. **Unique bounded review finding**: ordinary command queue could fill after start dequeued, causing
   cancel enqueue failure while local state already marked canceled. `20-congested-cancel-red.log`
   reproduces at actual row:0PASS/1FAIL,exit101, next received command was ordinary instead of exact cancel.
   Local original-router fix added bounded reserved cancel, no retry task/queue expansion/new pipe.
   `23-reserved-ipc-green.log` and final actual-row test PASS; original cancel ACK/incomplete cleanup do
   not release slot, connection replacement rejects old cancels. One review only; no new review loop.

## Checks actually executed

Unique evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-f-p208-production-20261023-143208`.
Per-command raw logs/commands.jsonl. Every Rust command checked shared Cargo/rustc lane, waited rather
than killed other owners, jobs1/incremental0/RUSTFLAGS codegen-units256. No long command was duplicated.

| Check | Actual final result |
| --- | --- |
| Formal production row + original read/install/result route | **8PASS**,0FAIL (`24-final-row.log`): includes full-queue reserved cancel |
| Original browser actual-view tests | **26PASS**,0FAIL (`25-final-browser.log`): pointer/Ctrl/Shift/keyboard/context/doubleclick/scroll/state and formal drag |
| Original application lifecycle | **10PASS**,0FAIL (`26-final-application.log`) |
| Protocol library | **69PASS**,0FAIL (`final-09-protocol.log`) |
| IPC library after cancel fix | **39PASS**,0FAIL (`27-final-ipc.log`) |
| Strict Desktop/Protocol/IPC Clippy `--tests -- -D warnings` | PASS,exit0 (`28-final-strict.log`) |
| Workspace `--tests` compilation | PASS,exit0 (`29-final-workspace.log`), not workspace test execution |
| Feature-unification checks | Actual Desktop tree has no archive isolated-codecs feature; worker tree uses original isolated-codecs/sevenz/unrar versions. No feature/dependency edits. |
| Matching worker build AFTER host and Rust feature/compile checks | PASS (`31-final-worker.log`), final SHA256 `BE4212A56E7D3A1323C7F14DD8433789431B96CC90CE6318121959A368F17DBF` |
| Isolated C# host first, then Tests | Fresh isolated artifacts,0warnings/0errors,exit0; `csharp-green-{host,tests}-build.log` |
| Final C# exact controlled set | **LIST5=RUN5=PASS5**,selected5/run5/pass5/skip0/fail0/not_run0,names/order equal; final worker hash before/after equal (`32-csharp-final-list.log`,`33-csharp-final-run.log`,`csharp-final-results.json`) |
| ONE complete Desktop after convergence | **1042/1042PASS**,0ignored/filtered,129.39s,exit0 (`34-full-desktop-once.log`).1478 source hashes observed; one concurrent `downloads/tests.rs` changed during run. This is NOT a frozen-workspace certification; no rerun and no unrelated fix. |
| Scoped format | PASS (`35-final-scoped-format.log`), only task Rust files, no global reformat |
| Source/doc gates | formal-row/STA/delayed-content/reserved-cancel ownership, shell gate, GPUI catalog/release/editor provenance, PlatformHost fixture selection, spec/task and scoped diff; source gates not native acceptance |

Exact final C# fixture names:
- `archive Phase F virtual formal wire alias source peer rejection`
- `archive Phase F virtual descriptor indexed streams lifetime`
- `archive Phase F virtual STA gesture cancellation responsiveness`
- `archive Phase F virtual actual worker delayed indexed contents`
- `archive Phase F virtual slow worker cancel source close disconnect`

Actual tiny owned ZIP fixture verifies descriptor causes zero materialization, target index requests
original worker bytes, repeated stream cursor independence, multiple original ordinals, changed source
reject and target-held stream readable after OLE/session retirement. Slow actual worker fixture covers
cancel/Escape/source-close/disconnect, original partial cleanup and no terminal before actual worker
cleanup; injected native loop proves later platform commands remain responsive. These are controlled
consumers, not actual DoDragDrop/Explorer receive evidence. Old Ole32 pre-cancel/ABI fixture was NOT
selected or rerun; its historical timeout is unchanged.

Honest retained failures: first Rust compile stopped on test temporary selector borrow; fixed test only.
Application hydration fixture initially used invalid noncontiguous ordinal9, then missing click_count1,
then tested deferred timing after the outer GPUI update had already drained it; corrected test inputs/
observation inside current callback. All logs retained, not counted as product regressions. A new Alert
feedback assertion initially found no test-support node; official rendering was retained and measured
semantic wrapper added. IPC local fix initially attempted nonexistent JSON dependency (no dependency
added; native fixed cleanup error code used), then its capacity1 test notification queue filled because
the fixture did not consume notifications; corrected fixture. No failure log relabelled PASS/overwritten.

## Preservation / remaining acceptance / stop

Opening baseline2991 tracked+preexisting-untracked files; `final-preservation.json` separates owned
changes/new paths from concurrent Clipboard/download/Explorer task edits. Never attribute cumulative
HEAD diff to this round. No baseline file deleted; unrelated edits preserved. No installer/build-package/
signing/maintenance script, Cargo.toml/Cargo.lock, vendor or production Generated change this round.

Current **sequence25** product remains untouched: **84/84 artifact files unchanged**, no extras. Setup
SHA256 remains `1CF5FCCC2ACD62B96A6128018642A87C2D21DA07BED9BC206355E21C663D07E5`.
**60 historical archive outcomes/acceptance reports unchanged**, including spike/negative/repair evidence.
The source hookup is NOT in the existing sequence25 binaries; no packaging/install happened. Installer
payload duplication/size issue is explicitly deferred to the next separately authorized packaging task.

**NOT RUN**: actual GPUI HWND/capture release and physical left-button continuity, live mouse/keyboard
interaction, real DoDragDrop external target, foreign-process COM apartment/marshaled GetData,
Explorer receive/cancel, mixed-DPI/UIA/accessibility-provider/user visual acceptance. Test HWND values
exist only behind cfg(test); production always obtains the real Window HWND and C# validates pipe peer.
Self-app startup denial was not bypassed. No Registry/COMregistration/service/bootstrap/install/signing,
P209/password manager/source recycle/dependency/vendor/backend change, commit or push.

**Stop at P208 formal source hookup pending independent acceptance.** Directory/unknown-size/password
interaction and other beyond-spike selections are explicit full-P208 remaining requirements, not hidden
capability removal. Prior A–E/JOB07/SAFE08/native/user debts remain. Not fullP208/PhaseF/firstrelease PASS.
