# Configurable archive preview budgets

## Scenario: One frozen preview budget from settings to rendering

### 1. Scope / Trigger

A supported single regular member is selected in the existing ArchiveBrowser. The existing
Phase E — Format expansion and ZIP editing owns this stabilization. Source references:
[archive call chain](../../../../docs/reference-code/archive.md),
[Bandizip volume scope](../../../../docs/reference-code/archive-volumes.md), and
[preview plan](../../../tasks/11-02-archive-workflow-optimization/preview-budgets.md).
Evidence: `repo:rust/crates/nxtools-core/src/archive.rs`,
`repo:rust/crates/nxtools-protocol/src/archive_browser.rs`,
`repo:rust/apps/nxtools-desktop/src/main_window/archive_preview_settings.rs`,
`repo:rust/apps/nxtools-desktop/src/archive/entry.rs`,
`repo:rust/apps/nxtools-desktop/src/archive/preview.rs`,
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveEntryContract.cs`,
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.cs`.

### 2. Signatures

`ArchiveSettings.preview: ArchivePreviewSettings` is an additive serde-default strict object.
Its six u32 fields are validated before save/use. Missing section adopts defaults without a
read/open write. `ArchivePreviewSettings::merge_changed(baseline,desired)` runs within the
original section transaction and changes only edited leaves.

| Field | Default | Inclusive range |
| --- | ---: | --- |
| text_limit_kib | 1024 | 1..8192 |
| text_max_lines | 20000 | 1..100000 |
| image_limit_mib | 32 | 1..128 |
| image_max_megapixels | 16 | 1..64 |
| image_decode_mib | 512 | 32..1024 |
| timeout_seconds | 120 | 5..120 |

`PreviewRequest.profile` freezes the complete validated profile. Text payload carries
`TextBudget` into `prepared_source_with_budget` / `document_with_budget` on the original
readonly MarkdownDocument backend. Existing default functions retain64KiB/1200lines for
Clipboard/Quick Launch.

ArchiveEntry version1 adds omitted-when-None `preview_options: Option<ArchivePreviewOptions>`;
C# uses nullable `ArchivePreviewOptions(ulong MaxBytes,uint TimeoutMs)` with WhenWritingNull.
The strict object requires exactly `max_bytes` and `timeout_ms`. Text1..8MiB, image1..128MiB,
timeout200..120000ms. Missing/null means legacy64KiB/8MiB/30000ms; a non-null options object
is valid only on preview/preview_text. Private worker BrowserEntry schema is unchanged.

### 3. Contracts

The distinct 预览 SettingPage uses released Kit0.8.0 Settings/SettingGroup/SettingItem/
SettingField::number_input(NumberFieldOptions) and default32DIP NumberInput. Text/image/work
sections share the original Settings scroll and deferred save owner. No custom control,
nested scrollbar, vendor patch, additional preview controller or global concurrency change.
Changed settings apply on the next selection/refresh; current request and rendered/source
views keep the frozen profile. Current-generation source/tab/selection/ABA checks remain.

KiB/MiB are binary; one megapixel is1,000,000pixels. All simultaneous limits apply. The member's
actual length is checked before bounded allocation/read; a tiny file does not reserve the
configured maximum. Preserve UTF-8/BOM/control policy and Markdown depth32/nodes8192, inert
resource placeholders and literal fallback. One shared bounded counter treats CR, LF and CRLF
as line endings, counts CRLF once and preserves trailing-terminator semantics. Preserve8192axis,8bit/static/format-specific
image admission. JPEG/WebP conservative32bytes/pixel working estimate charges actual encoded
bytes plus1MiB slack, not configured input capacity. Decoder allocation/output allowance and
final RGBA pixel allowance are separate; neither promises a process-wide memory cap.

C# Pending keeps original preview purpose/options; output is min(internal caller limit,
configured purpose limit). Only preview changes the existing runner TimeoutMs. Before private
materialize serialization, convert action and clear PreviewOptions so null omission removes
it. Source/metadata/spool/solid-work budgets, working64MiB/metadata128MiB, global worker/decode
slots, ordinary materialize/drag/authorize authority, full-member integrity and actual joins
remain unchanged. Visible Ready retains its original lease; stale/failed/retired work releases
once after actual completion. No cache-hit synchronous GPUI reentry.

Browser-selected output byte exhaustion retains fixed `SelectedOutputLimit` /
`selected_output_limit` through the original worker/native/session terminal. Only that category
maps to TooLarge in preview. Source acquisition, metadata, spool and working-budget exhaustion
remain ResourceLimit / ReadLimit, including unknown-size standalone stream members.

Existing accepted/start/decode/terminal and settings save lifecycle carry the original
correlation and elapsed/native error/cleanup observations. Log no paths, content, credentials
or config values. No frame/pointer/byte-tick diagnostic stream.

### 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Legacy config lacks preview section | New defaults in memory; unchanged config bytes |
| Invalid range, unknown/malformed preview config | Reject before save/use; prior live state retained |
| Stale settings owner edits one leaf | Preserve latest other preview leaves and other sections |
| Wire missing/null options | Original legacy purpose quota and30second runner timeout |
| Non-preview non-null options, missing/unknown/duplicate/type/range error | InvalidRequest before effects |
| Known/actual member exceeds bytes or text lines | Distinct file-size/line result with frozen threshold |
| Image exceeds axis/pixels or decoder budget | Distinct dimension/memory result with frozen threshold |
| Archive source/spool/solid/native resource failure | Read-resource result, not selected-file oversize |
| Changed selection/tab/source, close/cancel/disconnect | Original stale suppression and actual lease cleanup |

### 5. Good/Base/Bad Cases

Good: default preview displays70KiB UTF-8,2000lines and1024×768 JPEG; a smaller user quota
still rejects excess. Base: a legacy caller retains64KiB text and sends no new worker fields.
Bad: raise shared document globals, allocate128MiB for a tiny file, overwrite the entire nested
preview object, let preview options authorize drag/materialize, or change a running request's
budget after settings save.

### 6. Tests Required

Core legacy/default/range/strict/no-write and locked changed-leaf merge. Rust/C# wire exact
bounds, missing/null, malformed/duplicate/unknown and non-preview rejection. Actual official
Settings number input/save/failure/minimum-window reachability. Production selection command
snapshot/next-selection and retained/stale/cancel/source/tab lease regressions. Actual bounded
reads, text byte/line/UTF-8/control policy, image pixels/axis/depth/WebP/first-frame and actual
encoded-input accounting; shared document defaults remain unchanged. Actual native worker
custom larger/narrower quota, legacy refusal, internal-min cap, private-field omission,
runner timeout and output/source cleanup. Full Desktop and affected Core/Protocol/IPC suites,
strict scoped lint, workspace tests check and spec/Kit/editor gates are separate from installed,
physical pointer/DPI/reader/human visual acceptance.

### 7. Wrong vs Correct

Wrong: setting controls merely raise the UI's metadata check while native/read/render keep
fixed limits. Correct: validated Core profile -> immutable request -> preview-only wire ->
original native purpose budget/runner -> actual bounded read/decode -> same-budget original
readonly renderer -> current-selection publication and actual lease retirement.
