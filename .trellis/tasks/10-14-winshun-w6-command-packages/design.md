# W6 design

Parent ../10-14-winshun-w6-w9/design.md is canonical; this approved first slice is source-completed with parent joint acceptance. Historical W6 evidence and final parent outcomes distinguish source from unperformed installed/Release checks.

## Module/interface
Add host-neutral nxtools-extensions manifest/package/catalog types using existing serde/URL/UUID/SHA256/error dependencies; Core composes default-empty registry/enable/settings state without a dependency cycle. Preserve package identity, do not convert imported JSON into indistinguishable manual records. Manifest version1 supports declarative handlers/action descriptors and future required feature versions; unknown required capabilities fail preview. Source files and pure JSON packages follow parent limits/staging.

## Data flow
Read JSON -> bounded strict validation -> preview/provenance/conflict table -> user explicit import -> stage/sync/publish registry -> disabled package -> explicit enable -> existing Finder/Desktop action adapters. Preview/import/search cannot execute programs. Standalone package JSON needs no runtime/files; handlers declare module/action context. Website template percent-encodes substituted words using URL rules; program template uses existing prepare_custom_command separation, folders typed Shell open, fixed list owns stable item IDs. No imported runas or cmd/PowerShell wrapper shortcut.

## State/conflict/rollback
Normalized package/handler/item IDs bounded; keys/aliases checked against manual and enabled-package commands before enable. Show conflict and require alias change/disable, no implicit priority winner. Same-ID update preview replaces selected version only after commit, default-disabled if declarations change; unaffected settings retained by stable setting ID/type. Temp config error retains old runtime/catalog and registry. Core atomic section save rereads latest config; stale module snapshot never overwrites another section.

## UI/public API
Tools Extensions manager uses Kit0.7.1 Settings::new/pages, SettingPage::resettable(false), SettingGroup/SettingItem/SettingField public render slots for package lists and preview; real Form/Field for aliases/settings, DescriptionList for source/action details and Alert for risks/conflicts. Main existing1060x650/min920x480 layout, footer reachable through official scroll. Do not add private Settings state or whole old page wrapped as one custom element. Manager pointer/Tab/release once-only actions defer Core/application saves.

## Compatibility/verification
Default empty section/no packages means no extra runtime/code execution and same current Finder results/manual elevation. Real temp file/config import and actual Finder command/GPUI queue tests go red first. Package failure rollback and Unicode/argv/url/fixed list behavior are acceptance, not helper shape tests. Parent requirements/verification/privacy apply; scripts remain unavailable until second child joint gates pass.
