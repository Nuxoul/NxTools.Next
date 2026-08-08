$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

function Assert-Contains([string]$Text, [string]$Needle, [string]$Message) {
    if (-not $Text.Contains($Needle, [StringComparison]::Ordinal)) {
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
$publicKeyHash = [System.Security.Cryptography.SHA256]::HashData($publicKeyBytes)
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
if ($background.Contains('return { cancel: true };', [StringComparison]::Ordinal)) {
    throw 'Browser response hooks must not cancel before asynchronous acceptance.'
}

$handoffPolicy = Get-Content -LiteralPath (Join-Path $extension 'handoff-policy.js') -Raw
Assert-Contains $handoffPolicy 'response.handoffId !== expectedHandoffId' `
    'Desktop acceptance must echo the exact handoff ID before browser cancellation.'

$pageHook = Get-Content -LiteralPath (Join-Path $extension 'page-hook.js') -Raw
Assert-Contains $pageHook 'never suppress the page''s native click/download behavior' `
    'Blob capture must preserve the browser download while bounded transfer is unavailable.'

$nativeHostSource = Get-Content -LiteralPath `
    (Join-Path $root 'rust\apps\nxtools-browser-host\src\lib.rs') -Raw
Assert-Contains $nativeHostSource 'MAX_NATIVE_MESSAGE_BYTES: usize = 256 * 1024' `
    'Native Messaging input must use the product-level bounded frame limit.'
Assert-Contains $nativeHostSource 'DownloadCommand::BrowserSubmit' `
    'Native host must submit through the Rust download command boundary.'
Assert-Contains $nativeHostSource 'DownloadCommand::BrowserOpenDialog' `
    'Native host must distinguish a GPUI confirmation-window request from direct task creation.'
Assert-Contains $nativeHostSource 'connect_or_start(executable, !open_dialog)' `
    'A confirmation-window request must not silently start a headless download host.'
Assert-Contains $nativeHostSource 'handoff_id: Some(handoff_id.to_string())' `
    'Native host must echo the accepted handoff ID.'
if ($nativeHostSource.Contains('error = %error', [StringComparison]::Ordinal)) {
    throw 'Native host lifecycle diagnostics must not log raw error payloads or paths.'
}

$nativeHostCargo = Get-Content -LiteralPath `
    (Join-Path $root 'rust\apps\nxtools-browser-host\Cargo.toml') -Raw
Assert-Contains $nativeHostCargo 'name = "nxtools-browser-host"' `
    'Cargo target names must use a legal crate identifier; product renaming belongs to packaging.'
if ($nativeHostCargo.Contains('name = "NxTools.BrowserHost"', [StringComparison]::Ordinal)) {
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

$desktopDownloads = Get-Content -LiteralPath `
    (Join-Path $root 'rust\apps\nxtools-desktop\src\downloads.rs') -Raw
$newWindowStart = $desktopDownloads.IndexOf('fn open_new_download_window_with_request')
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
Assert-Contains $registrar 'DeleteSubKeyTree(keyPath, false)' `
    'Native Messaging registration must support exact current-user removal.'

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
