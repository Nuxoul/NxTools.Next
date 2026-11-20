# Archive, application icon and reader/update boundaries

## 1. Scope / Trigger

Changes to metadata-only 7z Open, packaged Windows application icons, About installer submission or Markdown heading pointer navigation must preserve the original feature owners. Evidence: `repo:rust/crates/nxtools-archive/src/worker_codecs.rs`, `repo:rust/apps/nxtools-desktop/src/main_window/about/update_download.rs`, `repo:csharp/NxTools.PlatformHost/Windows/ShellItemImageFactory.cs`, `repo:rust/apps/nxtools-desktop/src/tools/markdown_reader/outline.rs`. Reference notes remain behavioral authority; [task design](../../tasks/11-20-installed-feedback-fixes/design.md) records the requested changes.

## 2. Signatures

- `sevenz_budget::encoded_header(source, header, limits)`, `validate_blocks(archive)` and `blocks(archive, limits)` share coder validation/cost calculation.
- Existing `ExtractShellIcon { target, size }` remains the only icon command; `ShellItemImageFactory.Load(target, size, cancellationToken, out status)` owns native image-factory lifetime.
- `About::receive_update(id, started, ReleaseCheck, cx)` defers `DownloadApplication::submit_installer_download(source_url, file_name, expected_total_bytes, idempotency_key, cx)` to original `submit_with_idempotency(..., StartNow, Some(key), cx)`.
- Markdown retains released `Tree::new`, `ListItem`, `TreeState::set_selected_index` and deferred `Reader::jump_outline`.

## 3. Contracts

7z encoded headers always receive full workspace admission before decoding. Metadata-only `operation == "open" && drag.is_none()` validates payload declarations without charging dictionaries never allocated by that operation. Test/extract/selected/retained paths receive full payload admission before normalization or reader handoff. Original source, metadata, entry, output, native Job and cleanup limits remain unchanged.

Valid exact `shell:AppsFolder\family!application` targets have icon capability independently of file actions. Existing bounded STA/cache/request/coalescing/PNG owners remain. Namespace keys use the full target and requested size, a namespace selector and zero filesystem timestamp; no GetFullPath, file probe or extension fallback. Native creation requests IShellItemImageFactory IID `bcc18b79-ba16-442f-80c4-8a59c30c463b`, exact requested SIZE and ICONONLY=4. Release all returned COM/HBITMAP resources on success, failure, retry and cancellation. Cache-hit GPUI panel writes remain deferred.

About parses bounded release `tag_name`, `assets[{name,browser_download_url,size}]`, `draft` and `prerelease`. Only a newer numeric release and the current Windows x64 installer `NxTools.Next-setup-<version>.exe` qualify. Stable version-scoped engine idempotency prevents repeat submissions across dialog reopen. Existing directory, connection/adaptive defaults and HTTP/network/organization policy apply. Register one request before query dispatch; check it on receipt and again before deferred submission. Close retires pending work. An accepted task remains downloader-owned; no automatic installer execution. Fixed-stage diagnostics carry correlation, outcome, task ID and elapsed/native codes, not URLs or payloads.
Before constructing a request from mutable defaults, the download owner reuses an existing task with the same installer URL and filename. Preference changes therefore do not conflict with the original durable release receipt or mutate that task's destination/options.

Heading pointer-down stops propagation before the Base Tree records a folding click. Keep release propagation for Root text-selection cleanup, explicit Tree selection and deferred Reader navigation. Keyboard expansion/navigation and document/tab state remain owned by the original Tree/Reader.

## 4. Validation & Error Matrix

| Condition | Boundary result |
| --- | --- |
| Small encoded header, large valid payload dictionary, metadata Open | Complete catalog without payload decoder workspace allocation |
| Payload operation exceeds working limit | ResourceLimit before decoder/output/retained-reader handoff |
| Encoded-header workspace exceeds limit | ResourceLimit even for Open |
| Invalid/unsupported payload coder | CorruptArchive/UnsupportedMethod |
| Invalid namespace or Shell HRESULT failure | Original icon failure and static semantic fallback; no type-icon substitution |
| Missing/ambiguous installer or unsupported target | InstallerUnavailable; no task |
| Invalid release/URL or failed fetch | Fixed release/network failure; no current-version claim |
| Stale request or retired About | No submission or late publication |
| Missing/rejected download owner | Fixed admission feedback; no browser fallback |
| Parent heading pointer click | Navigate and retain descendants; release clears text gesture |

## 5. Good/Base/Bad Cases

Good: a 192 MiB-dictionary archive lists at the original 64 MiB Open limit, while insufficient-budget Test still refuses. Base: ordinary file icons/download tasks/tree keyboard behavior retain their existing pipeline. Bad: raise every archive limit, treat an AUMID suffix as a file extension, add a parallel updater downloader, or let heading navigation bubble into folder-toggle behavior.

## 6. Tests Required

- `repo:rust/crates/nxtools-archive/tests/read_work_contract.rs`: complete catalog, no output/decode work, full payload/retained refusal, encoded-header refusal and coder validation. Requests validate original schema-2 bulk requirements before worker launch.
- `repo:rust/crates/nxtools-archive/src/worker_codecs/sevenz_budget.rs`: actual header-only reader rejects packed-payload reads; full 195/196 MiB threshold remains. Work counters alone are not proof of absent ordinary decoder calls.
- `repo:csharp/NxTools.PlatformHost.Tests/PackagedApplicationIconTests.cs`: injected raw native leaf, real resolution/cache/PNG, exact IID/SIZE/flags and cleanup; no live Shell extraction.
- `repo:rust/apps/nxtools-desktop/src/quick_launch/installed_applications.rs`: actual visible-range request, coalescing, result identity, leased-panel cache hit and file-capability negatives.
- About parser plus real deferred engine admission, repeat/stale/closed/unavailable cases. Use an owned SQLite store with default queue paused before opening the engine; no external HTTP traffic.
- `repo:rust/apps/nxtools-desktop/src/tools/markdown_reader/outline_pointer_tests.rs`: repeated parent down/up in Preview/Split preserves child entries/bounds and ends text selection.

Controlled tests do not certify installed Shell appearance, physical pointer/DPI or packaged acceptance. The separately approved [Regular-only private font default](../nxtools-core/backend/data-directory.md#scenario-bundled-sarasa-regular-default) preserves the original font-selection transaction; [font findings](../../tasks/11-20-installed-feedback-fixes/fonts.md) record OFL/private-loading and raw sizes.

## 7. Wrong vs Correct

Wrong: call full payload `blocks` admission for metadata Open; exclude all namespace icons; submit a download synchronously while About is leased; swallow pointer release to stop tree folding.

Correct: admit only the workspace used by each stage, validate payload declarations regardless; route namespace artwork through the existing Shell pipeline; defer cross-owner admission with the live request guard; stop the tree's pointer-down tracking while preserving release and original navigation.
