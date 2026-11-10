# Ghost Downloader 3 reference

Verified2026-11-09 against `D:\Code\Ghost-Downloader-3-main\Ghost-Downloader-3-main`. Static source inspection only; no application/download/build was run. Root LICENSE and README declare GPLv3. This is a behavioral reference, not a source/resource reuse grant for NxTools.Next.

## Download-list icons

- `app/view/cards/task_cards.py::_refreshIcon`346–348 uses `QFileIconProvider().icon(QFileInfo(outputPath)).pixmap(48,48)` and a48px ImageLabel. Constructor209 and completed-state refresh323 own updates. File icons are not an extension-to-SVG map.
- `app/view/cards/draft_cards.py`46 uses16px file icons; `app/view/dialogs/task_drawer.py`179 uses48px. Missing-file fallback is delegated to Qt/Windows, not an explicit product icon map. There is no project-level file-icon cache or explicit device-pixel-ratio compensation in these calls.
- Operation/status glyphs are a separate FluentIcon path: `task_cards.py`78–92 and364–371 retain play/pause, speed, ETA and size semantics. `app/view/components/labels.py`42–75 caches those glyph pixmaps; `main_window.py`599–602 clears that cache on theme change.
- `app/assets/file_icons/build_icons.py`20–22 produces multi-size torrent/m3u8 assets; `app/platform/file_association.py`52–68 uses them for registered file associations, not list rows.

NxTools behavior references remain ABDM `TableDownloadItem.kt::NameCell`/`OSFileIconProvider.kt` (extension-cached OS icons) and old `D:\Code\NxTools\Views\MainWindow.axaml`1137–1138 (generic draft glyph). Preserve the current typed native/cache association rather than copying Qt controls or GPL code.

## Current NxTools icon boundary

`downloads.rs::DOWNLOAD_TASK_ICON_SIZE` requests48px; `DownloadApplication::task_icon_path` and original result handling associate bounded cached paths with tasks. `main_window/download_page.rs`1585–1600 renders the image in a40DIP transparent slot with Contain. `Windows/ShellIconExtractor.cs::ExtractOnSta -> Extract -> ExtractShellIcon` owns bounded STA, attribute-only unfinished types, completed targets, PNG creation, HICON cleanup and the existing bounded IconCache.

Windows SDK10.0.26100.0 `ShellAPI.h`1644/1655 declares SHGetImageList and SHIL_JUMBO=4 (normally256px); `commoncontrols.h`166–214 defines IImageList IID46EB5926-582E-4017-9FDF-E8998DAA0950 and its Add/ReplaceIcon/SetOverlayImage/Replace/AddMasked/Draw/Remove/GetIcon vtable. System image-list lookup retains SHGFI_USEFILEATTRIBUTES for reserved file/folder types, releases the COM reference and destroys returned HICONs. Large requests use a separate image-list cache key rather than legacy small/large SHGetFileInfo entries. Explicit indexed resources retain their original selector path. No Shell open, arbitrary archive-member lookup, registry mutation, new protocol or UI geometry is needed.

`SHGetImageList` returns an independent raw interface reference per query. `ShellIconExtractor.ExtractHighResolutionShellIcon` reads IImageList.GetIcon at vtable slot10 and releases that pointer with `Marshal.Release`; it does not create a shared managed RCW. The bounded STA runner permits eight real operations, so this ownership must remain correct under parallel extraction. `ExtractOnSta` retains native Win32/HRESULT values with fixed `shell_icon_*` failure categories. `IconCache.TryGetOrCreate` validates persisted PNGs inside its original per-key generation and regenerates unreadable entries without clearing unrelated cache items. The existing memory fast path,128-entry memory limit and2048-entry disk limit remain unchanged.

Regression owners: `ArchiveTypeIconTests.ParallelPngExtractionKeepsNativeImageListReferences` (48queries/concurrency8 with a new owned cache), `CorruptPngRegeneratesThroughOriginalSta`, `NativeFailureRetainsSafeCategoryAndCode` and `CompletedExecutableUsesOwnedCache`. These exercise native icon extraction without launching PlatformHost startup, registering Shell handlers or replacing the installed app.

Rust icon failure budgets belong to one live native connection. `DownloadApplication::handle_platform_notification` clears pending commands and failed-resource budgets on actual `Disconnected`; the next connection may request unresolved icons again. Repeated `Connected` notifications do not clear failures. Existing retry tokens, command IDs and resource assignments reject retired retries/results, while successful icon resources remain reusable. `main_window/download_page.rs::download_task_row` uses official `IconName::File` until a resolved PNG replaces it; the fallback is not a download-action glyph.

## Features worth comparing

| Feature | Verified implementation | Boundary / trade-off |
| --- | --- | --- |
| HTTP range-coverage progress | `features/http_pack/cards.py::SegmentedProgressBar`14–102; task segment positions map to merged covered spans | HTTP Range with known size/multiple workers only; not BT/HLS/unknown-total progress. Existing NxTools segment facts must remain authoritative. |
| Batch link templates | `app/view/dialogs/batch_url.py::expandTemplate`22–88 | Numeric padded ranges, step/alphabet ranges and alternatives; preview is bounded. Product input expansion needs its own finite intake limit. |
| Relink a paused task | `app/view/dialogs/edit_task.py`50–114 -> `task_service.py`448–450 -> `features/http_pack/task.py::relink`65–91 | Requires Range and equal length; equal length alone is not adequate evidence for NxTools persisted-resume identity. |
| Queue promotion and rebalance | `task_service.py::TaskQueue`222–270, `moveToFront`543–545 and `_rebalance/_pump`635–652 | Existing NxTools queue/admission owners remain; no second scheduler. |
| Serial collision decisions | `app/services/name_conflict_queue.py`41–80 | One active question and apply-to-rest; compare existing NxTools conflict queue rather than duplicate it. |

Do not copy the reference's final-path incomplete-file policy, sidecar format, GUI timers or weaker resource-identity checks into the existing NxTools engine. Suggestions above are not implementation commitments.
