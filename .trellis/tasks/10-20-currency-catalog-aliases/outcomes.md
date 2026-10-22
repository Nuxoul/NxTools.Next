# Complete currency catalog, aliases and searchable target — outcome

## Delivered

- Replaced runtime ExchangeRate-API/Frankfurter fetching with the requested fawazahmed0 Currency API catalog plus fixed CNY-base bulk rates. Uses documented equivalent `.min.json` endpoints on jsDelivr and the upstream-documented Cloudflare fallback. Target changes do not issue per-amount or per-currency requests; all conversion remains local.
- Removed the 31-code availability restriction. Current complete provider catalog contains339 entries, including fiat, metals and cryptocurrencies, with2..5-character codes in the captured snapshot. Runtime accepts bounded2..16ASCII-alphanumeric provider identifiers containing a letter; empty names display the code. Core now normalizes identifier syntax only, so USDT/1INCH/new provider codes survive restart rather than silently becoming CNY. The common Chinese-name table is display metadata only.
- Added ₩/￦/韩元/韩币 -> KRW and $/＄/刀/美刀/美元/美金 -> USD, plus common explicit dollar symbols, local currency names and symbols. Explicit catalog code wins a generic symbol. ¥/￥ remains CNY unless explicit JPY. Original prefix/suffix/case-insensitive/mixed-list examples still work. Longest known markers and current-catalog membership replace the old fixed-three-letter parser.
- Whole-copy restrictions remain256bytes/8amounts, bounded finite decimal values, no prose extraction, no clipboard rewrite, no raw payload logs or remote copied amounts. Existing pause/exclusions/writeback/capture-policy and deferred generation/capture/pause notification guards remain.
- Catalog and CNY rates are fetched as one validated pair, never treating names as rates or mixing half-successes. Same key sets, valid nonfuture date, CNY1, positive finite rates, names/code/count/response/cache budgets are checked before publishing or atomically persisting version2. Failed refresh/commit retains lastgood; startup reads v2 first and falls back read-only to v1. Legacy v1 triggers new-source refresh. One immutable NetworkPolicy route/client, singleflight/manual/daily owners retained; request8s/connect3s/sourcepair7s/overall16s, no redirects,128KiB per reply/512KiB cache/max4096 entries.
- Bundled CC0 full catalog seeds offline selection only. No runtime rates are bundled. The captured CNY response is test-only. Original CC0 license/provenance and THIRD_PARTY_NOTICES are included; no provider implementation code copied.
- Target dropdown is the original official Kit0.7.1 Select with **SearchableVec** and `searchable(true)`, not merely a search box on unfiltered Vec. Searches code, API English name, common Chinese name and dollar aliases. Original32px trigger/compact Form/460x320 keyed settings window/deferred owned-field save remain. Escape/no-match do not save; status updates do not clear an active query; catalog refresh preserves target and adds new choices; temporarily unavailable saved target remains labelled rather than silently reset.

## References / actual probes

Both user-supplied public endpoints returned200: catalog339/rates339, date2026-10-19. Upstream README documents daily data, minified equivalents and Cloudflare fallback; LICENSE is CC0-1.0. Raw/public files were kept in session scratch for review. Runtime tests use local sockets or checked-in immutable fixtures, not public network.

- Bootstrap catalog SHA256: `72c81a54212045b8a2cced272379591a2d1d8b6c2a28f9b9b94a0b02a3657617`.
- Captured CNY fixture SHA256: `6ae7b315fb7af14e30e9b7e85924fde89458294db3b36123c88b030fd08f2b3b`.
- Original LICENSE-CC0 copied identically; hash comparison passed.
- `docs/reference-code/currency.md` records provider/schema/component/call-chain invariants and points to `third_party/currency-api/`.
- Exact public Select/SearchableVec/delegate/confirmation/cancel APIs were reviewed in locked0.7.1 source. No additional component patch or dependency upgrade. Cargo.lock is byte-identical to this task's baseline.

## Red/green and corrections

Initial test edit was rejected by anchor visibility; the first filtered run executed0tests and is not evidence. After landing the regression, actual `currency_catalog_aliases_user_` ran1test and failed on the requested unsupported alias before implementation. The final same-symptom regression passes.

One initial compiler failure was a static-fallback/runtime-reference lifetime inference in `currency_catalog`; replaced the combinator with explicit matching, no ownership/type bypass. Source review caught that plain Vec does not implement filtering; official SearchableVec was selected before claiming search acceptance. The subsequent production UI test actually selects KRW from typed code, USDT from Tether, USD from 美刀, rejects no-match Enter, cancels via Escape, survives status notification and selects a newly refreshed NEWCOIN without resetting prior target.

The old provider-specific test body was moved/replaced with new-schema contract tests while retaining original whole-copy examples, cross-conversion, no-prose/budget checks, cache restart/corruption/atomic commit failure and local HTTP timeout/redirect/fallback/privacy assertions. No tests were ignored or warnings suppressed.

## Final performed verification

All Cargo commands used jobs1/incremental0/RUSTFLAGS=-C codegen-units=256, all-machine Cargo/rustc preflight, scratch logs and full-operation timeouts; no peer killed/contacted.

| Check | Result |
| --- | --- |
| Complete Desktop | **855passed,0failed/ignored**,110.90s |
| New catalog/provider/alias/capture focused filter | **7passed,0failed/ignored** |
| Runtime/privacy/settings/search focused filter | **4passed,0failed/ignored** |
| Core full library | **60passed,0failed/ignored** |
| Network full library | **2passed,0failed/ignored** |
| Desktop/Core tests strict Clippy `-D warnings` | PASS |
| Workspace `--tests --locked --offline` check | PASS |
| Scoped rustfmt / git diff --check | PASS |
| Clipboard lifecycle, Clipboard UI, UI state, Desktop shell gates | All4 PASS |
| CC0 source/license and no dependency-lock change | PASS |

New tests cover every bundled code, variable/digit-leading identifiers/metals/new runtime identifiers, original amount examples, won/dollar aliases, explicit-code priority, malformed/prose/limits, paired catalog/rate validation/missing keys/wrong base/date/values/budgets, complete actual339entry captured schema, version1+2 restart, cache corruption, actual Windows atomic-rename denial preserving lastgood/cleaning temp and bounded synthetic local HTTP complete-pair fallback/budget/redirect/timeout/request privacy. Existing previous clipboard, preview, notification-height/duration and Quick Launch Enter tests remain in the full passing suite.

Logs in session scratch: `currency-catalog-red1.log`, `currency-catalog-focused*.log`, `currency_catalog_-integrated1.log`, `currency_tool_-integrated1.log`, `currency-catalog-ui2.log`, `currency-catalog-final-{domain,runtime-ui,core-network,clippy,workspace,desktop}.log`, and `*.currency-catalog.log` gates.

## Operations / remaining limits

No delegation/cross-session message, commit/archive, package/install, ordinary product launch, service operation or live user clipboard/history/config access. Previous unrelated dirty changes and existing runtime processes preserved. Public probes contained no copied amounts/text. Test processes/config/SQLite/HTTP are synthetic/temporary.

No new screenshots or direct visual inspection, physical keyboard/pointer/Windows-IME, installed/native HWND, mixed-DPI/reader/UIA acceptance or24hour provider soak. Daily reference rates and2decimal conversion are not trading/financial precision guarantees. Catalog includes crypto/metals/legacy identifiers as supplied, not only current legal-tender currencies. Symbols with real-world ambiguity retain the documented defaults; this is not a promise that every possible slang/symbol is supported. Initial offline catalog may later differ from provider; successful paired refresh is authoritative, failure retains lastgood. No source/test blocker remains; installed/user visual acceptance is separate.

## Final documentation hygiene

Owning specs/reference/task record updated after final code verification. Final spec-contracts (106documents/14owners), component catalog (79pages/78lockedentries) and scoped whitespace checks all pass. No diagnostic probes or warning suppression remain; no behavior edits after the full passing run.
