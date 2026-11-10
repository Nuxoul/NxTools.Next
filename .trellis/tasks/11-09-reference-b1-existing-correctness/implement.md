# B1 execution

1. Record dirty baseline, inspect references/specs/current chains; retain existing unrelated changes.
2. REF-01: add red-capable production pointer/keyboard regression; fix current-setting routing with identity-guarded next-turn dispatch; focused GPUI/clipboard/lifecycle validation.
3. REF-02: add same-URL store-isolation background/API regression, observe red; propagate transient originating context through single/batch/redirect paths; cover gates, closed/replaced tabs, API absence and Native Messaging roundtrip.
4. REF-03: add actual interception/host regression for fixed reasons, ownership and compensation; extend original reply/popup projection without weakening durable acceptance or identity checks; validate timeout/disconnect/enqueue-only cases.
5. Run affected extension suites and isolated DOM fixtures; host/download IPC/message/engine and focused/full Desktop checks as relevant. One optional directed review. Fix concrete in-scope failures and retest.
6. Record exact counts/results and baseline blockers; update source-backed reference/spec contracts and taskbook B1 status. No commit/archive/package/install/native product launch.

Before each check identify the concrete failure it can detect and the corrective action. All Rust commands set CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and TEMP/TMP=$env:PI_SCRATCH_DIR in the same command. One global Cargo lane; use background yield/wait for long operations and inspect surviving processes after timeout before retry.

Explicitly recheck the reported message expected8/product9 assertion and Desktop strict-Clippy visibility/boolean issues; do not weaken assertions or add blanket suppression. Distinguish source, compiled production events, actual host fixtures, isolated DOM and unperformed installed/native/physical/DPI/visual acceptance in outcomes.md.
