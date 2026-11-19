# B5 C# native outcomes

## Implemented boundary

Wire shape is additive and typed:

```json
{"command":"suspend_system","data":{"hibernate":false}}
```

`hibernate:false` requests Sleep; `hibernate:true` requests Hibernate. The production call chain is:

`PlatformCommandParser.ParseSuspendSystem` → `PlatformPipeClient.ExecutePlatformCommandAsync` → `WindowsPlatformAdapter.SuspendSystem` → `ICompletionSystemActions.RequestSuspend(bool)` → `WindowsCompletionSystemActions.RequestSuspend`.

The serial command reader detaches both `suspend_system` and the existing bounded `shutdown_system` through `PlatformCommandSequenceDispatcher`. A held suspend task therefore does not block a later ordinary command, and dispatcher drain retains the task until its real delegate completes.

## Native behavior

`WindowsCompletionSystemActions` uses the Windows power capability APIs before mutation:

- Sleep: `IsPwrSuspendAllowed`.
- Hibernate: `IsPwrHibernateAllowed`.
- Unsupported capability returns `success:false`, `native_error_code:50`; there is no fallback to the other action.
- The original process token is opened with query/adjust-privilege access, `SeShutdownPrivilege` is resolved and enabled with `AdjustTokenPrivileges`, and every actual Win32 error is returned unchanged.
- `SetSuspendState(hibernate, false, false)` performs the requested action without force-critical or wake-event disabling substitutions.
- The prior privilege state is restored after the call and the process-token handle is closed on every post-open path.

Generic command diagnostics retain the original correlation ID and now include `action=sleep|hibernate|shutdown`; terminal records include `terminal=succeeded|failed`, `elapsed_ms`, and `native_error_code`. No request payload or user data is logged.

## Changed paths

- `csharp/NxTools.PlatformHost/Windows/CompletionSystemActions.cs`: extended the original interface and Windows implementation with capability checks, narrow shutdown-privilege handling, `SetSuspendState`, error preservation, privilege restoration, and token closure.
- `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs`: added the typed `SuspendSystem` adapter method.
- `csharp/NxTools.PlatformHost/Protocol/PlatformCommandParser.cs`: added `SuspendSystemCommand(bool Hibernate)` and strict command-name parsing.
- `csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs`: routed `suspend_system` through the production adapter and added bounded action/terminal diagnostic fields.
- `csharp/NxTools.PlatformHost/Protocol/PlatformCommandSequenceDispatcher.cs`: detached Sleep/Hibernate and bounded shutdown from the serial reader.
- `csharp/NxTools.PlatformHost.Tests/Program.cs`: added controlled full generic dispatch coverage for Sleep, Hibernate, unsupported `50`, denied `5`/`1314`, native failure `87`, exact result envelopes/diagnostics, and held-suspend responsiveness/drain.
- `csharp/NxTools.PlatformHost.Tests/TestEffects.cs`: classified both exact `B5 completion` symbols as reviewed `Fixture` tests with no native power or process-token effects.

## Verification

The safe fixture builds used the existing restored toolchain and package graph.

Behavioral RED:

- `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` — exit `0`, warnings `0`, errors `0`.
- Exact LIST for `B5 completion suspend system wire routes through production dispatch` — exit `0`, selected `1`, run `0`, effect `Fixture`.
- Identical RUN — exit `1`, selected/run/fail `1/1/1`; the existing generic production dispatch returned unsupported for `suspend_system`.

GREEN/final:

- `dotnet build csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj --no-restore` — exit `0`, warnings `0`, errors `0`.
- Scoped `git diff --check` across the seven C# paths and this outcome file — exit `0`.
- `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` — exit `0`, warnings `0`, errors `0`.
- Exact LIST with filter `B5 completion` — exit `0`, selected `2`, run `0`, both reviewed `Fixture`, no native/restricted selection.
- Identical RUN — exit `0`, selected/run/pass/skip/fail/not_run `2/2/2/0/0/0`.
- The four stderr lines `Platform command 'suspend_system' failed.` are the expected generic failure notices for the controlled unsupported, access-denied, privilege-not-held, and native-failure envelopes. Diagnostic assertions verified correlation, action, terminal state, elapsed field, and exact native code.
- The held delegate was released and `DrainAsync` completed before the fixture exited. Tests opened no process token and invoked no capability query, `SetSuspendState`, PowerShell power command, GUI, registry, service, package, or product process.

The existing Debug harness was also verified without building: `csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe --fixtures --list "B5 completion"`, then identical filter without --list. Bodies, typed fake and exact TestEffects entries were inspected. LIST2/RUN2/PASS2/FAIL0/SKIP0/not_run0; exits0/0. Logs `b5-platformhost-final-list.log`, `b5-platformhost-final.log`, `b5-platformhost-final-results.json` are in the session scratch. No native capability/token/power call or current Desktop exit occurred.

## Limitation

Controlled native results prove parser/routing/result/diagnostic/error preservation and detached-reader lifecycle. They do not establish that a physical machine accepted Sleep or Hibernate; real power capability and transition acceptance were intentionally not executed.
