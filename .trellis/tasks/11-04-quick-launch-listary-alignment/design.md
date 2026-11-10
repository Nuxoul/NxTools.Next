# Design

## Finder

Admission keeps the existing FileQuery literal/initialism/pinyin/homophone rules and typed instant actions. A private ranking module owns category classification and merging on FinderCatalog, avoiding new public SearchResult fields or another search owner.

Canonical history maps existing app:/indexed: path keys into one normalized app:path memory key. Existing SQLite rows remain untouched; writes increment one canonical row and update aggregated memory only after transaction commit. Prefix associations are collected once per merged query.

Match first, fold path duplicates, fold same-name application shortcuts, build rank keys, reserve the first10general-results segment ahead of weak application/history entries, then apply the final limit. Enabled package candidates join admission before this final merge, so alias learning is not lost by an intermediate cap/rerank. Command actions keep independent identities even with the same executable.

## Launcher

Use launcher-only52DIP result bodies and32DIP icons; application and file title roles follow Listary's16/15DIP distinction with12DIP supporting parent paths. Keep the current width, theme and4/0gap preference; attached-dialog compact rows remain independent.

The existing immutable display snapshot retains complete text. Public released text APIs perform width-dependent middle ellipsis and preserve full tooltip/accessibility content. The existing panel remains query/selection owner. Primary height follows committed result content up to a470DIP viewport, with compact pending/empty/error presentation. Async publications schedule size synchronization on the original retained window after the panel lease; query/session guards and no-focus native behavior remain.

## Verification boundary

Finder uses temporary roots and SQLite fixtures. Desktop regressions drive actual panel/query/result publication and measured paint/bounds, plus existing lifecycle/IME/stale-action tests. No package/service/native installed acceptance is implied by those tests.
