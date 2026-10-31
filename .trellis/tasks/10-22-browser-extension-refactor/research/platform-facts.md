# Platform facts verified 2026-10-22 (Phase A)

## Chrome `downloads.onDeterminingFilename`
Source: https://developer.chrome.com/docs/extensions/reference/api/downloads (fetched 2026-10-22).
> Each listener must call suggest exactly once, either synchronously or asynchronously. If the listener
> calls suggest asynchronously, then it must return true. If the listener neither calls suggest
> synchronously nor returns true, then suggest will be called automatically.

Consequence: current Next listener (`background.js:4217`) defers `suggest` without returning `true`, so
Chrome auto-resolves immediately and proceeds (including its own Save As prompt) while the handoff is
in flight. Confirms R1.1.

Legacy reference `D:\Code\NxTools\browser_extension\nxtools-interceptor\background.js:4012` called
`suggest()` synchronously before the (cancel-first, unsafe) handoff, so the defect was introduced by
Next commit 9581d83 when the call was deferred; the legacy cancel-before-accept behavior is still not copied.

## MV3 `background.scripts` + `background.service_worker`
Source: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/background
(fetched 2026-10-22).
- Chrome: before 121 refuses to load an MV3 extension with `background.scripts`; from 121 the key is ignored.
- Firefox: `service_worker` unsupported; uses `background.scripts`. From Firefox 121 the background page
  starts regardless of `service_worker` presence. Product already requires Firefox `strict_min_version` 140.
- Firefox background pages have no `importScripts`.

Decision for R1.2: source manifest declares both keys and `minimum_chrome_version: "121"` so an older
Chromium reports an explicit version requirement instead of a load failure; `background.js` calls
`importScripts` only when it exists. Firefox packager keeps the source `scripts` list and drops
`service_worker`. Permissions/CSP/key unchanged.

## Baseline
Session scratch `ext-refactor-baseline/`: full extension source copy, `BrowserExtensionPackageService.cs`,
`browser-native-messaging.ps1`, SHA256 of 31 files, `git status` (unrelated dirty archive/clipboard/
markdown-editor work belongs to other tasks and must not be touched). Baseline Node suite 44/44 pass.
