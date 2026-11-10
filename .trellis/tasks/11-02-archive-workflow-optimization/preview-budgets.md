# Configurable archive preview budgets

Parent: **Phase E — Format expansion and ZIP editing / in_progress**. This is preview stabilization in the existing task, not another OPT package.

## Requested result and boundary

User2026-11-09 asks for Bandizip split-format support, more useful preview resource limits and settings controls. Implement only configurable archive-preview budgets and their original consumers. Preserve unrelated dirty work, existing read/worker/cache/Job/preview ownership, whole-member integrity, source leases, current two-operation/decode admission, cancellation and true cleanup. No package/install/deployed-file replacement/registry/service action, new codec/vendor/dependency, new preview renderer or commit/push.

Existing limits are64KiB/1200lines for text and8MiB/8192axis/8Mi-pixels for images. Fixed32MiB JPEG/WebP decoder admission plus9MiB input estimate rejects even1024×768. Shared Clipboard/Quick Launch rich-document defaults are separate and must not increase.

## Settings and defaults

Add serde-default `ArchiveSettings.preview: ArchivePreviewSettings` in Core, strict object with six u32 fields:

| Field | Display | Default | Allowed inclusive |
| --- | --- | ---: | --- |
| `text_limit_kib` | 文本大小上限（KiB） | 1024 | 1..8192 |
| `text_max_lines` | 文本最大行数 | 20000 | 1..100000 |
| `image_limit_mib` | 图片文件大小上限（MiB） | 32 | 1..128 |
| `image_max_megapixels` | 图片最大像素（百万） | 16 | 1..64 |
| `image_decode_mib` | 图片解码预算（MiB） | 512 | 32..1024 |
| `timeout_seconds` | 解压工作时限（秒） | 120 | 5..120 |

MP is1,000,000pixels; KiB/MiB are binary units. Validate before save/use; no unlimited value. Missing preview settings adopt these new defaults without rewriting config on opening. Merge only changed preview fields under the original section lock, preserving newer unrelated/preview settings. Settings changes apply to subsequent selection/refresh, not a running preview's immutable snapshot.

UI: existing official Kit0.8.0 Settings/SettingPage/SettingGroup/SettingItem/SettingField::number_input(NumberFieldOptions), default32DIP controls, original page scroll and deferred save owner. Add a distinct 预览 page with text/image/work groups and concise simultaneous-limit/next-preview help. Original input/pending/error behavior remains; no nested scrollbar or custom number control. No vendor patch. Existing normal/max fields and minimum window are tested through production GPUI.

## Wire/native contract

Add `preview_options: Option<ArchivePreviewOptions>` to Rust ArchiveEntryCommand, defaultNone/omitNone. Strict object `{max_bytes:u64,timeout_ms:u32}` is valid only for `preview`/`preview_text`. `max_bytes` must be1..8MiB for text or1..128MiB for image; timeout200..120000ms. Absent/null retains legacy64KiB/8MiB and30000ms behavior for every action. Non-preview non-null options, unknown/duplicate/malformed fields and excess values reject. Current Desktop always sends options derived from its validated snapshot.

C# adds the symmetric strict nullable record and validates before native effects. Pending retains original preview purpose/options; selected OutputLimit is min(original internal caller limit, configured purpose limit). Start uses the configured TimeoutMs only for the original preview. Strip PreviewOptions when converting the browser-entry command to private materialize payload; do not change the worker browser-entry schema. Native working/metadata/source/spool budgets, ordinary extraction/materialization/drag/authorize paths, source authority and slot cleanup remain unchanged. Existing accepted/start/terminal correlation, native cause and cleanup diagnostics stay; no payload/path/config-value logs.

## Decode and presentation contract

Freeze full Core preview settings in PreviewRequest; carry the text rendering budget through PreviewPayload to the existing document backend. Preselection metadata, actual bounded file read, decoder and rendered document must use that snapshot. A limit changed while loading cannot alter that request or admit its stale result. Selection/tab/source/generation retirement stays original.

Text uses configured bytes/lines while retaining UTF-8/BOM and control-character policy. Parameterize the existing shared safe-document pipeline for archive callers only; retain original Clipboard/Quick Launch64KiB/1200 defaults and Markdown depth32/node8192 policy with literal fallback. Avoid a second renderer or global default increase.

Images retain supported8bit/static formats, fixed8192axis and all format-specific admission. Use configured byte/pixel/decode limits; JPEG/WebP conservative working admission uses actual encoded length plus fixed slack, not the configured maximum as if it were allocated input. Preserve bounded decoder/output pixel memory and no animation. File reads must not preallocate a configured128MiB for a tiny member: use validated actual file length for bounded allocation. No removal of codec limits or false global-memory guarantee; decoder budget is not whole-process memory.

Messages identify member byte/line, pixel/dimension, decoder-memory or underlying archive read/resource failure with the relevant configured threshold where known. Do not turn a source/spool/solid-work failure into a claim that the selected member is oversized.

## Verification seams and order

Use existing public/settings/native seams: Core config serde/validation/changed-field merge; strict Rust/C# ArchiveEntry wire; production ArchiveSettingsView controls and persisted save; actual preview selection command; decode_bytes/document rendering; original ArchiveEntrySession/worker/native output. Add red-capable cases at those seams before implementation, one vertical slice at a time.

Required cases: new defaults/legacy JSON/no opening write; boundary/invalid values; stale settings merge; actual UI field change/save/scroll; >64KiB text and >1200lines admitted under new defaults but refused under narrower settings;1024×768 JPEG accepted with appropriate budget and rejected with a deliberately low one; pixel/byte/line limits, actual input accounting and type/cancel policies preserved; native preview above legacy size succeeds only with valid options; absent options stay legacy; options do not authorize materialize/drag or leak to worker; timeout reaches original runner; immutable settings snapshot and stale/cleanup/retained-preview ownership.

Every Cargo command sets jobs1/incremental0/RUSTFLAGS codegen-units256, one global lane, long timeout. Run affected Core/Protocol/IPC tests and preview/settings/related document tests, one Desktop full suite and affected strict Clippy after implementation. C# exact reviewed fixture list/run, actual worker when retained fixture available. One bounded review maximum, concrete fixes/affected retests only. Final spec/Kit/editor/source checks are separate from installed/DPI/user acceptance.
