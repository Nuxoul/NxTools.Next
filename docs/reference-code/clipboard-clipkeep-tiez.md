# ClipKeep and TieZ clipboard reference index

Inspected: 2026-10-19. Scope: source research only. This note supplements, and does not replace, [the legacy/current clipboard contract](clipboard.md), [UI authority](../../.trellis/spec/shared/ui-design.md), or an approved implementation plan.

## Provenance and evidence boundary

| Reference | Local checkout | Branch and immutable revision | License |
| --- | --- | --- | --- |
| [ClipKeep](https://github.com/Fomovet/clipkeep) | `D:\Code\clipkeep` | `main`, `8c4f66dbbc51d9a2de3e5064c832d9084b41d51a` | MIT, root `LICENSE` |
| [TieZ](https://github.com/jimuzhe/tiez-clipboard) | `D:\Code\tiez-clipboard` | `master`, `9699dad185a76d17c09788c953734b17c262260a` | GPL-3.0 text, root `LICENSE` |

Both repositories were cloned for this request; origin, branch, HEAD, clean status and `git fsck --no-dangling` were checked. NxTools.Next comparison uses HEAD `42f60a8ee9e78402461d5c42f5b55ca5cc254c18` **plus the existing dirty working tree**, not the commit alone. Existing changes were not reset or overwritten.

No reference scripts, dependencies, applications, native clipboard operations, builds, benchmarks or test suites were executed. The built-in browser could not reveal this session. A separate browser loaded ClipKeep's fixed-revision `assets/screenshots/main.png`, but returned encoded screenshot data rather than an inspectable image: this is **not direct visual inspection**. UI observations below are source-backed composition observations, not screenshot approval or interaction acceptance.

ClipKeep package version is `0.1.0`, with Rust 2024, Tauri 2, Vue 3 and SQLite. TieZ package/native manifest version fields are `1.0.0`, with React/TypeScript, Tauri 2 and Rust 2021; manifest fields are not necessarily the effective channel/release version. Do not infer Windows support or native correctness solely from README platform tables: TieZ contains both Windows and macOS adapters.

TieZ is design/behavior evidence, not permission to transplant GPL code into NxTools.Next's MIT/Apache-2.0 codebase. Independently implement any selected idea and evaluate dependencies/assets/licenses separately. ClipKeep's MIT license still requires preserving notices if code is actually reused; none was copied in this research.

## ClipKeep entry map

Paths/line ranges are relative to its checkout at the revision above.

| Source | Verified responsibility |
| --- | --- |
| `native/src/lib.rs:162-211` | Capture orchestration and committed history updates. |
| `native/src/clipboard.rs:48-100,109-177` | Content-keyed, expiring, one-shot writeback suppression; listener and restart boundary. |
| `native/src/clipboard.rs:194-205,207-449` | Exclusions, size/native-header preflight, bounded read retry, text/pixel/image-file intake. |
| `native/src/db.rs:8-16,95-215` | Preview size, list cap, protection/order policy, hash identity, WAL/schema migration. |
| `native/src/db.rs:230-305` | Hash upsert, ordinary-history byte budget and usage accounting. |
| `native/src/db.rs:324-435` | Lightweight list versus full-content get/edit. |
| `native/src/images.rs:41-50,136-251` | Decoded-pixel hash, sharded original/thumbnail files, atomic writes and bounds. |
| `native/src/images.rs:263-390`, `native/src/atomic_file.rs:5-31` | Managed paths, reparse rejection, orphan cleanup and fsync/replace boundary. |
| `native/src/lib.rs:423-464`, `native/src/win.rs:428-458` | Hide, background write, foreground restoration and simulated paste. |
| `native/src/lib.rs:1049-1205` | Reused main-window show/hide, visible flag, focus epoch and opening grace. |
| `native/src/win.rs:77-263,282-393` | Low-level hotkey hook, atomic handoff, dispatcher and hook reinstallation. |
| `native/src/lib.rs:775-879` | Backup import decoding and database transaction. |
| `native/src/lan.rs:20-40,94-133,297-473` | LAN-only addressing, pairing secrets, AEAD message/ACK and save-before-ACK. |
| `ui/src/App.vue:69-173,249-585` | Debounced/stale-guarded fetch, shortcuts, group operations and lazy full-content editor. |
| `ui/src/App.vue:699-902` | Tabs/group picker, search, pause banner, list metadata and explicit action controls. |
| `ui/src/components/SettingsView.vue:297-343` | Ordinary/protected/database storage accounting. |
| `native/app.manifest:14-18` | Default `requireAdministrator`. |
| `.github/workflows/ci.yml:10-34` | Declared build/fmt/test/clippy CI, not a run performed here. |

### Verified chains and transferable ideas

1. **Small list projection:** capture -> schema-v3 preview (400 characters) -> `db::list` returns preview/metadata/thumbnail -> full preview/edit calls `db::get` by ID. Full SQL text remains available for search. This avoids transporting and preparing full multi-megabyte text for every list row.
2. **Separate image representations:** capture/decode -> pixel identity -> sharded original and thumbnail -> list uses thumbnail, actions retain original. `images::write` caps the thumbnail at 640x240, preserves aspect ratio and never upscales. Derivative generation is not a reason to discard the original.
3. **Writeback identity:** write registers a hash -> listener matches that hash within a two-second lifetime -> consumes only that event. An unrelated user copy is not dropped merely because a boolean flag was set. NxTools needs a richer identity because transformed/mixed formats can differ.
4. **Capacity explanation:** ordinary history counts toward a byte budget; favorites, pins and grouped items are protected and separately reported. Database/WAL usage is another explicit quantity. This is a useful accounting UI, not evidence that logical byte totals bound all physical disk use.
5. **Durable LAN acknowledgement:** explicit send -> private-address validation -> ChaCha20-Poly1305 frame -> receiver decode -> `Core::save_text` -> history change -> encrypted ACK. Receiving adds history without replacing the receiver's active system clipboard.
6. **Discoverable actions:** All/Favorites plus searchable group picker reduces long horizontal navigation; a paused-state banner resumes capture; per-item preview/more actions and shortcuts expose capability without a long permanent toolbar.

### Limits that must not migrate

- `LIST_LIMIT=1000` and no list cursor/offset: unlimited storage does not mean unlimited browsability. Vue `v-for` is not viewport virtualization. Keep NxTools' bounded cursor/window owner.
- Search is `%query%` text `LIKE`, not FTS or indexed full-text search. Debounce does not eliminate a large scan.
- Paste command hides the panel and returns before background paste completion; failures go to stderr. `SendInput` count is ignored. Do not weaken NxTools' correlated result/retry/usage boundary.
- Default elevation, permanent hook/5ms dispatcher and periodic reinstallation are implementation choices, not permission to replace NxTools' C# `RegisterHotKey` policy or elevate the whole desktop.
- Format support is narrower: text/images and image-only file drops; no arbitrary native-format preservation. Pixel equality alone cannot replace NxTools' typed-payload-plus-native-format identity.
- Import allows a large file, reads it fully and retains decoded image work before committing; a failed import may leave files for later startup cleanup. Do not replace bounded archive/shared-asset ownership with this path.
- SQLite/backups and LAN configuration secrets are plaintext. AEAD transport is not at-rest protection. LAN receiver is serial and has connection timeouts, not a general exposed-service admission design.

## TieZ entry map

Paths/line ranges are relative to its checkout at the revision above.

| Source | Verified responsibility |
| --- | --- |
| `src/features/clipboard/components/VirtualClipboardList.tsx:1-123` | Virtuoso viewport rendering, keyboard scroll visibility and load-more boundary. |
| `src/shared/hooks/useHistoryFetch.ts:50-143` | Sequence-guarded fetch, history offset paging, search cap and list retention. |
| `src/shared/lib/searchQuery.ts:1-37` | Leading `tag:` / `note:` parser, not a general query language. |
| `src/shared/lib/searchMatch.ts:6-62` | Content/preview/source/tag/note matching and server-result compatibility. |
| `src-tauri/src/app/commands/history_cmd.rs:306,420-545` | Search, tag management, full-content and pinned-order commands. |
| `src-tauri/src/infrastructure/repository/tag_repo.rs#SqliteTagRepository` | Separate item-tag repository rather than source-only categories. |
| `src/shared/lib/utils.ts:282-327` | Configurable sensitive-preview masking, including URL/email behavior. |
| `src/shared/lib/sensitiveTags.ts:5-13` | Explicit sensitive/password tag recognition. |
| `src/features/clipboard/components/ClipboardItem.tsx:1069-1106` | Memoized sensitive preview and rich-preview format heuristics. |
| `src/shared/hooks/useImageThumbnail.ts:6-75` | List thumbnail lookup, in-flight coalescing, bounded resolved map and stale/unmount guard. |
| `src-tauri/src/app/commands/thumbnail_cmd.rs:12-110,305-315` | Versioned thumbnail cache, dimensions, blocking worker, cache-key and cleanup helpers. |
| `src-tauri/src/services/paste_queue.rs:22-75,85-150,159-240` | ID queue, serialized steps, visibility-aware focus, payload preparation and progress behavior. |
| `src/shared/lib/searchMatch.test.ts`, `utils.test.ts`, `sensitiveTags.test.ts`, `quickPasteOrder.test.ts` | Inspected/discovered focused pure-logic test sources; not executed. |
| `package.json#scripts`, `e2e/README.md` | Declared Vitest/native/e2e checks; no pass/coverage claim here. |
| `src/features/app/hooks/useSplitView.ts:5-109`, `src/shared/hooks/useWideWindow.ts:3-16` | Width-gated split detail, hover intent and keyboard/pointer arbitration. |
| `src-tauri/src/services/clipboard/pipeline.rs:81-106,311-339,372-460` | Staged capture, sensitivity tagging, rich-image processing and duplicate metadata preservation. |
| `src-tauri/src/services/clipboard/utils.rs:2502-2574,2886-2978` | Bounded sensitive detection and rich-image acquisition. |
| `src-tauri/src/infrastructure/repository/clipboard_repo.rs:657-728,1411-1721`, `src-tauri/src/infrastructure/encryption.rs:1-40` | Tagged-field encryption and hybrid plaintext/encrypted search; Keychain/DPAPI master-key protection. |
| `src-tauri/src/services/cloud_sync.rs`, `src-tauri/src/services/e2e.rs`, `docs/SYNC-NOTIFICATIONS.md`, `docs/E2E-ENCRYPTION.md` | WebDAV data plus MQTT notifications; optional E2E rather than a mandatory zero-knowledge design. |
| `docs/LAN-TRANSFER-ENCRYPTION.md:63-101`, `e2e/MANUAL-CHECKLIST.md` | Plain-link/legacy-route limits and explicit native/manual coverage gaps. |

### Verified chains and transferable ideas

1. **Sensitive rendering is a projection:** item/type + mask settings -> `formatSensitivePreview` -> rendered row. Separately, the capture pipeline can attach a sensitive tag and the repository encrypts selected tagged fields. Masking is not itself encryption, and selective field encryption is not full attachment/database protection. Explicit reveal, preview/editor/UIA surfaces and payload-safe diagnostics need independent NxTools rules.
2. **Search intent is shared:** search string -> `parseSearchQuery` -> backend search request and frontend matcher. `tag:` is exact tag selection; `note:` is note-only search. A typed common query model avoids frontend/backend predicate drift.
3. **Logical organization differs from source grouping:** tags and notes are item metadata. NxTools' current custom categories match source process names and should not be silently redefined as tags.
4. **Coalesced derivative lookup:** list card -> `useImageThumbnail` -> shared in-flight promise -> `get_image_thumbnail` -> blocking worker -> versioned cache -> stale-safe result. Frontend resolved entries are capped at600; disk cleanup uses age and byte limits. These are source settings, not recommended NxTools constants or a measured LRU guarantee.
5. **Virtualization and memory are different:** Virtuoso limits mounted rows and keeps keyboard selection visible. `useHistoryFetch` still appends payload arrays. Keep NxTools' existing320-row window instead of transplanting this controller.
6. **Sequential-paste product intent:** capture can implicitly enqueue IDs -> one step at a time -> same normal-paste payload preparation -> visible panel restores target, hidden invocation avoids stale foreground restoration. This distinction is useful, but its completion semantics need rewriting; no mature visual queue editor is claimed.

### Limits that must not migrate

- Search requests `limit: 200`, disables further paging, and catches failures into `[]`. Keep NxTools' complete bounded query path and distinct empty/error/loading states.
- Queue step pops the item before preparation; preparation failure still emits `queue-item-pasted`. Later simulated-key dispatch is not proof the target consumed content. NxTools must retain the failed step, use one correlated in-flight operation, and advance only on its defined native-success boundary; never claim target-app acknowledgement without evidence.
- `prepare_next_paste_item` contains a placeholder branch after looking up content. The actual step path is elsewhere; comments are not a finished prefetch contract.
- Rich HTML preview heuristics are not permission for automatic HTML/Markdown interpretation or remote-resource loading in NxTools. Preserve literal rendering and native-format storage first.
- macOS/Windows focus, keyboard and native-thumbnail code cannot be imported across NxTools' ownership boundary. C# remains the Windows API owner; Rust owns domain and GPUI renders.
- Sensitivity detection is conditional and skips inputs over5000 UTF-8 bytes or data URLs (`src-tauri/src/services/clipboard/utils.rs:2510-2512`). Encryption is selective, not an all-history promise; portable mode follows different rules. Do not infer attachment encryption from an encrypted path field.
- Rich HTML processing can fetch remote HTTP(S) images (`src-tauri/src/services/clipboard/utils.rs:86-143,2886-2978`). Do not add implicit networking during clipboard capture or permit unreviewed private-network/redirect access.
- WebDAV/MQTT E2E is optional and not offered to new setups in the inspected documentation; LAN retains plain-link/legacy routes, including automatic fallback for larger files. Treat these as compatibility/privacy trade-offs, not end-to-end protection by default.

### Additional workflow and test evidence

- Wide mode starts at550px and adds a split detail surface; hover intent and keyboard/pointer arbitration share the selected-detail owner. This is an optional layout idea, not a new NxTools geometry target.
- AI result handling uses a before/after draft and checks original content before replacement (`src/shared/hooks/useAiActions.ts:70-160`). This is useful conflict handling for existing optional extension processing, not approval for payload upload.
- Pinned image/sticker actions are in `src/features/clipboard/components/PinStickerWindow.tsx`; manual text intake is in `src/features/tag/components/TagManager.tsx`, with backend entry `src-tauri/src/app/commands/clipboard_cmd.rs:179-216`. Do not confuse these interactive windows with passive hover preview.
- No OCR command/dependency/UI implementation was found by the scoped repository audit. OCR is a possible independent feature, not a verified TieZ capability at this revision.
- `e2e/README.md:1-4` explicitly mocks Tauri despite rendering real React UI; Windows native focus/clipboard/shortcut correctness is not established by those browser tests. Tests were not executed here.

## UI source assets, not accepted screenshots

ClipKeep: `assets/screenshots/{hero,main,organize,lan,settings}.png`.

TieZ: `docs/images/{毛玻璃,书,便利贴,3d}.png` and their dark variants. These are theme showcase assets, not a reason to add glass/paper/retro palettes. Retain the approved GPUI Kit Default Light/Dark theme. Useful composition candidates are concise metadata, clear primary content, small conditional actions, compact filters and explicit detail surfaces; actual paint/readability/keyboard/native/DPI acceptance remains outstanding.

## NxTools adaptation boundary

The [comparison and prioritized candidates](../audits/2026-10-19-clipboard-reference-comparison.md) distinguish existing capabilities, new product gaps and performance hypotheses.

Any selected follow-up must first reread [clipboard.md](clipboard.md), this note, the exact reference source and the owning phase plan, then add a red-capable regression at the real owner boundary. Preserve reusable popup, one passive preview, generation/session guards, hidden final native bounds before reveal, search/IME focus, cancellation/failure retry, native-format compatibility, reference-safe cleanup and deferred cross-entity callbacks.

No feature, phase reorder, dependency, theme, listener replacement, cloud service, build/package/install or live-data operation is authorized by this research note.

## 2026-10-19 authorized Markdown/code preview follow-up
User separately approves local Markdown/code auto-preview; the earlier research-only exclusion is not implementation authority by itself. Rechecked TieZ TextFilePreview.tsx:4,26-31: md/markdown extensions display literal pre text, not parsed Markdown. ClipboardDetailPane.tsx:533 and ClipboardItem.tsx:2801 -> shared/components/HtmlContent.tsx sanitize/render captured HTML, not a Markdown engine. ClipKeep ui/src and package.json contain no Markdown rendering dependency/entry. No GPL code copied.
Official editor page fetched HTTP200; Editor is source editing/highlighting, `.language("markdown")` needs matching grammar features. Locked0.7.1 component input/editor.rs delegates to Input/EditorState; input/popovers/mod.rs::render_markdown calls official TextView::markdown. component text/compat.rs exposes TextView/TextViewState, scrollable/selectable/on_link_click/MarkdownExtensions; Base text node ImageNode::source permits URI-backed HTTP images, so passive clipboard projection must remove resource-bearing nodes BEFORE rendering. No public arbitrary-snippet language detector found in locked Editor/highlighter/LanguageRegistry; language registry looks up a supplied name/alias. Base Markdown parser uses GFM and includes table/code/HTML/image nodes.
Legacy ClipboardPreviewWindow.cs constructor29-95,Update97-159,placement200-237 remains passive Window/one reused surface; ClipboardHistoryPopupWindow constructors/Open127-167,Hide206-259 and hover749-905 preserve search focus/cancel/update-before-place/hide. No legacy Markdown behavior found. Next new rendering must preserve its acknowledged C# noactivate/bounds/show and frame-before-reveal ownership, not replace the passive HWND or primary focus.
Actual follow-up regression verified TSX grammar requires generic TypeScript/JS captures, C++ query requires C capture inheritance, and C# lockedconfig highlights are empty. Existing0.7.1 public LanguageRegistry::language/register supports project-owned query supplements without parser/source patch. Added production tests verify each of15manual language choices has a real tree and styled captures, Markdown rendered_text, readonly input, manual/source pointer, noHTTP resource requests and long scroll. Implemented scope is local preview only; automatic detection remains deliberately heuristic/uncertain->literal and does not promise semantic/compile validation.

## 2026-10-20 user-rejected interaction correction
User explicitly supersedes list-literal-only and classifier-gated Markdown presentation, and rejects the15-minute pause menu. Rechecked ClipKeep ui/src/App.vue::onKey (486-570): window-level keydown routes arrows/PageUp/PageDown/Enter/Ctrl+1..9 after overlay guards, not solely search focus; labels expose Ctrl+number. Its paused banner is not the requested new top-right pause/play icon. Rechecked legacy ClipboardHistoryPopupWindow.axaml.cs61-167,248-259,321-369: tunnel key handling, search restoration and reusable Hide -> close menu -> hide preview -> inherited Hide; unmodified ClipboardPreviewWindow.cs29-95 remains passive. TieZ ClipboardItem.tsx key listeners remain reference behavior only, no GPL code copied. New layout is an explicit user request, not an invented reference feature.
Locked Component0.7.1 text/compat.rs exposes TextView::new/markdown, max_lines, selectable, scrollable, stream_fade and on_link_click; Base TextViewState owns parsing/notification. Styled Button exposes icon/accessibility_label/tooltip with original keyboard ownership. List summaries may now have bounded safe Markdown presentation while effects still hydrate originals by ID. Task: .trellis/tasks/10-20-clipboard-interaction-feedback/; no native show/hide or capture gate change is authorized by this presentation correction.

## 2026-10-20 screenshot-driven rendered-height correction
User rejects the first Markdown row/preview result as clipped and removes manual language menu authority. Rechecked old ClipboardPreviewWindow.cs::Update97-134 -> ApplyTextSize160-178 uses raw text estimates (not Markdown), constructor29-95 retains passive toolwindow, PlaceNear200-214 work-area clamp, ApplyPassiveWindowStyle223-254 native noactivate/topmost. This legacy raw estimate cannot size new official rich Markdown. ClipKeep App.vue text card lives in natural li/content layout, not Next's fixed24/40/96height buckets. Official locked TextView::scrollable(false) renders all blocks in fit-content mode; max_lines intentionally clips and must not stand in for actual content height. Use GPUI public layout measurement of exact official presentation before native allocation, preserve initial/reuse native ACK/frame/show/focus invariants. Task .trellis/tasks/10-20-clipboard-content-height/. No C#/protocol/native API rewrite or GPL source copying.

## 2026-10-20 user icon-gradient and cell-separation amendment
User supplies separated cells and left-to-right solid-to-transparent tint reference, explicitly superseding continuous borderless rows for Clipboard only. Rechecked old Controls/ClipboardHistoryListView.cs40-41,189-205,249-276:3pxrowgap, row fill/border, original preview/source icon and selected/hover distinction. Popup Open127-153/search focus/Hide248-259 menu->preview->native hide unchanged. New gradient is requested product presentation, not copied legacy behavior. Locked Component0.7.1 ListItem164-175,187-275 public Styled/child keeps official hover/selected rendering; GPUI-pre0.3.8 color.rs865-897 linear_gradient/linear_color_stop supports alpha falloff. Use typed static background on icon rail, not an overlay masking text; no framework/dependency patch or icon decoding. Task .trellis/tasks/10-20-clipboard-row-separation/.

## 2026-10-20 source-icon color correction
User clarifies hue belongs to actual copied-from application icon, not ClipboardKind; prior yellow/blue/type mapping is rejected. Current verified capture chain clipboard_worker.rs source_icon_staging_path -> content_digest -> promote_asset(source-{digest}.png) -> CaptureSource.process_icon_path -> history row icon. Palette must use this asset; no application scan/native hook/protocol change. Reuse serialized worker/strict managed-root/source identity and bounded cache, not render IO/decode. Official current gradient composition unchanged; immutable source icon across Text/Image/Files must yield same hue. Missing/failed source icon uses theme-neutral fallback. Task .trellis/tasks/10-20-clipboard-icon-dominant-color/; earlier no-icon-decoding exclusion is superseded only by bounded worker-side source-icon color extraction, not arbitrary clipboard image loading.

## 2026-10-20 explicit text-card maximum
User now requests text card maximum210DIP (or settings); implement direct210DIP, not a new settings/config feature. Rechecked legacy Controls/ClipboardHistoryListView.cs189-205,249-276: fixed row rect/cell border/source preview; Popup Open127-153/Hide248-259 retains reusable focus/menu/preview/hide, Preview constructor29-95 remains passive. These references do not define the new rich-card210DIP value: it is explicit user authority, superseding only unbounded history text growth. Kit0.7.1 ListItem164-175/187-275 public Styled and child composition allow a whole-card max plus bounded summary body; preserve header/actions/feedback, natural size below cap, external8DIPgap and original non-scroll list/full passive preview owners. Image max190DIP/file62DIP unchanged. No native lifecycle/full payload/framework change. Task .trellis/tasks/10-20-clipboard-text-height-cap/.

## 2026-10-21 actual shortcut/search/preview feedback
Rechecked old ClipboardHistoryPopupWindow.axaml.cs93-106 registers tunnel KeyDown and tunnel+bubble TextInput;127-153 Opened/Activated and242-246 restore search focus,321-369 accepts empty-search digits/numpad1..9/0 directly,394-427 forwards non-control non-search text to existing SearchBox and caret.1028-1085 resolves immutable cached/index item and hides popup before original accepted action.206-259 preparation/reuse/menu->preview->basehide and passive PreviewWindow29-95 remain. Controls/ClipboardHistoryListView.cs pointer enter/exit450-470 solely feeds hover preview, pointerrelease472-491 routes original action; popup delayed hover749-857 validates cancellation/current item before show. Native Avalonia base is unavailable, not a new lifecycle authority. User now requires Ctrl+index direct original-target paste and type-anywhere search; current production GPUI red shows no-focused-descendant Ctrl+2 is lost and typing after control focus is dropped. Existing tests forced search focus and omitted these boundaries. Recent installed logs show pause applied/preview hover-ready+reveal but no action.started; no shortcut rejection diagnostic existed, so exact installed key path remains unobserved. Reuse original Kit InputState/platform text/IME and original passive preview dismiss, no native gesture/controller/framework fork. Task .trellis/tasks/10-21-clipboard-input-preview/.
