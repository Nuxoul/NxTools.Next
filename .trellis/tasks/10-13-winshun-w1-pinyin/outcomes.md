# W1 integrated source outcome

## Delivered
- Reviewed WinShun Pinyin/NameMatcher/test/data-generator sources read-only. No reference execution or new runtime dependency.
- Finder shares full spelling, initials, mixed nonempty syllable prefixes and standard polyphonic readings through FileQuery. Examples报告 bg/baogao/baog/bgao,银行卡 yinhang/yhk,重庆 chongqing/zhongqing,绿色 lvse/luse. ASCII digits/case and original UTF-8 highlight offsets covered.
- Only ordinary positive fuzzy name terms gain fallback. Explicit operators, exclusions, path/parent filters remain literal; no W2 correction. Literal/ASCII scores, application GBK initials, usage/history/rank/generation/cancel remain original. Frozen catalog comparison retains every old result and its complete score/order; only new low-tier matches are permitted.
- Name-mask prefilter conservatively keeps non-ASCII names for eligible terms. Indexer production persistence/MFT/USN/service/cache files were not changed in this task; its actual synthetic search fixture exercises grouped/row/delta/scoped/excluded/compound/cancel paths.
- Desktop result-range producer calls shared FileQuery. Existing public GPUI0.3.8 StyledText::with_highlights renderer, original actions/input/window/preview owners unchanged. Action filtering helper stays literal; no new control.
- Static83,968byte pronunciation table,415syllables,20,924mapped characters, U+4E00..U+9FFF.2..63ASCII term bytes, up to3readings/6letters, finite query-offset state propagation; no pronunciation-product enumeration/per-name transliteration. Deterministic independent generator and checked-in MIT inputs/license/hashes/notice retained. Characters outside range are not guessed.

## Red and intermediate findings
Initial real FileQuery/external-merge regression0/3passed: full spelling missing, external merge returned only literal1instead of3. New implementation passes these boundaries. One transient edit removed a closure start and caused syntax failure; repaired, not product evidence. Indexer fixtures initially selected no files (default context flags false), then overfilled200resultcap; corrected explicit flags and reduced160base rows without changing production filters/rank/cap. One new literal-score expectation miscounted12byte filename and was corrected from5993to5992 using unchanged6000-(12-4) formula. Generated-script misplaced line was caught by readback before execution and repaired.

## Executed final verification
All Rust jobs1/incremental0/codegen-units256, command-local verified1.96.0 toolchain, oneCargo,21600s command timeout.
- Finder76/76:lib35 + calculator10 + custom4 + environment5 + file_query16 + pinyin_query6.0failed/ignored.
- Exact synthetic Indexer:W1 1/1,R8 5/5,existing filtered-headroom1/1. No unrestricted Indexer suite/real volume scan.
- Desktop659/659,0failed/ignored, includes shared W1 ranges plus existing input/select/preview/native-result fixture lifecycle.
- Strict affected Finder/Indexer/Desktop Clippy--tests -D warnings, workspace--tests compilation and fmt--all--check:exit0.
- Quick Launch/indexer architecture gates exit0;spec contracts87documents/12owners;git diff--check exit0 (existing unrelated CRLF normalization warnings only).
- Data regeneration followed by --check:exact inputs/outputs pass.

Synthetic Debug2000mixed names,9samples,each query500hits:p50microseconds file2029,baogao2230,bgao2178,yinhang2180,chongqing2217. This is bounded synthetic caller cost, not a pre/post speedup, installed latency, resident memory, Release or LocalSystem acceptance.

## Logs and remaining boundaries
Session scratch:w1-finder-red.log,w1-finder-first.log,w1-finder-green-1.log,w1-finder-indexer-green.log,w1-indexer-desktop.log,w1-indexer-desktop-2.log,w1-quality.log,w1-final-behavior.log,w1-final-behavior-2.log,w1-source-gates.log,w1-quality-final.log. Intermediate failures retained; final integrated behavior and post-test quality/document gates are green.
No package/install/product launch/live Shell/clipboard/browser/registry/service/volume operation, commit/push/reset. Broad preexisting dirty changes preserved. Installed/high-DPI/physical/reader/Release search latency and service acceptance remain unperformed; not blockers for this source slice. W2–W10 remain deferred; previous installer does not contain W1. No speculative plugin expansion.
