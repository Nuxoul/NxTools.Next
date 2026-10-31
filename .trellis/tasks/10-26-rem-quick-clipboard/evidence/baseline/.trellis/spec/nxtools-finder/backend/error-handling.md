# Finder errors

Evidence: `repo:rust/crates/nxtools-finder/src/lib.rs#apply_refresh_snapshot`, `repo:rust/crates/nxtools-finder/src/custom_command.rs#split_arguments`, `repo:rust/crates/nxtools-finder/src/query.rs#FileQuery`.

Catalog IO/SQL uses anyhow context; command/template validation returns actionable String. Keep parse/validation errors separate from native launch failure. Failed scan should not replace last-good visible catalog; don't silently interpret it as empty applications. Invalid unmatched quotes/placeholders reject before execution; normal Windows backslash isn't generic escape. Raw query/path/context messages are not diagnostic safe. [Shared errors](../../shared/engineering.md).
