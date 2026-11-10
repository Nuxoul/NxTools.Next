# Release 45 feedback repair

Parent: **Phase E — Format expansion and ZIP editing / in_progress**. Historical OPT scoped acceptance and the [original diagnosis](release45-drag-diagnosis.md) retain their attribution.

## Change boundary

The original drag window has explicit identity `拖出解压`, real per-member byte progress and concise status. The connected seam is worker `ProgressWriter` → retained `ArchiveEntrySession.OnVirtualRunner` → `ArchiveVirtualContent` → `ArchiveVirtualDragSession` → original public event → `ArchiveApplication.drag_progress_event` → `JobProgressView`.

Worker progress is bound to operation plus current demand request/index/original ordinal. Native publication retains session/generation, adds ordered presentation snapshots and distinguishes processed bytes, sealed bytes and receiver completion. Unknown member/aggregate totals stay unknown. Already sealed members are charged once; switching members resets only current progress.

Files in scope are the existing worker progress/demand modules, native retained-entry/content/drag owners, symmetric drag DTO, original Desktop drag/progress owner and their boundary tests. Root names use Windows UTF-16 component/path limits while the serialized command keeps its UTF-8 transport budget. Performance changes require same-workload stage measurements and preserve original reservations, token/sequence/ACK order, CRC/size/native publication and actual cleanup.

No replacement progress owner, codec, OLE loop, destination-path protocol, physical collect/move phase, source mutation, service/registry/deployment change or phase renumbering is included.

## UI selection

| Shape | Exact released API | Visible tier and parent adaptation |
| --- | --- | --- |
| Existing independent drag window, stacked current/overall groups | Component 0.8.0 `progress/progress.rs::Progress::{new,value,loading,accessibility_label}` via existing `progress_group` | Medium, 8 DIP; ordinary vertical layout, no Toolbar adaptation; original theme/reduced-motion policy |
| Caption and background/close | Original `WindowCaption` and official `Button` in `JobProgressView` | Existing 34 DIP caption, outlined footer action; no shared component or native budget change |

Original window lifecycle remains hidden creation → C# no-activate → final device-pixel bounds → show. Close/background only retires presentation; original source close/disconnect/cancel, one still-pressed/source-HWND/capture admission, dedicated STA/OLE and async target-held streams retain their owners. Cross-entity application updates remain deferred.

## Reference boundary

Read the saved Bandizip ZIP and output lifecycle traces before implementation. They distinguish 16 KiB codec capacity, conditional 32 KiB output staging and a separate queue/backpressure policy; these are static reconstructed paths, not the actual branch used by the supplied whole-archive drag.

Legacy `D:\Code\NxTools\Views\DownloadCompletedWindow.axaml.cs::DragFileSurface_PointerPressed/StartFileDragAsync` and `Services\WindowsFileDragService.cs::TryDragFile` retain pressed admission and one native `DoDragDrop`, but expose real-file CF_HDROP, not virtual progress. Next's current native session/content and hidden progress-window boundaries are the relevant retained owners.

## Verification

All executed checks below use the original owners. Cargo ran offline with one build job, incremental disabled and codegen-units256. C# native fixtures were effects-reviewed, exact LIST then identical RUN against an explicit matching worker, with no skipped case.

| Boundary | Executed result |
| --- | --- |
| Final worker / C# Debug host; optimized helper / C# Release measurement host | Builds pass; C#0 warnings/errors |
| Rust output unit boundary |7/7 pass: exact32KiB/tail, ACK sequence, metadata/source verification/commit drain, cancel/abort and sticky first failure |
| Actual-process binary worker |8/8 pass: negotiation, byte equality, pause/resume, cancel/EOF, malformed/stale ACK refusal |
| Worker extraction contracts |10/10 pass: CRC, source identity, readmission and no invalid commit |
| Worker progress tests |5/5 pass: initialized current demand and bounded original frames |
| Drag wire / IPC |13/13 protocol and3/3 IPC pass |
| Original Desktop serialized drag entry to rendered view |1/1 pass: title, independent bars, concise states, member switch, unknown totals, wrong/stale IDs/sequences, terminal/background lifecycle |
| Ordinary progress / preview / Root name |14/14 progress,4/4 preview lease and1/1 long-CJK Root-freeze pass |
| Native progress/async/name/preview |7 cases pass: actual slow consumer, native ABI, cancel/abort/disconnect, current bytes before Seal with wrong-demand injection, wire names, HGLOBAL descriptors, preview lease adoption |
| Native retained context |2/2 setup/lifecycle selections pass: descriptor-only and1/2/16 demands, one worker/source/catalog, first-ready slot retention and actual cleanup |
| Same-source optimized measurements |20 successful operations:208 files/206572163 bytes independently verified on each route; actual cleanup |
| Strict Clippy |Archive lib with isolated-codecs, protocol lib and Desktop binary pass with `-D warnings` |
| Source/document gates |spec-contracts, gpui-release-contracts, markdown-editor-contracts and3 Root-drag architecture checks pass; scoped rustfmt/diff whitespace and repair/evidence links pass |

The current-byte native regression detects the original missing nonzero snapshot before Seal; the original serialized UI regression detects missing dual bars/developer prose, and the output boundary detects uncoalesced writes. The long-CJK name regression passes through real Rust serialization and native HGLOBAL construction. Full evidence: [verification logs](../../../.diagnostics/archive-root-drag/20261104-r45-repair/verification-index.md), [native contract](release45-native-progress.md), [name contract](release45-name-repair.md), [performance measurements](release45-performance-repair.md).

One bounded read-only review of coalescer order/failure, demand identity and native/visible progress lifecycle found no concrete defect. These results establish automated and controlled-native repair evidence, not installed or visual acceptance.

## Outstanding acceptance

Physical GUI/DOpus/Bandizip execution, new reverse engineering and Release packaging/installation require separate scoped authorization. The original source is read-only; owned fixtures/output directories do not execute archive members. DOpus visible staging and destination filesystem topology remain separate findings. The unrelated ordinary Extract completion screenshot has not supplied a reproducible defect.

## Independent acceptance

[Independent verdict](release45-independent-acceptance.md): **partial**. R45-UI01/02/03 and R45-N01 pass the automated/controlled-native scope. **R45-AC01 (Medium)** blocks output-coalescing acceptance: the supported multi-token 7z failure-cleanup order flushes another member's pending tail, and a failed cleanup-triggered write suppresses the earlier token's explicit abort. Two compiled original-output-boundary assertions reproduce the finding. DOpus visible staging and physical/installed Release acceptance remain open.

### R45-AC01 implementation follow-up

[Directed repair](release45-ac01-repair.md) makes every abort discard pending extraction bytes and bypass the coalescer's sticky failure without clearing it. Three multi-token regressions fail on the original branch and pass on the repaired branch; output10/10, actual-process bulk8/8 and extraction10/10 pass. The native7z preservation selection remains failed at its low-output-budget ResourceLimit/IoFailure assertion, reproduced with both the retained pre-coalescing worker and final worker. R45-AC01's local regression is repaired; this does not replace the independent verdict or close full native/physical/Release acceptance.

### Native7z preservation follow-up — 2026-11-05

[Resource-limit repair](release45-sevenz-limit-repair.md) preserves native `source_acquire` resource_limit as Rust ResourceLimit instead of IoFailure. The actual-worker regression and original C# low-budget case both demonstrate RED/GREEN. Full native selection now reports LIST1/RUN1/PASS1/SKIP0, including all10cases and the previously blocked collision/plain-corrupt/encrypted-corrupt cleanup assertions. Output10, bulk8, extraction11 and7z readmission1 pass; R45-AC01 remains green. No budget/codec/native policy or performance strategy changes; independent reacceptance and physical/Release decisions remain separate.
