# Correction: built-in extensions missing on a new computer

## User authority / observed failure
The user expected the supplied extensions to be built in and reports that a new computer shows none after installation. Sequence10 only delivered independent ZIP artifacts; its signed payload has zero extension package files and AppConfig::load_or_create initializes an empty registry. Reproduction command against the actual sequence10 manifest returned PACKAGED_SAMPLE_FILES=0 and all3 source ZIPs present. This is a delivery/integration omission, not an interpreter or trust-state explanation.

## Smallest change and owners
Embed the exact manifest-listed source bytes of the existing3 examples in the production Rust extensions library, using compile-time include_bytes; no external development path/ZIP/network dependency at runtime. Core load_or_create seeds these into original config-parent/extensions immutable snapshots and the original PackageRegistry under the existing locked reread transaction before feature services receive the config. A serde-default one-time registry marker distinguishes an old config awaiting seeding from an explicitly removed/empty catalog. Do not alter AppConfig::default, ordinary load_from or config save semantics outside this bootstrap.

- Built-in packages: example.github-public-proxy, example.download-link-list, example.clipboard-text-tools; exact existing names/files/handlers, no script redesign.
- All start disabled/untrusted; automatic events/providers and GitHub third-party opt-in remain off. Python is not installed/downloaded/approved automatically.
- Existing same-ID entries always win, preserving content, settings/runtime/trust/enablement. No silent update or replacement. Repeated load does not rewrite config or restore deleted items. Corrupt config/stage/commit failure must not overwrite last-good configuration or claim initialization complete.
- Rust owns compiled resources/initial state, original immutable stage and Core config lock; existing GPUI catalog renders committed registry. No new controller/store/menu/layout or C#/IPC/native owner.
- Expected files: new extensions bundled resource module/export; registry bootstrap marker; Core bootstrap module/load_or_create composition/regressions and existing tracing dependency for bounded lifecycle diagnostics; reference/spec/task/example overview docs; release evidence. Product scripts and per-package READMEs remain byte-identical.
- Diagnostics: extension.bundled.accepted/staged/terminal, one startup correlation, bounded counts/elapsed/success only; no script contents/paths/payloads and no per-frame/polling.

## Reference review
Read docs/reference-code/winshun.md first, then read-only D:/Code/WinShun-main/src/plugin/Catalog.cpp14–29/35–75: scans user entries before bundled entries; same-ID user wins. Enabled IDs are separate state. Next intentionally uses immutable staged bytes and one-time locked config registration, not filesystem watcher/Qt discovery. Existing NxTools has no equivalent plugin seed path. Preserve Next import/trust/runtime/effect and native popup lifecycles; this bootstrap adds no UI show/hide/focus/cancellation/native boundary.

## Verification / corrected delivery
Add red-capable Core test at actual load_or_create with an isolated new AppPaths, not only archive-preview tests. Cover defaults, code hashes, fresh/legacy/same-ID/preserved sections, repeated load, deletion, concurrency and failure bytes. Compare embedded previews byte/hash-for-byte with existing directories; package gate must verify all3 embedded markers in packaged production Desktop and first-run initializer call. Run affected Core/extensions/Desktop tests and strict/workspace/fmt/source/spec gates with workstation memory policy. Build corrected sequence11 only after rereading actual publisher ledger; same key, backup sequence10, no automatic install/deploy/launch/services/user scripts. User/native/visual acceptance stays pending.
