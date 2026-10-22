# Currency tool and Quick Launch Enter outcomes

## Delivered
- Other Tools currency tile, default off; configure opens/reuses the original opaque normal-focus460x320 feature-settings shell. Official Kit0.7.1 ListItem/Icon/Label, Form/Field, default32px Select, Small24px outlined Configure/Update, shared first-row Switch and Alert; no Toolbar child variant transformation, dependency upgrade, custom-painted replacement or new notification manager.
- Whole standalone amount/short-list parser <=256bytes/8items, case-insensitive supported ISO prefix/suffix, symbols ($=USD), explicit ISO priority. User examples `5CAD 2.5cad 32$ €43` and mixed prefixes supported; prose/unknown currencies/malformed/excess input rejected. Default target CNY,31 target choices, locked owned-field settings merge.
- Cached daily reference rates; fixed CNY-base ExchangeRate-API primary and Frankfurter fallback; cross-rate calculation stays local. Requests never include copied text/amount. One immutable existing NetworkPolicy route, no redirects,128KiB/256rate budgets,8s request/3s connect/12s whole sequence. Versioned bounded cache load and atomic write, lastgood retained; explicit no-data/missing-currency/failure/stale/source/reference-date/last-fetch UI. Sources attributed in settings and result. Frankfurter subset may lack a chosen currency; no fabricated result.
- Existing clipboard event owner calls currency after pause/content/writeback/process/capture policy guards; one list -> one existing shared notification. Original clipboard/history formats unchanged. New capture, settings/pause/reconnect/disconnect invalidate queued/deferred notices. In-flight generic rates can still populate cache after disabling, but cannot release retired amount notices. One cancellable daily timer and one in-flight request.
- Repaired demonstrated Quick Launch Enter no-op: stale IME-confirmation latch swallowed next ordinary Enter when native IME consumed confirmation without emitting PressEnter, also after cancel followed by typing. Production marked-text/commit/cancel + real KeyDown/KeyUp tests failed0/2; diagnostic state before=(false,true,false,true), after=(false,false,false,true,None), no pending action. Keep same-gesture confirmation consumption; reject live preedit; clear latch on input-focused KeyUp when no composition remains. No search/ranking/indexer/native-focus/close-policy changes.

## References and dependency
- Read Listary managed SearchWindowBase constructor/show/activate/focus/KeyBindings first, then current SwiftList `App/Views/QuickSearchWindow/Helpers/QuickSearchWindowInputHandler.cs` (Key.ImeProcessed falls through; plain Enter executes). Proprietary Listary native engine is unavailable. Note updated before repair; no reference implementation copied.
- Public endpoint/documentation probes previously returned200 for both requested APIs. ExchangeRate-API free daily endpoint requires attribution and permits caching; Frankfurter is daily/reference, not realtime trading. Automated tests use local HTTP only.
- Relative to task baseline Cargo.lock adds only Desktop's existing workspace reqwest dependency; no package upgrades. Kit remains exact0.7.1.

## Performed verification
All Cargo commands used jobs1/incremental0/RUSTFLAGS=-C codegen-units=256 and all-machine build-lane checks; no peer processes killed/contacted.
- Initial parser placeholder red:0passed/1failed.
- Final currency focused8/8; production clipboard/shared-notice/privacy, deferred/stale completion, singleflight/daily-timer ownership/save failure, production tile/settings pointer+keyboard/persistence/bounds/reuse/busy/error; parser/cross-conversion/cache/commit failure and local HTTP fallback/budget/redirect/timeout.
- Enter focused3/3; old-result/searchpending protection1/1; original IME gate1/1. Actual OpenShellTarget/pending action assertions replace an initially too-broad any-command assertion (icon traffic can satisfy it).
- Core59/59, Network2/2, zero failures/ignored.
- Strict Clippy Desktop/Core/Network --tests -D warnings and `cargo check --workspace --tests --locked --offline` passed.
- Complete Desktop **843/843**,0failed/ignored,108.55s. Prior Clipboard thumbnails/summary/pause/rich preview and existing suites remain green.
- Clipboard lifecycle,Clipboard UI lifecycle,Quick Launch lifecycle,shared UI state lifecycle gates passed. Spec-contracts106documents/14owners and component catalog79pages/78locked entries passed. Scoped format initially found only main.rs startup binding linewrap; corrected without behavior change.
- Kit test snapshots omit disabled (None on all controls): assertion corrected to actual busy label and interaction/singleflight behavior, not a production bypass. Compiler/import/borrow and one collapsible-if lint failures were repaired; final compiled checks above passed.
- Logs in session scratch: currency-final-focused.log,enter-final-focused.log,enter-pending.log,enter-existing-ime.log,currency-core-network.log,currency-final-clippy.log,currency-final-workspace.log,currency-final-desktop.log; exact reproduction enter-repro-exact.log and diagnostic3.log.

## Limits / operations not performed
No delegation/cross-session messages,commit,task archive,package,install,product launch,service operation,live clipboard/history access or real native execution. Existing running protected apps/services were not stopped or changed. Test HWND-less GPUI events/temporary SQLite/config/local sockets do not certify installed behavior, physical keyboards/Windows IMEs, reader/UIA, user visual approval or100/125/150/mixed-DPI. Daily timer ownership/due threshold is tested; no wall-clock24hour/network soak. No claim that every possible intermittent Enter cause is fixed: the reproduced IME latch cause is fixed; actual installed confirmation remains separate.

## Final source hygiene
Final scoped rustfmt --check and git diff --check both exit0; updated spec-contracts and component-catalog gates pass after documentation/linewrap-only closeout. No production diagnostic probes or broad warning suppressions remain.
