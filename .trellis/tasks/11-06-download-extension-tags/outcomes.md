# Download extension tags — delivery

## Result

The selected category's extension field uses an icon/name header, wrapping outlined removable Tags and an outlined Add pill below the bounded tag area. Add opens the original in-window field with Enter confirmation and Escape/Cancel. Leading dots/ASCII case and duplicates normalize; compound suffixes remain intact. The original explicit Apply/Cancel, category directory/order/enabled state and URL-rule semantics are unchanged.

Production changes are limited to `downloads/settings_view.rs` and its private `category_extensions.rs`; the original category layout test accepts natural tag height while the ordinary name/directory/URL inputs retain32DIP. No dependency, vendor, palette, protocol, download engine, native window or service change.

## Verification

- New production tag tests:3passed,0failed. Covers actual pointer removal/no autosave, original Add/input/Enter, bulk duplicates and tar.gz, pending draft projection, Escape cancellation, saving-disabled mutation and scroll reachability above the fixed footer.
- Existing category/settings preservation:7passed,0failed. Category list/search/layout, engine validation ranges, failed-save rollback, sort preference and original picker/Apply/Cancel passed.
- Download UI lifecycle, GPUI release provenance and pinned Markdown editor source gates passed.
- Desktop strict Clippy remains blocked by preexisting `archive/browser_drag.rs:448` unnecessary_lazy_evaluations; that file is unchanged by this task. `cargo clippy -p nxtools-desktop --tests -- -D warnings -A clippy::unnecessary_lazy_evaluations` passes with that command-line exception. No source allow was added. Scoped rustfmt and diff whitespace checks pass; spec navigation checks126documents/14owners.

## Background-process explanation

Earlier read-only process/service observation identified an ordinary Desktop-owned PlatformHost, independent `NxToolsIndexerV1` LocalSystem indexing service, and independent `NxToolsUpdateV1` LocalSystem update service using another PlatformHost executable. Desktop was still running at that observation; invisible UI did not establish process termination. One additional PlatformHost had an exited parent but unreadable path/arguments, so its provenance was not established.

The latest read-only snapshot contains no NxTools.exe, NxTools.PlatformHost.exe or NxTools.Indexer.exe; both named services are Stopped with PID0 and Auto startup configuration. No process termination, service stop/disable or service-configuration change was performed in this task. Automatic service startup and independence from the desktop are existing product behavior, not a leak diagnosis. Ordinary Desktop startup supplies --parent-pid and owns supervisor disposal; no live exit regression or installed lifecycle repair is claimed.

## Limits

No package, installation, installed GUI screenshot, physical/DPI interaction or user visual acceptance was performed. Changes remain in the working tree. The supplied screenshot defines the shape; test bounds and component adoption do not certify visual equivalence.
