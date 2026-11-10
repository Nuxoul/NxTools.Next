# REF-18 — Explicit ordinary numbered-file join

Status: **accepted at source, production TestPlatform and owned-native fixture boundaries**. [Coordinator acceptance](coordinator-acceptance.md) closes B3-02 with independent REF18 11/11, IPC62/62 and retained native raw4/4 evidence. [Repair evidence](acceptance-repair.md), initial source/native results and the [executable contract](../../spec/platformhost/raw-numbered-join.md) retain their stated layers. B4–B7 remain paused.

## Source chain and design

Original MainWindow Other Tools ListItem tile → deferred original tool-window map/open-or-activate → slice-local JoinWindow/BaseRoot/controlled caption → stable official Form/Field/Input/Medium32 buttons → metadata Inspect via origin-bound original PlatformBridge/router → original PlatformPipeClient serial reader synchronous admission + detached dispatcher → per-pipe RawNumberedJoinSession → existing bounded STA runner → original ArchiveSourceLease and ordinary ArchiveOutputSession.

A discovered preview returns a frozen native preview ID and bounded ordered names/lengths/count/total. The official Dialog displays the selected output and explicit ordinary-byte/unknown-final-part/source-preservation facts before any Start. Source edits retire preview; later-part selection is supported. Default680×600/min480×360 use a scrolling body, fixed footer, indeterminate official Progress and32DIP actions. Existing Files/Save picker, native HWND, focus and sole notification forwarder remain; no replacement manager/transport/store.

Inspect/Start/Cancel carry operation/session/generation and original connection identity. Pending is installed before send. Both first open and cache-hit activation defer beyond MainWindow's lease; replies defer through weak window/entity. Cancel ACK does not release the UI pending operation, IPC reservation or native slot. Caption/footer close requests cancellation and waits for actual cancelled+cleanup-complete terminal. Late success/cleanup failure remains visible. Disconnect reports output status unknown; original per-pipe native owner cancels and drains actual work.

Native family admission is same-directory/same-basename, canonical numeric `.001/.002/...`, ordinal1..1024, checked≤32GiB total and≤262144 UTF-8 name bytes. Detectable gaps/aliases and classic `.z01` reject before output; no unknown-final-part completeness claim. Start re-admits native identities/metadata and holds all bounded read-only source leases through actual completion. One65536byte buffer streams content once into an original fresh no-replace final name. Same-family numeric output is refused before content reads. Existing parent required; no source deletion, whole-content buffering, hash, second content scan, candidate/rename or format conversion.

Original reader and shared native event thread do no transfer IO. The existing named max1 STA runner has no early outer timeout/cancellation; the inner token controls bounded transfer. Session Stop/DisposeAsync awaits real worker/handle finally. Low-noise accepted/start/state/terminal diagnostics retain correlation, elapsed/fixed/native code and cleanup facts, without names/paths/content or per-chunk logs.

## Behavioral RED → GREEN

Log root: `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.

| Boundary | RED | GREEN |
| --- | --- | --- |
| Real ordinary Windows join | `b3-ref18-red-list.log` LIST1; `b3-ref18-red.log` RUN1/FAIL1 `raw_join_unavailable` | Later-part preview and exact original byte-stream join pass in native4 below. |
| Original router full-queue Cancel reservation | `b3-ref18-ipc-red-list.log` LIST1; `b3-ref18-ipc-red.log` RUN1/FAIL1 `real operation admission: Busy` | `b3-ref18-ipc-green2.log` PASS1: coalesced fair reserved Cancel, no long-op timeout, ACK retains reservation, actual terminal releases, old connection rejects. |
| Same-family new numeric output | `b3-ref18-preflight-red-list.log` LIST1; `b3-ref18-preflight-red.log` RUN1/FAIL1 `numbered family output refused before content I/O` | `b3-ref18-preflight-green-list.log` LIST4; `b3-ref18-preflight-green.log` RUN4/PASS4/SKIP0/FAIL0/not_run0, including zero reads/bytes/output for this case. |
| Actual source input + Discover pointer action | `b3-ref18-ui-red3-list.log` LIST1; `b3-ref18-ui-red3.log` RUN1/FAIL1 `real raw operation queued: receiving from an empty channel` | `b3-ref18-ui-final-list.log` LIST9; `b3-ref18-ui-final.log` RUN9/PASS9/FAIL0/ignored0. |

## Executed verification

- `cargo test -p nxtools-desktop --bin nxtools-desktop ref18_ -- --list`, then identical filter with `--nocapture --test-threads=1`:9/9. Actual input/Discover queue, explicit Dialog Escape/single Start/Enter-repeat, Cancel ACK and late success, caption close only after actual terminal, cleanup failure visible, stale generation/old connection/deferred callback under live lease, official marked-text IME Enter/source-edit invalidation, default/minimum scrolling/output/fixed32DIP footer/Dialog confirm bounds, original picker result identity/single selection/focus, actual MainWindow tile first-open/cache-hit/retire/reopen.
- `cargo test -p nxtools-protocol ref18_`: LIST1 in `b3-ref18-protocol-list.log`; PASS1 in `b3-ref18-protocol-green.log`. Strict shapes, redacted Debug and ordinal/generation guards. Final affected library/preservation results are in [B3 outcomes](outcomes.md).
- Native build `b3-ref18-preflight-green-build.log`:0warnings/0errors. Exact fixture selection/effects were LISTed before identical RUN via the existing `NxTools.PlatformHost.Tests.exe --fixtures "raw numbered join"`; final native4 log pair above has no skipped/not_run entries.

The four reviewed native fixtures are:

1. `raw numbered join later part byte exact`:320123known bytes in three out-of-order parts, selection`.003`, ordered metadata-only preview and independent ordinary byte-for-byte output readback. No output execution.
2. `raw numbered join family source refusals`: missing middle, aliases, numeric1..12order, unsupported suffix,1025family, source/family changes, existing foreign/source/new-family output and missing parent refusal. Sources/foreign files remain.
3. `raw numbered join stream cancel failure handles`:2MiB+71source exceeds the single65536byte buffer. Actual read/write maxima≤65536, reads==writes>32 and exact bytes show one content pass. Held source leases refuse writers; exclusive RW reopen after return proves release. Precommit cancel cleans only incomplete output; postcommit cancel retains success. Controlled write error112 returns `write_file`, cleanuptrue and no incomplete output/source mutation.
4. `raw numbered join serial reader physical drain`: GUID named pipe invokes original ReadLoop/dispatcher/session. While actual Write is held, Ping→Pong and invalid clipboard metadata parsing still respond; exact Cancel ACK does not terminalize Start, replacement is Busy, and DisposeAsync remains pending until worker release. Actual cancelled/cleanup-complete result, output removal, source reopen and correlated secret-free lifecycle diagnostics follow. No real clipboard/Shell/hotkey operation is invoked.

Native effects are GUID-owned scratch files, one GUID pipe, original Windows source/output handles and bounded workers. Production memory evidence is the finite buffer/bounded metadata/handle construction plus actual observed transfer block/byte counts, not total-process peak profiling or a timing performance claim.

## Changed paths

Desktop `tools/{mod.rs,raw_join.rs,raw_join/layout.rs,raw_join/tests.rs}`, `main_window/tools_page.rs`, `platform_events.rs`; Protocol/IPC `src/{lib.rs,raw_numbered_join.rs}`; C# `Windows/Archive/RawNumberedJoin.cs`, `Protocol/{RawNumberedJoinContract.cs,RawNumberedJoinSession.cs,PlatformPipeClient.cs,PlatformCommandSequenceDispatcher.cs}`; tests `RawNumberedJoinTests.cs`, `RawNumberedJoinReaderTests.cs` and original Program catalog. Original reference/owner/task records and cross-layer spec/index links accompany the slice.

## Limitations

Generic raw discovery cannot prove that an unknown final part exists. Physical IME/DPI/provider/OS-close/hotkey/Shell interaction, human visual, installed/Release and full-workspace acceptance remain separate. TestPlatform and owned reader/native fixtures are not those acceptance levels. No product/reference/browser launch, packaging/install/deployment/system mutation, commit or push was performed. B3 is accepted at the [coordinator's stated boundaries](coordinator-acceptance.md); B4–B7 remain paused.
