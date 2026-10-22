# Approved implementation design

## Boundaries
Keep Rust domain/configuration/layout, GPUI entity/rendering, and C# Windows ownership. Preserve current dirty source and prior installer fixes. No dependency upgrade or real service/clipboard/Shell operation during verification.

## Presentation
Use the released Settings/Form/Field/Input/Select/Menu/Toolbar components through public slots. Keep original input entities, keyed task windows, native placement and passive preview boundaries. A compact monitor contains readable filename, progress, a two-column metric area, explicit resume/limit status and expandable segment table; URL and destination remain available in main details rather than the progress window. Terminal transitions bypass periodic metric throttling. Tool status is secondary and notification cards have an opaque backing surface. Native overlay transparency must be verified at the compositor/window boundary.

## Native actions
Add a typed OpenWith Shell action and C# native chooser. Explorer reveal uses Shell folder selection/open first and executable fallback, followed by actual window/foreground verification. Register pending IDs before dispatch, preserve cancellation, error and task-window generation ownership. Indexer deployment distinguishes an intact current tuple from an intact old tuple; old approved tuple is an upgrade, not unconditionally reusable. Upgrade must require explicit UAC and use existing atomic deployment; never weaken pipe admission.

## Foreground-dependent limiting
Persist enable and rate in download settings. C# publishes bounded foreground identity changes via the existing event owner; Rust determines whether another application is active. The temporary limit is combined with user global/task limits without rewriting their persisted values. Restore immediately on disable or return to NxTools. No pointer/frame/byte diagnostic stream.

## Themes
Persist System/Light/Dark through the system-owned atomic config update. Kit active palette and custom business colors derive from the same selected mode. Existing windows are notified after leases release. A C# system-theme snapshot/change event supplies System mode; unknown/disconnected state uses the current safe default. Theme file import and external catalog bundling are deferred.

## Search
Precompute application name normalization/initialism/pinyin on catalog mutation. For base file names, group identical names and reuse scoring, then apply per-file path/type/exclusion and ranking unchanged. A selective auxiliary candidate index must be an optimization only: eligible substring requirements can narrow candidates, unsupported/short/wide/complex query shapes fall back to the complete scan. Compare old and new results using deterministic fixtures, record allocation/memory/time trade-offs, and preserve delta/cancellation.

## Diagnostics and evidence
Reuse correlated action lifecycle events; add low-noise theme/foreground-limit/deployment transitions with no payload secrets. Red-capable production boundary tests precede fixes. Source, compiled tests, inspected images, owned native fixtures, physical input and installed release acceptance are separate evidence. Real service restart/base+delta/search/idle CPU and packaging remain separately authorized.