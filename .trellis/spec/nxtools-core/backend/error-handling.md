# Core errors

Evidence: `repo:rust/crates/nxtools-core/src/lib.rs#load_from`, `repo:rust/crates/nxtools-core/src/lib.rs#update_section`, `repo:rust/crates/nxtools-core/src/lib.rs#NetworkSettings`.

Core composition returns anyhow::Result with IO/serde context; settings validate with existing String errors. Normalize is not validation and must not silently broaden native privilege or erase unknown settings. Missing/corrupt section-save input remains error. Do not turn parse failure into defaults outside explicit first-run creation.

Current IO context includes file paths; never assume Display/Debug is safe for diagnostics. Caller maps to bounded fixed failure stage and preserves native code where appropriate. See [shared error/privacy contract](../../shared/engineering.md). Test actual disk failure and preserved bytes rather than only error text.
