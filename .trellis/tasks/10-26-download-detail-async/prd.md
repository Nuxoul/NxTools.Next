# Audit #12: asynchronous selected download detail

## Scope / baseline
Implement directly in this session, no delegation. Live dirty baseline captured 2026-10-26T07:39:57Z: 398 files and 3750 prior evidence files, including audit #11. No HEAD rollback, real data/network/native/product/full Desktop/package/install/commit.

## Verified gap and source chain
Production render_downloads defers an entity callback, but request_task_detail synchronously calls window_support::load_download_task_detail -> engine.handle_command(TaskDetail) -> engine/projection.rs::command_task_detail -> task snapshot + SqliteDownloadStore::load_transfer_plan / load_task_timeline(300), both taking the original Mutex<Connection>. refresh_snapshot also synchronously reloads selected detail. defer only releases the currently leased entity; it is not background IO.

ABDM first: ProgressDownloadPage::ProgressDownloadPage / RenderPartInfo renders the original monitor item, BaseSingleDownloadComponent::extraDownloadProgressInfo combines scoped streams; close/cancel/onDismiss retire original component presentation. Then legacy DownloadTaskViewModel header/cookie/segment projections and AddLog(300), MainWindow OnOpened/OnClosing/OnClosed with HideToTray were reviewed. No AWT/Avalonia native internals are inferred. Next retains docked five TabBar Small underline tabs, original DescriptionList/state components, fixed clients and rem list measurement. This is request lifecycle, not visual redesign.

## Smallest change boundary
- Engine projection adds a narrow background TaskDetail admission on the existing download runtime's spawn_blocking pool. It still invokes the original typed command/store; no engine, connection, schema, transaction or protocol replacement.
- Original DownloadApplication owns task ID + request generation + correlation/pending + one actual worker. Rapid changes coalesce to one latest pending request; cancellation never pretends blocked IO stopped. No lock or GPUI lease across await.
- Original MainWindow synchronously retires its request token on selection/filter/module/close/release, then defers original application work. A request installs pending before dispatch. Completion is next-turn, checks current task/generation/token/session and cannot publish stale success or failure. Cache hits/duplicates remain nonreentrant.
- Refresh routes through the same bounded owner instead of a second synchronous detail path; live progress reconciles with the accepted detail.
- Keep original loading/error, selected full Header masking, timeline bound, scroll/focus/selection/batch/context/OLE and audit #11 projection. No #13 retained text or general async snapshot rewrite.

## Confirmed regression seams
User explicitly requests real request/storage boundary. First red: hold the actual synthetic engine store connection lock on a fixture thread, invoke original request_task_detail from its deferred UI owner, and schedule an unrelated next-application-turn UI update/release. A separate 3-second fixture watchdog prevents scheduler deadlock and must NOT be the unlock source. This demonstrates UI progress without fragile duration-only assertions. No fake result loader. Later real GPUI pointer selection/close/ABA and actual temporary SQLite failure/retry/cache/duplicate/late-result tests use the same production request and command path.

## Expected files / checks
Desktop downloads.rs + cohesive private detail_request module/tests, MainWindow/detail page lifecycle integration, existing window_support correlated loader, narrow engine projection API and feature-gated real-store test seam; Cargo feature/dev dependency only for test seam. References append verified chain before production edits. Preserve unrelated live changes and old evidence. Run focused details/DownloadPage/Downloads lifecycle, strict tests/production, workspace check, scoped/workspace fmt and source/provenance/document gates under natural-idle jobs1/incremental0/codegen256, serial. No full Desktop/network/native/release.

## Diagnostics
One original correlation ID reaches TaskDetail engine logging. Desktop records accepted/start/cache/retired/terminal with IDs, generation, counts, elapsed and typed code only. No URL/path/name/header/cookie/payload or raw error Display. No render/byte/pointer logs. Failure retains actionable original error state; stale results are quiet business-wise.
