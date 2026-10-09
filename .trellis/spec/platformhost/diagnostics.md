# PlatformHost diagnostics

Evidence: `repo:csharp/NxTools.PlatformHost/Diagnostics/PlatformLog.cs#Write`, `repo:csharp/NxTools.PlatformHost/Diagnostics/PlatformExceptionDiagnostics.cs`, `repo:csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs`.

IPlatformLog.Write receives level/event/correlation/structured fields. Observed sink capacity4096, TryWrite drop counting, daily JSONL/14-day cleanup and recursive sensitive-field redaction. Do not rely on redactor to sanitize arbitrary string values: fixed stage/exception type/HRESULT/stack/native code only; raw Exception.Message, command result data and child/URL/path/user payload forbidden. Pipe completion records has_data, not data itself. No pointer/frame/hotkey polling/byte-tick event streams; log state transitions/rate-limited repeated failures. Sink failure must not alter feature result.

Write is nonblocking; observed Dispose drains/joins synchronously, so do not claim bounded shutdown from TryWrite alone. No generic logging redesign in documentation scope. Tests use CreateForDirectory and structured/exception privacy assertions; do not read real logs. Safe compiled harness filter results are distinct from live native acceptance. [Shared privacy](../shared/engineering.md).
