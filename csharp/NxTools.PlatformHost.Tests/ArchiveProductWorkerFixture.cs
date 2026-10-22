using System.Buffers.Binary;
using System.Text.Json;

namespace NxTools.PlatformHost.Tests;

// Only a byte-identical copy of the current test apphost can select this negative fixture.
internal static class ArchiveProductWorkerFixture
{
    internal static async Task<int> RunAsync()
    {
        var input = Console.OpenStandardInput(); var output = Console.OpenStandardOutput();
        var header = new byte[4]; await input.ReadExactlyAsync(header);
        var length = BinaryPrimitives.ReadUInt32LittleEndian(header);
        if (length is 0 or > 1048576) return 2;
        var body = new byte[length]; await input.ReadExactlyAsync(body);
        using var request = JsonDocument.Parse(body); var root = request.RootElement;
        if (root.GetProperty("schema").GetInt32() != 2 || root.GetProperty("read_contract").GetInt32() is not (1 or 2)) return 2;
        var contract = root.GetProperty("read_contract").GetInt32();
        var id = root.GetProperty("operation_id").GetGuid();
        var mode = Path.GetFileNameWithoutExtension(root.GetProperty("path").GetString());
        if (mode=="legacy-strict" && (contract!=1 || root.TryGetProperty("read_transport",out _))) return 2;
        var caps = new List<string> { "source_lease_v1", "budget_reservations_v1" };
        caps.AddRange(NxTools.PlatformHost.Protocol.ArchiveReadWire.Capabilities((uint)contract));
        if (mode == "missing-capability") caps.Remove("metadata_credit_v1");
        if (mode == "crash") return 17;
        object accepted = mode == "old-version"
            ? new { schema = 1, operation_id = id, op = "accepted", capabilities = caps }
            : new { schema = 1, operation_id = id, op = "accepted", read_contract = mode == "wrong-version" ? (contract == 1 ? 2 : 1) : contract, capabilities = caps };
        await Send(output, accepted);
        if (mode is "old-version" or "wrong-version" or "missing-capability") { await Task.Delay(3000); return 0; }
        // A hanging negotiated worker is a negative cancellation/EOF fixture, never a success oracle.
        while (true) {
            var count = await input.ReadAsync(header); if (count == 0) return 0;
            if (count < 4) await input.ReadExactlyAsync(header.AsMemory(count));
            length = BinaryPrimitives.ReadUInt32LittleEndian(header); if (length is 0 or > 1048576) return 2;
            body = new byte[length]; await input.ReadExactlyAsync(body);
            using var control = JsonDocument.Parse(body);
            if (control.RootElement.GetProperty("op").GetString() == "cancel") {
                if (mode == "timeout-cancel") await Send(output, new { schema = 1, operation_id = id, op = "failure", error_code = "Cancelled" });
                return 0;
            }
        }
    }
    private static async Task Send(Stream output, object frame) {
        var body = JsonSerializer.SerializeToUtf8Bytes(frame); await output.WriteAsync(BitConverter.GetBytes(body.Length));
        await output.WriteAsync(body); await output.FlushAsync();
    }
}
