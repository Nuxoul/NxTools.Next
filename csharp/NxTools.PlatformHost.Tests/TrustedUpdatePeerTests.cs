using System.IO.Pipes;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class TrustedUpdatePeerTests
{
    internal static async Task OwnedPipeBindsRunningOwnServiceWithoutProcessAccess()
    {
        var name = "NxTools.PeerFixture." + Guid.NewGuid().ToString("N");
        using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
        using var client = new NamedPipeClientStream(".", name, PipeDirection.InOut, PipeOptions.Asynchronous);
        using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        var connected = server.WaitForConnectionAsync(deadline.Token);
        await client.ConnectAsync(deadline.Token);
        await connected;
        var reads = 0;
        TrustedUpdatePeer.Verify(client.SafePipeHandle, () =>
        {
            reads++;
            return new((uint)Environment.ProcessId, 4, 0x10);
        });
        Require(reads == 1, "actual pipe PID must be checked against read-only SCM, not process image access");
        foreach (var status in new[]
        {
            new TrustedUpdatePeer.ServiceIdentity((uint)Environment.ProcessId + 1, 4, 0x10),
            new((uint)Environment.ProcessId, 1, 0x10),
            new((uint)Environment.ProcessId, 4, 0x20),
            new(0, 4, 0x10),
        })
        {
            try
            {
                TrustedUpdatePeer.Verify(client.SafePipeHandle, () => status);
                throw new InvalidOperationException("unapproved peer accepted");
            }
            catch (InvalidDataException) { }
        }
        using var closed = new Microsoft.Win32.SafeHandles.SafePipeHandle(IntPtr.Zero, true);
        try
        {
            TrustedUpdatePeer.Verify(closed, () => throw new InvalidOperationException("SCM queried for invalid pipe"));
            throw new InvalidOperationException("invalid pipe accepted");
        }
        catch (System.ComponentModel.Win32Exception) { }
    }

    private static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
