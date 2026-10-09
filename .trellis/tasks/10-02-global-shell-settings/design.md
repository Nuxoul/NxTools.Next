# Global Shell Settings Apply Contract

## Data flow

```text
NetworkSettingsView save
 -> NetworkSettingsApplication::apply
 -> persist desired AppConfig
 -> PlatformBridge::send_command_correlated(ConfigureDesktopShell)
 -> PlatformPipeClient command dispatch
 -> WindowsPlatformAdapter.EventWindow.ApplyDesktopShellConfiguration
 -> approved executable lookup
 -> Run-key registration
 -> NotifyIcon configure
 -> correlated PlatformCommandResult { success, native_error_code, data }
 -> Rust pending-command match
 -> success publishes SystemSettings/close + toast; failure keeps window open + staged guidance
```

## Minimal repair

- Restore the exhaustive Rust match over `(failure_kind, failure_stage)`, explicitly handling `desktop_executable_unavailable`, `startup_registration_failed`, `tray_icon_failed`, enqueue/disconnect/timeout, and generic failure.
- Return a `WindowsCommandResult(false, safe message, NativeErrorCode, Data: "windows_shell_apply_failed")` from the C# catch. Preserve `Win32Exception.NativeErrorCode` / `ExternalException.ErrorCode` where present; do not return exception messages containing paths or machine details.
- Preserve current persistence semantics and correlation IDs. Do not set shared live SystemSettings on a failed/stale result; `close_to_tray` remains fail-closed when its prerequisite shell config fails.
- Do not change approved executable or UAC/security gates.

## Tests

- Pure Rust stage mapping for every C# `Data` token plus timeout/disconnect/enqueue and unknown.
- Rust GPUI/result test: matching success applies/closes; matching failure leaves UI open, persists desired config, leaves live close-to-tray false; stale ID does not mutate.
- C# injected boundary tests for missing executable, startup registration failure, tray failure, and unexpected exceptions, asserting success flag, safe Data token and native code.
- Validate with desktop focused tests/fmt/clippy and full PlatformHost tests.