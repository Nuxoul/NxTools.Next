# Outcomes

## Delivered

- `DownloadSourcePage { url, title }` is optional on `DownloadRequest`; absent metadata is serde-defaulted and omitted from serialized requests. Legacy request bytes and durable submission receipts retain their identity. The resolution resource fingerprint excludes the source page while accepting independent user edits to name and destination.
- BrowserHost protocol2 accepts optional camelCase `sourcePageUrl/sourcePageTitle`, bounded to16KiB URL and4096 UTF-8 title bytes. Its existing request builder carries the page through single, batch and controlled prepare into unchanged command14/IPC6, SQLite `request_json` and selected `TaskDetail`; no new schema, cache, command, network probe or log payload is added.
- Extension handoff uses only captured in-memory page facts in automatic (including original-URL provenance after redirect), popup single and batch payloads. Header/Cookie opt-outs do not remove page metadata; `contextUrl` remains Cookie context, Referer remains a request header, and missing/worker-lost page facts remain unknown. Persistent cache projection is unchanged.
- Exhaustive request fixtures in the Download, Download IPC, BrowserHost, Desktop and CLI owners explicitly use `source_page: None`; cloning/controlled confirmation and paused URL replacement retain existing source-page metadata. Narrow owner contracts were updated in `.trellis/spec/browser-extension/contracts.md`, `.trellis/spec/nxtools-browser-host/backend/quality-guidelines.md` and `.trellis/spec/nxtools-download/backend/database-guidelines.md`. The ABDM/legacy/Mister behavioral chains were already indexed in `docs/reference-code/mister-download-manager.md:235-251`; no external source was copied.

## Verification (2026-11-13)

| Boundary | Result |
| --- | --- |
| Runtime red: real length-prefixed BrowserHost → owned current-user pipe → engine → reopened SQLite | 1 failed at `source_page: null` before production code (`provenance-runtime-red.log`). |
| BrowserHost library: native framing, real SQLite reopen, localhost redirect probe, selected `TaskDetail`, controlled prepare, bounds, batch, legacy defaults | 55 passed (`provenance-host.log`); after extending the selected detail assertion, that affected test passed again (`provenance-detail-green.log`). Engine reopen is not process exit; the fixture host still owns a clone. |
| Download contracts: message, storage, resolution, controlled handoff, paused URL, domain and engine | 182 passed across eight test binaries, including 107/107 engine contract (`provenance-download-contracts.log`). After adding a source page to the paused replacement/reopen fixture, its affected binary passed 11/11 (`provenance-paused-green.log`). |
| Download IPC library | 9 passed (`provenance-ipc.log`). |
| Desktop selected request/name, retained draft, paused-source and controlled handoff | 1+1+1+7 passed (`provenance-desktop-name.log`, `provenance-desktop-b4_ref08_actual_paused_task_opens_official_source_draft_and_escape_discards.log`, `provenance-desktop-external_link_reuse_preserves_populated_draft_without_new_probe.log`, `provenance-desktop-controlled_handoff_tests.log`). |
| Extension production-entry tests across Chromium/Firefox and Native Messaging source gate | 320 passed (`provenance-extension-green.log`), gate passed (`provenance-native-gate.log`). |
| Strict affected all-target Clippy: BrowserHost, Download, Download IPC, Desktop, CLI | Passed (`provenance-clippy-green.log`). |
| Scoped rustfmt (`skip_children=true`) on51 touched Rust files, scoped diff whitespace, spec contracts, GPUI release and pinned editor source gates | Passed (`provenance-format.log`, `provenance-diff-check.log`, `provenance-spec-gate.log`, `provenance-release-gate.log`, `provenance-editor-gate.log`). |

Logs reside in `D:\.pi-desktop\data\scratch\7ea4a545-4448-4484-afe2-464aa37f261c`. No Cargo/rustc process remained at closeout. The full Download library and full Desktop test sweep were not run; the independent historical B6 native-refusal assertion in `ref13_successful_width_prevents_repeated_extra_connection_refusals` is outside this slice. Synthetic extension APIs, local HTTP/SQLite and source gates do not certify installed-browser behavior, native Windows focus/DPI, visual or user acceptance. No packaging, installation, registry change, product launch or commit was performed.
