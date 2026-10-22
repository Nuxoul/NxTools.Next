# Executable runtime environment contracts

## 1. Scope / Trigger
Explicit Extensions -> Run Environment or package management -> Run Environment; import path row and picker errors. No automatic discovery or plugin execution on render/save.

## 2. Signatures
`PackageRegistry::{set_runtime_defaults(BTreeMap<String,PathBuf>),set_runtime_override(&str,Option<PathBuf>),runtime_executable(&str)->Result<Option<PathBuf>,PackageError>}`; original Core atomic wrappers `AppConfig::{save_extension_runtime_defaults,set_extension_runtime_override}`. Protocol `PickExtensionRuntime{program,current_path}` / `InspectExtensionRuntime{program,executable}`; C# `ExtensionRuntimePicker.Pick`, `ExtensionRuntimeInspector.InspectAsync` -> typed command result; success inspect data program/executable/version/sha256.

## 3. Contracts
Only python/node/powershell/pwsh defaults, absolute nonempty bounded control-free paths. Override before default, legacy approval fallback only without explicit config. EXE staged only. Validate before mutation/persistence; effective change calls original revoke; no Save-induced trust/enable. Runtime publication/execution validates selected identity. UI native requests pending-before-dispatch, <=4, correlation ID across Rust/IPC/C#, fixed accepted/terminal events elapsed/native error no paths/output; view request UUID+input+window current before reply. Deferred cross-entity effects avoid leased-entity reentry. Picker300s/probe transport30s/native process3s; ordinary Job256MiB/2processes/output4KiB/cleanup success gate.

## 4. Validation & Error Matrix
Cancel editor -> zero writes and retired reply; typing/reopen -> late data ignored; current browse -> original input updated; disconnect/timeout -> pending cleared/local retry. Bad relative/empty override -> keep draft and unchanged file. Invalid probe schema/program/path/hash/version -> no usable result. Native995 -> silent cancel; other failures fixed class/code, no raw exception payload. Original picker root cause unknown remains separate.

## 5. Good / Base / Bad
Good: configure virtualenv python.exe, Save, inspect source, separately trust, separately enable. Base: inherited default no-op keeps trust. Bad: choosing path implicitly grants trust, external EXE override, PATH execution on render, arbitrary stdout displayed, stale result changes another draft.

## 6. Tests Required
Import actual height red/green; domain script_packages defaults/override/legacy/EXE/revoke; Core temp bytes/reopen/section preservation; runtime publication queued cancellation and bypass rejection; GPUI environment tests real pointer/actions/viewport/SaveCancel/late/current/disconnect/timeout; C# owned version argv/Job/cleanup/hash/privacy and picker errors. Full desktop/scoped strict/workspace/fmt/source/spec checks; visual and installed native acceptance reported separately.

## 7. Wrong vs Correct
Wrong: Small24 beside Input32; duplicate import CTA; runtime Input editable inside authorization; selected path is approval; cancelled future means native cleanup complete. Correct: released matched Medium tier; one intentional import; readonly current config re-resolution on approval; independent saves; native runner retains permit until real cleanup, UI merely retires old reply. Exact evidence and remaining limits in outcomes.md.
