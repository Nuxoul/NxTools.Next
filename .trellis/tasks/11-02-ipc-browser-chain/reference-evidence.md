# Verified references and executable boundary

## Review order and sources

AB Download Manager was inspected first. Its `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/DefinedHandlers.kt` forwards AddDownload through application IPC; `integration/server/src/main/kotlin/com/abdownloadmanager/integration/model/DownloadCredentialsFromIntegration.kt` carries one generic header map; `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/integration/IntegrationHandlerImp.kt` enters ordinary HTTP/HLS add-download ownership. Browser stdio EOF ends NativeMessagingHost. The separate browser-extension repository is not locally available; no ABDM Blob/popup preference implementation is claimed.

Legacy `D:\Code\NxTools\browser_extension\nxtools-interceptor\background.js:460,1170,2564` and `popup.html:192` establish Header as master gate and Cookie as subordinate. The redundant headers/headerText/cookie shape is not a behavioral invariant. Legacy page-hook→content-script→LocalApiService staging→MainWindowViewModel confirmation verifies Blob confirmation behavior, not cleanup ownership; no equivalent cancel/close/orphan cache lifecycle was found. Legacy eager WebSocket/reconnect is superseded by the approved demand-only Next contract.

WinShun `D:\Code\WinShun-main\src\mcp\LineChannel.{h,cpp}`, `src\plugin\Runner.cpp` and `src\plugin\runner\main.cpp` verify overlapped duplex/event waits, CancelIoEx plus completion drain, bounded missing/busy retries and no cumulative control-stream lifetime budget. Its numeric limits and plugin authority are not adopted.

Full verified source chains/invariants are indexed in `docs/reference-code/download.md`, section `2026-11-02 IPC/browser handoff lifecycle`, and `docs/reference-code/winshun.md`, section `IPC/browser lifecycle reference recheck`.

## Current owner contracts

- Native protocol2 remains256KiB. Host defaults missing fields and tolerates unused legacy headerText. Header16KiB/Cookie64KiB limits remain. Extension sends headers without Cookie and an optional dedicated cookie exactly once; Header-off also excludes Referrer and Cookie API reads.
- Download command9 / Download IPC handshake2 and stable pipe incompatibility behavior remain. Engine claims complete Blob files in `browser-blob-staging/pending` before confirmation delivery. Original import/discard and engine startup own release; BrowserHost only expires top-level upload files.
- Popup runtime port `nxtools-popup` is same-extension popup.html only. Open→disconnect holds one demand lease. Pending requests/full Blob operations keep their original leases.1s idle release, passive startup/diagnostics/snapshot and no periodic reconnect remain.
- Native local pipes use explicit overlapped read/write events and direct retirement notification; initial absolute deadline/newline disarm,1MiB frame, identity/four-slot Indexer admission,16frame queue and500ms write bound remain.
- Broker `CreateControlReader` selects `ExtensionLineReader.Unlimited`. Per-line256KiB/strictUTF8/truncated failure remain; finite runner constructor retains legacy1/4MiB and data_plane2 combined25/28MiB stdin/stdout budgets, stderr64KiB. Read `.trellis/spec/platformhost/native-contracts.md` scenario `Unlimited broker control lifetime with finite run budgets` for the complete owner contract.
