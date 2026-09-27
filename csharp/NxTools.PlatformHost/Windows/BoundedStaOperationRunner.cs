namespace NxTools.PlatformHost.Windows;

internal static class BoundedStaOperationRunner
{
    private static readonly object ActiveGate = new();
    private static readonly Dictionary<string, int> ActiveOperations = new(StringComparer.Ordinal);

    public static bool TryRun<T>(
        string threadName,
        TimeSpan timeout,
        Func<CancellationToken, T> operation,
        out T? result,
        out string? error) =>
        TryRun(threadName, timeout, 1, operation, out result, out error, CancellationToken.None);

    public static bool TryRun<T>(
        string threadName,
        TimeSpan timeout,
        Func<CancellationToken, T> operation,
        out T? result,
        out string? error,
        CancellationToken cancellationToken) =>
        TryRun(threadName, timeout, 1, operation, out result, out error, cancellationToken);

    internal static bool TryRun<T>(
        string threadName,
        TimeSpan timeout,
        int maxConcurrency,
        Func<CancellationToken, T> operation,
        out T? result,
        out string? error,
        CancellationToken cancellationToken)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(threadName);
        ArgumentOutOfRangeException.ThrowIfLessThan(maxConcurrency, 1);
        ArgumentNullException.ThrowIfNull(operation);
        lock (ActiveGate)
        {
            ActiveOperations.TryGetValue(threadName, out var activeCount);
            if (activeCount >= maxConcurrency)
            {
                result = default;
                error =
                    $"{threadName} is still finishing a previous bounded operation.";
                return false;
            }
            ActiveOperations[threadName] = activeCount + 1;
        }
        var operationCancellation = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        var completed = new ManualResetEventSlim();
        T? capturedResult = default;
        Exception? capturedError = null;
        var thread = new Thread(() =>
        {
            try
            {
                capturedResult = operation(operationCancellation.Token);
            }
            catch (OperationCanceledException)
                when (operationCancellation.IsCancellationRequested)
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
                    if (ActiveOperations.TryGetValue(threadName, out var remaining)
                        && remaining > 1)
                    {
                        ActiveOperations[threadName] = remaining - 1;
                    }
                    else
                    {
                        ActiveOperations.Remove(threadName);
                    }
                }
                completed.Set();
            }
        })
        {
            IsBackground = true,
            Name = threadName
        };
        try
        {
            thread.SetApartmentState(ApartmentState.STA);
            thread.Start();
        }
        catch (Exception exception)
        {
            lock (ActiveGate)
            {
                if (ActiveOperations.TryGetValue(threadName, out var remaining)
                    && remaining > 1)
                {
                    ActiveOperations[threadName] = remaining - 1;
                }
                else
                {
                    ActiveOperations.Remove(threadName);
                }
            }
            operationCancellation.Dispose();
            result = default;
            error = exception.Message;
            return false;
        }

        bool finished;
        try
        {
            finished = completed.Wait(timeout, cancellationToken);
        }
        catch (OperationCanceledException)
        {
            operationCancellation.Cancel();
            result = default;
            error = $"{threadName} was cancelled.";
            return false;
        }
        if (!finished)
        {
            operationCancellation.Cancel();
            result = default;
            error = $"{threadName} timed out after {timeout.TotalMilliseconds:0} ms.";
            return false;
        }
        if (cancellationToken.IsCancellationRequested)
        {
            operationCancellation.Cancel();
            result = default;
            error = $"{threadName} was cancelled.";
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
        operationCancellation.Dispose();
        return true;
    }
}
