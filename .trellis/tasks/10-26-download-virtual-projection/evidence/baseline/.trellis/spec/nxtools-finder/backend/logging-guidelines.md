# Finder diagnostics

Evidence: `repo:rust/crates/nxtools-finder/src/lib.rs#SearchResult`, `repo:rust/crates/nxtools-finder/src/lib.rs#CatalogStatus`.

Caller owns bounded refresh/search/action lifecycle. Query strings, target paths, custom argv, history keys and labels may contain payload; log only request/generation/kind/count/length/elapsed, no full SearchResult or catalog Debug. A scan error with path context must be mapped before logging. No keystroke/result-row per-frame logs. [Shared diagnostics](../../shared/engineering.md).
