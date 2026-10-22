using System.Buffers.Binary;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Extensions;

namespace NxTools.PlatformHost.Tests;

// Selected only through --archive-worker-fixture in the current test apphost.
internal static class ArchiveWorkerFixture
{
    internal static async Task<int> RunAsync()
    {
        var input = Console.OpenStandardInput();
        var output = Console.OpenStandardOutput();
        var header = new byte[4];
        await input.ReadExactlyAsync(header);
        var length = BinaryPrimitives.ReadUInt32LittleEndian(header);
        if (length is 0 or > 1048576) return 2;
        var body = new byte[length];
        await input.ReadExactlyAsync(body);
        using var request = JsonDocument.Parse(body);
        var root = request.RootElement;
        if (root.GetProperty("schema").GetInt32() != 1 || root.GetProperty("op").GetString() != "start") return 2;
        var id = root.GetProperty("operation_id").GetGuid();
        var operation = root.GetProperty("operation").GetString();
        var path = root.GetProperty("path").GetString()!;
        var mode = Path.GetFileName(path);
        if (mode == "marker") File.WriteAllText(path, "executed");
        if (mode == "memorylimit")
        {
            var limit = ExtensionNative.CurrentJobLimits().JobMemory.ToUInt64();
            if (limit > (192UL << 20)) return 2; // Fixture never attempts an unbounded allocation.
            var allocation = VirtualAlloc(IntPtr.Zero, (UIntPtr)(limit + (4UL << 20)), 0x3000, 4);
            var nativeCode = System.Runtime.InteropServices.Marshal.GetLastWin32Error();
            if (allocation != IntPtr.Zero) { VirtualFree(allocation, UIntPtr.Zero, 0x8000); return 3; }
            var failure = JsonSerializer.SerializeToUtf8Bytes(new { schema = 1, operation_id = id, op = "failure",
                error_code = "ResourceLimit", native_error_code = nativeCode });
            await output.WriteAsync(BitConverter.GetBytes(failure.Length)); await output.WriteAsync(failure); await output.FlushAsync();
            return 0;
        }
        if (mode == "job")
        {
            var limits = ExtensionNative.CurrentJobLimits();
            await Send(output, id, "accepted", new { flags = limits.Basic.Flags, memory = limits.JobMemory.ToUInt64(), processes = limits.Basic.ActiveProcesses });
        }
        else if (mode == "badlength")
        {
            BinaryPrimitives.WriteUInt32LittleEndian(header, 1048577);
            await output.WriteAsync(header); return 0;
        }
        else if (mode == "badjson")
        {
            await output.WriteAsync(new byte[] { 1, 0, 0, 0, 255 }); return 0;
        }
        else if (mode == "truncated")
        {
            BinaryPrimitives.WriteUInt32LittleEndian(header, 20);
            await output.WriteAsync(header); await output.WriteAsync(new byte[] { 1, 2 }); return 0;
        }
        else if (mode == "badutf8")
        {
            BinaryPrimitives.WriteUInt32LittleEndian(header, 2);
            await output.WriteAsync(header); await output.WriteAsync(new byte[] { 0xc3, 0x28 }); return 0;
        }
        else if (mode == "badid") await Send(output, Guid.NewGuid(), "accepted");
        else if (mode == "stderr")
        {
            Console.Error.Write(new string('S', 20000)); Console.Error.Flush();
            await Send(output, id, "accepted");
        }
        else if (mode == "missing") { await Send(output, id, "accepted"); return 0; }
        else if (mode == "failure") { await Send(output, id, "failure"); return 0; }
        else await Send(output, id, "accepted", new { operation });
        if (mode is "sleep" or "marker")
        {
            while (true)
            {
                await input.ReadExactlyAsync(header);
                var size = BinaryPrimitives.ReadUInt32LittleEndian(header);
                if (size is 0 or > 1048576) return 2;
                body = new byte[size]; await input.ReadExactlyAsync(body);
                using var control = JsonDocument.Parse(body);
                if (control.RootElement.GetProperty("op").GetString() == "cancel")
                {
                    // Intentional uncooperative worker proves owned Job termination after grace.
                    await Task.Delay(Timeout.Infinite);
                }
                if (mode == "sleep" && control.RootElement.GetProperty("op").GetString() == "write")
                    await Send(output, id, "progress");
            }
        }
        await Send(output, id, "complete");
        return 0;
    }
    [System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
    private static extern IntPtr VirtualAlloc(IntPtr address, UIntPtr size, uint allocation, uint protect);
    [System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
    private static extern bool VirtualFree(IntPtr address, UIntPtr size, uint kind);
    private static async Task Send(Stream output, Guid id, string op, object? data = null)
    {
        var body = JsonSerializer.SerializeToUtf8Bytes(new { schema = 1, operation_id = id, op, data });
        var header = new byte[4]; BinaryPrimitives.WriteUInt32LittleEndian(header, (uint)body.Length);
        await output.WriteAsync(header); await output.WriteAsync(body); await output.FlushAsync();
    }
}
