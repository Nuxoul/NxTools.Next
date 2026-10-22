# Currency catalog and reference-rate source

Verified 2026-10-20 for the user-requested provider replacement and aliases/search slice. This supersedes the initial ExchangeRate-API / Frankfurter fetch choice in `.trellis/tasks/10-20-currency-tool-enter-repair/`, not its clipboard privacy, local calculation, notification or configuration ownership.

## Provider / source review

- User-supplied catalog: `https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies.json`.
- User-supplied CNY base: `https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies/cny.json`.
- Both probes returned HTTP200. Catalog contained339 code/name entries; CNY response contained339 rates and reference date2026-10-19. Codes span2..5bytes in this snapshot, including `1inch`, crypto and metals; some names are empty. No clipboard text/amount was sent.
- Actual upstream README at `https://github.com/fawazahmed0/exchange-api` documents `.min.json` equivalents, daily reference data and Cloudflare fallback `https://latest.currency-api.pages.dev/v1/`. LICENSE is CC0-1.0. Runtime requests only the fixed catalog and CNY-base endpoints, never a user amount/currency query or arbitrary redirect.
- Complete catalog and captured CNY response were reviewed. The catalog is names only, not rates; conversion uses `amount / rates[source] * rates[target]` with both rates expressed per CNY. No alternate USD fetch is needed for target changes.

## Provenance / offline evidence

`third_party/currency-api/README.md` and original LICENSE-CC0 retain provenance. Bootstrap catalog is `rust/apps/nxtools-desktop/assets/currency-catalog.json`, SHA256 `72c81a54212045b8a2cced272379591a2d1d8b6c2a28f9b9b94a0b02a3657617`; it enables offline full-choice discovery, never fabricates runtime rates. Captured CNY JSON under `src/tools/currency-fixtures/cny-2026-10-19.json` is a test-only fixture, SHA256 `6ae7b315fb7af14e30e9b7e85924fde89458294db3b36123c88b030fd08f2b3b`. Tests use checked-in bytes or local synthetic HTTP, not live provider availability. No provider implementation was copied.

## Current call chain / invariants

`ClipboardApplication::handle_platform_event` -> existing pause/writeback/process/capture-policy guards -> `handle_currency_capture` -> `parse_amounts_in(text,current_catalog)` -> cached local cross-rate calculation -> deferred current generation/capture/pause token check -> existing shared NotificationCenter. Capture payload is never rewritten, sent to provider or logged.

`refresh_currency_rates` installs one UUID/pending before background work -> one immutable NetworkPolicy route/client -> concurrently read catalog and CNY response from fixed source -> validate both complete key sets/date/base/rates/names/budgets -> atomic version2 cache write -> matching completion publishes catalog/rates together. Source failure tries the documented alternate complete pair; no mixed-provider half snapshot. Failure preserves lastgood. Startup prefers v2, falls back read-only to original v1 and requests new-source refresh. Daily/manual/singleflight/disable/privacy cancellation owners remain unchanged.

Strict whole-copy <=256bytes/8amounts supports case-insensitive full catalog codes and longest known symbol/name marker. Explicit code wins generic symbol. `$`, `刀`, `美刀`, `美元`, `美金` map USD; `₩`, `￦`, `韩元`, `韩币` map KRW. Ambiguous ¥/￥ stays CNY unless explicit JPY. Reject prose and malformed numeric tokens. No claim of every colloquial word or ambiguous world currency symbol being unambiguous.

## Component selection

Locked0.7.1 public `SelectState::searchable(true)` + `SearchableVec<CurrencyChoice>` + `Select::search_placeholder/menu_width/menu_max_h` meets the requested searchable single dropdown. Plain Vec supplies items but no filtering. Exact source reviewed: vendored unmodified `select.rs`, `searchable_list/{delegate,vec,state}.rs`; no additional component patch or controller. Keep official32px trigger, original compact Form/Field,460x320 normal-focus single keyed settings window, deferred owner writes. Search code/English name/common Chinese display/aliases. Only catalog revision change replaces delegate; status notification does not clear typed query. Confirm persists; Escape/no-match do not. Catalog refresh preserves target and exposes new codes; unavailable saved code is retained and labelled.

## Diagnostics / evidence limits

Refresh UUID spans accepted/start, bounded source-pair terminal and overall terminal with rate count/fixed failure/elapsed. No amount/text/raw HTTP/path secrets in logs. Production synthetic keyboard/search/capture/SQLite/config/local HTTP regressions and fullDesktop checks are recorded in `.trellis/tasks/10-20-currency-catalog-aliases/outcomes.md`. They do not certify installed/native/physical/reader/mixed-DPI or user visual acceptance. No package/install/service/product launch is authorized by this note.
