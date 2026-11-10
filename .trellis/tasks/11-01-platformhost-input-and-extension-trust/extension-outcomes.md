# M1/U5 extension outcomes

## Result

Content-script HTTP(S) link interception, drop-target handling, and media-overlay download actions now require trusted browser input. Scripted events remain passive: they send no extension message and do not consume the browser default; trusted CDP mouse input retains the original direct-link and overlay flows, and rejected direct handoff still replays one marked synthetic anchor click to the browser.

No new diagnostics, dependencies, permissions, manifest entries, UI, or background/page-hook behavior were added.

## Files changed

- `browser_extension/nxtools-interceptor/content-script.js:712,729,1149,1166,1236` — early `if (!event.isTrusted) return;` in the resource-choice click, overlay main click, drag-over, drop, and direct-download click handlers.
- `browser_extension/nxtools-interceptor/tests/content-browser.cjs:151-289` — production real-DOM regressions for synthetic no-op/default preservation, CDP trusted direct/overlay behavior, and `replayBrowserDownload` pass-through.
- `.trellis/spec/browser-extension/contracts.md:45-46` — U5 browser-native `downloads.onDeterminingFilename` takeover contract and independent manual drop-target semantics.
- `.trellis/tasks/11-01-platformhost-input-and-extension-trust/extension-outcomes.md` — extension implementation and verification record.

## Behavior red/green

### Red before production fix

Command:

```powershell
node .\browser_extension\nxtools-interceptor\tests\content-browser.cjs
```

Result: exit `1`; `23/27` passed. The four behavior failures were:

- `synthetic-direct-click-preserves-browser-default`: event was prevented and sent `content_direct_download_request`.
- `synthetic-drop-is-passive`: drag/drop were prevented and sent `drop_target_download`.
- `synthetic-overlay-main-click-is-passive`: event was prevented and sent `content_get_resource_choices` plus `page_download_media`.
- `synthetic-overlay-choice-click-is-passive`: event was prevented and sent `page_download_media`.

Log: `$env:PI_SCRATCH_DIR\input-extension-red-content-browser.log`.

### Directed green

Command:

```powershell
node .\browser_extension\nxtools-interceptor\tests\content-browser.cjs
```

Result: exit `0`; `27/27` passed. This includes synthetic direct click/drop/overlay no-op assertions, CDP `Input.dispatchMouseEvent` trusted direct and overlay assertions, and the marked synthetic browser-replay pass-through assertion.

Log: `$env:PI_SCRATCH_DIR\input-extension-green-content-browser.log`.

## Integrated verification

Commands and results:

```powershell
$extensionTests = (Get-ChildItem .\browser_extension\nxtools-interceptor\tests -Recurse -Filter '*.test.js').FullName
node --test $extensionTests
```

- `11` test files; `65/65` tests passed; exit `0`.
- Log: `$env:PI_SCRATCH_DIR\input-extension-all-node-tests.log`.

```powershell
node .\browser_extension\nxtools-interceptor\tests\content-browser.cjs
node .\browser_extension\nxtools-interceptor\tests\refine-browser.cjs
node .\browser_extension\nxtools-interceptor\tests\layout-browser.cjs
node .\browser_extension\nxtools-interceptor\tests\redesign-browser.cjs
```

- `content-browser.cjs`: `27/27` passed.
- `refine-browser.cjs`: `20/20` passed.
- `layout-browser.cjs`: `38/38` passed.
- `redesign-browser.cjs`: `34/34` passed.
- Logs: `$env:PI_SCRATCH_DIR\input-extension-content-browser.log`, `input-extension-refine-browser.log`, `input-extension-layout-browser.log`, and `input-extension-redesign-browser.log`.

```powershell
python .\tests\architecture\spec-contracts.py
git diff --check
```

- Spec contracts: exit `0`; `120` documents and `14` workspace owners passed.
- Diff check: exit `0`; no whitespace error. Git also emitted a line-ending warning for the unrelated dirty `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs`.
- Logs: `$env:PI_SCRATCH_DIR\input-extension-spec-contracts.log` and `input-extension-diff-check.log`.

## Cookie scope finding

`manifest.json:14-30` grants `cookies` plus HTTP/HTTPS host permissions. `bg/request-context.js:68-78` calls `chrome.cookies.getAll({ url })` and serializes every returned nonempty `name=value`, sorting longer paths first; it does not filter `httpOnly` or `sameSite`, so URL-matching HttpOnly and `SameSite=Strict` cookies returned by the browser are included. SameSite is not re-evaluated against a top-level-site/initiator context in this code.

The lookup is a fallback only when `includeCookies` is enabled and no captured Cookie header already exists (`bg/request-context.js:101-114`). It queries the target URL and context/referrer URL, with `www.youtube.com`, `youtube.com`, and `m.youtube.com` fallbacks for Google Video/YouTube (`:57-98`), then deduplicates exact `name=value` pairs. Cookie values remain privileged worker/request context and are not logged or projected to the popup.

## Drop-target finding

The manual drop target is intentionally independent of automatic interception and per-site disablement. Both the legacy source (`D:\Code\NxTools\browser_extension\nxtools-interceptor\content-script.js:1881-2110`, `background.js:4284-4308`) and current source keep `dropTargetEnabled` separate from `interceptDownloads`; current `bg/router.js:243-270` validates the dropped HTTP(S) URL and native acceptance without consulting `settings.interceptDownloads` or `isSiteEnabled`, unlike the direct-download branch at `:154-188`.

The README did not explicitly define toggle coupling. The verified compatibility behavior is now explicit in `.trellis/spec/browser-extension/contracts.md:46`; runtime behavior was not changed.

## Page-hook Blob trust model

`page-hook.js` remains page-context instrumentation, not a user-gesture trust boundary. `dispatchBlobDownloadRequest` (`:302-344`) accepts only an anchor with a `blob:` URL, no pass-through marker, and either a `download` attribute or retained Blob metadata, then dispatches `nxtools-blob-download-request`. The hook deliberately covers both its patched `HTMLAnchorElement.prototype.click` programmatic path (`:1038-1053`) and the page click listener (`:1096-1106`) without checking `Event.isTrusted`.

The page realm therefore may request handoff for Blob objects it created or exposed. The isolated content/background/Rust boundaries still validate the Blob URL, retained source/metadata, bounded capability and upload sizes, site/intercept policy, and acceptance; failure recreates the marked browser pass-through download. This M1/U5 slice changes only ordinary content-script HTTP(S)/drop/overlay gesture admission.

## Poster/thumbnail preservation

The current `content-script.js` was compared with the task-entry copy at `$env:PI_SCRATCH_DIR\host-input-baseline\browser_extension__nxtools-interceptor__content-script.js`. Removing exactly the five new `if (!event.isTrusted) return;` lines produces an exact full-text match with the baseline. `videoPosterUrl`, `pageThumbnailUrl`, `poster: videoPosterUrl(element)`, and `pageThumbnail: pageThumbnailUrl()` remain present, preserving the pre-existing poster/thumbnail work represented by `content-script-existing.diff`.

## Limitations

Verification used production scripts in temporary headless Chrome profiles with mocked extension APIs and blocked HTTP/HTTPS/WebSocket traffic. It does not certify an installed extension, real Native Messaging, live cookie capture, a real browser profile, physical mouse/drop input, or user visual acceptance. Trusted behavior evidence is CDP mouse dispatch; physical drag/drop was not performed.