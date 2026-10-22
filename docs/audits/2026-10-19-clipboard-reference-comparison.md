# Clipboard comparison: NxTools.Next, ClipKeep and TieZ

Date: 2026-10-19. Status: **research and proposed candidates, not an implementation authorization or accepted UI redesign**.

## Executive finding

Both references are useful, for different reasons:

- **ClipKeep:** focused task flow, lightweight list projection, real thumbnails, explicit storage accounting and a small manually initiated LAN-transfer workflow.
- **TieZ:** richer item organization, sensitive previews, sequential-paste workflows and coalesced derivative-image loading.
- **NxTools.Next:** already has richer Windows native-format preservation, target-specific image compatibility, worker ownership, bounded cursor paging and correlated action completion. These should remain the foundation rather than being replaced with either reference's controller.

The most promising small follow-ups are **lightweight list data + derived thumbnails**, **discoverable privacy/pause/keyboard actions**, and **typed search filters**. Item tags/notes and sequential paste are valuable but need explicit product decisions. Cloud sync is a separate feature, not a clipboard optimization.

## 1. Checked-out sources and method

| Project | Local path | Branch | Revision |
| --- | --- | --- | --- |
| ClipKeep | `D:\Code\clipkeep` | `main` | `8c4f66dbbc51d9a2de3e5064c832d9084b41d51a` |
| TieZ | `D:\Code\tiez-clipboard` | `master` | `9699dad185a76d17c09788c953734b17c262260a` |
| NxTools.Next | Current workspace | Existing branch unchanged | `42f60a8ee9e78402461d5c42f5b55ca5cc254c18` plus the current dirty working tree |

Reference origins, HEADs, branches, clean status and `git fsck --no-dangling` were verified. The comparison reads implementation, not only README promises. Exact external paths/call chains and license boundaries are in [the new reference index](../reference-code/clipboard-clipkeep-tiez.md). Existing migration authority remains [clipboard.md](../reference-code/clipboard.md).

No third-party application/script/install/build/test or native clipboard action was run. No user history, settings or credentials were opened. No product code was changed. Screenshots were located, but direct image inspection was unavailable; a separate static-image capture returned encoded data only. Consequently UI findings are **source-backed layout/interaction candidates**, not judged screenshot quality, physical interaction or user acceptance. No comparative timing, memory or binary-size claim is made.

## 2. Capability comparison

| Area | NxTools.Next current source | ClipKeep | TieZ | Assessment |
| --- | --- | --- | --- | --- |
| Captured representations | Text, images, files and bounded native formats; target-specific image/FileDrop policy | Text/images, including image-only Explorer file drops; narrower format scope | Multiple types and HTML-related paths | Keep our richer native-format boundary. |
| Duplicate handling | Global typed payload plus complete native-format equality; revives existing ID | BLAKE3 indexed identity; images keyed by decoded pixels | Has independent identity/merge logic; not a reason to change our format semantics | Explore indexed candidate lookup, not pixel-only identity. |
| List data | 80-row pages, at most 320 payload rows; full text still projected | 400-character stored preview, full body loaded on demand, cap 1000 rows | Virtualized UI and lazy-content/thumbnail paths; history payload can accumulate | Combine our bounded paging with lighter data. |
| Search | Content-only default; extended text/path/source/title scope; stable cursor | Text `LIKE`, no FTS | Source/tag/note matching; `tag:`/`note:` prefixes; search cap 200 | Borrow intent/filter UX; preserve our complete paging/error state. |
| Organization | Built-in kinds/favorites and custom **source-application** categories | One item group plus favorite/pin | Per-item tags and notes | Real semantic gap: source grouping is not item tagging. |
| Image list | Direct original-file image source with 96-entry image resource admission | Separate original and max640x240 thumbnail | Versioned max320px thumbnail; request coalescing and disk cleanup | Strong performance candidate; measure first. |
| Keyboard | Up/Down/Enter and Ctrl+Enter on search; Tab-accessible controls | Ctrl+1..9, PageUp/Down, management/group shortcuts | Richer shortcut and queue workflows | Add optional direct actions through current dispatcher. |
| Privacy controls | Capture policy/limits, global per-app blacklist; raw list/preview text | Paused banner and tray control; sensitive native exclusions | Sensitive tags, masked preview and selective tagged-field encryption with Keychain/DPAPI; not all-history encryption | Separate detection, display, persistence and network policies. |
| Retention/storage | Item/day policies; pin/favorite protection; reference-safe asset purge; backup/compact/migrate/archive | Ordinary-history byte budget and protected-content accounting | Thumbnail age/byte cleanup plus richer policy options | Add visibility and optional capacity policy without weakening retention. |
| Paste completion | Pending ID before dispatch; failure unlock/retry; usage only after confirmed result | Hides then background paste; stderr failure only | Serial queue, but pop-before-preparation and misleading failure progress | Our completion/cancellation rules remain mandatory. |
| Extensibility | Trusted extension actions, committed-worker seam, item-local progress and exact-result navigation | Focused built-ins | Many built-in advanced workflows | Prefer current extension seam for optional transforms; do not add another runtime. |
| Data portability | Managed `.nxtclip`, online backup, backup-first compact, staged migration | Backup/import merge | Multiple import/sync paths | Already strong; compatibility import requires a dedicated adapter. |

### NxTools evidence map

- `repo:rust/crates/nxtools-clipboard/src/lib.rs#ClipboardHistory::capture_payload` (660-758): typed-payload query, complete native-format comparison, duplicate revival and transaction.
- `repo:rust/crates/nxtools-clipboard/src/lib.rs#ClipboardHistory::open` (473-579): current tables/indexes and additive schema initialization. No claimed WAL/FTS/content-digest index.
- `repo:rust/crates/nxtools-clipboard/src/query.rs#query_page` (339-413) and `#query_items` (417-466): count/query scope, full-text projection, ordering, cursor/offset APIs.
- `repo:rust/apps/nxtools-desktop/src/ui/mod.rs` (80-88,501 onward,572-607): 100ms debounce, 80-row pages, 320-row window, 96-image admission and existing keyboard owner.
- `repo:rust/apps/nxtools-desktop/src/ui/history.rs#text_content` (611-627): full-body character count and whitespace-normalized single-line summary; `#image_content` (654-689): original image path.
- `repo:rust/apps/nxtools-desktop/src/ui/preview.rs#preview_body`: raw full text/image preview projection.
- `repo:rust/crates/nxtools-clipboard/src/categories.rs#add_category_source` and `query.rs` (359-363,437-441): custom categories map to source process names, not independent item tags.
- `repo:rust/apps/nxtools-desktop/src/clipboard_worker.rs`: serialized 256-job worker, superseded-query handling, content-addressed assets and commit subscriptions.
- `repo:rust/crates/nxtools-core/src/lib.rs#ClipboardSettings` (905-960): persistent enabled flag, limits/retention/actions, no temporary-pause deadline or mask policy in this model.
- `repo:rust/apps/nxtools-desktop/src/clipboard.rs#capture_policy` (1512-1517): existing capture-enabled gate. This is not evidence for an implemented temporary-pause UI or native pause acknowledgement.
- `repo:csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs#TryPublishClipboardContent`: native listener, exclusions, size/format intake, bounded retry/publication.
- `repo:csharp/NxTools.PlatformHost/Windows/WindowsPasteCoordinator.cs`, `repo:rust/apps/nxtools-desktop/src/clipboard_preview_presentation.rs`: native action/presentation ownership to preserve.
- `repo:rust/crates/nxtools-clipboard/src/archive.rs`, `storage.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard/extension_adapter.rs`: existing storage/extension seams.

The prior [local performance research](../../.trellis/tasks/09-27-clipboard-performance-platformhost-crash/research/nxtools-clipboard-evidence.md) already records bounded-window and cursor improvements. Its old retained-vector diagnosis was superseded; it must not be presented as a current leak.

## 3. UI and interaction candidates

### A. More capability without a larger popup

Borrow ClipKeep's concise content/metadata/action hierarchy and explicit more/detail affordance, not its CSS/palette. Show shortcuts as hints rather than a permanent wall of instructions. Keep independent pin/favorite status visible even when the management rail is hidden. A group overflow/search picker can help many categories, but cannot silently replace our approved drag/order/wheel/context behavior.

For NxTools, preserve the default300x440 client, user resize, stable search input/IME, item selection distinct from hover, and one passive preview. Do not automatically expand into TieZ's split-detail workspace or copy decorative glass/paper/retro themes. A full-detail editing surface can be evaluated separately from the passive hover window; passive preview must not become an interactive editor.

### B. A real temporary capture-pause control

ClipKeep has an immediate paused-state banner with resume. NxTools has a capture-enabled model, but a temporary pause is a different state: pause for a chosen duration/until resumed, show a durable state indication, retain history browsing/pasting, and avoid changing other tools. Reuse the existing capture owner rather than another listener. Native pause should happen before expensive reads if that is the approved policy; a Rust-only post-capture drop must not be called "capture stopped" without qualification.

### C. Search scope made understandable

TieZ's narrow `tag:`/`note:` parser is useful discoverability evidence. NxTools can expose application/type/date filters with a shared typed query model and compact filter indicators. `app:`, date ranges and combined clauses are **our proposed extension**, not a claimed TieZ parser capability. Plain text must remain plain content search by default. Invalid filters need a local validation state, not silent empty results.

### D. Sensitive preview and explicit reveal

Masking phone/email/IDs or manually marked entries can reduce shoulder-surfing exposure. Reuse one sensitivity projection for list, hover/full detail, tooltip and accessibility text; deliberate reveal must not also paste. Keep original bytes for explicit copy/paste, with a clear policy for saving sensitive history. Masking does not mean database/backup/thumbnail encryption or secret detection accuracy; do not make that promise.

## 4. Proposed candidate order and acceptance

These are recommendation buckets, **not canonical implementation phases**. No existing phase is renamed or advanced.

| ID / priority | Proposed scope and benefit | Reference and NxTools landing area | Required proof before claiming completion |
| --- | --- | --- | --- |
| C1 / first | Lightweight history summaries; load full body only for action/edit/detail/extension | ClipKeep `db.rs::list/get`; our ClipboardHistory/query, worker, panel/history | Large text fixtures; tail-only search still matches; exact full payload/native formats loaded by immutable ID; deleted/stale item fails correctly; measure retained bytes and render/query p50/p95. |
| C2 / first | Derived thumbnails; stop decoding full-size originals for small rows | Both image pipelines; our worker asset owner and history image presentation | Bounded generation queue/coalescing; version/content key; original bytes unchanged; shared-reference deletion, orphan/migration/archive behavior; alpha/animation policy; compare 4K scroll CPU/private memory/first preview. |
| C3 / first | Optional number shortcuts, PageUp/Down and concise shortcut hints | ClipKeep App.vue keyboard path; our one panel dispatcher and current selection | IME/edit/modal exclusion; stable visible ID after refresh; held/modifier behavior; one command only; failure does not hide/advance usage; Tab focus retained. |
| C4 / first | Explicit timed pause/resume and persistent paused-state presentation | ClipKeep paused state; existing ClipboardSettings/capture policy/native config | Pause acknowledged at owning boundary; capture during pause and deadline/restart semantics; no accidental loss after resume; browsing/paste still work; no second listener/poll loop. |
| C5 / next | Sensitive-preview mode, manual mark and explicit reveal | TieZ mask utils/item path; our history/preview/projection and policy | All disclosure surfaces including UIA; Unicode/false positives/short strings; reveal gesture not paste; no raw diagnostic payload; honest at-rest-storage wording. |
| C6 / next | Typed app/type/date filters, then tag/note filters if those features are approved | TieZ shared query intent; our query/worker scope+cursor | Initial count/page/cursor use identical predicates; cancellation latest-wins; local date/time boundaries; invalid syntax/error distinct from no matches; 10k/100k search fixtures. |
| C7 / next | Per-item tags and notes, independent of source categories | TieZ tag repo and ClipKeep item group; our same history connection/metadata owner | No semantic rewrite of source categories; tag rename/remove/count; duplicate revival/edit/import preservation; retention protection explicitly decided; cancel publishes nothing. |
| C8 / next | Multi-select text merge and sequential-paste queue | TieZ paste_queue; our existing correlated native action path | Explicit selection order/separator; one pending ID/step; failed write/focus/input retains step; target closed/UIPI/disconnect/cancel; no automatic retry duplicate; queue progress not inferred from simulated consumption. |
| C9 / next | Storage breakdown, optional byte budget and previewable cleanup | ClipKeep ordinary/protected accounting; our retention/archive/storage | Count unique shared assets, recoverable-deleted rows and derived caches honestly; partial failures; backup-first destructive operations; item/day limits remain; protected set can exceed quota without false guarantees. |
| C10 / benchmark-led | Digest-index candidate lookup for global dedup | ClipKeep indexed hash; our capture_payload/native_formats_match | Versioned digest of typed fields plus full native-format identities; collision candidate still exact-compared; preserve equal-text/different-format rows, stable IDs/metadata; benchmark 10k/100k duplicate/new captures. |
| C11 / later | Explicit LAN text send, no automatic clipboard takeover | ClipKeep AEAD+durable ACK; our optional separate transfer service + committed worker intake | Explicit pairing/revocation/secret store, bounded frames/queues/replay policy, save failure no ACK, no receiver clipboard overwrite, cancellation/diagnostics. Separate product scope. |
| C12 / separate scope | External editor / AI / cloud sync; OCR only as an independent idea | TieZ workflow directions and AI conflict-checked draft; no OCR implementation found at this revision; existing trusted extension seam preferred | External editing needs temp-file ownership/version conflict/cleanup. AI/OCR need explicit payload/runtime policy. Sync needs key recovery, conflict/deletion semantics and budgets. No automatic implementation. |

C1/C2/C10 are performance hypotheses, not current measured defects. A 320-row count bound does not bound arbitrary full-body bytes; a thumbnail may improve list cost but cannot remove the intentional800ms hover delay/native acknowledgement cost. Record cold/warm stages rather than simply reducing delays.

For C10, do not switch SHA-256 to BLAKE3 solely because a comment says "10x faster". The likely architectural opportunity is indexed candidate narrowing, not the brand of digest. The current content-addressed files already have hashes; their identities must not be confused with the complete clipboard-item identity.

## 5. Architecture and UI adaptation rules

- Rust owns typed query/metadata/queue/asset policy and logical layout; the existing worker owns SQLite. C# owns Windows listener/clipboard/focus/input/native image APIs; GPUI owns rendering.
- Reuse current session/generation/command IDs. Install pending before dispatch; asynchronous/cached completions defer across owners and verify current ID/HWND/session. Terminal success/failure clears busy state. Native success is not a target-app receipt.
- Keep one reusable panel, one passive preview and the established hide/show/frame/readback/cancel rules. Avoid borrowing a different reference lifecycle just to implement a visual feature.
- Official Kit0.7.1 composition first: current `ListItem` for history presentation, stable `InputGroup` for search, `Popover` for local filter editing, `Tag` for non-action filter/status labels, `Kbd` for hints, actual `Button` actions, `Checkbox` only for explicit multiselect, and existing `Settings` hierarchy for settings. These are candidates; exact released API/compound, visible variant/size and parent adaptations must be recorded before implementation.
- Outlined app commands must retain their actual Button appearance; `Toolbar.child` forces ghost+compact. Do not invent a Card/Grid or repaint ordinary controls. Keep official Default Light/Dark/fonts and approved density.
- Keep current notifications, extension trust/runtime and item-local progress owners. Optional processing is not a reason for an independent clipboard monitor/runtime or automatic payload upload.
- Define low-noise correlated start/state/terminal/elapsed/native-stage events before new operations. Never log clipboard content, tags/notes/search text, credentials, source titles or payload paths.

## 6. What not to copy

1. **ClipKeep's permanent administrator requirement/hook strategy:** not compatible with our boundary by default; no new elevation/global hook authorization.
2. **ClipKeep's1000-row all-at-once UI or TieZ's200-result search cap:** our bounded cursor paging is a better fit.
3. **Hide-and-forget paste or pop-on-failure queue:** our retryable result boundary must remain stronger.
4. **Pixel-only/image-only dedup:** would merge items whose native formats differ.
5. **Plaintext pairing secrets or treating transfer encryption as storage encryption.**
6. **Unbounded backup decoding/payload accumulation:** preserve our resource/reference rules.
7. **TieZ theme gallery or rich HTML rendering:** not approved theme/runtime behavior for this product.
8. **WAL/FTS switches without measured need:** current single-owner writes do not automatically benefit from a second connection; FTS token semantics do not equal current Chinese substring search.
9. **Direct TieZ code reuse:** GPL-3.0 is not a permissive drop-in license for our MIT/Apache project. No reference code was copied.
10. **Implicit networking or overclaimed privacy:** TieZ rich-HTML processing downloads remote images; its sensitivity detector skips long/data-URL inputs, selected tagged fields are encrypted but ordinary history remains outside that guarantee, cloud E2E is optional, and LAN has plain-link/legacy compatibility. Borrow explicit policy separation and key protection, not an unconditional security claim.

TieZ also has a width-gated split-detail mode and reusable compact preview, plus conflict-checked AI before/after drafts. These can inform future approved detail/extension workflows. They do not authorize widening our popup, making hover preview interactive, or sending clipboard payload to a network service. Its browser E2E mocks Tauri; real Windows lifecycle acceptance remains distinct.

## 7. Verification and limits

Performed during this research:

- Both Git clones, immutable revision/origin/branch/clean-state checks and `git fsck --no-dangling`.
- Read-only source comparison at the references above and current local Clipboard/Core/Desktop/C# owners, including legacy hide-before-restore evidence.
- Located UI assets; **did not directly inspect their pixels**. Built-in browser visibility failed; isolated static-image navigation succeeded but returned only encoded screenshot data.
- `python tests/architecture/spec-contracts.py`: passed after the documentation update (106 documents,14 workspace owners). This gate checks source paths/navigation/structure, not third-party behavior or visual/native acceptance.
- Focused documentation/reference-path/link/line-range checks and whitespace checks are recorded in the session closeout.

Not performed: dependency installation, third-party tests, product Cargo/.NET build/tests, benchmark, product launch, real clipboard/history mutation, physical input/DPI/reader checks, package/install/service work or commit. Reference README speed/memory/size claims are not comparative evidence. The direct visual comparison remains a follow-up limitation, not a blocker to this source-based report.
