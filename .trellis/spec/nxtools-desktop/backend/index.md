# Desktop backend and GPUI composition

Evidence: `repo:rust/apps/nxtools-desktop/src/runtime.rs#DesktopRuntime`, `repo:rust/apps/nxtools-desktop/src/main_window.rs`.

## Pre-Development Checklist
Read [architecture](../../shared/architecture.md), [engineering](../../shared/engineering.md), [approved UI/current matrix](../../shared/ui-design.md), matching reference/owning phase plan, then [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). State/layout Rust; rendering GPUI; Windows C#; callbacks defer cross-owner updates.
For UI work, FIRST read [component selection](../../shared/ui-component-selection.md), [current0.10.0 release/API delta](../../../../docs/reference-code/gpui-kit-0100.md), then [historical catalog](../../../../docs/reference-code/gpui-kit-component-table.md)/[API index](../../../../docs/reference-code/gpui-kit-component-index.md) and exact current released source. Current Kit0.10.0/defaultpre0.3.12 carries the approved Component1/Base5/core2/Windows4 union, including notification duration/header-action, live noactivate, Base editor-host whitespace/cell geometry and retained material composition/client-boundary work. Ordinary TextView retains upstream0.10.0 behavior; pinned editor revision and original assertions remain. Requested shape/compound/variants/size/parent adaptations remain; Toolbar.child still forces ghost+compact. Earlier versioned feature evidence remains dated; tests/imports do not certify visual acceptance.
Download external intake and automatic browser confirmation follow [controlled handoff/source-driven resolution](../../browser-extension/controlled-handoff.md): external sources auto-probe, manual clipboard/Paste remain explicit, and fixed-window ownership transfers without aborting the live handoff.

## Quality Check
Desktop full tests plus production bounds/action/lease/generation regression and affected native gates; strict scoped lint/check/fmt per [verification](../../shared/verification.md). Source token gates do not prove approved palette, accessibility or real lifecycle. Visual/keyboard/pointer/DPI acceptance remains separate, no product launch/service/DOpus without authorization. [Entry](../../index.md).

Current2026-10-22 approved amendment: [unified document/image/spacing/cadence/theme contracts](ui-experience-contracts.md) supersede historical no-editor and built-in-palette-only restrictions only within this explicit slice. Both carried framework patches remain unchanged.

P208 formalrow2026-10-23: `archive/browser_drag.rs` is private state of the original ArchiveBrowser,
not a second selection controller. Down candidate/held>=4DIP move/direct formal platform enqueue;
actual Window HWND, source/snapshot/ordinal/codepage frozen at start. Projection/refresh/source/close
retire/cancel original operation; CommandResult request receipt is deferred through ArchiveApplication.
Official ListItem32DIP/Alert/Toolbar composition retained. Tests exercise actual rows, no GPUI on_drag,
pre-extract/clipboard substitute or self-app/native acceptance bypass.
Directory/unknownsize2026-10-23: original `model_drag.rs` expands immutable BrowserModel.entries
without IO; explicit/implicit/empty directories, parent/child overlap dedup and unknown file sizes
retain original source/ordinal. Complete selections use shared frozen source context and the real
1MiB related-envelope frame; no separate root/descriptor/file/request-count or512MiB drag cap.
Original path/native representation, general extraction resources, active-worker admission and
connection-locked bridge remain. Logical file count is not worker concurrency or live lease count.
See [current experience contracts](ui-experience-contracts.md#scenario-automatic-archive-preview-and-complete-drag-selections).

Ordinary7z edit integration follows [format contract](../../nxtools-archive/backend/sevenz-edit.md).
Original Browser/ModifyDraft/OperationForm/Job/selection/picker owners remain; actual worker eligibility
is mandatory, legacy missing facts refuse edits. Replacement uses one regular file and distinct
wire action. Implicit folder rename freezes original prefix plus real descendant ordinal. Existing
Kit0.8.0 outlined commands/Form/Field and Small nonclosable warning Alert disclose full non-solidLZMA2
reencoding/coexistence-space unknown; original progress window shows actual fixed phases, not saved
percent. Cross-entity callbacks remain deferred. Compiled TestWindow evidence is not user/DPI approval.

TAR Step3 reuses the same Browser/ModifyDraft/Form/Job owners with optional completed-worker
`tar_edit` facts, missing facts fail closed; no Desktop TAR predecode/admission loop. Original
ordinal/prefix selection and deferred owner boundaries remain. Official Form/Field and Small
nonclosable warning Alert disclose full TAR/outer rewrite, parameters, normalized new metadata,
unknown original+candidate/spool space and no intrinsic TAR payload checksum. No Step4 interaction
unification. [TAR edit contract](../../nxtools-archive/backend/tar-edit.md); counts and native/user
limits belong only to the task checkpoint.

Step4 now connects qualified TAR-family application entries and all-Modify final source/member
guards, original Ask/private archive-object presentation, terminal warnings and one-shot ready-new-
snapshot path restoration. [Edit interaction contract](../../nxtools-archive/backend/edit-interactions.md)
supersedes only the preceding Step3 statement that interaction unification was deferred; the format
engines/native leases and user/DPI/physical-drag acceptance limits are unchanged.

HZ3 batch changes read [archive batch projection](archive-batches.md): original queue/records,
same logical member/new native attempt, explicit safe recheck, original progress-window Table
summary and bounded relationship retirement; no new scheduler/stop-on-error/persistent history.

Release38 drag/preview follow-up: [automatic preview and complete selection](ui-experience-contracts.md#scenario-automatic-archive-preview-and-complete-drag-selections)
removes preview enablement state, keeps descriptor renaming separate from original ordinal content,
and distinguishes read/decode resource stages. Original OLE/controller/cache/focus owners remain.
OPT-04 visible Ready retains its exact original native lease in BrowserSlot until original preview retirement; failed/stale/owner-lost decode releases once. See the same [preview contract](ui-experience-contracts.md#scenario-automatic-archive-preview-and-complete-drag-selections) and [retained native runtime](../../nxtools-archive/backend/product-read.md#scenario-drag-scoped-retained-materialization).

HZ5 extraction changes read [Stored ZIP reclamation](../../nxtools-archive/backend/reclamation.md): default-off official controls, irreversible consent, fresh Start continuation, ordinary-only readmission/retry and deferred source-snapshot retirement.

OPT-07 changes also read [browser/Find/atomic intake](archive-browser-workflows.md) and [unified batch/consent](archive-batches.md#scenario-opt-07-unified-surface-and-protected-consent): snapshot/query/connection identity, transition-cached rendering, current reply eligibility, single original surface and actual pending-location retirement. Native secrets/picker remain in [PlatformHost](../../platformhost/archive-workflows.md).

Configurable archive preview follows [one frozen preview budget](archive-preview.md): Core leaf-wise
settings merge, official six-field page, preview-only optional wire, original native purpose limits,
actual-length read/decode and budget-aware shared document rendering. Clipboard/Quick Launch defaults
and original source/slot/lease/cancellation owners remain independent.

Download progress Server information follows [original response observation](../../nxtools-download/backend/server-information.md): original task/entity/window and compact dimensions, official Small DescriptionList slots/Tooltip, same-response probe provenance and memory-only selected metadata. Time is 探测响应, not Ping/RTT.

Explicit numbered-file joining is opened from Archive Browser's official PopupMenu beside New Archive. Follow [its cross-layer contract](../../platformhost/raw-numbered-join.md): stable Form inputs, explicit Dialog consent, deferred weak/window routing including cache-hit activation, and actual terminal-bound close/cancel ownership.

Archive Browser drag-out follows [ordinary extraction after destination location](../../../tasks/10-16-archive-manager/drop-selection-repair.md): every source sends one existing locate folder, freezes original source/selection/prefix at the held gesture, and submits an ordinary Here Extract only from the current located receipt on a later application turn. Passwords belong to that job; no visible pre-unlock action or virtual member demand. Source identities and original ordinals are independent of stripped output paths; cancellation/reconnect/retirement cannot start late work.
