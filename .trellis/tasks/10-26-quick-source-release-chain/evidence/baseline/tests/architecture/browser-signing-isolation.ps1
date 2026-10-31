param([string]$PackageSource = '', [string]$RegistrarSource = '', [string]$IdentitySource = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $PackageSource) {$PackageSource = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\BrowserExtensionPackageService.cs'}
if (-not $RegistrarSource) {$RegistrarSource = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\BrowserNativeMessagingRegistrar.cs'}
if (-not $IdentitySource) {$IdentitySource = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\BrowserExtensionSigningIdentity.cs'}
$package = Get-Content -Raw -LiteralPath $PackageSource
$registrar = Get-Content -Raw -LiteralPath $RegistrarSource
$identity = Get-Content -Raw -LiteralPath $IdentitySource
$tests = (Get-ChildItem (Join-Path $root 'csharp\NxTools.PlatformHost.Tests') -Filter '*.cs' |
    ForEach-Object { Get-Content -Raw -LiteralPath $_.FullName }) -join "`n"
if ($package -notmatch '(?s)ExportWithSigningKey\(.*?string signingKeyPath' -or
    $package -notmatch 'CreateCrx3Bytes\(zipBytes, signingKeyPath\)' -or
    $package -notmatch 'LoadOrCreateSigningKey\(signingKeyPath\)') {
    throw 'CRX test isolation requires an explicit key path carried through the actual signing call chain.'
}
if ($tests -match 'BrowserExtensionPackageService\.Export\s*\(' -or
    $tests -match 'BrowserExtensionSigningIdentity\.ProductionKeyPath') {
    throw 'Tests must not invoke the production-key export entry or production signing path.'
}
if ($identity -match 'File\.(Write|Create|OpenWrite)|Directory\.CreateDirectory' -or
    $registrar -match 'LoadOrCreateSigningKey|File\.WriteAllText.*pem') {
    throw 'Identity lookup and registration may only read existing keys; export remains creator.'
}
if ($package -match 'browser_extension_key\.pem' -or $registrar -match 'browser_extension_key\.pem' -or
    $identity -notmatch 'browser_extension_key\.pem' -or
    $package -notmatch 'BrowserExtensionSigningIdentity.CrxId' -or
    $registrar -notmatch 'BrowserExtensionSigningIdentity.ChromiumId' -or
    $registrar -notmatch 'ReadExistingChromiumId\(signingKeyPath\)') {
    throw 'Key location and SPKI/Chromium mapping must have one concrete owner.'
}
if ($package -notmatch 'Path.Combine\(currentDirectory, "browser_extension"' -or
    $registrar -match 'Directory.GetCurrentDirectory' -or
    $package -notmatch 'nxtools-download-interceptor@nxtools.local' -or
    $registrar -notmatch 'allowed_extensions = new\[\] \{ command.FirefoxExtensionId \}') {
    throw 'Exporter/registrar fallback differences and fixed Firefox identity must remain intact.'
}
Write-Host 'PASS explicit isolated signing path, read-only identity, single key/ID owner and preserved browser fallbacks (source only)'
