# Desktop interaction polish — archive slice

## Delivered source

- Fresh and missing-field menu recommendations contain ZIP quick-create, 7z quick-create and New Archive. Explicit empty/custom arrays, including explicit historical five-command arrays, retain their exact selections. Integration stays disabled by default; saving does not apply Registry changes.
- `create_zip` / `create_7z` have independent helper CLSIDs and `ExplorerCommandHandler` plans for ordinary files and folders, in both cascade and flat layouts. Official `IExplorerCommand::GetTitle` derives the first ordered selected file stem or full folder name and fixed extension. `Invoke` forwards the supplied complete `IShellItemArray` through the original once-only reader/forwarder. Editable `create` retains its original grouped DelegateExecute command.
- Quick commands enter the original Desktop parser/intent queue, normalize every source, construct the original `CreateDraft`, fix ZIP/7z independently of saved format, and admit one combined-source request through the original Job/FIFO/native owner. They open no editable form. Native existing-target confirmation, source validation, password options, cancellation, verification and publication retain their existing owners.
- Quick argv carries `--archive-correlation-id`; the same UUID becomes the original native Job operation/session ID. Accepted, planned and admission-terminal events expose count/state/elapsed only; original native Job lifecycle diagnostics remain.
- New Archive sources have an independent maximum144DIP scroll viewport, single-line filename/size rows, full-path Tooltip, original add/paste/remove controls and a count/known-size/unknown-size summary. Output/destination rows use compact horizontal Form fields; format/level, password, advanced options and fixed Medium footer remain.
- Archive preferences use the Download settings' filled official SettingGroup panels and shared aligned label/control row. Ordinary dropdowns/numbers share a200-design-pixel control width; sidebar uses the same164-design-pixel seed. All original pages, values, callbacks, section-owned saves and actions remain.

## Reference and released components

[Archive command reference](../../../docs/reference-code/archive-polish.md) records Bandizip resource480/677-family evidence, retained raw selection/output planning, absence of a legacy NxTools general archive owner, exact SDK10.0.26100.0 IExplorerCommand vtable, the complete product/native chain and lifetime invariants.

Composition uses Kit0.8.0 Settings/SettingPage/SettingGroup/SettingItem/SettingField, Form/Field, Input/Select/Switch/Checkbox/NumberInput, Collapsible, Tooltip and real outlined Buttons. `Toolbar.child` is not used for outlined commands. No dependency, vendor, native-window budget or persistence-owner change.

## Boundary coverage

- Core: fresh/missing recommendations, disabled integration, explicit empty/custom/five-command preservation and strict saved choices.
- Protocol/startup: fixed action/format vocabulary, complete ordered paths, explicit correlation, invalid correlation/arity and creation-only menus without archive extensions.
- Application: original intent admission → one original creation Job, saved-format override, complete paths/output name/destination and native ID continuity; retired or partly unreadable selections start no job or form.
- Form: real mounted source viewport bounds, actual remove Button callback and unchanged fixed footer.
- Preferences: real mounted Download-aligned row, label/control centroids and original Switch; existing section-save tests remain.
- C#: actual unmanaged GetTitle/Invoke slots on the original identity, query-only lifetime, final Release/readmission, complete ordered once-only forwarding, folder/dotfile naming, correlated quick argv and pure flat/cascade Registry plans. Existing compensation/approval/strict-parser fixtures include the new commands.

## Verification handoff

`rustfmt --edition 2024 --config skip_children=true` completed successfully on the owned changed Rust files. Parent-observed behavioral prerequisites are recorded in session scratch `archive-red-observed` and its named Core/Desktop/C# logs. Compiled green tests are owned by the parent serialized lane; this worker ran no Cargo or dotnet commands.

Focused Rust commands (with the required workstation environment):

```powershell
$env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'
cargo test -p nxtools-core --test archive_shell_defaults archive_polish_
cargo test -p nxtools-core --lib shell_settings_legacy_defaults_and_strict_choices_are_persisted
cargo test -p nxtools-protocol --lib archive_shell_integration
cargo test -p nxtools-protocol --lib desktop_archive_intent
cargo test -p nxtools-ipc --lib desktop_archive_intent
cargo test -p nxtools-desktop archive_polish_ -- --test-threads=1
cargo test -p nxtools-desktop archive_launch::tests -- --test-threads=1
cargo test -p nxtools-desktop archive::forms::tests -- --test-threads=1
cargo test -p nxtools-desktop main_window::archive_page_tests -- --test-threads=1
```

C# fixture filters after the parent's build:

- `archive polish quick-create title selection`
- `archive polish quick-create recommendation plans`
- `archive association settings fixture`
- `archive Phase F P207 identity queue cancel lifetime`
- `archive Phase F P207 final release without execute readmits`
- `archive Phase F P207 partial failures reject whole selection`

The two new fixtures were registered by the parent in Program/TestEffects. This slice does not edit those shared files. Source snapshots are retained under session scratch `baseline/archive/`; exact changed-file and validation handoff is `archive-implementation-ready.json`.

## Integration and limitations

Parent merges shared task/spec/index references and updates the native classic-menu contract's historical missing-field/five-default wording to the new recommendation contract. Explicit historical plan overloads and explicit saved arrays stay unchanged.

Compiled green verification, real Explorer activation/proxy behavior, Registry apply/revert, physical keyboard/pointer/hover, DPI, screenshots and user acceptance remain unverified. No product run, Registry/service action, build/package, install, commit or push was performed by this slice.
