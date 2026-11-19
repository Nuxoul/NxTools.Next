# Winc-inspired preview and Win11 default-app handoff

## Reader presentation

The existing GPUI TextView now uses reader-local typography based on the reviewed Winc 0.1.796 Markdown preview: centered 880/1240 DIP reading measures, responsive 20/32 DIP gutters, relaxed prose line height, six heading levels and H1/H2 divider rules. Code blocks have a dedicated language/copy lane, and wide tables scroll inside the document rather than changing its width. The active NxTools theme supplies the colors. Narrow and split views open the existing outline tree in a Kit Popover; wider windows can retain a 208 DIP sidebar. Both named edit buttons remain in the preview title bar with no F2/F3 binding. Document selection, tab identity, settings, exports and version history retain their owners.

## Windows default applications

The Markdown-specific native action uses the same registered application name as the existing HKCU entry and opens its app-specific Default Apps page on Windows 11. Only the established fallback opens general Default Apps on older Windows or missing registration. An `opened` receipt confirms only a Shell launch. Returning to the reader's configuration re-queries `.md` and `.markdown` independently. The action never changes Windows defaults directly; the archive action is unchanged.

## Verification

- Desktop `markdown_reader`: **14 passed**. A 680 DIP document no longer reserves an outline column; clicking the actual outline trigger leaves the prose viewport unchanged and displays the long Chinese heading on one line. The sidebar is verified at 1120 DIP. The code action copies the original source through the GPUI test clipboard. The settings receipt test confirms opening the page does not mark either extension default, and that readback can mark only `.md` default.
- Rust association protocol: **1 passed**. PlatformHost and its fixture harness built without warnings or errors; **5 targeted C# fixtures passed**, including the fixed Win11 URI, fallback and preserved archive action.
- Documentation, GPUI catalog/release, pinned editor provenance and scoped whitespace checks passed.

## Installed acceptance

Light/Dark screenshots, real Win11 Settings navigation, per-extension confirmation and Explorer double-click were not exercised in the installed application. No package, install or live association change was performed.
