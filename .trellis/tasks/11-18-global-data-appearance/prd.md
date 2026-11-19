# Global data and appearance settings

## Scope
- Global Settings shows the active software data directory. Choosing a valid empty local directory immediately starts migration through normal resource drain and automatic restart.
- Persisted configuration, launcher/download databases, managed Clipboard history/assets and extension packages/data move together. Independently configured Clipboard storage and downloaded files remain at their explicit locations.
- The default layout remains unchanged. A fixed bootstrap locator selects custom config/data subdirectories. Runtime cache, locks, logs, native registration/signing material and LocalSystem indexer data retain their existing system locations.
- New/missing theme preference defaults to System; explicit persisted choices survive.
- Appearance allows selecting installed UI fonts, using the existing Apply/Cancel transaction.
- Firefox interception is compared with Mister Download Manager and Ghost Downloader and corrected without changing Chromium handoff semantics.
- Extensionless unfinished tasks request the same Shell generic file resource as completed-file fallback.

## Invariants
Directory selection is an immediate operation, independent of unapplied settings drafts. Native picker cancellation, stale/closed source and disconnect do not start migration. Busy state prevents duplicate migration and Apply/close. Cross-entity work is deferred. Normal shutdown drains downloads, extensions and Clipboard before copying. Source remains intact until destination is durably committed; failed copies/commit leave the previous locator authoritative and restart from the old location. Copy does not overwrite nonempty targets. Managed asset/package references are rewritten, not user content or external download paths.

## Components
Existing Kit 0.8.0 Settings/SettingPage/SettingGroup/SettingItem public render slots; Medium outlined Button for directory selection, visible active path and restart notice. No Toolbar.child adaptation. Font selection uses official searchable Select and existing drafts. Same 720x480 client with official scrolling; no vendor changes.

## Diagnostics
One UUID spans picker admission, selected/validated, resource drain, copy, commit and restart. Only stage, elapsed time, counts and native error codes are logged; no paths, file names, payloads or credentials.

## Verification
Temporary-directory tests cover default/custom resolution, directory validation, destination collision, copy failure/locator failure and Clipboard managed asset rebasing. Production GPUI tests cover picker command correlation, cancellation/closed-owner handling, busy controls, font draft/apply/cancel and theme propagation. Extension behavior tests cover pre-download Firefox interception and fallback. Release/physical Firefox/installed font/DPI acceptance is separate from source and fixture evidence.
