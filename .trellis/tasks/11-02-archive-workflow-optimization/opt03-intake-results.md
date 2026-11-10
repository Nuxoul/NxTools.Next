# OPT-03 — Complete Add intake and results

## Status

**Implemented and verified at the authorized automated/owned-native boundary.** G-12/G-13/G-21/G-22/G-23 and touched G-18/G-19 pass within that scope. Parent remains **Phase E — Format expansion and ZIP editing**. OPT-02 independent acceptance remains the namespace baseline; OPT-00/01, G-00 and complete G-04 remain open. OPT-04 onward is not started.

## Delivered behavior and owners

| Delivery | Original code owner | Result |
| --- | --- | --- |
| Editable Add sources | `forms.rs::modify_fields`, `forms_intake.rs` | Small outlined per-source Remove, first/middle/last/empty/re-add; pending/submitted states disable invalid actions. Local draft only, truthful count/unknown size, no render-time recursive scan. |
| Incoming choices | `application.rs::handle_drop`, `jobs.rs::open_create_paths` | Explicit Open-first, New-all and capability-gated Add. New retains every source in the original Create form. Add freezes original browser/source/snapshot/folder through asynchronous intake and final submission. |
| File Paste | `clipboard_requests.rs`, protocol `archive_clipboard.rs`, C# `ClipboardFileDropReader` and original EventWindow STA | HDROP-only, atomic bounded batch, borrowed-handle resource balance. Original pending registered before send and rechecked after defer. Text-focused Ctrl+V remains text paste; COPY/CUT gives no deletion authority. |
| Actual Add results | `worker_conflict/add.rs`, original ZIP/7z/TAR edit owners, `ArchiveWorkerRunner.ReadFrames`, `ArchiveJobSession`, `jobs.rs`, `browser.rs` | Actual added/overwritten/renamed names, <=256 display members/65536 path UTF-8 bytes plus full total. Skip/merge-only excluded; business operation stays complete. Matching published/committed/clean completion refreshes only its original context; names map to new snapshot IDs. Published late cancellation refreshes without success selection; unpublished/unknown/unclean outcomes do not. |
| Existing Create options | `forms_layout.rs`, `forms.rs`, `draft.rs`, existing seven writer routes | Seven formats retain their admitted options. Header encryption requires matching nonempty password; format switch clears unsupported split intent. Source/exclusion/root/secret/frozen-request behavior preserved. [Detailed mapping](opt03-create-options.md). |
| Complete each-input outputs | `create_output_plan.rs`, protocol `archive_create_destination.rs`, native `ArchiveCreateDestination.Observe`, original `submit_form_admitted` | Complete disk/requested/generated reservation before any job. Numbered compound names, Windows case comparison and split-part reservations. Normalized disk occupancy indexed once. Full/near-full queue rejects the whole inadmissible batch; no silent partial admission. Late target appearance uses original native confirmation. |
| Create location and post-Test | `ArchiveOutputSession.ResultLocation`, `ArchiveWorkerRunner.ExecuteAsync`, `jobs.rs::finish_attempt/job_terminal` | Capture actual committed ordinary/split primary before disposal. Test uses that confirmed location, not request concatenation. Create counts/commit state/journal/publication/location survive Test success/failure/cancel; cleanup remains conjunctive. Running disconnect retains saved Create facts and cleanup-held slot. |

Ordinary Create members, each-input output archives and Add members keep separate collision policies. Rust owns state/plans/layout, GPUI renders through existing entities, C# owns Windows/source/output/native publication. New observations and display paths grant no write authority. Diagnostics retain operation/job/attempt correlation and low-noise lifecycle states, never input paths/member names/clipboard contents/secrets.

## UI component ledger

| Surface | Released composition | Variant/size and parent adaptation |
| --- | --- | --- |
| Add/Create sources | Existing Form::vertical/Field and bounded rows; Button::new/label/outline/on_click | Small outlined Remove; direct layout, no Toolbar.child; stable Inputs, original scroll body and fixed Medium footer |
| Incoming choice | Original Root/Dialog/DialogFooter/DialogAction/DialogClose with Buttons | Medium outlined Open-first/New-all/Add; disabled Add reason; original focus/cancel/retirement |
| File Paste | Original source row, explicit Button and guarded Ctrl+V | Medium outline; deferred cross-entity application, native reader remains EventWindow-owned |
| Add result summary | Original browser install/selection plus Alert::info/with_size(Small) | Direct flex-none parent; Ready-only total/subset; fresh row IDs, no alternate selection controller |
| Locate | Original progress/result actions | Existing terminal/publication/cleanup eligibility; Shell failure cannot rewrite archive publication |

Kit0.8.0/pre0.3.9 remains pinned. Production TestWindow controls are exercised through original callbacks, including scrolling body controls into view. This is not physical visual acceptance.

## Reference and lifecycle invariants

Saved Bandizip input/Create/lifecycle reports and raw records are the behavioral reference: `0x140148b20` retains the full list and dispatches only accepted Open/New/Add; `0x14013d910` checks disk/planned names before tasks; `0x1401b2450` gates reopen/Test on writer success. Saved `0x1400f6d10` and Ark `0x18001fd80` establish format/source/options normalization. No Bandizip execution, new reverse engineering or proprietary code reuse.

The indexed Next chain preserves constructor → centered WindowOptions → single Root → original close/retirement. All cross-entity writes defer, including immediate results. Picker/Paste/output observations retain original session/generation/connection; Add additionally retains source/snapshot/folder. Closing, navigation, refresh or late receipt cannot retarget work. Missing location is unknown; unknown cleanup never releases a queue slot. [Reference chain](../../../docs/reference-code/archive.md) and [executable contracts](../../spec/nxtools-desktop/backend/archive-batches.md).

## Regression evidence

| Boundary | Recorded negative evidence | Passing assertion |
| --- | --- | --- |
| Source/input controls | Missing real Remove/New-all/Paste controls; old receipt-after-disconnect appended sources | Real buttons, complete New list, atomic Paste, retirement and text focus |
| Frozen Add target | Picker navigation appended stale source; final submit did not settle stale folder | Original picker/result/submit refuses stale folder/snapshot/connection |
| Actual Add display | Frozen OPT-02 worker emitted no `add_result`; old browser reselected skipped entry | Actual final outcomes, fresh snapshot IDs, bounded display/full catalog |
| C# private frame | Actual native Add returned `protocol_violation` when `add_result` was absent from the frame whitelist | Original runner/Job emits successful clean private results |
| Published Add late cancel | Original Job terminal produced zero refresh requests for confirmed publication | Refresh once, retain cancelled outcome, no invented success selection |
| Create post-Test | Request-derived location differed from actual native primary; Test overwrote publication | Native location and durable Create facts retained through all tested outcomes |
| Create options/naming | Old-behavior comparison: enabled header encryption without password, retained unsupported split value, same-name draft refused before observation | Production control/draft/submission regressions pass. This is an explicit old-behavior sensitivity comparison, not a claim of earlier test-first execution. |
| Large supported destination | 8193-name numbered-collision fixture exceeded its 5-second application-turn threshold | Preindexed namespace passes the same bound and selects `report (8194).gz` |

Negative and positive logs are retained in this session scratch under `opt03-*-red.log`, `opt03-create-old-behavior-{red,green}.log`, `opt03-add-late-cancel-{red,green}.log` and the verification logs below. Compiler/fixture setup failures are not behavioral RED evidence.

## Executed verification

Test groups overlap; counts below are not additive.

| Check | Result | Evidence level |
| --- | --- | --- |
| Original Desktop Job suite | 107/107 | Original queue/forms/notification/terminal, AC-01/02/03, non-Add refresh, post-Test and all15 output-plan tests |
| Original forms / drafts / progress feedback | 41/41, 12/12, 13/13 | Real Kit TestWindow controls and original draft/progress owners |
| OPT03 focused entry suite | 16/16, followed by Add subset8/8 including the additional late-cancel test | Controlled original notifications/read installation, not native publication |
| Protocol / IPC / CLI archive selections | 50/50, 17/17, CLI3 unit+4 integration | Strict optional DTOs, bounded routing, original cleanup/publication preservation |
| Add actual worker contract | 1 test,35 scenarios across ZIP/7z/TAR/TGZ/TXZ | Actual worker, controlled native replies, independent Python/7-Zip candidate decoding; each format includes301 added members/256 display subset |
| Create actual writer contract | 2/2;9 admitted configurations across7 formats plus illegal combinations | Actual writer/readback, signatures/options/volume checks; controlled native replies |
| Native OPT03 exact LIST/RUN | 5/5, zero skips |21 Add scenarios; native ordinary/split locations, late cancel, precommit failure and post-observation late collision; owned HGLOBAL and held destination enumeration; strict result DTO |
| Native Create independent interoperability |8 exact selections, all pass/zero skips;43 created outputs | ZIP levels/roots/AES;7z levels×solid×encryption;TAR/TGZ/TXZ/GZ/XZ; independent .NET/Python/stock7-Zip decoding |
| OPT-02 native preservation |3 exact selections,79 scenarios, zero skips | Accepted namespace/conflict/encryption/no-change/single-publication behavior retained |
| Strict Clippy |5 related packages, all targets, `-D warnings` passed | Desktop/protocol/IPC/CLI/archive with isolated codecs |
| Documentation/dependency gates |120 specs/14 owners; catalog/release/editor all pass | Source/link/provenance only |

Final native worker: `opt03-final-worker\NxTools.ArchiveWorker.exe`, SHA256 `E3DFF478BA9CBED906314E345685EECDAA4C1A4852F746D7019061D62F5487AE`. Matching identity is required by existing native fixtures. Earlier Create interoperability/OPT-02 preservation used `9B4B10B5D032C6F9A6172CB903F9F0A25A37D6E262AB56C971D7E3EAD4130B47`; the subsequent worker delta is formatting and a private ZIP addition-plan type alias, with final worker/native OPT03 rerun passing.

C# final build: zero warnings/errors. Actual native Create exports `ordinary.zip.json`, `split.zip.json`, `split.7z.json` are consumed through `NXTOOLS_OPT03_CREATE_WIRE`; absent exports fail rather than synthesize facts. Ordinary/split ZIP primary is `.zip`; split7z primary is `.001`. Full native publication and cleanup remain distinct from Desktop replay.

Primary logs: `opt03-worker-final.log`, `opt03-native-{list,run}-final.log`, `opt03-native-preservation-{list,run}.log`, `opt03-desktop-final.log`, `opt03-create-old-behavior-green.log`, `opt03-wire-cli-preservation.log`, `opt03-clippy-final.log`, `opt03-static-gates.log`.

## Limitations and stop point

Physical clipboard/picker/Explorer/product GUI, IME/DPI/accessibility and user visual acceptance require separate authorization. No packaging/signing/install, user archive mutation, source recycling/HZ5, service/registry action or commit/push was performed. No general performance speedup is claimed from the bounded naming regression. Native observations are advisory, and split publication remains per-volume rather than family-atomic.

G-00, OPT-00/01 overall closeout, complete G-04 and Phase E/Release acceptance remain open. Unified batch window and system multi-folder selection remain required later work. OPT-03 stops here; OPT-04 through OPT-07 are not started.
