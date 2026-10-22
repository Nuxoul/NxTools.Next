# ZIP extension import and Kit confirmation

## Goal
Replace the user-rejected raw technical text wall with a readable official Kit confirmation, and let ordinary users import one ZIP package that NxTools validates and installs into its own managed extension directory.

## Confirmed evidence
User supplied screenshot rejects current confirmation presentation. `rust/apps/nxtools-desktop/src/main_window/extensions_page/workspace.rs` currently appends name/source/hash/runtime/actions/permissions as plain text. `rust/apps/nxtools-desktop/src/extensions.rs:263–341` previews JSON/directories and confirms through original Core import. `rust/crates/nxtools-extensions/src/bundle.rs:39–103` stages script code into content-addressed managed directories; no ZIP intake exists. `csharp/NxTools.PlatformHost/Windows/ExtensionManifestPicker.cs:9–18,49–75` currently selects JSON. Workspace already provides zip8.3.0; no dependency upgrade needed. Reference note `docs/reference-code/winshun.md`; read-only WinShun Install.cpp inspect/unpack/place/discard verifies ZIP root/single wrapper, temporary candidate cleanup and owned placement. Reference behavior is evidence, not implementation to copy or weaken Next path checks.

## Requirements
R1. Ordinary Extensions import accepts ZIP, not loose JSON/manifest/directory. One root extension.json or exactly one enclosing package directory is accepted. Existing installed registry and lower-level developer JSON/directory APIs remain compatible; no second ordinary import mode.
R2. Rust validates archive and manifest/files before confirmation without writing a live package. Reject traversal/absolute or drive/UNC/ADS paths, Windows reserved/ambiguous/case-colliding names, duplicate entries, links/special entries, encrypted/unsupported/corrupt archives, ambiguous/multiple roots and missing listed files. Bound archive bytes, entries and actual decompressed bytes against existing package budgets; never trust declared compressed sizes alone. No generic shell extraction.
R3. Confirm writes an immutable package snapshot under existing config-parent/extensions/<id>/<content-hash> via original staging/config transaction. Cancel or failed validation does not install/change config; failed replacement retains original registry/package. Source ZIP is not a runtime dependency and can be deleted after installation. Do not delete old/unrelated snapshots or broaden garbage collection.
R4. Real released Kit0.7.1 Dialog with title/description, structured read-only DescriptionList, semantic feature/permission sections and Accordion technical details. Name/version/purpose first; source package basename prominent/full path subordinate; clear import-versus-replace outcome. Exact whole-user permission warning remains visible, not hidden in technical details. Hash/runtime budgets/entry filenames stay available in collapsed technical details. Explicit default32px confirm/cancel footer, proper primary/default variants and busy/error state. No custom palette or control geometry.
R5. Dialog lifecycle uses one original Root/overlay/focus owner, current preview generation/content identity; deferred cross-entity callbacks; no duplicate Enter/click commit. Cancel/Escape/close retires candidate; no auto-trust/enable. Import success shows clear next steps to environment/trust/enable, failure retains actionable retry. Keep1060x650/min920x480 and original176px workspace rail.
R6. Native C# picker returns ZIP path with fixed failures/native codes and silent cancellation; Rust owns archive parsing. Bounded accepted/validation/confirm/terminal diagnostics with correlation, no archive contents or payload paths.

## Acceptance criteria
A1 (R1–R3): Actual owned ZIP root and single-wrapper import -> preview -> confirm -> config reopen, installed bytes independent of removed source ZIP. Cancel no config/live-directory change. Invalid/corrupt/path escape/bomb/duplicate/link fixtures reject without outside writes. Replacement failure preserves last-good package and authorization semantics.
A2 (R4–R5): Actual GPUI pointer/keyboard/close/generation/busy/failed-save tests; default/minimum viewport and long Chinese content. Real Kit compounds/semantic controls adopted, details disclosure works, footer reachable, exact warning visible. No synchronous cross-owner lease reentry.
A3 (R6): Owned native picker fixture confirms ZIP filter/current path/cancel/failure; no real user dialog required. No automatic interpreter/plugin execution.
A4: Affected Rust/Core/Protocol/Desktop and C# fixtures, strict scoped lint/workspace tests compilation/fmt/source/spec. Actual isolated GPUI Light/Dark default/minimum confirmation screenshots individually inspected; user acceptance separate.

## Out of scope
RAR/7z, network catalog/download/update, signature marketplace, automatic interpreter/dependency install, package-defined UI, permission sandbox, W10, installed services/user data/ACL/registry, release packaging/install/commit/push. Prior runtime settings work preserved.

## Planning status
Requirements converged; user explicitly approved final summary with “开始！”. Implementation authorized within this scope; user visual/installed acceptance remains separate.
