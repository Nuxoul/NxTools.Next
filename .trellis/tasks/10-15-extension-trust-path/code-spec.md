# Executable trust and PATH contracts

## 1. Scope / Trigger
User rejects trust technical wall, extension action tier inconsistency and manual-only interpreter setup. Explicit opening environment/trust triggers bounded discovery for absent configuration; no render-loop execution or automated plugin invocation.

## 2. Signatures
`ExtensionManagerView::{open_environment,open_trust,inspect_trust}`; original typed `InspectExtensionRuntimeCommand{program,executable}` empty executable selects PATH; `PickExtensionRuntimeCommand` for manual selection. `environment::parse_probe` validates response identity/version/hash. `ExtensionApplication::approve_runtime_checked` rehashes detected executable; Core `approve_extension_checked(path,id,content_hash,runtime,expected_selected)` compares fresh selected config inside original locked section transaction. `trust_revision` observes terminal save, `trust_failure` controls retry independent of localized messages.

## 3. Contracts
Rust owns draft/config/generation/trust; C# native PATH/process; GPUI official components render. Unconfigured interpreter -> fixed program.exe from bounded process PATH (<=256segments/32767chars), skip relative/malformed/WindowsApps, no cwd/shell/registry/PATHEXT. Explicit path including whitespace/missing must fail, not fallback; local canonical \\?\ drive spelling accepted/normalized without allowing device/UNC paths. Probe fixedversionargs/3s/256MiB/2processes/4KiB and actual cleanup/hash gate unchanged. Only native owned fixture executables are run in tests, no installed user interpreter.

Automatic discovery updates environment draft only; Save explicit. Package discovery with no default becomes explicit override rather than silently saving None inheritance. Typed path edits/cancel/reopen/request UUID reject late responses. Trust retains attempted explicit path separately from verified candidate; failed manual inspect retries same path, no silent PATH switch. Discovery/cancel writes0, no auto trust/enable. Explicit approval binds detected hash and current content, compares expected selected config at lock boundary; ordinary original approve compatibility persists detected override only when unconfigured. Effective change still revokes trust/disable/clear intents. Closed/other draft cannot receive late reply; cross-owner updates defer, no leased-entity reentry.

Official0.7.1 Dialog/DescriptionList/GroupBox/Accordion, no disabled editable-looking runtime Input. Version/PATH/config summary and purpose visible; fullpath/hash/budgets collapsed. Risk warning and runtime recovery actions fixed outside scroll above one32px footer; Dialog Root owns pointer/keyboard/close/focus. Extension text actions defaultMedium32; Toolbar propagates defaultSmall to children, so compound owner explicitly Medium. Icons keep semantic tiers. Existing themes/fonts/windows/176rail unchanged; no global restyle/private fork.

## 4. Validation & Error Matrix
Absent PATH -> actionable message/manual browse, Confirm disabled. Explicit broken path -> no silent fallback. Wrong program/path/hash/version reply -> no usable candidate. Cancel/typed/stale reply -> no save or wrong draft update. Selected config/code/runtime changed -> reject approval. Failed commit -> explicit trust_failure resets draft; terminal revision prevents old approval closing newly submitted dialog. Successful trust -> close, remain disabled until separate enable.

## 5. Good / Base / Bad
Good: open trust -> Python version auto detected -> inspect purposes -> explicitly trust -> separately enable. Base: existing virtualenv override honored. Bad: require copying full path for ordinary PATH install, SmallToolbar silently shrinks MediumButton, readonly Input suggests editing, automatic detection implies authorization, selected manual probe failure switches candidate invisibly.

## 6. Tests Required
Observed red actual24vs32 and absent PATH command logs; green production render/request tests. Real temporary config discovery/cancel0bytes/approval/reopen override/disabled; explicit retry pinned, package discovered override actually saved, normalized path parse/native InspectAsync success; Core stale selection no trustwrite; C# owned PATH precedence/all4names/invalidsegments/WindowsApps/empty/bounds/explicitnoFallback/Job/hash/privacy. FullDesktop/Core/C#safe/scopedstrict/workspace/fmt/gates/spec; actual isolated LightDark default/min/missing/env/list/detail PNGs inspected. User/native/DPI/install evidence separate.

## 7. Wrong vs Correct
Wrong: `.with_size(Medium)` on child but Toolbar remainsSmall; `original.is_empty()` rejects discovery; clear only selected draft path before failed probe. Correct: size compound owner, pass empty typed native command, retain attempted_path distinct from candidate, validate response and rehash at original locked trust admission. See outcomes.md for actual checks, one review findings repaired and release limits.
