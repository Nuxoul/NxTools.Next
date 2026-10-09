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
