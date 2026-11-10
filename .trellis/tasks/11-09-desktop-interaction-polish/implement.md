# Implementation

## Archive

- Core and protocol expose separate ZIP quick-create, 7z quick-create and editable New Archive commands. Fresh configuration and missing command fields recommend all three; every explicit saved command array is preserved.
- The existing out-of-process Shell helper implements selection-aware `IExplorerCommand::GetTitle` and `Invoke`. Each invocation forwards the complete ordered selection once. The correlation ID becomes the original archive Job operation/session ID.
- ZIP and 7z commands use normalized sources, `CreateDraft::requests` and the original FIFO/Job without opening an editable form. New Archive retains its original draft, validation, cancellation and publication owners.
- New Archive has a 144 DIP independent source viewport with filenames, sizes, full-path tooltips and original source actions. Settings use Download-aligned official filled groups, shared field rows, a 200-design-pixel control lane and a 164-design-pixel navigation seed.

## Download

- Each native icon query owns its raw `IImageList` reference and releases that exact reference. The existing HICON, bounded STA runner, per-key cache generation and cache limits remain authoritative.
- Invalid persisted PNGs regenerate through the original cache owner. A real PlatformHost disconnect retires connection-scoped failure budgets without discarding successful icons; duplicate Connected events do not reset retry limits. Missing icons use the official neutral File asset.
- The keyed task window remains 440×184 collapsed and 440×280 expanded, with a fixed footer and no collapsed-body scroll. Progress filename text uses 14/20 above 12/16 body metrics.
- Progress prioritizes filename, bytes/progress, speed and ETA. Resume/limits/connections/segments remain in the original expansion above the existing Table.
- Server information shows host, a short route and the observed HTTP request-to-headers duration. Semantic success/warning/danger colors cover 0–100/101–300/301+ ms. Missing observations have no duration element. Peer IP is not projected into presentation or accessibility text.
- The category heading, help, official Medium Select and Small outlined Add action share one wrapping row. The official search field and virtual list remain intact; public option content uses official Label with stable category-ID identity and accessible text. Select mirrors the original `selected_category`; Apply/Cancel and the existing section saver remain the only persistence boundary. Input-driven synchronization is deferred beyond the active entity lease.

## Quick Launch

- `result_entrance` offers Slide Up, Slide From Right, Fade and None. Released Base `Timing` and `Easing::EaseOut` drive one finite 120 ms Tween; movement is 8 DIP upward or 12 DIP from the right, with fading in animated modes.
- Admission is query-generation/identity scoped, including delayed provider batches. Offscreen arrivals settle; selection, reorder, icon updates, repaint and virtualization do not replay entrances. Reduced motion, None, unknown values and retired sessions settle immediately without a retained animation task.
- Ordinary typed search uses a stable, monitor-clamped 620×530 DIP expanded viewport. Empty/hidden/reopened search-only mode remains 60 DIP; action and recent-directory modes retain their original content geometry.
- The fixed focusable result Button wraps animated inner content. Selection, hit rails, focus, preview and native placement retain their original owners.
- The official settings dropdown updates only its field through the original section saver. Failed persistence does not change the live setting.

## Boundaries

Rust owns state/layout, C# owns Windows APIs, and GPUI owns rendering. Kit/Base/Component/Assets remain 0.8.0 with gpui-pre0.3.9. Diagnostics carry operation/generation IDs, fixed state categories, counts, elapsed time and native error codes without payloads or frame logging.

Reference and component evidence is recorded in [archive-slice.md](archive-slice.md), [icon-slice.md](icon-slice.md), [download-layout-slice.md](download-layout-slice.md) and [quick-launch-slice.md](quick-launch-slice.md). Verification and acceptance limits are recorded in [outcomes.md](outcomes.md).
