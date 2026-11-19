using System.Diagnostics;
using System.IO.Pipes;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal interface IElevatedInputClientPlatform
{
    bool VerifyHelper(NamedPipeServerStream pipe, int processId);
    bool RequiresElevatedInput(long window);
    Task WaitForExitAsync(int processId, CancellationToken cancellationToken);
}

internal sealed class ElevatedInputClientPlatform : IElevatedInputClientPlatform
{
    public bool VerifyHelper(NamedPipeServerStream pipe, int processId) =>
        ElevatedDirectoryHelperPeerVerifier.IsPackagedHelper(pipe, processId);

    internal static uint WindowProcessId(long window)
    {
        _ = GetWindowThreadProcessId(new IntPtr(window), out var processId);
        return processId;
    }

    internal static long ForegroundWindow() => GetForegroundWindow().ToInt64();

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();
    public bool RequiresElevatedInput(long window)
    {
        _ = GetWindowThreadProcessId(new IntPtr(window), out var processId);
        return processId != 0 && ElevatedDirectoryHelperPeerVerifier.IsProcessElevated(processId);
    }

    public async Task WaitForExitAsync(int processId, CancellationToken cancellationToken)
    {
        using var helper = Process.GetProcessById(processId);
        await helper.WaitForExitAsync(cancellationToken);
    }

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
}
