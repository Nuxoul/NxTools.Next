# Application catalog boundary

Listary managed Class291 reads FOLDERID_AppsFolder in an STA, keeps nonempty package-family/AUMID/display-name entries and supplies its application provider. Legacy NxTools has no Quick Launch implementation. Reference details remain in docs/reference-code/quick-launch-listary-results.md.

C# query_installed_applications returns data JSON applications[{name,target}], target shell:AppsFolder\\AUMID. Windows enumeration and Shell activation remain native; Rust owns merge, aliases, history and rank. Unknown installed packages use their actual localized names without a registration list.

Finder retains root and installed snapshots separately and rebuilds one merged searchable catalog, preserving native rows across root refresh. A single descriptor mapping supplies bilingual aliases and suppresses the classic fallback when its modern package is installed. Classic calculator/paint fallbacks use calc.exe/mspaint.exe. Accepted native snapshots alone change installed membership; failures retain last-good rows.

QuickLaunchApplication uses one pending correlation/connection state installed before dispatch. Reopen can request a fresh snapshot; concurrent requests coalesce. Disconnect/timeout/disable retire pending ownership. Current success updates the original catalog and reuses its current-query refresh; no late result reveals or focuses a window. Native targets open through the existing Shell command; file-only capabilities are unavailable.
