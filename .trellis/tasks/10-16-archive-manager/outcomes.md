# Archive manager — implementation and historical research outcomes

## Historical documentation-only checkpoint (superseded authorization, preserved evidence)

Documentation only: [handoff](README.md), [PRD](prd.md), [technical design](design.md), [UI specification](ui-spec.md), [phase plan](implement.md), [56 acceptance scenarios](acceptance.md), curated implement/check JSONL context, retained reference screenshot, and [durable reference note](../../../docs/reference-code/archive.md) with its catalog entry.

At the original documentation checkpoint no product implementation/dependency/build/native operation was performed; task status was planning and implementation_authorized=false. Those are historical facts, superseded by the current approval below.

## Research actually performed

- Read installed Bandizip7.32 resources, language inventory, EULA and version/format strings at `D:\Tool\Bandizip`; invoked `bz.exe` help. Invalid `help c/x` forms returned parameter-parsing output, not a successful operation test. No compression/extraction interoperability or interactive Bandizip lifecycle was exercised.
- Source-reviewed Archiver and SupaZip through independent read-only surveys, with detailed path/type evidence condensed into the reference note. Their GUI frameworks are Slint and egui; benchmark figures are reference-reported, not rerun here.
- Read local project architecture/config/native/UI contracts and inspected locked Kit0.7.1 public sources for the selected compounds. This is not compiled integration or product visual acceptance.
- Retained the user's image at `assets/bandizip-layout-reference.png`; verified SHA256 `da5f63eacb75b9ce508d39671e2d4d4180d33d5fd40958b740634545d5ab9488` matches the original.
- No Bandizip decompilation/disassembly or proprietary code reuse. EULA wording and legal exceptions are recorded without inventing an absolute legal determination.

## Verification

Initial document pass:
- `python tests/architecture/spec-contracts.py`: PASS,94 documents/13 owners; links/structure/evidence only.
- `python tests/architecture/gpui-component-catalog.py`: PASS,79 pages/79 rows/78 locked entries; local catalog only.
- `python .trellis/scripts/task.py validate .trellis/tasks/10-16-archive-manager`: PASS, initially warned about oversized injected files. UI ledger split into ui-spec.md and full-read links retained; current manifests contain13 implement and10 check entries.
- Local handoff consistency probe:56 unique case IDs,6 matching phase titles, valid JSON/JSONL/source paths and image hash. Found the then-not-yet-written outcomes.md link; this file completes it.
Final rerun: all checks exited0. Handoff probe PASS (56unique cases,6consistent phases,44valid Markdown file links,no placeholders/trailing whitespace,exact screenshot hash); spec-contracts PASS94documents/13owners; component-catalog PASS79pages/79rows/78locked entries; task context validation PASS13implement/10check with **no truncation warnings**; scoped `git diff --check` PASS. All injected files are below32768bytes. These gates do not certify feature behavior, native extraction safety, compiled compatibility or user acceptance.

Verification scope is documentation consistency only; future implementation acceptance remains unexecuted.

## Current implementation authorization

User authorizes product development and isolated verification for canonical A–F, including RAR core in A, required splits/ZIP edits in E and genuine native drag in F. Navigation, dimensions/defaults/technical direction are approved; no repeated scope confirmation. Necessary dependency additions still need exact capability/license/Windows build/resource evidence. Release/install/real registry/default-association/service actions, commits and pushes remain restricted.

### Remaining technical capability gates
- Codec exact release/features/license/resource/cancellation and independent format compatibility.
- DataTable accessible multi-select public-interface proof or approved narrow fallback.
- Native reparse-safe output/commit and real delayed-materialization OLE drag.
- Legacy NxTools interaction search before asserting no migration behavior exists.

Historical not-run list: product tests/build/lint, benchmarks/GPUI renders/physical/DPI/Shell/UIA/user/Release acceptance. Current actual execution is recorded separately below; do not infer PASS from historical documents.

## Implementation checkpoint — approval synchronized

- Read all task documents, AGENTS/shared architecture/engineering/verification, owner indexes and complete mandatory UI selection/table/index/authority.
- Preserved the pre-existing dirty workspace; baseline path status saved outside the project in session scratch.
- Scope/phase/requirements/acceptance/metadata now follow latest approval. Phase A is in progress; no phase exit or product completion claimed.
- Change boundary: new host-neutral engine crate for typed metadata, bounded codecs/path/encoding/control; native no-reparse/identity/commit through C#. No reuse or relaxation of clipboard/extension ZIP policy. Later UI/native layers follow canonical dependencies.
- Product invariants: independent activating/keyed browser; deferred generation-scoped results; owned temporary/output lifetime; no unsafe automatic extraction/deletion; one still-pressed C# OLE loop. No platform command reader/shared-event thread blocking.
- This entry records approval/document review only, not compiled/native/visual acceptance.

## Current partial Phase A implementation (2026-10-17)

See [exact checkpoint](phase-a-checkpoint.md) for sources, commands, counts and remaining gates. Latest Rust **30/30**, strict Clippy with isolated-codecs, workspace check and fmt pass. C# build0warnings/0errors; precise safe subset **16/16,0skip** passes, including actual C# Job→Rust→native ZIP/RAR4/5/7z extraction. New7z/RAR tests were red (UnsupportedFormat/UnsupportedOperation) before implementation; each now runs9success/failure operations. Plain literals/solid hashes were independently verified using bsdtar. Eight independent archives×open/test=16codec operations also pass. Earlier extension ZIP7/7 and clipboard5/5 remain historical passing regressions; policies/dependencies were unchanged here.

Approved C# worker architecture now has fixed helper/suspended Job admission/memory/process caps/private frames/own-Job cancellation/actual cleanup. Exact sevenz-rust2=0.23.0 and unrar_sys=0.5.8 Windows builds passed; notices/full UnRAR license/fixture hashes retained. Rust owns codecs; output is C# held-directory/no-reparse/exclusive partial/no-replace. RAR_TEST callback receives no native destination.7z/RAR partials commit after full decode/integrity/source recheck. No RAR creation or unrestricted unpack fallback.

**Blocking failure:** identity-bound atomic overwrite independently reproduced **1run/1fail,exit1,replace_file:32**. It remains registered; full/default C# suite is **not green**. NTFS probe isolates protected target's lack of FILE_SHARE_DELETE as the conflict; allowing delete sharing or releasing target reopens a replacement race. No unsafe fallback implemented. [Overwrite gate](research/native-overwrite-gate.md) records evidence and proposed backup→no-replace publish→restore. Alternative is non-atomic (brief name gap), **not approved/implemented**, and requires a new narrow decision.

Phase A remains incomplete: additional formats/encryption/create/source traversal, metadata/backpressure/diagnostics/native attrs/MOTW/recycle/resources/performance and remaining safety cases are open. Desktop/PlatformHost IPC/package mapping unintegrated; fixture helper copies removed. B–F remain first-release obligations, not omitted or started out of order. No screenshot/physical/native Shell/DPI/UIA/user visual/Release acceptance claimed.

Earlier editing-stop count was corrected after refreshed grounded edits; no current editing blocker/bypass. No owned build/worker leftovers observed. No Release/package/install, real registry/default-association/service operation, commit or push performed.

Final documentation/architecture checks all exit0: spec-contracts100documents/14owners; GPUI catalog79pages/79rows/78locked entries; task context15implement/12check, no truncation warning; PlatformHost fixture-selection source gate; scoped git diff --check (pre-existing Program.cs CRLF→LF warning only). These are source/document gates, not product/native/visual acceptance.
