# Compression / archive manager — handoff index

**Authorized deliverable: product implementation and isolated verification, covering Phases A–F.**
The latest user approval supersedes the earlier documentation-only/MVP/P2 proposal. Product labels are Chinese; engineering documents are English.
Do not declare the first release complete at A–D. Preserve exact phase names/order and record actual evidence.

## Runtime feedback — 2026-10-30

[Current runtime correction](runtime-feedback-2026-10-30.md) records current-result availability,
Ctrl-only indexed Clipboard paste, initially absent/on-demand archive preview, independent preview
preparation resources and complete directory drag without tiny business caps. Related Desktop and
controlled native lifecycle checks are recorded separately from live-service/physical acceptance.
This later behavior supersedes earlier always-allocated preview and item-count cap wording.

## Release38 drag / automatic preview repair — 2026-10-30

[Current repair](release38-drag-preview-repair.md) defines complete ordinal-preserving drag,
stage-owned preview budgets, automatic preview without historical opt-out, menu order and cleanup.
[Installed Release38](installer38-code5-repair-outcomes.md) is already upgraded; this repair does
not authorize packaging, installation, installed-product launch or HZ5. Historical optional-preview
wording is superseded by the current user-approved automatic behavior.

## Current task replanning — 2026-10-28

Read [continuation plan](continuation-plan.md) first for the consolidated user feedback,
non-ZIP edit delivery, HZ2progress/HZ3batch/HZ4crash/HZ5source-reclaim requirements,
ordered C0–C8 slices and acceptance gates. This is task-file planning, not a prompt to
another session or product implementation authorization. Root PRD/design/implement/acceptance
link this amendment; canonical A–F history remains. HZ1 AC01–03 stays independently accepted.
User decisions remain explicitly pending; HZ5 is included but default-off/destructive and gated.
Only [current checkpoint](phase-a-checkpoint.md) owns execution status, not historical summaries.
Use the [pinned local edit reference](../../../docs/reference-code/archive-editing.md) and
HatchZip focused notes; old0.7.1/preview-optional/ZIP-only/no-manager descriptions below are
historical where later approved contracts or this explicitly planned scope supersede them.

## Read in this order

| File | Purpose |
| --- | --- |
| [Reference investigation](../../../docs/reference-code/archive.md) | Bandizip7.32 menu/dialog/settings inventory; local open-source source maps, licenses, pitfalls and evidence limits |
| [prd.md](prd.md) | Approved first-release requirements, required/optional/excluded capabilities, browser and operations |
| [design.md](design.md), [ui-spec.md](ui-spec.md) | Ownership/data/settings/safety plus exact Kit0.7.1 component ledger, window/form layouts, validation and UI acceptance |
| [implement.md](implement.md) | Canonical Phase A–F titles/order, prerequisites, work and exit gates |
| [acceptance.md](acceptance.md) |56 named future acceptance scenarios, expected observable behavior and separate evidence levels |
| [assets/bandizip-layout-reference.png](assets/bandizip-layout-reference.png) | User's exact retained layout reference; SHA256 `da5f63eacb75b9ce508d39671e2d4d4180d33d5fd40958b740634545d5ab9488` |
| implement.jsonl / check.jsonl | Curated spec/research context for the next AI; not executable code |

Full current UI authority and exact API index are intentionally linked rather than injected as truncated files: read [.trellis/spec/shared/ui-design.md](../../spec/shared/ui-design.md) and [locked Kit index](../../../docs/reference-code/gpui-kit-component-index.md) explicitly before UI changes, including the later amendments. Injection is navigation, not a substitute for full applicable-source review.

## Current source continuation (2026-10-27)

[Single current checkpoint](phase-a-checkpoint.md) and [icons/windows/preferences/Tree/tabs evidence](window-tabs-icons-drag-outcomes.md) record the latest approved implementation. Browser presentation now uses one official tab host; old per-archive HWND wording below is historical. Prior failures, production-lint/global-fmt limits and unrun native/user acceptance remain explicit. No package/install/system mutation authority.

## HatchZip follow-up research (2026-10-27)

[Five-capability research handoff](hatchzip-research-handoff.md) and [reference index](../../../docs/reference-code/hatchzip.md) cover space preflight, progress, batch summaries, real-process crash tests and destructive in-place reclamation. This is source/docs evidence only, not implementation or authorization to mutate source archives. Read the dedicated safety/format/ownership gates before future work; current product status remains solely in phase-a-checkpoint.md.

## User-confirmed direction

- A first-class **压缩** category with related settings in NxTools.Next.
- Opening an archive shows a dedicated archive window, not merely a settings page or instant extraction.
- Window structure follows the supplied Bandizip screenshot: menu/actions, folder tree, details list, optional comment, status.
- UI uses real released GPUI Kit components and local component-selection/theme/native rules.
- Research includes `D:\Tool\Bandizip`, `D:\Code\Archiver-main` and `D:\Code\supazip-master`.

## Research summary

Bandizip resources expose browsing, extract shortcuts/smart destination, creation levels/encryption/split/solid options, overwrite/password dialogs, ZIP editing/comments, file associations/context menus, filename code pages, previews, temp-file editing, security/MOTW, testing and advanced tools. The reference note inventories these without declaring every resource command supported by the installed edition.

Archiver supplies useful streaming/progress/raw-copy ZIP/virtual-folder patterns, but its GUI is **Slint**. SupaZip supplies typed-error/limits/format-registry examples, but its GUI is **egui**, with source gaps relative to its docs. Neither is a GPUI UI implementation to copy. Both require adapting safety, overwrite, encoding and lifecycle contracts rather than importing them wholesale.

No Bandizip decompilation or disassembly was performed. The installed EULA restricts it except where applicable law permits; public resources and licensed source were sufficient. No proprietary binary/code/skin/icon dependency or redistribution is proposed. `bz.exe` was invoked for help only; no user archives were modified.

## Approved stages — implementation authorized

1. **Phase A — Engine contracts and safety foundation**
2. **Phase B — Compression module and settings**
3. **Phase C — Archive browser and entry lifecycle**
4. **Phase D — End-to-end operations and desktop launch intents**
5. **Phase E — Format expansion and ZIP editing**
6. **Phase F — Explorer integration and genuine native drag**

First release requires **A–F**, not the former A–D MVP. RAR4/RAR5 browse/extract/test belongs to Phase A; RAR creation is excluded. Required split reads/creation and ZIP edits remain in E; genuine native drag remains in F. No phase is renumbered or skipped.

### Latest user approval

- ZIP add/delete/rename/new-folder/comment, real split formats and true multi-item Explorer drag-out are required before first-release completion.
- Navigation is **下载 → 速启 → 剪贴板 → 扩展 → 压缩 → 其他工具**.
- Browser960×640/min720×480, documented defaults and technical direction are approved.
- Necessary compression dependencies may be added after exact version/features/license/Windows build/resource verification. No arbitrary upgrades; GPUI Kit remains exactly0.7.1.
- Source changes and isolated fixtures are authorized. Release packaging, installation, real registry/default-association/service operations, commits and pushes require separate authorization.

### Verified technical constraints for the next AI

- Locked Kit0.7.1 DataTable has **single-row** TableSelection. Multi-select needs a production-component fixture/public-seam evaluation; a HashSet or extra paint alone does not supply keyboard/UIA semantics. Any narrow fallback needs approval.
- Toolbar.child(Button) converts it to ghost/compact. Use official Toolbar.content composition for visible outlined header commands.
- TreeEvent exposes expansion/collapse, not a selected-change event. Selection sync must use the real observed owner and defer writeback.
- Native archive drag cannot start after asynchronous extraction and mouse-up. Required Phase F drag enters one C# delayed-materialization OLE loop from a still-pressed source-HWND gesture; explicit extraction is not a substitute.
- Existing platform pipe server is Rust-owned, with C# as native client. Archive launch intents need a bounded authenticated Rust route before the current second-launch summon return.
- Destination safety must handle existing/replaced reparse parents, not merely reject `../`. Cancellation cleans owned partial/new outputs, never claims full rollback of replaced user files.
- Config defaults apply to absent fields in valid legacy config, not malformed JSON. Persistence and observed native integration application are separate outcomes.

## Execution instructions

Read the linked task documents, reference note and applicable owner specs before code. Continue directly from Phase A under the approval above; do not ask again about approved scope/defaults/navigation/dimensions. Verify codec and Kit capabilities with bounded real fixtures. A narrow custom multi-select fallback still needs specific approval if the public DataTable seam proves insufficient. Preserve dirty changes, native ownership and deferred cross-entity/generation boundaries. Record red-capable tests and actual compiled/native/visual evidence. Do not perform the separately restricted operations above.

## Verification status

The earlier documentation-only results remain historical in [outcomes.md](outcomes.md). Current implementation progress and remaining gates are recorded there separately; documentation checks do not certify product/native/user acceptance.

## Current implementation checkpoint

Latest explicit amendment starts **Phase F — Explorer integration and genuine native drag** spike only.
[Current checkpoint](phase-a-checkpoint.md) and [Phase F spike results](phase-f-spike-outcomes.md) own
current source/controlled evidence, real OLE NOT RUN/time-out and P207 authority gate. Production
archive-row drag stays disconnected until independent spike acceptance; no actual Registry/Ffull/
first-release approval. Earlier stage summaries below are retained history.

Latest Phase E continuation: [single current checkpoint](phase-a-checkpoint.md) and
[authorized delivery outcomes](phase-e-delivery-outcomes.md) supersede historical stage/status
paragraphs below. Current UI authority is0.8.0/defaultpre0.3.9; old0.7.1notes are historical.
P202/P203 source/isolated paths are delivered; integrated gates and independent/live acceptance
remain pending. No automatic Phase F or first-release completion.

Latest explicit dependency amendment starts **Phase B — Compression module and settings** while
Phase A remains incomplete. [Phase B handoff](phase-b-settings-outcomes.md) records the delivered
settings/navigation, actual checks and outstanding live visual/production-lint limitations.
Older “No Phase B” statements below are historical, not the current stage authorization.

[Single current checkpoint](phase-a-checkpoint.md) alone owns current integration status. [Tree creation](tree-create-evidence.md) records7z/TAR/TGZ/TXZ continuation; [ZIP output preparation](zip-create-output-preparation.md) remains the saved green checkpoint. Independent [side-effect failure](zip-create-remediation-acceptance.md), [original creation rejection](zip-create-acceptance-review.md) and historical remediation/evidence remain retained,not overwritten PASS. The [matrix](phase-a-capability-matrix.md) links format evidence. Phase A remains incomplete; no B/release/native-user acceptance. Do not infer current counts or next work from older narrative checkpoints.

Current bounded read-only product chain: [contracts](product-read-contract.md),
[wire](product-read-wire.md), [historical integration](product-read-evidence.md), preserved
[independent findings](product-read-acceptance.md) and [P2 repair evidence](product-read-p2-remediation.md).
Older7z [typed-I/O repair](sevenz-create-typed-io-remediation.md) and preserved independent
findings remain historical; only the single checkpoint owns latest counts. No Phase B.

Bounded resources: [pre-run contract](resource-acceptance-contract.md),
[actual Debug evidence/limits/red-green](resource-acceptance-evidence.md),
[raw attempts/PIDs/exact names](resource-acceptance-results.json).100k engine/private-worker
capability is distinct from runner8MiB refusal and product4096/4MiB ResourceLimit;firstpaint
unmeasured. Unique checkpoint above owns current verification/blockers,not historical counts.

Resource-tool correction: [minimal repair contract](resource-tool-repair-contract.md) owns only
safe-stop, required-sample validity and dispatch-time-origin changes. The independent
[resource review](resource-acceptance-review.md) and all original results remain immutable;
new evidence is additive and the single checkpoint alone reports current verification.

Retained-output summary P2: [independent finding](resource-tool-repair-acceptance.md) is immutable;
[Python-only repair evidence](resource-summary-p2-repair.md) records bounded line diagnostics,
independent summary completeness, preserved attempts/facts and actual red/green. Original three
tool corrections and C#132/132 are prior independent evidence, not rerun by this Python repair.
The historical whole-process exit cause remains unknown; Phase A remains in progress.

Explicit finite product100k readonly transport: [budget/compatibility contract](product-100k-contract.md),
[actual red-green/Debug evidence](product-100k-evidence.md), [all-attempt results](product-100k-results.json).
Version1/default ceilings and historical refusals remain unchanged; version2 is explicit per-operation
open/test only. First stoppedattempt and one declaredrepairrepeat are retained. Current counts and
remainingPhaseAexits belong only to phase-a-checkpoint.md. NoUI/FIFO/write/recycle/deployment/PhaseB.

[Independent product100k acceptance](product-100k-acceptance.md): primary100kreadcorrectnessPASS,
but fullobservationFailed/Stopped on statusPermissionError and follow-upreuseNOTverified.
Current Python-onlystatusreadP2 [contract](status-read-p2-contract.md)/[repair evidence](status-read-p2-evidence.md)
records finite native32/33recovery and failclosed stale/invalid/errorbehavior. No newlargeobservation
or resourceacceptance implied; uniquecheckpoint above owns currentstatus. Historicalreports/results unchanged.

Separately authorized ONE [post-status-repair100k observation](product-100k-post-status-observation.md)
and [new result ledger](product-100k-post-status-observation-results.json) were subsequently
[independently accepted PASS](product-100k-post-status-acceptance.md) for explicit finite100k
correctness, Debug observation and original-connection postcleanup reuse. Original observations
and historical Failed/Stopped remain unchanged; the old contender is still unknown. No repeat.

Windows local long-path continuation: [bounded contract](windows-long-path-contract.md) records
ordinary public paths versus internal native namespaces, actual product open/test and ZIP
native extraction/creation/publication regressions. Only the unique checkpoint owns current
verification and acceptance state. No Phase A/ENG08/A–F, UNC, UI, recycle or deployment signoff.

[Windows long-path independent acceptance](windows-long-path-acceptance.md) is PASS for its localZIP
scope; no complete ENG08/PhaseA claim. Current continuation is the
[real worker cancellation boundary contract](worker-cancel-contract.md) and
[implementation evidence](worker-cancel-evidence.md): distinguish original send/receive/workobserve/
process exit/nativecleanup, preserve originalbusy/leases/publication and finiteJob strategy.
Only the unique checkpoint owns currentcounts/acceptance. No nextfeature or PhaseB.

## Phase A exit convergence (current)

[Exit ledger](phase-a-exit-ledger.md) maps all original ENG/SAFE/P201/touchedIPC01 subconditions,
accepted evidence, finite function/evidence gaps and unresolved phase/native decisions.
[Worker cancellation independent PASS](worker-cancel-acceptance.md) joins the accepted100k and
localZIP long-path slices; none completes full ENG08/09 or Phase A. Sole next implementation
recommendation: ZIP bounded Auto filename candidates and safety re-projection,not started here.
Do not use historical “remaining work” paragraphs to reopen closed slices or add broad matrices.
