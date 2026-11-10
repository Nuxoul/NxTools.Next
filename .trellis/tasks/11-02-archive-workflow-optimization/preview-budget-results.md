# Configurable archive preview budgets — results

2026-11-09. Scope: **Phase E — Format expansion and ZIP editing / in_progress**. The archive-only settings and original preview consumers are implemented and verified at the compiled/owned-fixture boundary. Overall Phase E and installed/physical acceptance remain separate.

Contracts: [preview plan](preview-budgets.md), [cross-layer executable contract](../../spec/nxtools-desktop/backend/archive-preview.md), [reference call chain](../../../docs/reference-code/archive.md), [Bandizip volume evidence](../../../docs/reference-code/archive-volumes.md).

## Settings and behavior

The original 压缩包设置 has a distinct **预览** page with official Kit0.8.0 `Settings`, `SettingPage`, `SettingGroup`, `SettingItem` and `SettingField::number_input(NumberFieldOptions)`. Controls use the default32DIP size, original page scrolling and deferred save.

| Setting | Default | Inclusive range |
| --- | ---: | --- |
| 文本大小上限（KiB） | 1024 | 1..8192 |
| 文本最大行数 | 20000 | 1..100000 |
| 图片文件大小上限（MiB） | 32 | 1..128 |
| 图片最大像素（百万） | 16 | 1..64 |
| 图片解码预算（MiB） | 512 | 32..1024 |
| 解压工作时限（秒） | 120 | 5..120 |

KiB/MiB use binary units; one megapixel is1,000,000pixels. All simultaneous limits apply. Each image axis remains capped at8192pixels. Decoder budget is not a process-wide memory guarantee.

Settings auto-save only changed preview leaves within the original section transaction, preserving concurrent preview fields and other sections. Missing legacy preview configuration adopts defaults in memory without rewriting the file on opening. Changes apply on the next selection/refresh; a running request, rendered document and source view retain their immutable profile.

The same profile controls known-size admission, preview-only native output/timeout, actual bounded reading, text lines, image decoding and original readonly rendering. Reads allocate from actual member length rather than configured capacity. JPEG/WebP conservative working admission charges actual encoded bytes plus1MiB slack. Text handles CR, LF and CRLF consistently, counts CRLF once and preserves trailing-terminator semantics.

Shared Clipboard/Quick Launch defaults remain64KiB/1200lines. Source acquisition, metadata, compound spool, solid work, working memory, existing worker/decode concurrency, integrity and lease/cancellation/cleanup ownership remain independent. Native selected-member byte exhaustion uses `selected_output_limit` and becomes the preview size-limit result; underlying `resource_limit` stays a read-resource result. Missing/null preview options retain legacy wire behavior, non-preview non-null options reject, and the private BrowserEntry request schema is unchanged.

## Bandizip split support

The installed Bandizip.exe/ark.x64.dll are **7.32.0.1**; the official product page reviewed advertises **7.47**. Current documentation and installed static evidence have separate attribution.

| Family | Split reading | Split creation evidence |
| --- | --- | --- |
| ZIP | Official split-opening help and installed chain | Installed ZIP segmentation |
| 7z | Official split-opening help and installed chain | Installed7z segmentation |
| RAR4/5 | Installed volume callbacks | Not established as a creation format |
| ALZ / EGG | Official help explicitly includes split archives; installed readers are present | No confirmed creation chain |
| Numbered `.001/.002` files | Installed concatenation/join adapter | A joining route, not another archive writer |
| ZIP-based SFX | ZIP-based format boundary | Installed prefix/split-write chain with its own4GiB/prefix constraints |

A writable ZIPX format or recognized ACE/ARJ/WIM extension does not establish its volume variants. Exhaustive newest ZIPX split-writing and all legacy-volume branches are not verified. This change does not add NxTools formats.

## Verification

Groups below are separate and are not summed. LIST reports selection, not executed tests.

| Boundary | Actual result |
| --- | --- |
| Final Core library tests | 95passed /0failed /0ignored |
| Final IPC library tests | 61passed /0failed /0ignored |
| Final Protocol library tests | 114passed /0failed /0ignored |
| Final Desktop `preview_` selection | 111passed /0failed /0ignored |
| Final shared MarkdownDocument tests | 2passed /0failed /0ignored |
| Selected-output protocol, worker budget isolation and Desktop mapping regressions | Each1passed |
| C# build | 0warnings /0errors |
| Exact reviewed C# preview Fixture LIST | selected2 /RUN0 /not_run2 |
| Exact reviewed C# preview Fixture RUN | RUN2 /PASS2 /SKIP0 /FAIL0 /not_run0 |
| Rebuilt isolated-codecs worker | Build passed; actual Read→Entry→worker→native fixture passed |
| Strict Core/Protocol/IPC library Clippy | Passed with `-D warnings` |
| Strict isolated-codecs archive library Clippy | Passed with `-D warnings` |
| Final `cargo check --workspace --tests` | Passed; three downloads visibility warnings remain |
| Scoped Rust formatting | 21files passed |
| Full Desktop command | 1777passed /9failed /1ignored; not a green full-suite verdict |
| Package-wide strict test Clippy | Blocked by the five diagnostics below |

Coverage includes real official number input/save at all six maxima, concurrent-leaf preservation, render-no-write, failed-save live-state retention and1060×650/920×480 reachability. Production selection freezes a larger text profile while settings change, renders the result with that original profile, sends the latest budget on the next selection and releases the original lease exactly. Existing stale/ABA/tab/source/cancel/retained-preview cases remain passing.

Decoder coverage admits70KiB text,2000lines and1024×768 JPEG under defaults; narrower bytes/lines/pixels/memory reject. A normal JPEG with a128MiB input setting and32MiB decode budget succeeds using actual input accounting; a9MiB-padded version rejects at32MiB and succeeds at512MiB. Actual tiny-file allocation, UTF-8/control policy, static images, depth/WebP/first-frame admission and two-decode admission remain covered. CR-only limits pass in both archive and shared document paths.

The actual native fixture verifies configured larger/narrower quotas, legacy refusal, internal-min cap, private-field omission, actual200ms runner timeout and cleanup. Unknown-size `large.txt.gz` exceeds its selected quota with `selected_output_limit`, leaves no failed output/manifest or active worker slot, then succeeds and releases cleanly with a larger quota. Independent source admission still returns `resource_limit`.

Raw parent logs are retained in session scratch: `preview-contracts-final.log`, `preview-targeted-final.log`, `preview-shared-document-final.log`, `preview-library-clippy-final.log`, `preview-workspace-final.log`, `preview-desktop-full.log`, `preview-strict-clippy.log`, `preview-contract-clippy-final.log` and `preview-format-final.log`.

## Remaining verification limits

The one complete Desktop run is not green:

- Three `ac_repair_tests` require the absent `NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE` production export.
- Four `opt03_result_tests` require the absent `NXTOOLS_OPT03_CREATE_WIRE` ordinary/split Create exports.
- `application_tests::opt03_drop_new_keeps_all_sources_in_original_create_form` observes0forms instead of1.
- `main_window::tests::download_context_menu_omits_actions_that_cannot_run_for_the_selection` rejects `.disabled(` in the menu source.

Strict package-wide test lint is also not green:

- `downloads/settings_view.rs`: private-interface diagnostics for `FfmpegDiscoveryState`, `DownloadSettingsViewInitial` and `DownloadSettingsDraft`.
- `downloads/browser_handoff_impl.rs:436`: nonminimal boolean expression.
- `nxtools-core/tests/archive_shell_defaults.rs:40`: `unwrap_used`.

These creation/download/Shell-test paths are outside this preview change. Final scoped tests do not replace the recorded full-suite result. Workspace-wide formatting is not certified over unrelated dirty files.

No package/install, deployed-binary replacement, product launch, registry/service mutation, commit or push was performed for this preview work. Actual installed behavior, physical pointer/focus/IME/DPI and human visual acceptance are not claimed.
