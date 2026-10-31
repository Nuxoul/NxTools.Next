# Desktop diagnostics

Evidence: `repo:rust/apps/nxtools-desktop/src/runtime.rs#DesktopRuntime`, `repo:rust/apps/nxtools-desktop/src/downloads.rs`, `repo:rust/apps/nxtools-desktop/src/quick_launch.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard.rs`.

Runtime retains core LogGuard. Operation ID crosses Rust/IPC/C#; log accepted/start, meaningful transition/terminal and elapsed native/fixed failure codes. No query/text/URL/path/name/title/icon resource/header/cookie or raw command Debug. No render/hover/pointer/animation/byte-tick spam; coalesce progress and repeated failures. Temporary probes uniquely tagged then removed. UI labels requested by users are not telemetry. [Shared diagnostics](../../shared/engineering.md); generic string error interpolation must be explicitly sanitized, not trusted because it came from a typed owner.
