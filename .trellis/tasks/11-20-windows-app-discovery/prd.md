# Windows application discovery

## Goal
Quick Launch finds Windows built-in applications as a supported class, not a sequence of per-user-request entries. Calculator and Paint are required examples alongside Notepad; installed packaged applications are discovered dynamically through Windows AppsFolder.

## Acceptance
- Calculator/Paint have Chinese and English searchable names and valid original native launch routes.
- AppsFolder supplies localized display names and AUMID targets for installed packaged apps, including unknown future packages without adding a registration whitelist.
- Known Windows package aliases improve bilingual discovery without manufacturing entries for absent packages.
- Native snapshots merge with the existing Finder catalog; filesystem refresh preserves them, failed native enumeration retains last-good data, successful empty/replacement removes stale packages.
- Original ranking/history, query/session/generation, deferred GPUI callbacks and cancellation remain authoritative. Shell namespace applications cannot advertise file preview/reveal/elevation.
- Correlated bounded native enumeration runs off the serial command reader and UI thread; no names or AUMIDs in diagnostics.

## Scope
Finder catalog, original QuickLaunchApplication refresh, typed protocol and C# AppsFolder/Shell adapter. No UI redesign, indexer engine/service changes, dependency upgrades, history reset, package/install/live Registry/Shell acceptance.
