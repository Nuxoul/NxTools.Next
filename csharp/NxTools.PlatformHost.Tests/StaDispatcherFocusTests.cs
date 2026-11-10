using System.Reflection;
using System.Windows.Threading;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class StaDispatcherFocusTests
{
    public static Task DispatcherOwnersShutdownBeforeRelease()
    {
        VerifyBoundedRunnerDispatcherLifecycle();
        VerifyShellContextMenuDispatcherLifecycle();
        return Task.CompletedTask;
    }

    public static Task OneCommanderProcessGatePrecedesTreeWalk()
    {
        var gateType = typeof(BoundedStaOperationRunner).Assembly.GetType(
            "NxTools.PlatformHost.Windows.OneCommanderFocusProcessGate");
        var resolve = gateType?.GetMethod(
            "ResolveTopLevelWindow",
            BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public);
        if (resolve is null)
        {
            throw new InvalidOperationException("The OneCommander focus process gate is missing.");
        }

        var treeWalks = 0;
        var windowChecks = 0;
        var unrelated = InvokeResolve(
            resolve,
            processId: 14,
            isExpectedProcess: _ => false,
            findTopLevelWindow: () =>
            {
                Interlocked.Increment(ref treeWalks);
                return 120;
            },
            windowBelongsToExpectedProcess: _ =>
            {
                Interlocked.Increment(ref windowChecks);
                return true;
            });
        Assert(unrelated == 0, "An unrelated UIA sender produced a tracked window.");
        Assert(treeWalks == 0, "An unrelated UIA sender reached the raw-tree walk.");
        Assert(windowChecks == 0, "An unrelated UIA sender reached native window validation.");

        var target = InvokeResolve(
            resolve,
            processId: 42,
            isExpectedProcess: processId => processId == 42,
            findTopLevelWindow: () =>
            {
                Interlocked.Increment(ref treeWalks);
                return 240;
            },
            windowBelongsToExpectedProcess: window => window == 240);
        Assert(target == 240, "A OneCommander child without its own HWND lost its top-level ancestor.");
        Assert(treeWalks == 1, "The OneCommander sender did not perform exactly one ancestor resolution.");

        return Task.CompletedTask;
    }

    private static void VerifyBoundedRunnerDispatcherLifecycle()
    {
        var threadName = $"NxTools.Tests.DispatcherCleanup.{Guid.NewGuid():N}";
        Dispatcher? dispatcher = null;
        var shutdownObserved = 0;
        var slotRetainedDuringShutdown = 0;

        var completed = BoundedStaOperationRunner.TryRun(
            threadName,
            TimeSpan.FromSeconds(2),
            cancellationToken =>
            {
                dispatcher = Dispatcher.CurrentDispatcher;
                dispatcher.ShutdownFinished += (_, _) =>
                {
                    Interlocked.Exchange(ref shutdownObserved, 1);
                    var admittedDuringShutdown = BoundedStaOperationRunner.TryRun(
                        threadName,
                        TimeSpan.FromMilliseconds(250),
                        cancellationToken => true,
                        out bool? _,
                        out var nestedError);
                    if (!admittedDuringShutdown)
                    {
                        Interlocked.Exchange(ref slotRetainedDuringShutdown, 1);
                    }
                };
                return true;
            },
            out bool? result,
            out var error);

        Assert(completed && result == true, error ?? "The bounded STA fixture did not complete.");
        var ownedDispatcher = dispatcher
            ?? throw new InvalidOperationException(
                "The bounded STA fixture did not create its isolated Dispatcher.");
        Assert(Volatile.Read(ref shutdownObserved) == 1, "The bounded runner left its WPF Dispatcher active.");
        Assert(
            ownedDispatcher.HasShutdownFinished,
            "The bounded runner returned before WPF Dispatcher shutdown finished.");
        Assert(
            Volatile.Read(ref slotRetainedDuringShutdown) == 1,
            "The bounded runner released its named admission before Dispatcher shutdown.");

        Assert(
            BoundedStaOperationRunner.TryRun(
                threadName,
                TimeSpan.FromSeconds(1),
                _ => true,
                out bool? nextResult,
                out var nextError) && nextResult == true,
            nextError ?? "The bounded runner did not release its slot after thread cleanup.");
    }

    private static void VerifyShellContextMenuDispatcherLifecycle()
    {
        var worker = new ShellContextMenuSessionWorker("unused-fixture-target");
        Dispatcher? dispatcher = null;
        using var shutdownFinished = new ManualResetEventSlim();
        try
        {
            var tryInvoke = typeof(ShellContextMenuSessionWorker)
                .GetMethods(BindingFlags.Instance | BindingFlags.NonPublic)
                .Single(method => method.Name == "TryInvoke" && method.IsGenericMethodDefinition)
                .MakeGenericMethod(typeof(bool));
            Func<bool> operation = () =>
            {
                dispatcher = Dispatcher.CurrentDispatcher;
                dispatcher.ShutdownFinished += (_, _) => shutdownFinished.Set();
                return true;
            };
            object?[] arguments = [operation, CancellationToken.None, null, null];
            var invoked = (bool)(tryInvoke.Invoke(worker, arguments) ?? false);
            Assert(invoked, arguments[3] as string ?? "The Shell context-menu worker rejected fixture work.");
        }
        finally
        {
            worker.Dispose();
        }

        var ownedDispatcher = dispatcher
            ?? throw new InvalidOperationException(
                "The Shell context-menu fixture did not create its isolated Dispatcher.");
        Assert(
            shutdownFinished.Wait(TimeSpan.FromSeconds(2)),
            "The Shell context-menu worker exited without shutting down its WPF Dispatcher.");
        Assert(
            ownedDispatcher.HasShutdownFinished,
            "The Shell context-menu worker signaled exit before Dispatcher shutdown finished.");
    }

    private static long InvokeResolve(
        MethodInfo resolve,
        int processId,
        Func<int, bool> isExpectedProcess,
        Func<long> findTopLevelWindow,
        Func<long, bool> windowBelongsToExpectedProcess)
    {
        return (long)(resolve.Invoke(
            null,
            [
                processId,
                isExpectedProcess,
                findTopLevelWindow,
                windowBelongsToExpectedProcess
            ]) ?? 0L);
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }
    }
}
