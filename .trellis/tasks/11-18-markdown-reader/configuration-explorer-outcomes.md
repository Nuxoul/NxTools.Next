# Tool configuration and Explorer entry verification

The existing Tools catalog uses the reader's enabled state and the shared `配置…` window pattern. This window controls activation, reading defaults and Windows association registration. Manual preview remains secondary. Explorer hands off one eligible quoted Markdown path to the existing reader and does not summon the main window. Reading, title-bar edit mode buttons, drafts, history and export retain their owners.

## Directory regression

At the 680×460 reader minimum with a 184 DIP outline, the production tree placed adjacent title origins 34 DIP apart. The first short label measured 26 DIP, while the long Chinese heading extended to 130 DIP and overlapped the next row. The same test passes with width-constrained, single-line ellipsis, heading-depth indentation, full accessibility label and tooltip, and the original heading selection callback.

## Verification

- Desktop `markdown` filter: **32 passed**, including the red-to-green directory layout case, configuration window reuse, durable preference/failure publication, disabled/default-handler admission, direct reader routing, tab deduplication, and original editing/preview regressions.
- Desktop archive launch: **10 passed**; main-window preparation: **6 passed**. Indexer type-check passed for the expanded protocol. No indexer service or package was started.
- Core preferences: **1 passed**; protocol: **3 passed**; IPC: **4 passed**, including the actual isolated single-instance named pipe. PlatformHost built without warnings/errors; **5 scoped native fixtures passed** with controlled registry/query operations.
- Scoped Rust formatting and diff checks, spec/Kit/editor provenance and Desktop shell lifecycle gates passed. The previously failing full Desktop suite was not rerun for this correction.

## Installed acceptance

Windows owns the effective default choice. NxTools registers its Markdown opening option and reads back both extensions' actual status; it refuses to remove its handler while it is still selected. No live registration, change of default app, real Explorer double-click, physical input/DPI verification, installation or package was performed.
