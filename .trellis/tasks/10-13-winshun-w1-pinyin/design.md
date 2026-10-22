# W1 design and execution

## Smallest gap and owners
Finder presently builds legacy GBK initials for applications; FileQuery scores only literal names, and Desktop independently computes literal highlight ranges. Full Chinese spelling cannot survive indexed-name masks or unified external merge.

## Changes
- Finder pinyin.rs: retain legacy initial mapping; add immutable bounded term matcher and static reading lookup. A finite set of query offsets propagates across characters/reading prefixes, never pronunciation products.
- Finder query.rs: prepare eligible positive fuzzy terms once during parsing; only after literal failure attempt pinyin. Keep scoring tiers below the weakest normal fuzzy tier. Return byte ranges from the same matcher; relax Han name-mask rejection only when a term can use pinyin. Parent/path/excluded and explicit operators stay on original literal scorer.
- Finder lib.rs: preserve pre-W1 application scoring by evaluating original literal query and prepared legacy initials first; only missing results get new fallback. External results use shared query scorer.
- Deterministic generator plus MIT source data/license and generated compact binary table under Finder; THIRD_PARTY_NOTICES attribution. No dependency changes.
- Desktop quick_launch/search.rs: ordinary result highlighting calls shared FileQuery ranges; existing action-pattern helper remains literal. No controls/render/native/window/input/lifecycle changes.
- Indexer synthetic tests: verify actual name-only/complete query paths with in-memory snapshots, base/delta, cancellation and candidate masks. No MFT/USN/cache format/service edits planned.

## Invariants
Preserve pending/query generations, cancellation checks, unique-name groups, row fanout/headroom/ties, cold filters and complete fallback. No config/history schema, ranking-policy, native effects, clipboard preview or plugin changes. Existing GBK initials/ASCII ranking checked against frozen pre-W1 assertions, not rewritten oracle.

## Order and checks
1. Add FileQuery/catalog boundary tests and run red.
2. Generate attributed data, implement bounded matcher, query integration and highlights.
3. Run Finder; add actual synthetic Indexer and Desktop regression, run exact safe filters and full Desktop suite.
4. Strict affected Clippy, workspace --tests compilation, fmt, Quick Launch/indexer source gates and spec contracts. All Rust jobs1/incremental0/codegenunits256, locked command-local toolchain, one Cargo. No package/install/live native run.

## Component decision
Read complete GPUI Kit0.7.1 index and exact Label/StyledText APIs. Current production result title uses GPUI0.3.8 `StyledText::with_highlights(IntoIterator<Item=(Range<usize>, HighlightStyle)>)`; Label0.7.1 exposes literal Prefix/Full highlights only, not custom pinyin ranges. Retain existing public StyledText range presentation/controller: only the Rust UTF-8 producer changes. No component migration/layout/action/controller changes.
