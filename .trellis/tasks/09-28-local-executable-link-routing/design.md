# Technical Design

## Current behavior gap

`link_detector::classify` recognizes existing drive and UNC paths but stores only display/copy strings. `LinkPanelView::render` then treats every clicked value as a browser URL when auto-open is enabled, including `C:\...\Throne.exe`. The existing PlatformHost shell boundary already supports the required Explorer selection semantics.

## Typed target contract

Extend `DetectedLink` with a small target kind:

```text
Web
LocalFile
LocalDirectory
```

The detector continues to split conservative candidates and only accepts existing filesystem paths. For a path, use the filesystem kind at classification time; do not infer file-vs-directory from `.exe` or another extension. The reported executable is therefore covered by the same rule as every existing local file.

Keep `display` and `copy_value` unchanged so clipboard behavior does not change. A pure route helper in the panel module maps the kind plus settings to either browser-only/copy-only or one existing `ShellOpenAction`.

## Click lifecycle

The row callback remains ordered as follows:

1. log bounded kind metadata;
2. remove the panel window directly;
3. update the owning application only to record the copied echo and forget the panel handle;
4. queue the existing shell command for local targets, or launch the browser for a web target only when settings allow it;
5. write the original value to the clipboard.

The route helper must not own GPUI window removal or clipboard writes. Local files never enter the browser URL construction branch. Local directories use shell `Open`, while local files use `SelectInExplorer`.

## Diagnostics and tests

Replace raw `%copy_value`/`%url` fields in click/browser events with `target_kind`, bounded length, and action/accepted fields. This satisfies the diagnostic contract without exposing local paths, URL query data, or clipboard secrets.

Tests should cover:

- existing `.exe` fixture -> `LocalFile` -> `SelectInExplorer`;
- existing directory fixture -> `LocalDirectory` -> `Open`;
- HTTPS and bare domain -> `Web` and configured browser/copy-only mapping;
- nonexistent drive path -> no detection;
- copied row closes before echo capture can reopen the panel;
- route diagnostics contain no raw target value.

## Compatibility

No C# or protocol change is needed. `ShellTarget.TryCreate` continues to reject missing local targets at the native boundary and `ShellTarget.Launch` continues to call `explorer.exe /select,"target"`. If a file disappears between capture and click, report the existing correlated command failure; do not fall back to a browser.
