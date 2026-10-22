# Design

## Boundary
Core nested CurrencyConversionSettings under ClipboardSettings owns enabled/target defaultserde/validation; no HTTP/domain engine there. Desktop tools/currency owns strict bounded amount parsing,rate snapshot/validation/conversion/format and HTTPcache IO. ClipboardApplication existing event owner adds runtime singleflight/deadline/generation/cache and deferred sharedNotification ingress after existing privacyguards. UI separate currency settings view reuses original OtherTools tile/nativefeature shell,not another capture/notificationowner. No WindowsC# changes.

## Components
LockedKit0.7.1 official ListItem/Label/Icon with original focused_action tooltile shape180x160; SmalloutlineConfigure is distinct,stopspropagation. Existing feature_settings_window owns title/close,firstenable/body/scroll460x320. Form/Field+Select32px target,SmalloutlineUpdateButton,Label/Alert runtime update/date/source/error. No Toolbar.child ghostadaptation,custom painted controls,DataTable or palettefork.

## API / arithmetic / privacy
Primary GET https://open.er-api.com/v6/latest/CNY; fallback GET https://api.frankfurter.app/latest?from=CNY supports a subset. Cross-rate = amount / rate[source] * rate[target]. Response and cache read <=128KiB, <=256 finite positive rates, exact CNY base, no future date, fixed provider allowlist. HTTP8s/connect3s/whole provider sequence12s, no redirects, one shared immutable network snapshot. Atomic sibling currency-rates-v1.json retains lastgood on failure. Refresh due at24h fetch age; stale display also marks provider date >=4days old (weekend reference dates tolerated). Exactwholecopy <=256bytes/max8 amounts, supportedISO case-insensitive; $=USD, explicitISO overrides symbol. Finite bounded decimals, reference-only2fraction results; no clipboard rewrite/payload logs.

## Launcher
Actual production InputState marked/commit/cancel -> real KeyDown/KeyUp -> native pending action regression reproduced a stale pending_confirmation_enter after a native confirming key never emitted PressEnter, and after cancellation. Keep the current same-gesture consume gate, reject live marked text, then reset the gate on input-focused KeyUp once composition is inactive. Original searchpending/generation/selection/native command and close policy are unchanged. Listary managed->SwiftList current Helpers input path verified before repair; no proprietary native engine source available.

## Diagnostics
Refresh has one UUID across accepted/start, fixed-provider attempt terminal and overall terminal/elapsed/ratecount/fixedfailurekind; no clipboardtext/amount/path/rawHTTP error. Existing shared NotificationCenter ingress supplies bounded notification lifecycle diagnostics. One copy's list yields one notice with unique invocation ID (not a shared category key). Deferred notice release rechecks settings generation, latest capture serial, pause generation and current enabled/privacy gate; newcapture/pause/reconnect/disconnect/settings changes invalidate queued amounts. No per-byte/frame logs or second notification timer/manager.
