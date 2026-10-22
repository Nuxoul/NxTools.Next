# Currency clipboard tool and Quick Launch Enter repair

## Authorization / goal
User approved the final plan and said 开始. Current session only; no delegation/cross-session messages, implicit commit/archive/package/install/service/live clipboard/history operations. Add opt-in currency conversion under Other Tools; separately reproduce/repair occasional launcher Enter no-op.

## Currency requirements
Recognize whole standalone amounts or short lists, case-insensitive ISO code before/after amount, symbols $=USD,€=EUR; explicit CAD etc codes take precedence. Examples5CAD,2.5cad,32$,$32,€43,43EUR. DefaulttargetCNY,configurable; tool defaultoff. Do not extract numbers from prose/longcontent; bound list/count/size and coalesce one copy into one shared notification. Never rewrite clipboard. Preserve pause,process exclusions,writeback suppression.
Settings reuse existing tool native shell/official controls: enabled,targetcurrency,manualUpdateRates,lastupdate/status/source attribution. Bulkcached daily rates plus explicit refresh; send only fixedcurrencyAPI request,never clipboardtext/amount. Retain lastgood onfailure,explicit no-data message,reference-date/stale disclosure. Primary open.er-api fixedCNYbase; Frankfurter fallback supported subset. Respect responsebudgets,timeout/networkpolicy,singleflight and dailyprovider cadence. No invented conversion on missing rate.

## Launcher requirements
Reproduce through actual production input/key/IME->selection->native command seam before fix. Preserve IMEconfirmation-noexecute,searchpending-oldrowblocked,currentgeneration,cancellation and success-onlyclose. Do not change provider/ranking/indexerservice.

## Acceptance
Parser/moneyformat/rateswrongbase/invaliddata/failure/cache/restart/dayrefresh fixtures; real production clipboard->sharednotification with exclusions/pause and target settings persistence/tempfiles; OtherTools card/config singleowner/default460x320/busy/errorkeyboard callbacks. Actual launcher KeyDownUp/IME cancellation regression. Affected Core/Network/library and fullDesktop once at end,scopedstrictlint/check/fmt,architecture/spec gates. Real installed/native/DPI/visual acceptance separate.
