# Audit #11: bounded DownloadPage projection

## Scope
Implement directly in this session, without delegation. Remove eager full DownloadEngineTask, progress and icon copies before uniform_list. No real downloads/network, product/native activity, full Desktop run, packaging, installation or commit.

## Baseline
Captured current dirty bytes at 2026-10-26T04:30:58Z: 374 live files and 3242 immutable prior evidence files. HEAD is not the rollback basis. Evidence/baseline-manifest.json and historical-manifest.json identify exact inputs.

## Source/reference review
ABDM reviewed first: DownloadList.kt::DownloadList passes key={it.id} and the existing LazyListState to customtable/Table.kt::Table. Table remembers presentation sorting and composes itemsIndexed inside LazyColumn. BaseHomeComponent.downloadList filters status/category/search, removes duplicate IDs, then reconciles selection. Legacy MainWindowViewModel::{FilterTasks,SortTasks,SortBy,RebuildVisibleTasks} applies filter/search before stable presentation sorting and retains original object selection. MainWindow constructor/OnOpened/OnClosed and TaskRow_ContextRequested -> BuildTaskContextMenu retain original show/runtime/selection owners. Avalonia/AWT native framework internals are not available evidence.

## Design / invariants
- One original DownloadApplication task snapshot; change its storage to a shared immutable Vec, replaced only at the existing constructor/refresh/test replacement boundaries. No global cache, lock or second list model.
- download_tasks_for_view returns source indices, sorted by the unchanged stable comparator. Counts and IDs are derived from that same immutable snapshot. Queue default stays original order.
- uniform_list retains that exact snapshot and its lightweight index projection. Resolve index plus captured stable ID against that snapshot; never apply a stale index to a newer owner Vec. Read live progress/icons by the resolved ID only for requested range rows.
- Row construction borrows task/progress during synchronous composition; retained callbacks own stable IDs, existing entity handles and owned strings/path resources. No entity reference escapes its lease.
- Keep original selection anchor/Ctrl/Shift/batch/context/queue/OLE/deferred application/detail ownership, error state and scroll handle. No #12 detail IO/thread change or #13 text ownership change.
- GPUI pre0.3.9 uniform_list invokes its callback for one measurement row as well as visible ranges. Preserve rem48 row/28 header, all fixed clients and official InputGroup/Toolbar/PopupMenu/Tabs/Progress/DescriptionList variants and parent adaptations. Existing DataTable compatibility findings remain; no controller migration.
- No new diagnostics on repaint/scroll; existing low-noise business events remain.

## Regression first
Synthetic 1000/10000 tasks, substantial request and segment payloads, no runner. Observe the real render_downloads -> uniform_list -> download_task_row boundary: source payload identity, resource getter calls, requested ranges and actual row constructions. Deterministic bounded-work assertions are primary; timings are descriptive only. Verify repaint/end scroll, sort/filter/count/stable-ID refresh/removal and relevant existing interactions/rem measurements.

## Verification / evidence
Natural-idle serial Rust commands inherit jobs1/incremental0/codegen256. Run narrowed DownloadPage/Downloads and protected naming/sort/identity/rem regressions; strict production/test Clippy, workspace check, scoped/workspace fmt and relevant source gates. Preserve failed attempts and external blockers. Publish reversible dirty delta, actual English before/after measurements and distinct synthetic/native/visual limitations. No full Desktop run.
