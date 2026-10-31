# Archive browser built-in read-only preview contract

## Scope and source review (2026-10-27, before implementation)

User explicitly approves images and text/code/Markdown only. No PDF, Office, media playback,
execution, write-back, external fallback, dependency/vendor changes, packaging, installation,
product/real-GUI/service/Registry launch or commit. Parent owns Browser UI/selection and icon delegate
owns native icons. This file is the preview-specific reference follow-up; archive.md is not edited
because it is outside the delegated writable scope.

Reviewed AGENTS, Desktop/Protocol/PlatformHost owner and shared architecture/engineering/verification,
component selection, current 0.8.0 delta/table/API index, archive.md, markdown-editor.md, browser request,
Phase C browser and closeout contracts. Historical request's pending-scope wording is superseded only
by the explicit image/text approval above.

Reference chain re-read: D:/Code/Archiver-main/gui/src/main.rs:115 AppWindow::new ->136
wire_callbacks ->158 ui.run ->159 recursive shared-temp cleanup; :697 view_item ->708 temp_view_dir
->709 background spawn ->711 selected extract ->712 After::OpenFile only on success. Its global
password/PID-shared temp/recursive cleanup/external open are NOT reused. archive.md:16/419 records
no old NxTools general archive-window lifecycle; no proprietary Bandizip code inspected.

Actual Next native chain reviewed: ArchiveApplication::open_entry/materialize -> typed archive_entry
-> ArchiveEntryWire.Parse -> ArchiveEntrySession.Dispatch/Start -> original ArchiveEntryCache.Create
-> ArchiveWorkerRunner.ForBrowserEntry/Start -> ArchiveWorkerAdmission immutable copy -> fixed
held helper/source identity + browser_entry ordinal/snapshot/source -> protected ArchiveOutput native
writes -> actual exited/cleanup -> ArchiveEntrySession.Complete -> Lease.Seal. Runner.cs:137 admits
only materialize internally, so native preview command is normalized to materialize ONLY in Start
while the original Pending command retains preview purpose and finite limit. No private schema change.
Original output limits go to ArchiveAdmittedLimits and ArchiveWorkerOutputBridge; metadata alone
never grants preview success. Cache.cs:169 Seal records identity/hash/metadata/MOTW; :239 Release
validates those and exact empty ancestry, retains locked/edited/unknown files, never recursive delete.
EntrySession.cs:66 cancel clears password ticket/secret and cancels original runner; :137 refuses
new prompts after cancellation; :170 Finish clears ticket; :185 rechecks cancellation before reply.
Existing password.rs remains the private ticket/reply owner. Preview intercepts only its own pending
operations in entry.rs, using the same masked official Input/Dialog and password_reply transport.
Cancellation removes the exact operation's ticket, clears its input and defers close only if the
captured dialog FocusHandle still contains focus. Covered obsolete dialogs self-close only after
their own scope is exposed, never pop a different overlay. No new secret cache/runtime.

## Retained lifecycle / resources / diagnostics

Same activating Browser HWND and one BrowserModel selection owner; preview never opens another
window, steals focus, executes links/scripts or reaches Shell Open. Double-click keeps original
Open behavior. Parent supplies monotonic selection generation and Option<RowId>, including None
for empty/multiple selection. Backend independently checks model selection count and immutable row,
slot generation, snapshot and full source binding before materialization and application.

Use original entry pending records with explicit preview purpose index; latest desired selection
is stored in original ArchiveApplication. One outstanding preview read/decode per browser; latest
selection is coalesced while an older task actually drains. Global background decode permits=2.
Cancellation invalidates presentation, cancels original native operation, and retains its lease until
actual file read/decode exits. Stale native success releases the same lease; stale decoded results
release only after background completion. Close/refresh/cancel/disconnect/source-change retire state.
No filesystem work, read, header parse or decode runs in render.

Image actual encoded input <=8 MiB; currently enabled PNG/JPEG/GIF/WebP/BMP only, owned static frame.
Axis<=8192, pixels<=8*1024*1024, 8-bit channels only. Decoder max_alloc=32 MiB, decoded-buffer bytes
reserved before scratch admission (matching ImageReader::decode), final RGBA<=32 MiB separately;
conversion can retain original and RGBA at once, so this is NOT a claim all process memory is32MiB.
JPEG/WebP additionally require pixels*32 +9MiB <=32MiB (input copy/table scratch estimate), so their
admitted canvases are smaller. image0.25.10 max_alloc is explicitly non-strict; WebP wrapper does not
forward it. Read-only admission therefore refuses animated/ALPH WebP and allocating VP8L transforms,
color cache/meta-Huffman expansion; static VP8 and allocation-free VP8L remain available. No vendor
patch/dependency change. Native/allocator peak verification remains deferred, not certified64MiB RSS.
Text actual output<=64 KiB; strict UTF-8 with optional BOM,<=1200lines/65536chars and no controls
except CR/LF/TAB. Shared MarkdownDocument::prepared_source supplies passive safe projection/literal
fallback, ImagePolicy::Placeholder and no links/resource IO. Metadata-unknown files still pass actual
native output and Rust limit+1 checks. Rust checks regular/non-reparse file shape and final length.
Lease is held through actual background read/decode; payloads own pixels/text, not temporary paths.
Diagnostics share operation UUID accepted/materialized/decode/native terminal/terminal outcome,
elapsed and native code; no content/path/title/password/error payload or render/move/frame streams.

## Component and parent integration API

Requested shape: optional resizable right pane, single-selection live preview. Parent composes
released Kit0.8.0 h_resizable/resizable_panel, official Spinner/Empty/Alert and Small outlined Buttons
outside Toolbar.child ghost/compact transformation. Image uses img(owned Arc<RenderImage>) and
existing ui::preview_image::image_plan; text uses existing
ui::markdown_document::document(text, DocumentMode, window, cx), whose BlockEditor passive(true),
readonly state and ImagePolicy::Placeholder retain no active resource loading. No ClipItem adapter.

Backend entry points (called by parent on NEXT application turn, never inside leased Browser):
- ArchiveApplication::preview_entry(window_id: Uuid, row: Option<RowId>, selection_generation: u64,
  cx: &mut Context<Self>)
- ArchiveApplication::cancel_preview(window_id: Uuid, cx: &mut Context<Self>)

Parent must implement exactly:
`ArchiveBrowser::preview_result(&mut self, selection_generation: u64, row: Option<RowId>,
result: crate::archive::preview::PreviewResult, window: &mut gpui_kit::Window,
cx: &mut Context<Self>)`.
Backend dispatches it through cx.defer -> current-token check -> slot.window.update -> view.update.
Parent rejects stale local selection generation again. PreviewResult distinguishes Idle, Loading,
Ready(PreviewPayload), Unsupported, TooLarge, Directory, MultipleSelection, Cancelled,
PasswordCancelled, SourceChanged, Disconnected, Corrupt, Failed. PreviewPayload::Image contains
owned Arc<RenderImage> plus dimensions; ::Text contains owned text and DocumentMode.
The parent also supplies `preview_selection_matches(selection_generation, &Option<RowId>) -> bool`;
backend requires it before accepting/dispatching work, before password show/reply and at deferred
publication, in addition to current source/snapshot/generation/model checks. This blocks ABA.
Fixture hook: `prepare_preview_for_test(&mut self, cx) -> u64`, after setting original model selection.

## Verification status

No Cargo or dotnet build/run started by this delegate (parent owns serialized lanes). Protocol test
was authored before action implementation; red-capable baseline with actions absent is saved at
session scratch/archive-preview-red/archive_browser.rs. It was NOT executed red; do not claim it was.
Decoder/C# purpose and actual-output tests were added before their implementations; application
lifecycle tests subsequently exercise original dispatch/result/window and real background seams.

Parent C# harness registration (Program.cs + TestEffects.cs remain parent/integration-agent owned):
- `archive preview purpose strict wire` -> ArchivePreviewTests.PurposeWire (DTO-only fixture).
- `archive preview purpose actual output budgets` -> ArchivePreviewTests.ActualOutputBudgets
  (owned temp ZIP/cache plus original isolated worker; requires matching parent-built worker).
LIST and RUN the identical two fixture names. No Shell or native UI launched; actual-output fixture
asserts preview cannot subsequently gain Shell-open authority, and legacy materialize still exceeds
the small preview ceilings successfully.

Rust added:7decoder/resource tests,8application lifecycle tests and1protocol purpose test. Application
cases cover latest-only native drain, UI ABA, stale/closed lease release, actual decode lease hold,
unclean cleanup reservation, disconnect, unrelated dialog preservation and source-change retirement.
Scope excludes browser/UI files and original entry tests; source-only gates cannot certify interaction.
Compiled tests, strict lint/workspace check and actual allocator/native/physical/visual acceptance
remain NOT RUN by this delegate. Scoped rustfmt parsing and diff check passed; gate results below.

Source/document validation PASS: spec-contracts114documents/14owners; coherent GPUI release/two
approved file deltas; pinned Markdown57files/passive/lowbits/serializer provenance. Scoped rustfmt
with skip_children=true and git diff --check passed; no browser/UI/vendor/dependency formatting.
No compiled/native/visual success is implied. C# build and LIST/RUN registration remain parent work.

Newly verified decoder sources (read-only installed exact0.25.10, no registry edits):
- image/src/io/image_reader_type.rs311-322 reserves total_bytes before set_limits; from_decoder
  alone does not. io/limits.rs11-20/38-42 documents max_alloc non-strict behavior.
- image/src/codecs/webp/decoder.rs23-27/43-66 wraps image-webp without set_memory_limit;
  image-webp0.2.4/src/lossless.rs307-343 permits meta-Huffman groups unrelated to pixel count.
  Admission rejects that branch BEFORE decoder construction; no unverified global64MiB claim.
- image/src/codecs/gif.rs113-179 reads only first frame and can allocate a separate bounded frame
  buffer; explicit decoded reservation reduces remaining allowance. JPEG source copies encoded
  input during header construction; both input and 32bytes/pixel scratch estimate are accounted.
- Native Shell preview guard is per live cache lease, pruned via original cache identity after
  Create/sweep, not an unbounded operation map. Stop snapshots at most32leases and asynchronously
  retires using original identity/locked-file cleanup; Dispose awaits that exact cleanup task.

## Parent integration and actual final verification

Parent completed original Browser modules/menu/resizable/source controls and synchronous local
selection token retirement with next-turn backend requests. ExplicitOpen now cancels passivework
first (observedred1->green1), preserving originalMaterialize/consent and nativepreviewleaseOpenrefusal.
Actual final archive181/protocol82/fullDesktop1369+10 PASS; testsstrict/workspacecheckPASS. Current
matchingworker actualC# readonlypurpose/outputlimit fixtures2PASS, in exact23fixture combinedrun.
Six actualpreviewUI tests cover document/source/close/ABA/realpointer+keyboard/officialsplitter/
minimumthemes/imagebounds+release/longdocumentlastblock. These do not establish realuserGUI/DPI/IME/
provider/visual/Registry acceptance. Decoder/native purpose pre-fixreds were NOT executed.
Productionstrict remains blocked by original unrelatedSPACING_12/animated; globalfmt by original
network/page_rem files, changedscopedfmtPASS. See browser-completion-outcomes for commands/logs.
Image presentation uses existing GPUI ownedRenderImage/img Contain, not separate Clipboard image
window/mode policy; the image_plan mention above is available reuse evidence, not adopted geometry.
