using System.Runtime.InteropServices;
using System.Text;

namespace NxTools.PlatformHost.Windows;

internal static class StaDialogRunner
{
    private const uint WmClose = 0x0010;
    private const uint WmQuit = 0x0012;
    private const int CancellationGraceMilliseconds = 250;
    private const int CancellationJoinMilliseconds = 250;

    private sealed record ThreadResult(
        WindowsCommandResult? Result,
        Exception? Failure,
        bool Cancelled);

    // The caller already owns a bounded STA slot; keep it until the real modal call exits.
    internal static WindowsCommandResult RunInline(Func<WindowsCommandResult> showDialog, CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested) return CancelledResult();
        var threadId = GetCurrentThreadId();
        using var registration = cancellationToken.Register(() => RequestDialogClose(threadId));
        try { var result = showDialog(); return cancellationToken.IsCancellationRequested ? CancelledResult() : result; }
        catch (Exception failure) { return FailureResult(failure); }
    }

    public static WindowsCommandResult Run(
        string threadName,
        Func<WindowsCommandResult> showDialog)
        => Run(threadName, showDialog, CancellationToken.None);

    public static WindowsCommandResult Run(
        string threadName,
        Func<WindowsCommandResult> showDialog,
        CancellationToken cancellationToken)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(threadName);
        ArgumentNullException.ThrowIfNull(showDialog);
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }

        var completion = new TaskCompletionSource<ThreadResult>(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var nativeThreadId = 0u;
        var cancellationRequested = 0;
        var thread = new Thread(() =>
        {
            try
            {
                nativeThreadId = GetCurrentThreadId();
                if (cancellationToken.IsCancellationRequested)
                {
                    Interlocked.Exchange(ref cancellationRequested, 1);
                    completion.TrySetResult(new ThreadResult(null, null, true));
                    return;
                }

                var result = showDialog();
                if (cancellationToken.IsCancellationRequested ||
                    Volatile.Read(ref cancellationRequested) != 0)
                {
                    completion.TrySetResult(new ThreadResult(null, null, true));
                    return;
                }
                completion.TrySetResult(new ThreadResult(result, null, false));
            }
            catch (Exception error)
            {
                completion.TrySetResult(
                    cancellationToken.IsCancellationRequested ||
                    Volatile.Read(ref cancellationRequested) != 0
                        ? new ThreadResult(null, null, true)
                        : new ThreadResult(null, error, false));
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
        catch (Exception error)
        {
            return FailureResult(error);
        }

        using var registration = cancellationToken.Register(() =>
        {
            Interlocked.Exchange(ref cancellationRequested, 1);
            RequestDialogClose(nativeThreadId);
        });

        var cancelled = false;
        if (cancellationToken.CanBeCanceled)
        {
            try
            {
                completion.Task.Wait(cancellationToken);
            }
            catch (OperationCanceledException)
            {
                cancelled = true;
            }
        }
        else
        {
            try
            {
                thread.Join();
            }
            catch (Exception error)
            {
                return FailureResult(error);
            }
        }

        if (cancelled ||
            cancellationToken.IsCancellationRequested ||
            Volatile.Read(ref cancellationRequested) != 0)
        {
            RequestDialogClose(nativeThreadId);
            WaitForCancellationCleanup(completion.Task, thread, nativeThreadId);
            return CancelledResult();
        }

        if (!completion.Task.IsCompleted)
        {
            return new WindowsCommandResult(false, "Windows dialog did not return a result.", NativeErrorCode: 31);
        }

        var threadResult = completion.Task.GetAwaiter().GetResult();
        if (threadResult.Cancelled || cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        return threadResult.Failure is null
            ? threadResult.Result ?? new WindowsCommandResult(false, "Windows dialog did not return a result.", NativeErrorCode: 31)
            : FailureResult(threadResult.Failure);
    }

    private static void WaitForCancellationCleanup(
        Task completion,
        Thread thread,
        uint nativeThreadId)
    {
        try
        {
            if (completion.Wait(CancellationGraceMilliseconds))
            {
                return;
            }
        }
        catch
        {
            return;
        }

        if (nativeThreadId != 0)
        {
            _ = PostThreadMessage(nativeThreadId, WmQuit, IntPtr.Zero, IntPtr.Zero);
        }
        try
        {
            _ = thread.Join(CancellationJoinMilliseconds);
        }
        catch
        {
            // Late STA completion is intentionally left detached and observed by
            // the thread's non-faulting completion source.
        }
    }

    private static void RequestDialogClose(uint nativeThreadId)
    {
        if (nativeThreadId == 0)
        {
            return;
        }
        try
        {
            _ = EnumThreadWindows(nativeThreadId, (window, _) =>
            {
                if (!IsWindowVisible(window) || !IsDialogWindow(window))
                {
                    return true;
                }
                PostMessage(window, WmClose, IntPtr.Zero, IntPtr.Zero);
                return true;
            }, IntPtr.Zero);
        }
        catch
        {
            // Cancellation cleanup is best effort and remains bounded.
        }
    }

    private static bool IsDialogWindow(IntPtr window)
    {
        var className = new StringBuilder(64);
        if (GetClassName(window, className, className.Capacity) == 0)
        {
            return false;
        }
        return string.Equals(className.ToString(), "#32770", StringComparison.Ordinal) ||
               string.Equals(className.ToString(), "TaskDialog", StringComparison.Ordinal) ||
               string.Equals(className.ToString(), "TaskDialogHost", StringComparison.Ordinal);
    }

    private static WindowsCommandResult CancelledResult() =>
        new(false, "Windows dialog operation was cancelled.", NativeErrorCode: 995);

    private static WindowsCommandResult FailureResult(Exception error) =>
        new(false, error is EntryPointNotFoundException or DllNotFoundException
                ? "Windows dialog native entry point unavailable."
                : error is System.Runtime.InteropServices.ExternalException
                    ? "Windows dialog native presentation failed."
                    : "Windows dialog STA operation failed.",
            NativeErrorCode: error is System.ComponentModel.Win32Exception native ? native.NativeErrorCode : error.HResult);

    private delegate bool EnumThreadWindowsCallback(IntPtr window, IntPtr parameter);

    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumThreadWindows(
        uint threadId,
        EnumThreadWindowsCallback callback,
        IntPtr parameter);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr window,
        StringBuilder className,
        int maximumCount);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool PostMessage(
        IntPtr window,
        uint message,
        IntPtr wParam,
        IntPtr lParam);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool PostThreadMessage(
        uint threadId,
        uint message,
        IntPtr wParam,
        IntPtr lParam);
}
