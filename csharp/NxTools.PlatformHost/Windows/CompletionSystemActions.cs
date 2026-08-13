using System.ComponentModel;
using System.Diagnostics;
using System.Media;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal interface ICompletionSystemActions
{
    WindowsCommandResult PlayNotificationSound();

    WindowsCommandResult RequestShutdown(ushort delaySeconds);
}

internal sealed class WindowsCompletionSystemActions : ICompletionSystemActions
{
    private const ushort MaximumShutdownDelaySeconds = 60;

    public WindowsCommandResult PlayNotificationSound()
    {
        try
        {
            SystemSounds.Asterisk.Play();
            return new WindowsCommandResult(true);
        }
        catch (Exception error) when (error is InvalidOperationException or ExternalException)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }

    public WindowsCommandResult RequestShutdown(ushort delaySeconds)
    {
        if (delaySeconds > MaximumShutdownDelaySeconds)
        {
            return new WindowsCommandResult(false, "Shutdown delay exceeds the bounded policy.");
        }
        try
        {
            using var process = Process.Start(new ProcessStartInfo
            {
                FileName = Path.Combine(Environment.SystemDirectory, "shutdown.exe"),
                UseShellExecute = false,
                CreateNoWindow = true,
                ArgumentList =
                {
                    "/s",
                    "/t",
                    delaySeconds.ToString(System.Globalization.CultureInfo.InvariantCulture),
                    "/d",
                    "p:4:1"
                }
            });
            if (process is null)
            {
                return new WindowsCommandResult(false, "Windows did not start the shutdown adapter.");
            }
            if (!process.WaitForExit(5_000))
            {
                try
                {
                    process.Kill(entireProcessTree: true);
                    process.WaitForExit(2_000);
                }
                catch
                {
                    // The original bounded failure remains authoritative.
                }
                return new WindowsCommandResult(false, "Windows shutdown scheduling timed out.");
            }
            return process.ExitCode == 0
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    "Windows rejected the shutdown request.",
                    NativeErrorCode: process.ExitCode);
        }
        catch (Win32Exception error)
        {
            return new WindowsCommandResult(false, error.Message, NativeErrorCode: error.NativeErrorCode);
        }
        catch (Exception error) when (error is InvalidOperationException or ArgumentException)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }
}
