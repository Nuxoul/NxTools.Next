# Finder structure

Evidence: `repo:rust/crates/nxtools-finder/src/lib.rs#FinderCatalog`, `repo:rust/crates/nxtools-finder/src/query.rs#FileQuery`, `repo:rust/crates/nxtools-finder/src/custom_command.rs`, `repo:rust/crates/nxtools-finder/src/calculator.rs`, `repo:rust/crates/nxtools-finder/src/environment_variable.rs`.

lib owns application scan/snapshot/ranking and usage/recent directories; query/pinyin own matching; custom_command prepares typed argv, calculator/env yield typed search actions. File index results supplied externally by Indexer, not re-scanned by Finder per query. SearchResultAction separates OpenTarget/CopyText/RunCommand; native executor and GPUI KeepOpen/session policy live elsewhere. Preserve launchable rank precedence and bounded result behavior with tests, not unexplained score changes. [Architecture](../../shared/architecture.md).

Private `ranking.rs` owns result categories, path/shortcut merge and canonical history keys on the
original FinderCatalog. `search_candidates` separates admission from the one final merge shared by
ordinary and package searches. Listary-aligned category/recency rules in [quality](quality-guidelines.md)
supersede unconditional executable-boost precedence; no new query/native controller.
