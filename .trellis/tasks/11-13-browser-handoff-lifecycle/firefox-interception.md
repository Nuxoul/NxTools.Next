# Firefox pre-download interception

## Behavior

Firefox export grants `webRequestBlocking` only in `BrowserExtensionPackageService.CreateFirefoxManifest`; the shared manifest, local Chromium CRX and Chrome-store ZIP retain their original permissions. The existing ordered background scripts and exact extension/signing identities remain unchanged.

`background.js` registers `registerFirefoxResponseHandoff` before its Firefox `downloads.onCreated` fallback. Successful full HTTP(S) GET frame responses with attachment or downloadable MIME facts enter `runFirefoxResponseHandoff`:

`retained original response -> Pending confirmation -> durable Held -> persisted cancellation intent -> return cancellation -> matching onErrorOccurred -> verified browser_cancelled -> Commit`.

Pending creates no DownloadItem, paused history entry or competing desktop transfer. Response filename, known uncompressed length, range facts, originating request/referrer/redirect and exact container/private Cookie store enter the original rule and credential gates. Inline documents/media, unknown types, partial/empty/error responses and non-GET requests remain native. Chromium's asynchronous filename callback and manual/Blob/batch flows retain their original owners.

Rejection, user cancellation, desktop unavailability, timeout or disconnect before cancellation aborts the same handoff and releases the original response. No replacement GET or history deletion is used. A bounded source-matched decision prevents a second `onCreated` offer, including after long confirmation/request-cache expiry; a newer actual request or different Cookie store remains independent. Downloads not classified at the response boundary retain the existing pause/verify/cancel fallback.

## Ownership and diagnostics

The existing32-operation journal/admission bound, ten-minute confirmation limit and quiet one-second status demand apply. After returning cancellation, wait at most12seconds for the same request's terminal event. Response-start or actual DownloadItem evidence aborts Held instead of activating it. Missing cancellation evidence retains Held and `cancelling`; no guessed Commit or browser ownership restoration.

Session records contain only response/request/handoff/task IDs, phases, flags and timestamps. Recovery aborts lost pre-cancellation response owners; verified `browser_cancelled` reuses the original same-task Commit retries. Unknown cancellation remains held for recovery without a second task. In-memory fallback decisions retain bounded source/URL facts but no credentials and are not persisted.

`handoff_state` carries the same UUID across native requests and fixed state/reason/elapsed transitions. No URL, Header, Cookie, body, raw browser error or polling traffic enters these diagnostics. Existing Popup/window/layout/focus and native show/hide owners remain unchanged.

## References

[Verified ABDM, legacy NxTools, Mister and Ghost entries](../../../docs/reference-code/browser-integration-polish.md#firefox-response-interception-and-export-prerequisite).

Mister retains a frame response in a Promise-returning blocking listener and grants the permission in its Firefox manifest. Its app-enqueue acknowledgment is not durable acceptance. Ghost cancels/erases after creation and before desktop routing; that ordering cannot prevent Firefox download animation and is not used here. Behavioral reference only; no GPL implementation, comments, tests or assets are incorporated.

## Verification

- `tests/firefox-response-handoff.behavior.test.js`:38 production-background public-event cases covering Pending/no DownloadItem, Held/cancellation/Commit ordering, refusal/expiry, unavailable/timeout/disconnect fallback, wrong identity, duplicate events, long confirmation, method/redirect/context/Cookie gates,32-operation admission, recovery, diagnostics and Chromium control.
- Focused Firefox response + controlled handoff + method + Cookie-store + capture-rule Node group: **130 passed, 0 failed, 0 skipped**.
- Recursive extension Node suite: **363 passed, 0 failed, 0 skipped**.
- Browser signing isolation source gate: **passed**.
- Native Messaging source gate: **failed** at `tests/architecture/browser-native-messaging.ps1:82-84`, which bans every literal response-cancellation return regardless of asynchronous durable acceptance. Its replacement must check the actual Held/cancellation-observation boundary; the production-event tests exercise that boundary.
- `BrowserExtensionSigningTests` adds actual XPI blocking-permission/source-byte assertions and local CRX permission preservation; the existing store ZIP permission assertion remains. C# compiled fixtures are pending coordinated verification.

Exact isolated C# selection (GUID-owned source/output/PEM only; no registration or production key):

```powershell
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list "browser signing fixture CRX signature and reuse" "browser signing fixture missing corrupt Firefox cancellation"
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures "browser signing fixture CRX signature and reuse" "browser signing fixture missing corrupt Firefox cancellation"
```

## Acceptance limits

Installed Firefox Save As/animation/history, authenticated/container downloads and native-host interaction remain unverified. The response path requires the updated Firefox XPI export; a source manifest without blocking permission retains the controlled DownloadItem fallback. Scripted/unknown-type downloads may still create native UI before that fallback can pause them. Lost cancellation acknowledgment retains a non-transferring Held task rather than guessing activation.

No Cargo, browser/app launch, release packaging, registration or commit was performed.
