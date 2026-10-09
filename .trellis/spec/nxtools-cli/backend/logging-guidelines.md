# CLI diagnostics

Evidence: `repo:rust/apps/nxtools-cli/src/lib.rs#CliCommand`, `repo:rust/apps/nxtools-cli/src/main.rs`.

CLI stdout/JSON is explicitly requested product output and may expose task fields; stderr is actionable user error. Do not equate that with telemetry or copy argv/headers/URLs/paths into lifecycle logs. Feature diagnostics use request IDs, action/result kinds and elapsed counts through owning engine. No extra parse-loop logs. [Engineering privacy](../../shared/engineering.md).
