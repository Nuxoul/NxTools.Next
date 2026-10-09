$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

function Assert-Contains([string]$Text, [string]$Needle, [string]$Message) {
    if ($Text.IndexOf($Needle, [StringComparison]::Ordinal) -lt 0) {
        throw $Message
    }
}

$extension = Join-Path $root 'browser_extension\nxtools-interceptor'
$manifest = Get-Content -LiteralPath (Join-Path $extension 'manifest.json') -Raw |
    ConvertFrom-Json
if (-not ($manifest.permissions -contains 'nativeMessaging')) {
    throw 'Browser extension must request nativeMessaging.'
}
if ($manifest.host_permissions | Where-Object { $_ -match 'localhost|127\.0\.0\.1' }) {
    throw 'Browser extension must not retain local HTTP/WebSocket host permissions.'
}
if (-not $manifest.key) {
    throw 'Chromium extension identity must remain stable across unpacked and packed installs.'
}
$publicKeyBytes = [Convert]::FromBase64String([string]$manifest.key)
$sha256 = [System.Security.Cryptography.SHA256]::Create()
try { $publicKeyHash = $sha256.ComputeHash($publicKeyBytes) }
finally { $sha256.Dispose() }
$extensionAlphabet = 'abcdefghijklmnop'
$derivedExtensionId = -join ($publicKeyHash[0..15] | ForEach-Object {
        $extensionAlphabet[($_ -shr 4)] + $extensionAlphabet[($_ -band 15)]
    })
if ($derivedExtensionId -ne 'haohhnpjfggpdadhcbjakjpkmkljkpif') {
    throw "Chromium manifest key changed the preserved extension ID to $derivedExtensionId."
}

$background = Get-Content -LiteralPath (Join-Path $extension 'background.js') -Raw
Assert-Contains $background 'chrome.runtime.connectNative(NATIVE_HOST_NAME)' `
    'Browser extension must use Native Messaging.'
Assert-Contains $background 'const { response, handoffId } = await sendDownloadItem(downloadItem);' `
    'Browser download must wait for the desktop response.'
Assert-Contains $background 'expectedHandoffId: handoffId' `
    'Safe handoff policy must bind acceptance to the submitted handoff ID.'
Assert-Contains $background 'openDialog: settings.openDialog' `
    'Browser confirmation-window preference must cross the Native Messaging boundary.'
Assert-Contains $background 'sanitizeCapturedHeaderObject(resource.requestHeaders)' `
    'Page-derived resource headers must be bounded and allowlisted at the cache boundary.'
Assert-Contains $background 'importScripts("handoff-policy.js", "capture-policy.js", "blob-transfer-policy.js", "batch-handoff-policy.js")' `
    'The service worker must load the bounded batch handoff policy.'
Assert-Contains $background 'sendDesktopRequest("create_tasks"' `
    'Selected resources must cross Native Messaging as one correlated batch request.'
Assert-Contains $background 'await persistResourcesNow()' `
    'Stable per-item handoff IDs must be durable before the native batch request is sent.'
Assert-Contains $background 'NxToolsHandoffPolicy.buildExtensionLogRequest' `
    'Extension diagnostics must use the tested transport/correlation separation policy.'
Assert-Contains $background '}, 15000, correlationId)' `
    'Ordinary handoff must preserve one correlation ID from extension to Rust.'
Assert-Contains $background 'info.menuItemId !== "nxtools-download-link"' `
    'The context-menu listener must reject undeclared menu identifiers.'
Assert-Contains $background 'result?.ok !== true || result?.accepted !== true' `
    'The context-menu success badge requires explicit native acceptance.'
if ($background.IndexOf('return { cancel: true };', [StringComparison]::Ordinal) -ge 0) {
    throw 'Browser response hooks must not cancel before asynchronous acceptance.'
}

$handoffPolicy = Get-Content -LiteralPath (Join-Path $extension 'handoff-policy.js') -Raw
Assert-Contains $handoffPolicy 'response.handoffId === expectedHandoffId' `
    'Desktop acceptance must echo the exact handoff ID before browser cancellation.'
Assert-Contains $handoffPolicy 'requestId !== correlationId && !isRequestIdPending(requestId)' `
    'Best-effort extension log responses must not collide with pending feature requests.'
$batchPolicy = Get-Content -LiteralPath (Join-Path $extension 'batch-handoff-policy.js') -Raw
Assert-Contains $batchPolicy 'MAX_BATCH_ITEMS = 32' `
    'Browser batch handoff must enforce the product item-count bound.'
Assert-Contains $batchPolicy 'response?.data?.queued === true' `
    'Browser batch acceptance must prove queue-only task creation.'
Assert-Contains $batchPolicy 'batch handoff response identity mismatch' `
    'Browser batch responses must bind every result to the submitted handoff identities.'
Assert-Contains $batchPolicy 'runBatchHandoffRequest' `
    'Browser batch transport failure must emit one correlated terminal lifecycle event.'

$pageHook = Get-Content -LiteralPath (Join-Path $extension 'page-hook.js') -Raw
Assert-Contains $pageHook '__nxtoolsBlobChunkRequest' `
    'Blob bytes must cross the page boundary only as bounded transferable chunks.'
Assert-Contains $pageHook '__nxtoolsBlobFallbackRequest' `
    'Blob interception must retain an explicit browser-download fallback.'
Assert-Contains $pageHook 'blob.slice(offset, offset + length).arrayBuffer()' `
    'Blob upload must read bounded slices without Base64 Native Messaging amplification.'
if ($pageHook.IndexOf('uploadUrl', [StringComparison]::Ordinal) -ge 0) {
    throw 'The one-time local upload capability must remain outside the untrusted page realm.'
}

$blobPolicy = Get-Content -LiteralPath (Join-Path $extension 'blob-transfer-policy.js') -Raw
Assert-Contains $blobPolicy 'MAX_BLOB_BYTES = 2 * 1024 * 1024 * 1024' `
    'Browser Blob handoff must enforce the same fixed size ceiling as BrowserHost.'
Assert-Contains $blobPolicy 'http:\/\/127\.0\.0\.1:' `
    'The extension must reject Blob capabilities outside the exact loopback upload route.'
Assert-Contains $blobPolicy 'result?.ok === true' `
    'Only an explicit positive Blob handoff response may suppress the retained browser fallback.'
$contentScript = Get-Content -LiteralPath (Join-Path $extension 'content-script.js') -Raw
Assert-Contains $contentScript 'NxToolsBlobTransferPolicy.handoffAccepted(result, lastError)' `
    'Runtime disconnects and missing service-worker responses must trigger the retained Blob fallback.'
$background = Get-Content -LiteralPath (Join-Path $extension 'background.js') -Raw
Assert-Contains $background '}, 15000, correlationId)' `
    'Blob prepare must preserve the shared feature correlation ID.'
Assert-Contains $background '}, 20000, correlationId)' `
    'Blob commit must preserve the same feature correlation ID.'
Assert-Contains $background 'for (let attempt = 0; attempt < 2; attempt += 1)' `
    'An uncertain Blob commit must replay once with the same handoff and correlation before fallback.'
Assert-Contains $background 'error?.nativeRejected === true' `
    'A deterministic native rejection must not be retried as a startup uncertainty.'

$nativeHostSource = Get-Content -LiteralPath `
    (Join-Path $root 'rust\apps\nxtools-browser-host\src\lib.rs') -Raw
Assert-Contains $nativeHostSource 'MAX_NATIVE_MESSAGE_BYTES: usize = 256 * 1024' `
    'Native Messaging input must use the product-level bounded frame limit.'
Assert-Contains $nativeHostSource 'DownloadCommand::BrowserSubmit' `
    'Native host must submit through the Rust download command boundary.'
Assert-Contains $nativeHostSource 'DownloadCommand::BrowserBatchSubmit' `
    'Native host must submit bounded browser batches through one typed Rust command.'
Assert-Contains $nativeHostSource 'DownloadSubmitMode::QueueOnly' `
    'Browser batch intake must remain queue-only until the window anti-storm policy is implemented.'
Assert-Contains $nativeHostSource 'DownloadCommand::BrowserOpenDialog' `
    'Native host must distinguish a GPUI confirmation-window request from direct task creation.'
Assert-Contains $nativeHostSource 'DownloadCommand::BrowserOpenBlobDialog' `
    'Completed Blob staging must preserve the GPUI confirmation-window preference.'
Assert-Contains $nativeHostSource 'DownloadCommand::BrowserImportBlob' `
    'BrowserHost must commit Blob staging through the Rust artifact boundary.'
Assert-Contains $nativeHostSource 'browser_native_host.blob_staging_cleanup_failed' `
    'BrowserHost must report correlated staging cleanup failures instead of swallowing them.'
Assert-Contains $nativeHostSource 'DesktopStartMode::for_browser_request(open_dialog)' `
    'A confirmation-window request must not silently start a headless download host.'
Assert-Contains $nativeHostSource 'Self::Interactive => &["--browser-dialog"]' `
    'Browser confirmation cold start must carry a dedicated desktop launch intent.'
Assert-Contains $nativeHostSource '.arg("--browser-correlation-id")' `
    'Browser confirmation cold start must preserve its correlation ID across the process boundary.'
Assert-Contains $nativeHostSource 'handoff_id: Some(handoff_id.to_string())' `
    'Native host must echo the accepted handoff ID.'
if ($nativeHostSource.IndexOf('error = %error', [StringComparison]::Ordinal) -ge 0) {
    throw 'Native host lifecycle diagnostics must not log raw error payloads or paths.'
}

$nativeHostCargo = Get-Content -LiteralPath `
    (Join-Path $root 'rust\apps\nxtools-browser-host\Cargo.toml') -Raw
Assert-Contains $nativeHostCargo 'name = "nxtools-browser-host"' `
    'Cargo target names must use a legal crate identifier; product renaming belongs to packaging.'
if ($nativeHostCargo.IndexOf('name = "NxTools.BrowserHost"', [StringComparison]::Ordinal) -ge 0) {
    throw 'Cargo binary target names cannot contain product filename punctuation.'
}

$downloadModel = Get-Content -LiteralPath `
    (Join-Path $root 'rust\crates\nxtools-download\src\lib.rs') -Raw
Assert-Contains $downloadModel 'pub expected_total_bytes: Option<u64>' `
    'Browser-reported total size must reach the Rust download request.'

$downloadEngine = Get-Content -LiteralPath `
    (Join-Path $root 'rust\crates\nxtools-download\src\engine.rs') -Raw
Assert-Contains $downloadEngine 'take_browser_dialog_receiver' `
    'Sensitive browser dialog requests must use the in-process GPUI channel.'
Assert-Contains $downloadEngine 'browser_dialog_available' `
    'Headless download hosts must reject confirmation-window requests.'
Assert-Contains $downloadEngine 'browser_blob_staging_root' `
    'The download engine must derive Blob paths from its dedicated staging root.'
Assert-Contains $downloadEngine 'import_blob_artifact' `
    'The download engine must own the final Blob artifact move/copy boundary.'
Assert-Contains $downloadEngine 'download.browser_blob.discard_failed' `
    'The engine must report correlated native failures while discarding unconfirmed Blob staging.'
Assert-Contains $downloadEngine 'download.browser_blob.rollback_failed' `
    'The engine must report a database/artifact compensation failure as a distinct terminal.'

$desktopDownloads = Get-Content -LiteralPath `
    (Join-Path $root 'rust\apps\nxtools-desktop\src\downloads\browser_handoff_impl.rs') -Raw
$newWindowStart = $desktopDownloads.IndexOf('fn open_new_download_window_with_request')
if ($newWindowStart -lt 0) { throw 'Could not locate the add-download window lifecycle entry.' }
$newWindowEnd = $desktopDownloads.IndexOf('pub(crate) fn open_schedule_window', $newWindowStart)
if ($newWindowStart -lt 0 -or $newWindowEnd -le $newWindowStart) {
    throw 'Could not isolate the add-download window lifecycle.'
}
$newWindowLifecycle = $desktopDownloads.Substring($newWindowStart, $newWindowEnd - $newWindowStart)
Assert-Contains $newWindowLifecycle 'native_window_handle(window)' `
    'Browser add-download handoff must resolve the real GPUI HWND.'
Assert-Contains $newWindowLifecycle 'PlatformCommand::SetWindowTopmost' `
    'The add-download window must preserve the legacy always-on-top behavior.'
Assert-Contains $newWindowLifecycle 'PlatformCommand::ActivateWindow' `
    'The browser-created add-download window must explicitly request foreground activation.'

$desktopMain = Get-Content -LiteralPath `
    (Join-Path $root 'rust\apps\nxtools-desktop\src\main.rs') -Raw
Assert-Contains $desktopMain 'focus: launch_mode.main_window_focus()' `
    'A browser confirmation cold start must not let the main window claim focus.'
Assert-Contains $desktopMain 'show: launch_mode.main_window_show()' `
    'A browser confirmation cold start must keep the main window hidden until native minimization.'
$platformEvents = Get-Content -LiteralPath `
    (Join-Path $root 'rust\apps\nxtools-desktop\src\platform_events.rs') -Raw
Assert-Contains $platformEvents 'PlatformCommand::SetWindowMinimized' `
    'Browser startup must cross the typed PlatformHost boundary to show the main window minimized.'
Assert-Contains $platformEvents 'download.browser_startup.main_window_minimized' `
    'Browser startup minimization must report the observed native command result.'
$windowsAdapter = Get-Content -LiteralPath `
    (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.cs') -Raw
$windowsAdapter += Get-Content -LiteralPath (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.NativeMethods.cs') -Raw
Assert-Contains $windowsAdapter 'ShowWindow(window, SwShowMinNoActive)' `
    'PlatformHost must show the hidden main window minimized without activation.'
Assert-Contains $windowsAdapter 'IsWindowVisible(window) && IsIconic(window)' `
    'PlatformHost must verify the native minimized state before reporting success.'

$downloadStorage = Get-Content -LiteralPath `
    (Join-Path $root 'rust\crates\nxtools-download\src\storage.rs') -Raw
Assert-Contains $downloadStorage 'MAX_BROWSER_HANDOFFS: i64 = 4_096' `
    'Persistent browser handoff idempotency must have a fixed cardinality bound.'
Assert-Contains $downloadStorage 'BROWSER_HANDOFF_RETENTION_MS' `
    'Persistent browser handoff idempotency must have a time bound.'

$registrar = Get-Content -LiteralPath `
    (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\BrowserNativeMessagingRegistrar.cs') -Raw
foreach ($browserKey in @(
        'Software\Google\Chrome\NativeMessagingHosts',
        'Software\Microsoft\Edge\NativeMessagingHosts',
        'Software\Mozilla\NativeMessagingHosts')) {
    Assert-Contains $registrar $browserKey "Missing Native Messaging registration for $browserKey."
}
Assert-Contains $registrar 'ReleaseBinaryManifest.BrowserHostSha256' `
    'PlatformHost must verify the packaged BrowserHost before registering it.'
$registrationStore = Get-Content -LiteralPath (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\BrowserRegistrationTransaction.cs') -Raw
Assert-Contains $registrationStore 'DeleteSubKeyTree(keyPath, false)' `
    'Native Messaging registration must support exact current-user removal.'
Assert-Contains $registrar 'BrowserRegistrationTransaction.Apply' `
    'Browser manifests and all registry keys must share the compensating transaction.'
Assert-Contains $registrationStore 'store.Restore(changes[index].Target, prior[index])' `
    'Registration must restore captured prior state after partial mutation.'

$protectedInstaller = Get-Content -LiteralPath `
    (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\IndexerServiceInstaller.cs') -Raw
Assert-Contains $protectedInstaller 'ValidatePackagedSibling(hostPath, "NxTools.BrowserHost.exe")' `
    'Protected deployment must include the browser native host.'

$package = Get-Content -LiteralPath (Join-Path $root 'scripts\package.ps1') -Raw
Assert-Contains $package '-p nxtools-browser-host' `
    'Packaging must build the browser native host.'
Assert-Contains $package 'NxTools.BrowserHost.exe' `
    'Packaging must copy the browser native host.'
Assert-Contains $package 'target\$profile\nxtools-browser-host.exe' `
    'Packaging must rename the legal Cargo target to the product BrowserHost filename.'
Assert-Contains $package 'browser_extension\nxtools-interceptor' `
    'Packaging must include the preserved browser extension.'

$cargoLock = Get-Content -LiteralPath (Join-Path $root 'Cargo.lock') -Raw
Assert-Contains $cargoLock 'name = "nxtools-browser-host"' `
    'The checked-in lockfile must include the browser native host workspace package.'

Write-Host 'Browser Native Messaging architecture gate passed.'
