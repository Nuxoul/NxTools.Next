# Finder quality

Evidence: `repo:rust/crates/nxtools-finder/src/lib.rs#tests`, `repo:rust/crates/nxtools-finder/tests/file_query.rs`, `repo:rust/crates/nxtools-finder/tests/custom_command.rs`.

Assert actual prepared argv, last-good refresh, durable history and launchable ranking. Keep calculator/environment/file-query contracts distinct; no unexplained score changes. Temp roots/SQLite only. Native execution/KeepOpen/preview tests stay in Desktop/C#. [Verification](../../shared/verification.md); retain quote contract below.

## Scenario: Windows custom-command arguments

### 1. Scope / Trigger
- Changes to custom-command template/positional argument splitting.

### 2. Signatures
- `split_arguments(value: &str) -> Result<Vec<String>, String>` in custom_command.rs; command preparation consumes its exact argument vector.

### 3. Contracts
- Ordinary backslashes are literal. Backslashes preceding a quote follow run parity: pairs produce literal slashes; odd remaining slash escapes the quote, even run changes quote state.
- Preserve quoted spaces, trailing separators and literal escaped quotes; empty quoted argument remains an argument.

### 4. Validation & Error Matrix
- Unbalanced quote -> existing typed String error, not silently altered arguments. Valid paths/quote runs -> exact argv, not collapsed separators.

### 5. Good/Base/Bad Cases
- Good: prepared quoted Windows directory retains trailing backslash.
- Base: plain unquoted argument unchanged.
- Bad: treat every backslash as a generic escape and strip path separators.

### 6. Tests Required
- `prepared_windows_paths_preserve_backslash_runs_and_quoted_trailing_separators` and escaped-quote regression assert actual prepared command arguments.
- Keep Finder library suite; SwiftList ArgQuoting.cs is reference, not a Unix shell parser.

### 7. Wrong vs Correct
```text
Wrong: backslash always consumes the following character.
Correct: literal except for a run immediately preceding a quote; preserve parity.
```

## Scenario: W1 bounded pinyin names and shared highlights

### 1. Scope / Trigger
Chinese positive name matching in Finder catalog and Indexer, with GPUI result highlights.
### 2. Signatures
`FileQuery::score_name`, `score(name, path)`, `match_ranges(name) -> Vec<Range<usize>>`, `name_mask_may_match(mask)`. Private `score_literal`/`score_prepared` retain pre-W1 catalog admission.
Evidence: `repo:rust/crates/nxtools-finder/src/query.rs#FileQuery`, `repo:rust/crates/nxtools-finder/src/pinyin.rs#PinyinTerm`, `repo:rust/crates/nxtools-finder/tests/pinyin_query.rs`.
### 3. Contracts
Ordinary positive fuzzy terms only get new pronunciation fallback after literal failure. Explicit operators, exclusions, path/parent terms stay literal.2..63ASCII letters/digits, at least one letter; one immutable query-offset bitset, at most3readings/6letters per character, no reading-product enumeration/per-name transliteration allocations. U+4E00..U+9FFF static MIT data; unsupported characters literal/legacy only. Each Han consumes a nonempty reading prefix; ASCII self-match, v accepts u. New score1000 minus bounded byte start/span; old literal scores and existing application GBK-initial tier unchanged. Mask must conservatively retain non-ASCII names when a term can use pinyin; selective exact postings remain literal safe supersets. UTF-8 ranges share the actual pinyin matcher; Desktop keeps existing StyledText rendering and literal action filtering. No payload/per-row logging.
### 4. Validation & Error Matrix
Noneligible/overlong/punctuation/unknown reading -> no new fallback, original literal query remains. Scoped drive/parent/path failures -> no pinyin bypass. Stale search -> original cancellation, no publication. Name mask uncertainty -> score normally, never silently lose a candidate.
### 5. Good/Base/Bad Cases
Good: baogao/baog/bgao/bg finds报告, yinhang finds银行, highlights original UTF-8 Han span. Base: ASCII score unchanged, existing jsq application score retained. Bad: Cartesian readings, transliterate every indexed name per query, let !bg exclude报告, or filter Han before pinyin scoring.
### 6. Tests Required
Finder pinyin_query plus table/63length and real catalog refresh/legacy retained-results tests; exact Indexer w1_ fixture covers mask/groups/row/delta/exclusion/compound/cancel. Desktop w1_ ranges and full suite; R8 safe-superset/headroom regressions, strict lint/workspace/fmt. Data `python scripts/generate-pinyin-data.py --check` verifies reviewed hashes/exact generated files. Synthetic Debug timing is not installed/Release speed evidence.
### 7. Wrong vs Correct
Wrong: `pinyin(name)` creates many spelling strings then separate UI highlighting guesses characters.
Correct: prepare one bounded term automaton; score and highlight share its UTF-8 span, preserve original query/native owners.

## Scenario: Application classes and merged result ranking
### 1. Scope / Trigger
User-approved Listary-informed launcher ranking replaces unconditional6500executable promotion.
Evidence: `repo:rust/crates/nxtools-finder/src/ranking.rs`,
`repo:docs/reference-code/quick-launch-listary-results.md`.
### 2. Signatures
Private `FinderCatalog::{search_candidates,rank_results}` gathers then merges/ranks/caps once;
public ordinary/package/homophone search signatures and SearchResult payload remain unchanged.
### 3. Contracts
Application identity comes from StartMenu/Desktop/System, not executable extension. Shared query
score>=2000 distinguishes current good matching from weak subsequence/pinyin tiers; no claim of
Listary native scoring equivalence. Order: typed instant actions, exact keyword history, prefix
history, good used results, good applications, first10general results, weak used results, weak
applications, remaining general results. Retain generated-path demotion within match quality.
Instant commands retain keyword learning within their class. Match before dedup so a nonmatching
alias cannot suppress a matching candidate. Same path keeps application display information and
merged score/history; same-name application shortcuts fold after path merging, ordinary same-name
files do not. Distinct command/extension actions never dedup by their executable target.
Package candidates join before the single final merge, preserving duplicate-alias prefix learning.
### 4. Validation & Error Matrix
Weak ordinary exe versus exact directory -> exact directory; good formal app versus general row ->
app class; newer exact-query choice -> precedes old frequent choice; duplicate paths -> one result;
ordinary same-name different paths -> both; command argument variants -> both.
### 5. Good/Base/Bad Cases
Good: app rows remain prominent without promoting every helper.exe. Base: literal/pinyin admission
and typed action payload unchanged. Bad: lower the final result cap before category/history merge.
### 6. Tests Required
`repo:rust/crates/nxtools-finder/tests/listary_ranking.rs`,
`repo:rust/crates/nxtools-finder/tests/listary_merge.rs`; retain full catalog/query/pinyin/homophone/
custom-command/environment/calculator/package suites and strict test-target lint.
### 7. Wrong vs Correct
Wrong: score+=6500 whenever target ends in exe. Correct: explicit source/match/history categories,
ordinary score inside a category and final bounded merged results.
