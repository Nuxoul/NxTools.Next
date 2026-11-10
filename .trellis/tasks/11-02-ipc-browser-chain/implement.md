# Execution

1. Record baseline and reference chains; inspect original native and browser ownership boundaries.
2. M7/L5: add and execute old-code idle measurement and behavioral regressions; implement cancellable duplex native waits, missing-pipe backoff and transient accept recovery; rerun focused IPC/Indexer tests.
3. L6/L7: add real-background wire/preference/batch and popup lifetime regressions, plus temporary-file engine/host confirmation ownership regressions; execute red before production repair, then green.
4. L9: add bounded streaming regression through the control-reader construction, execute old-code cumulative-limit red, then introduce explicit unlimited mode and verify finite/line/UTF-8 boundaries.
5. Run requested safe owner suites, CLI checks, strict Clippy, changed-file formatting, C# selected fixtures, all Node tests and isolated network-blocked DOM gates. Update reference/spec/outcomes, validate task contexts and close the task without commit/push.

## Work allocation

- Native transport work may own IPC/local_pipe, IPC platform accept and Indexer session retirement adaptations. Its Cargo ownership excludes Download/BrowserHost/Desktop while their source is being changed independently.
- Parent owns extension L6/L7 and Download/BrowserHost Blob ownership, task/reference/spec integration and final Cargo verification.
- C# L9 is independent of Rust and extension paths. Review only exact safe fixtures, never the full native harness.
- No concurrent edits to the same path. Worker reports and logs use the parent shared scratch directory `D:\.pi-desktop\data\scratch\6a21b011-4230-4aa5-a2ed-f2dc1b4153e6`.

## Evidence

Behavioral red/green results, old/new M7 measurements and exact final commands are recorded in outcomes. Compilation/selection failures do not count as red. Known unrelated date-dependent calendar and Archive formatting differences remain outside scope. Installed LocalSystem release gates remain deferred.
