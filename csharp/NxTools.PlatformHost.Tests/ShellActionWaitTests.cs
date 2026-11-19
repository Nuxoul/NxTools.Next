using System.Diagnostics;
using System.Reflection;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class ShellActionWaitTests
{
    internal static async Task InvokeWaitsBeyondMachineDeadline()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var exited = new ManualResetEventSlim();
        var apartment = ApartmentState.Unknown;
        var worker = new ShellContextMenuSessionWorker(
            $"unused-shell-invoke-{Guid.NewGuid():N}",
            (_, _, _) =>
            {
                apartment = Thread.CurrentThread.GetApartmentState();
                entered.Set();
                try
                {
                    if (!release.Wait(TimeSpan.FromSeconds(10)))
                    {
                        throw new TimeoutException("The controlled Shell invoke action was not released.");
                    }
                }
                finally
                {
                    exited.Set();
                }
            });
        var invocation = Task.Run(() => worker.Invoke(1, IntPtr.Zero, CancellationToken.None));

        try
        {
            Assert(entered.Wait(TimeSpan.FromSeconds(2)), "The worker did not enter the controlled Invoke action.");
            await Task.Delay(TimeSpan.FromMilliseconds(5400));
            Assert(!invocation.IsCompleted, "Invoke retained the five-second machine completion deadline.");

            release.Set();
            var result = await invocation.WaitAsync(TimeSpan.FromSeconds(2));
            Assert(result.Success, result.Error ?? "The released Invoke action did not succeed.");
            Assert(apartment == ApartmentState.STA, "The controlled Invoke action did not run on the worker STA.");
            Assert(exited.IsSet, "Invoke returned before the controlled action actually exited.");
        }
        finally
        {
            release.Set();
            worker.CancelCurrent();
            _ = exited.Wait(TimeSpan.FromSeconds(2));
            if (!invocation.IsCompleted)
            {
                _ = await Task.WhenAny(invocation, Task.Delay(TimeSpan.FromSeconds(2)));
            }
        }
    }

    internal static async Task MachineWaitRetainsFiveSecondDeadline()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var exited = new ManualResetEventSlim();
        var worker = new ShellContextMenuSessionWorker($"unused-shell-machine-{Guid.NewGuid():N}");
        var tryInvoke = typeof(ShellContextMenuSessionWorker)
            .GetMethods(BindingFlags.Instance | BindingFlags.NonPublic)
            .Single(method => method.Name == "TryInvoke" && method.IsGenericMethodDefinition)
            .MakeGenericMethod(typeof(bool));
        Func<bool> operation = () =>
        {
            entered.Set();
            try
            {
                if (!release.Wait(TimeSpan.FromSeconds(10)))
                {
                    throw new TimeoutException("The controlled machine Shell action was not released.");
                }
                return true;
            }
            finally
            {
                exited.Set();
            }
        };
        var arguments = new object?[] { operation, CancellationToken.None, null, null };
        var stopwatch = Stopwatch.StartNew();
        var invocation = Task.Run(() => (bool)(tryInvoke.Invoke(worker, arguments) ?? false));

        try
        {
            Assert(entered.Wait(TimeSpan.FromSeconds(2)), "The worker did not enter the controlled machine action.");
            var completed = await invocation.WaitAsync(TimeSpan.FromSeconds(7));
            stopwatch.Stop();

            Assert(!completed, "The machine Shell wait unexpectedly lost its completion deadline.");
            Assert(stopwatch.Elapsed >= TimeSpan.FromSeconds(4.5) &&
                   stopwatch.Elapsed < TimeSpan.FromSeconds(7),
                "The machine Shell wait did not retain the five-second lifecycle boundary.");
            Assert(arguments[3] is string error && error.Contains("five seconds", StringComparison.OrdinalIgnoreCase),
                "The machine Shell timeout did not preserve its existing result.");
            Assert(!exited.IsSet, "The machine timeout claimed that the held action had exited.");
        }
        finally
        {
            release.Set();
            worker.CancelCurrent();
            Assert(exited.Wait(TimeSpan.FromSeconds(2)),
                "The controlled machine action did not reach its actual exit.");
            if (!invocation.IsCompleted)
            {
                _ = await Task.WhenAny(invocation, Task.Delay(TimeSpan.FromSeconds(2)));
            }
        }
    }


    internal static async Task CancellationReturnsPromptlyWithoutWorkerReuse()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var exited = new ManualResetEventSlim();
        using var cancellation = new CancellationTokenSource();
        var invokeCalls = 0;
        var target = $"unused-shell-cancel-{Guid.NewGuid():N}";
        var worker = new ShellContextMenuSessionWorker(
            target,
            (_, _, _) =>
            {
                Interlocked.Increment(ref invokeCalls);
                entered.Set();
                try
                {
                    if (!release.Wait(TimeSpan.FromSeconds(10)))
                    {
                        throw new TimeoutException("The controlled cancelled Shell action was not released.");
                    }
                }
                finally
                {
                    exited.Set();
                }
            });
        ShellContextMenuSessionWorker? replacement = null;
        var invocation = Task.Run(() => worker.Invoke(1, IntPtr.Zero, cancellation.Token));

        try
        {
            Assert(entered.Wait(TimeSpan.FromSeconds(2)), "The worker did not enter the cancellable Invoke action.");
            var stopwatch = Stopwatch.StartNew();
            cancellation.Cancel();
            var result = await invocation.WaitAsync(TimeSpan.FromSeconds(1));
            stopwatch.Stop();

            Assert(!result.Success, "Cancellation unexpectedly published Invoke success.");
            Assert(result.Error?.Contains("cancelled", StringComparison.OrdinalIgnoreCase) == true,
                "Cancellation did not return the Shell cancellation result.");
            Assert(stopwatch.Elapsed < TimeSpan.FromSeconds(1), "Cancellation did not return promptly.");
            Assert(!exited.IsSet, "Cancellation claimed that the held native action had exited.");

            worker.Release();
            replacement = ShellContextMenuSessionWorker.Acquire(target);
            Assert(!ReferenceEquals(worker, replacement),
                "The cancelled worker was reused while its previous action was still running.");
            Assert(Volatile.Read(ref invokeCalls) == 1,
                "The held worker accepted another Invoke action before its original action exited.");
        }
        finally
        {
            replacement?.CancelCurrent();
            release.Set();
            worker.CancelCurrent();
            Assert(exited.Wait(TimeSpan.FromSeconds(2)),
                "The controlled cancelled action did not reach its actual exit.");
            if (!invocation.IsCompleted)
            {
                _ = await Task.WhenAny(invocation, Task.Delay(TimeSpan.FromSeconds(2)));
            }
        }
    }

    internal static async Task MachineTimeoutRetiresQueuedInvoke()
    {
        using var machineEntered = new ManualResetEventSlim();
        using var machineRelease = new ManualResetEventSlim();
        using var machineExited = new ManualResetEventSlim();
        var invokeCalls = 0;
        var target = $"unused-shell-retirement-{Guid.NewGuid():N}";
        var worker = new ShellContextMenuSessionWorker(
            target,
            (_, _, _) => Interlocked.Increment(ref invokeCalls));
        var tryInvoke = typeof(ShellContextMenuSessionWorker)
            .GetMethods(BindingFlags.Instance | BindingFlags.NonPublic)
            .Single(method => method.Name == "TryInvoke" && method.IsGenericMethodDefinition)
            .MakeGenericMethod(typeof(bool));
        var requests = (System.Collections.Concurrent.BlockingCollection<Action>)
            (typeof(ShellContextMenuSessionWorker)
                .GetField("_requests", BindingFlags.Instance | BindingFlags.NonPublic)!
                .GetValue(worker) ?? throw new InvalidOperationException("Missing Shell worker queue."));
        Func<bool> machineOperation = () =>
        {
            machineEntered.Set();
            try
            {
                if (!machineRelease.Wait(TimeSpan.FromSeconds(10)))
                {
                    throw new TimeoutException("The controlled machine operation was not released.");
                }
                return true;
            }
            finally
            {
                machineExited.Set();
            }
        };
        var machineArguments = new object?[] { machineOperation, CancellationToken.None, null, null };
        var machineWait = Task.Run(() => (bool)(tryInvoke.Invoke(worker, machineArguments) ?? false));
        Task<WindowsCommandResult>? invokeWait = null;
        ShellContextMenuSessionWorker? replacement = null;

        try
        {
            Assert(machineEntered.Wait(TimeSpan.FromSeconds(2)),
                "The worker did not enter the held machine operation.");
            invokeWait = Task.Run(() => worker.Invoke(1, IntPtr.Zero, CancellationToken.None));
            Assert(SpinWait.SpinUntil(() => requests.Count == 1, TimeSpan.FromSeconds(2)),
                "Invoke was not queued behind the held machine operation.");

            var machineCompleted = await machineWait.WaitAsync(TimeSpan.FromSeconds(7));
            Assert(!machineCompleted, "The held machine operation did not reach its five-second timeout.");
            Assert(machineArguments[3] is string error &&
                   error.Contains("five seconds", StringComparison.OrdinalIgnoreCase),
                "The machine timeout did not retain its existing result.");
            Assert(!machineExited.IsSet,
                "The logical machine timeout claimed that the active operation physically exited.");

            var invokeResult = await invokeWait.WaitAsync(TimeSpan.FromSeconds(1));
            Assert(!invokeResult.Success,
                "A queued Invoke succeeded after its worker was retired by a machine timeout.");
            Assert(Volatile.Read(ref invokeCalls) == 0,
                "The queued Invoke action executed after its worker was retired.");
            Assert(!machineExited.IsSet,
                "Retiring the queued Invoke interrupted the active machine operation.");

            worker.Release();
            replacement = ShellContextMenuSessionWorker.Acquire(target);
            Assert(!ReferenceEquals(worker, replacement),
                "The retired worker was returned from cache before its active operation exited.");
        }
        finally
        {
            replacement?.CancelCurrent();
            machineRelease.Set();
            worker.CancelCurrent();
            Assert(machineExited.Wait(TimeSpan.FromSeconds(2)),
                "The held machine operation did not reach its actual exit.");
            _ = await Task.WhenAny(machineWait, Task.Delay(TimeSpan.FromSeconds(2)));
            if (invokeWait is not null && !invokeWait.IsCompleted)
            {
                _ = await Task.WhenAny(invokeWait, Task.Delay(TimeSpan.FromSeconds(2)));
            }
        }
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }
    }
}
