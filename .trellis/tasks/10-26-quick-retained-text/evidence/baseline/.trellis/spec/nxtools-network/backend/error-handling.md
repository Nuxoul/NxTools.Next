# Network errors

Evidence: `repo:rust/crates/nxtools-network/src/lib.rs#NetworkPolicyError`, `repo:rust/crates/nxtools-network/src/lib.rs#normalize_and_validate`.

Typed variants InvalidSettings, LockPoisoned and Client preserve cause. Validate before publishing new policy; rejected settings cannot replace live route. Do not silently fall back direct after explicit manual proxy failure. Downstream caller maps source error into its feature fault; request deadlines/HTTP status are not owned here. Raw reqwest Display may carry sensitive endpoint data; use fixed kind/mode/source for diagnostics per [shared engineering](../../shared/engineering.md).
