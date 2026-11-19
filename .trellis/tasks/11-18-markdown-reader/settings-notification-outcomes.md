# Markdown settings notifications

## Behavior

Markdown settings publishes operation results through the existing application NotificationCenter. Official NotificationList retains presentation, duration, pause, dismissal and passive-window ownership. The settings page retains observed per-extension association status and its existing controls; there is no private transient Feedback field or result Alert.

- Durable preference saves emit Success; failed saves emit Error and retain the previous applied preferences.
- Registration and removal emit Success. Opening Default Apps emits Info with per-extension instructions, without claiming the default changed.
- Explicit status refresh emits Info or Error. Constructor and activation queries remain silent on both success and failure.
- A refused disable/removal explains that the Windows default must change first. An eligible disable retains its operation ID through the durable save and emits one terminal result.
- Each result retains the originating settings window and an invocation-specific UUID. Center updates are deferred. Duplicate, unrelated and superseded command results cannot publish another notification.

## Verification

`cargo test -p nxtools-desktop markdown_reader -- --test-threads=1`: **28 passed**, including seven settings regressions.

The settings regressions use the production SettingsView under Base Root and an actual NotificationCenter. They cover pointer activation of the preference and association buttons, durable save and corrupt-config failure, unchanged setting-row bounds after results, source-window metadata, default-app readback, busy/no-op saves, refusal to disable a current handler, successful disable, explicit errors and failed submission, disconnect, automatic-query silence, and duplicate/superseded completion.

The complete reader filter also covers the existing source/preview typography, tabs, split divider, outline pointer selection, document loading/save conflicts, HTML export and history paths.

## Verification limits

These are compiled GPUI TestWindow and owned temporary-configuration checks. Installed/native notification display, physical input, mixed DPI and full Desktop strict lint were not rerun for this slice. No package, installed executable, real association or Windows default was changed.
