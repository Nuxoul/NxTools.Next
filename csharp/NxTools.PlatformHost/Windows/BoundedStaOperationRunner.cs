namespace NxTools.PlatformHost.Windows;

internal static class BoundedStaOperationRunner
{
    private static readonly object ActiveGate = new();
    private static readonly HashSet<string> ActiveOperations =
        new(StringComparer.Ordinal);

    public static bool TryRun<T>(
        string threadName,
        TimeSpan timeout,
        Func<CancellationToken, T> operation,
        out T? result,
        out string? error)
    {
        lock (ActiveGate)
        {
            if (!ActiveOperations.Add(threadName))
            {
                result = default;
                error =
                    $"{threadName} is still finishing a previous bounded operation.";
                return false;
            }
        }
        var cancellation = new CancellationTokenSource();
        var completed = new ManualResetEventSlim();
        T? capturedResult = default;
        Exception? capturedError = null;
        var thread = new Thread(() =>
        {
            try
            {
                capturedResult = operation(cancellation.Token);
            }
            catch (OperationCanceledException)
                when (cancellation.IsCancellationRequested)
            {
            }
            catch (Exception exception)
            {
                capturedError = exception;
            }
            finally
            {
                lock (ActiveGate)
                {
                    ActiveOperations.Remove(threadName);
                }
                completed.Set();
            }
        })
        {
            IsBackground = true,
            Name = threadName
        };
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();

        if (!completed.Wait(timeout))
        {
            cancellation.Cancel();
            result = default;
            error = $"{threadName} timed out after {timeout.TotalMilliseconds:0} ms.";
            return false;
        }
        if (capturedError is not null)
        {
            result = default;
            error = capturedError.Message;
            return false;
        }
        result = capturedResult;
        error = null;
        return true;
    }
}
