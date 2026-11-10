# OPT-07 — Required batch and browser workflow delivery

## Scope and current status

OPT-07 is [independently accepted at its authorized automated/owned-native boundary after directed repair](opt07-ac01-ac03-independent-acceptance.md); O7-AC01/02/03 are closed. The [original partial verdict](opt07-independent-acceptance.md) and [repair verification](opt07-ac01-ac03-repair.md) retain their historical scope. Overall stays `in_progress`, **Phase E — Format expansion and ZIP editing**. OPT-02/03/04/05/06 acceptance and closed O5-AC01/02 remain intact. G-00, OPT-00/01 overall, complete G-04, physical/Release and source-recycling prerequisites remain open. This is the final named implementation package, not overall closeout.

Authority: [task brief](prd.md), [design](design.md), [ordered scope](implement.md#opt-07--required-batch-and-browser-workflow-delivery), [requirements map](requirements-map.md), [acceptance](acceptance.md) and [parent checkpoint](../10-16-archive-manager/phase-a-checkpoint.md). [Verification](opt07-verification.md) records exact scopes and evidence; [45-row reconciliation](opt07-coverage.md) preserves every required row. Historical independent reports and measurements retain their own attribution.

## Delivered boundaries

| Required behavior | Actual owning seam | Delivered contract |
| --- | --- | --- |
| One batch progress / decisions | Original Jobs/submit_batch/show_job/JobProgressView | Membership before presentation, one original HWND; independent simultaneous tickets/details; same two-running/FIFO and Stop confirmation |
| Explicit password reuse | Original prompt/reply and C# pipe-session BatchSecret | Unchecked per prompt, SAME_PROCESS protected memory; first eligible generation1 Test/Extract prompt only; exact ticket/batch isolation, wrong invalidation and whole-batch retry/retirement |
| Qualified single Retry | Original observe_job/retry_facts/retry_job/ArchiveSpace | Immutable preterminal baseline, affirmative zero publication/actual cleanup and explicit fresh original receipt/attempt/session/connection recheck |
| Warning Locate | Held output -> Run/Job terminal -> job_location | Manual SucceededWithWarnings with actual native location/cleanup; automatic Succeeded-only; pending native receipt retains original Job |
| Departed-child Up | BrowserModel::up/Command::Up | Parent projection then one exact child selection/focus/reveal; root no-op |
| Archive-wide Find | Original browser Root/catalog/query/action owner | Separate Ctrl+Shift+F/View menu; existing Ctrl+F unchanged; original RowId/noncontiguous ordinal, query-generation pointer guard, original actions/final submits |
| New/Add multi-folder | Original native owner-bound STA -> typed pending -> form | Genuine IFileOpenDialog multiselect source adapter; Destination single; atomic combined count/UTF-8/overlap checks and frozen Add context |
| Browser/preview | Original model/projection/read/preview/cache owners | Refresh/encoding folder or nearest ancestor; deferred Tree reads latest model; transition-cached readiness/visibility, bounded rendering and original lease/generation lifetimes |

Rust owns state/layout/policy, GPUI renders, C# owns Windows APIs. No scheduler/runtime/global cache/dependency/vendor/theme/codec/backend upgrade or raised concurrency/resource/deadline is introduced. Source recycling remains unavailable under its separate native prerequisite.

## Reference and lifecycle invariants

[Archive intake](../../../docs/reference-code/archive.md#opt-07-required-batch-and-browser-workflow), [batch reference](../../../docs/reference-code/archive-batch-workflow.md), [navigation/Find reference](../../../docs/reference-code/archive-browser-workflows.md) and [picker reference](../../../docs/reference-code/archive-picker.md) record inspected saved Bandizip constructor/base/native show/close and action chains. They are static reconstructed pseudocode plus assembly/callees, not reference execution or reusable proprietary source. No indexed old NxTools general archive owner is available; package/FFmpeg ZIP uses are not one.

Saved Find chain: modal0x14013b940 -> loaded-catalog0x140111260 -> original-index0x140112c50 -> original extract/edit or tree/list navigation. Backspace0x1401574b0 -> parent projection ->0x1401a73c0 selects/focuses/ensures visible one departed child. Encoding reopens/restores location or surviving ancestor. Next keeps stronger generation/lease guards and original two-running/FIFO rather than the reference serial scheduler.

Next lifecycle: ArchiveApplication::open_canonical -> sole BrowserHost HWND/Root -> original source/read/generation -> deferred ArchiveBrowser::install. Tab/close/disconnect retire original session authority. Pointer/keyboard selection share original model/preview fingerprint -> deferred preview_entry -> materialization/Seal/visible consumer lease. Find query only reads loaded metadata; explicit action/final submission rechecks current original snapshot/connection. Internal preview never executes members; Open/OpenWith/nested-open retain original confirmation/capability/temp leases. Transferred consumers and encrypted-drag grants remain independently owned.

## Released UI component ledger

Kit/Base/Component/Assets0.8.0 and default gpui-pre0.3.9, current product palette/fonts and existing Root/window are retained.

| Requested shape | Exact released API/source | Variant/size / parent adaptation |
| --- | --- | --- |
| Batch summary/details | Original JobProgressView; stateless table::{Table,TableHeader,TableBody,TableRow,TableHead,TableCell}; progress::Progress | Default560x260/min480x260, one bounded scroll body/fixed footer; direct Medium32 outlined Buttons |
| Password decision/reuse | Root Dialog, Form::vertical, Field, stable masked Input; Checkbox::new().checked().on_click() | Default-unchecked current ticket consent; original focus/clearing owner |
| Whole-archive Find | Dialog::{child,title,footer,on_close}, DialogFooter; Form/Field/InputGroup/stable InputState; official ListItem in uniform_list |720wide query surface,32px bounded virtual rows; Medium outlined result actions in fixed footer |
| Toolbar/menu/picker | Original Toolbar.content, PopupMenuItem and Button | Recognizable official boundaries; no Toolbar.child ghost/compact adaptation |

## Executable contracts and evidence

- [Batch/consent/Retry/Locate](../../spec/nxtools-desktop/backend/archive-batches.md#scenario-opt-07-unified-surface-and-protected-consent).
- [Find/browser/atomic intake](../../spec/nxtools-desktop/backend/archive-browser-workflows.md).
- [Protected-memory/native picker](../../spec/platformhost/archive-workflows.md).
- [Additive Job wire](../../spec/nxtools-protocol/backend/directory-structure.md#scenario-batch-password-wire) and [connection-scoped capability](../../spec/nxtools-ipc/backend/directory-structure.md#scenario-connection-scoped-multi-folder-capability).

Actual production callbacks/serialized events protect the boundaries. Current verification distinguishes controlled password/picker14, actual native5, original saved-wire replay and GPUI/model work counts. Low-frequency diagnostics retain original correlation/categories/counts/elapsed/native codes without queries/paths/member names/passwords/payload/raw Debug. Full test/evidence identities and remaining gates are in [verification](opt07-verification.md).

## Known remaining gates

- [Directed independent acceptance](opt07-ac01-ac03-independent-acceptance.md) closes O7-AC01/02/03. Current Desktop61/61, strict lint/format/Debug and source/spec gates pass; earlier native/preservation results retain original attribution. No directed-repair blocker remains within the accepted automated/owned-native boundary.
- G-18 real GUI/Explorer/focus/keyboard/Light-Dark/default-minimum/DPI and real G-28 system multi-folder interaction require separate authorization and acceptance. Controlled native adapters/GPUI bounds are not that evidence.
- G-29 requires original-object production recycle capability and continuous source lifetime through Create and required post-Test. Production remains unavailable; no reopened-path authority, sharing relaxation or permanent-delete fallback.
- G-00, OPT-00/01 overall, complete G-04, overall Phase E and physical/Release remain open. Locked format dependency/replay costs and unmeasured I/O/resource metrics retain historical limitations.
