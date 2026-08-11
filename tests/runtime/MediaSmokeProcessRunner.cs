using System;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace NxTools.MediaSmoke;

public sealed record BoundedProcessResult(
    int ExitCode,
    bool TimedOut,
    string StandardOutput,
    string StandardError);

public static class BoundedProcessRunner
{
    public static BoundedProcessResult Run(
        string executable,
        string[] arguments,
        int timeoutMilliseconds,
        int maximumCapturedCharacters)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(executable);
        ArgumentNullException.ThrowIfNull(arguments);
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(timeoutMilliseconds);
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(maximumCapturedCharacters);

        using var process = new Process
        {
            StartInfo = new ProcessStartInfo
            {
                FileName = executable,
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardInput = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                StandardOutputEncoding = Encoding.UTF8,
                StandardErrorEncoding = Encoding.UTF8
            }
        };
        foreach (var argument in arguments)
        {
            process.StartInfo.ArgumentList.Add(argument);
        }
        var started = false;
        try
        {
            if (!process.Start())
            {
                throw new InvalidOperationException("The bounded child process did not start.");
            }
            started = true;
            process.StandardInput.Close();

            var outputTask = DrainBoundedAsync(process.StandardOutput, maximumCapturedCharacters);
            var errorTask = DrainBoundedAsync(process.StandardError, maximumCapturedCharacters);
            var timedOut = false;
            using var timeout = new CancellationTokenSource(timeoutMilliseconds);
            try
            {
                process.WaitForExitAsync(timeout.Token).GetAwaiter().GetResult();
            }
            catch (OperationCanceledException) when (timeout.IsCancellationRequested)
            {
                timedOut = true;
                KillAndWait(process);
            }

            Task.WhenAll(outputTask, errorTask)
                .WaitAsync(TimeSpan.FromSeconds(5))
                .GetAwaiter()
                .GetResult();
            return new BoundedProcessResult(
                process.ExitCode,
                timedOut,
                outputTask.Result,
                errorTask.Result);
        }
        finally
        {
            if (started && !process.HasExited)
            {
                KillAndWait(process);
            }
        }
    }

    private static async Task<string> DrainBoundedAsync(
        StreamReader reader,
        int maximumCapturedCharacters)
    {
        var captured = new StringBuilder(Math.Min(maximumCapturedCharacters, 4096));
        var buffer = new char[4096];
        while (true)
        {
            var read = await reader.ReadAsync(buffer.AsMemory()).ConfigureAwait(false);
            if (read == 0)
            {
                return captured.ToString();
            }
            var remaining = maximumCapturedCharacters - captured.Length;
            if (remaining > 0)
            {
                captured.Append(buffer, 0, Math.Min(read, remaining));
            }
        }
    }

    private static void KillAndWait(Process process)
    {
        try
        {
            if (!process.HasExited)
            {
                process.Kill(entireProcessTree: true);
            }
        }
        catch (InvalidOperationException) when (process.HasExited)
        {
        }
        using var cleanupTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        process.WaitForExitAsync(cleanupTimeout.Token).GetAwaiter().GetResult();
    }
}
