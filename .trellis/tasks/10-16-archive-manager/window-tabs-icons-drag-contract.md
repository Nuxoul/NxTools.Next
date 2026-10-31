# Approved archive window / icon / Tree continuation

The user supplied an explicit implementation request on 2026-10-27. This is a continuation of the existing canonical A–F plan, not a new phase or release approval. Current state is only in phase-a-checkpoint.md. All historic failures and installation error 1 OPEN remain unchanged.

## Boundaries and requested changes

- Preserve current detailed columns, folder/type icons, marquee, internal read-only image/text/code/Markdown preview, original controllers, source identities, worker/Job/leases and password authorization.
- Integrate approved blue Nx application and orange Nx archive PNG originals through existing official resource/release owners; true 16/20/24/32/40/48/64/128/256 icon frames, no real associations or cache reset.
- Share the current browser official caption, fix duplicate native/custom captions and minimum-client scrolling. Compact progress distinguishes actual committed output from processed codec statistics, truthful unknown totals and original cancellation semantics.
- Persist only submitted reusable preferences through original locked disk-reread/field-merge writer; no passwords, grants, source IDs, ordinals, running state or stale selection context.
- Tree held gestures use original virtual-file OLE chain and budgets. Root drag adds one safe display-derived folder including directory descriptors, without flattening or truncation.
- Replace one-window-per-source presentation with one browser host containing original independent browser entities. Existing ArchiveApplication session/source/entry/preview/lease owners remain. Same canonical source reactivates an existing tab without another read. Different sources add/activate tabs in the host. No tab session restoration across restart.

## Browser host lifecycle / official selection record

The smallest current gap is ArchiveApplication::open_canonical opening a new native window for each distinct canonical path. The original windows map remains the per-session registry, not a map of independent HWND owners. A private BrowserHost renders official TabBar/Tab and holds strong references to the original ArchiveBrowser entities. The application keeps the existing weak references and owns cleanup. New browser_host.rs plus localized application.rs/launch_entry.rs integration are necessary; browser.rs only needs caption hosting and tab-switch gesture retirement after its independent Tree change settles.

Exact released Component0.8.0 tab/tab_bar.rs exposes selected_index, track_scroll, menu, max_width, children and on_click; Tab exposes label/aria_label/suffix/on_click, NOT a close builder. Use default Tab variant, Medium32DIP, max_width and official overflow menu. A suffix official XSmall ghost Button owns close with stopped propagation, not an additional selection dispatcher. Public parent TabBar overrides child selection callbacks intentionally. No custom painted tab/control or vendor patch.

The approved browser TitleBar is released Component0.8.0 title_bar.rs:34DIP height/34DIP control width/12DIP left inset/Small icons, native WindowControlArea Min/Max/Close/Drag, live is_maximized restore icon, semantic theme hover/pressed/danger close. Shared caption stays one per HWND. Active browser content remains its original entity and scroll/filter/tree/selection/sort/preview state; switching cancels only transient pointer candidates/marquee and focuses the current original browser. Inactive read completion must not steal focus or activate its tab. Window X retires every session; tab close/F4 retires only that session; last tab closes host. Host identity guards deferred updates so close/reopen cannot mutate a replacement host. Nested depth and parent lease are unchanged and independent.

Native foreground uses existing typed PlatformCommand::ActivateWindow owned by C#; restore/activation must not touch progress/preview/other windows. Source normalization remains original background fs::canonicalize and existing path identity; Windows case-equivalent canonical paths must reuse the same tab. No filename-only identity or synchronous cross-entity callback writeback. Open-request acknowledgement remains independent of metadata load.

## Pre-implementation reference verification

Read archive.md and archive-virtual-drag.md before current code. Rechecked licensed Archiver gui/src/main.rs AppWindow::new115 -> callback wiring -> ui.run158; activate669 -> view_item697 and selected extraction-before-open remain behavior references; shared temp recursive cleanup159 is not copied. SupaZip nested GUI main.rs run_native682 is eframe lifecycle, not a GPUI or tab controller. Legacy tracked grep again identifies package/FFmpeg ZIP/image/video owners, not a general archive browser; this is a bounded tracked search, not a claim about all generated/untracked sources.

Current production chain: launch_entry::handle_launch_intent/open_internal -> deferred ArchiveApplication::open_with_completion -> background canonicalize -> open_canonical -> original ArchiveReadClient start -> deferred consume/install. Original close -> retire_browser -> cancel read/entry/preview + release parent leases; no worker slot release invented. The host changes presentation ownership only. No proprietary native lifecycle or Listary native source was available/inferred.

## Red-capable seams / diagnostics

User-approved tests: real open request -> native window handles and actual official tab pointer selection/close; original read transport results and stale cleanup; actual Tree down/move -> original virtual command; actual root descriptors/content paths; production progress state/render/hit bounds; actual disk reopen/new process/default/corrupt/concurrent field merge. Work one vertical slice at a time. Keep existing right-panel drag/marquee/preview/password/cancel tests.

Tabs emit bounded archive.browser.tab_opened/tab_activated/tab_closed and host_closed events with session/host/correlation IDs, counts/reason/elapsed/native code only. No paths/titles/member names, pointer/frame or byte ticks. Native activation receipt is not user-confirmed foreground.

## Authorization / verification

No packaging, signing, installer execution, product launch, service/Registry/COM/file-association mutation, system icon cache clearing, commit or push. Source integration of existing packaging resource lists is allowed; actual build/package effectiveness is reported separately. No dependency/version/vendor change. All Cargo is one checked serial lane with jobs1/incremental0/codegen256. C# exact audited LIST -> same RUN only. One final full Desktop after convergence; relevant strict lint/check/fmt and spec/catalog/release/editor/source gates. Real HWND/physical/DPI100/125/150/200/IME/UIA/user visual acceptance remains NOT RUN unless separately authorized. Installation diagnostic chain is unchanged.

## Initial observed red

`cargo test -p nxtools-desktop browser_open_requests_share_host_and_duplicate_does_not_reread -- --test-threads=1`: actual TestWindow IDs1/2 differed, 0passed/1failed/1369filtered. The first integration compile then identified Button ElementId and missing ButtonVariants import plus an in-flight form syntax state; retained logs are archive-tabs-red.log and archive-tabs-green1.log in this session scratch. Neither compile failure is reported as green. Native ActivateWindow was rechecked at WindowsPlatformAdapter.NativeMethods.cs258: IsIconic -> ShowWindow(SW_RESTORE) -> thread attachment -> BringWindowToTop/SetForegroundWindow/SetFocus -> foreground/focus readback; reuse, no adapter behavior change.
