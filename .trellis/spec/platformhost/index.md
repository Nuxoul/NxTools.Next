# C# PlatformHost native boundary

Evidence: `repo:csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj`, `repo:csharp/NxTools.PlatformHost/Program.cs`, `repo:csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs`. net8.0-windows/win-x64 with WinForms/WPF; one native adapter process, not a GPUI state owner.

## Pre-Development Checklist
Read [authority](../shared/architecture.md), [engineering](../shared/engineering.md), matching indexed reference, [native contracts](native-contracts.md), [diagnostics](diagnostics.md) and [verification](../shared/verification.md). Preserve source HWND, ID/generation, pixel units, focus/noactivate and STA ownership before changing APIs.

## Quality Check
Source gates for shell/download/preview/browser boundaries plus exact name-filtered C# harness when compiled checks authorized. `dotnet build csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj --no-restore`; `dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -- "centered worker admission"` is a compiled native-fixture test, not source check. Filters are substring matches; verify actual RUN/PASS, zero-match exit0 isn't evidence. Never unfiltered safe suite: explicitly exclude real DOpus navigation cases. No actual registry/service/installer/live Shell acceptance implied. [Entry](../index.md).
