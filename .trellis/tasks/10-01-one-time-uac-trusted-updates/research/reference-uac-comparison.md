# Reference Comparison: Throne-dev and satelite-proxy-main

## Scope

Read-only comparison for the NxTools one-time UAC/trusted-update investigation. Neither external project was modified.

## Throne-dev

Evidence paths:

- `script/windows_installer.iss:17-40, 103-153`
- `src/main.cpp:244-284, 325-333`
- `src/global/Configs.cpp:86-103`
- `src/sys/windows/AutoRun.cpp:250-304`
- `src/database/SettingsRepo.cpp:56-57`

Findings:

- The Inno installer uses `PrivilegesRequired=lowest`; it supports a normal per-user install and an optional all-users/admin mode. The installer itself is not a permanently elevated application.
- The application has an explicit user setting (`windows_set_admin`) for running the whole app as administrator. On Windows startup, if that setting is enabled and the process is not elevated, `main.cpp` calls `WinCommander::runProcessElevated(...)` and exits the normal instance. This is a feature setting, not a silent update mechanism; if enabled it can cause recurring UAC on launches.
- AutoRun is represented as a scheduled task and can be configured for an elevated task based on the admin setting. It does not establish a protected-binary update trust chain.
- The inspected source/package flow contains an Inno installer and release packaging, but no evidence of a resident trusted updater, signed update manifest, or LocalSystem service replacing a protected application tuple.

NxTools implication: Throne's quieter normal startup is primarily because normal use/install is standard-user and the product does not use a protected service tuple for routine app deployment. Its optional admin-run feature is not a suitable update design to copy.

## satelite-proxy-main

Evidence paths:

- `src-tauri/tauri.conf.json:28-37`
- `src-tauri/tauri.windows.conf.json:1-21`
- `scripts/build-windows.ps1:98-109, 111-201`
- `.github/workflows/release.yml:3-132`
- `src-tauri/src/commands/core.rs:163-252`
- `src-tauri/src/core/elevate.rs:1-188`
- `src-tauri/src/core/manager.rs:664-715`
- `src-tauri/src/portable.rs:1-113`

Findings:

- Tauri bundles an NSIS installer, but the inspected `tauri.conf.json`, Windows overlay, build script, and release workflow show no Tauri updater plugin, updater endpoint, public key, Authenticode signing configuration, or silent app-update broker.
- `check_app_update` only checks the latest GitHub release tag with a six-hour cache/backoff. The inspected command does not install or replace the application.
- `download_core` updates bundled proxy-core assets, which is separate from updating the Satelite application itself.
- `core/elevate.rs` uses `ShellExecuteExW("runas")` only to launch the proxy core/helper for Windows TUN mode. `manager.rs` tracks that elevated child PID and binds it to a Windows Job Object to avoid orphaned processes. This is an operation-specific elevation, not an update authorization model.
- Portable mode keeps data/resources beside the executable and explicitly avoids installation; it is another reason normal startup does not require UAC.

NxTools implication: Satelite demonstrates a good pattern for isolating elevation to the single operation that needs it and tracking the elevated child lifetime. It does not solve trusted silent updates because it has no protected service update chain in the inspected code.

## Comparative conclusion

- Neither reference project provides evidence for “one UAC once, then silently replace a protected service-owned application tuple.”
- Throne avoids routine UAC by keeping ordinary install/start standard-user and making admin mode explicit.
- Satelite avoids routine UAC by keeping the app standard-user and elevating only the Windows TUN child when needed.
- NxTools is structurally different: the LocalSystem indexer and protected `ProgramData` Desktop/PlatformHost/Indexer tuple require an elevated writer when a new package is not already trusted. Removing that prompt without adding a real trust root would weaken the security boundary.
- The practical NxTools options remain: add Authenticode publisher signing and a service-mediated signed update, or use a one-time user-consented pinned update key with a protected service broker. Current unsigned binaries and multiple stale install paths must be resolved before choosing between them.
