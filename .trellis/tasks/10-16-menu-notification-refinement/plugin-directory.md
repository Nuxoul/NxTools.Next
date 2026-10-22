# Extensions: open the managed local plugin directory

## Requested boundary
User asks for an Extensions-page button that opens the local plugin directory. Add **打开插件目录** to the existing header, opening `config_path.parent().join("extensions")`, the same managed root used by normal imports and built-in seeding. Create the root if absent, on the background executor. Do not change registry/config bytes, trust, enablement, runtime or execute/import plugins. No packaging/install/service/user-data operations.

## Component and lifecycle decision
Existing header shape is a title plus outlined commands. Inspected mandatory selection rules/current table/locked API index and released Kit0.7.1 toolbar.rs and button/button.rs: use original `Toolbar.content` plus existing `app_button`, real `Button::outline`, default Medium32. Do not use sized Toolbar.child, which forces ghost/compact. Preserve actual minimum744x430/default header bounds, Light/Dark painted outlines and official roving focus with five commands.

Rust ExtensionApplication owns the path and one pending operation token, installed before async creation/dispatch. Reuse its existing bounded platform-request map/result/disconnect/timeout path, preserving runtime-environment behavior; operation identity distinguishes directory diagnostics/copy. Send existing typed `OpenShellTarget { target, action: Open }` through C#, never Rust process launch or a new native command. Reviewed unchanged WindowsPlatformAdapter.OpenShellTargetAsync: ShellTarget validation -> dedicated STA Launch -> correlated terminal success/native code. Shell success does not certify Explorer foreground. Failures are retryable with safe user copy; late replies cannot complete a newer operation. Cross-owner UI callbacks remain deferred.

## Verification
First extend actual header paint regression before implementation. Add owned-temp/fake-bridge coverage for exact target, absent/existing directory, duplicate/pending, native failure, disconnect/late result, timeout and create/admission failure. Verify no config/package mutations, no Explorer launch, actual pointer/keyboard once-only dispatch and minimum/default containment. Run Desktop suite, strict scoped lint, workspace check, fmt and relevant source/documentation gates. Generated render or tests are not installed/native/visual user acceptance.
