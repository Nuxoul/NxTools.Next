using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text.Json;

namespace NxTools.PlatformHost.Windows.Archive;

internal readonly record struct ArchiveBulkBinding(byte Kind, Guid OperationId, Guid CallId, Guid Token,
    ulong Sequence, ulong Offset, uint Length)
{
    internal JsonElement NativeCall(string action) => JsonSerializer.SerializeToElement(new {
        call_id = CallId, action, token = Token, sequence = Sequence, offset = Offset, length = Length });
    internal bool Valid => Kind is >= 1 and <= 3 && OperationId != Guid.Empty && CallId != Guid.Empty &&
        Token != Guid.Empty && Sequence > 0 && Length is > 0 and <= ArchiveBulkFrame.MaxPayload;
}

// A successful native read transfers ownership to the original input pump, never a business event.
internal sealed class ArchiveBulkReply(ArchiveBulkBinding binding, byte[] payload) : IDisposable
{
    internal ArchiveBulkBinding Binding { get; } = binding;
    internal byte[] Payload { get; } = payload;
    public void Dispose() => CryptographicOperations.ZeroMemory(Payload);
}

internal static class ArchiveBulkFrame
{
    internal const int HeaderLength = 76;
    internal const int MaxPayload = 65536;
    internal const uint Discriminator = 0x80000000;
    internal static bool ValidBodyLength(uint size) => size is >= HeaderLength and <= HeaderLength + MaxPayload;
    internal static ArchiveBulkBinding HeaderBinding(ReadOnlySpan<byte> body) => new(body[1],
        new Guid(body.Slice(4, 16), bigEndian: true), new Guid(body.Slice(20, 16), bigEndian: true),
        new Guid(body.Slice(36, 16), bigEndian: true), BinaryPrimitives.ReadUInt64LittleEndian(body[52..]),
        BinaryPrimitives.ReadUInt64LittleEndian(body[60..]), BinaryPrimitives.ReadUInt32LittleEndian(body[68..]));
    internal static ArchiveBulkBinding Parse(ReadOnlySpan<byte> body, byte kind)
    {
        if (!ValidBodyLength((uint)body.Length) || body[0] != 1 || body[1] != kind || body[2] != 0 || body[3] != 0)
            throw new InvalidDataException("Invalid binary archive header.");
        var binding = HeaderBinding(body);
        var actual = BinaryPrimitives.ReadUInt32LittleEndian(body[72..]);
        if (!binding.Valid || actual > binding.Length || actual != body.Length - HeaderLength || kind == 1 && actual != binding.Length)
            throw new InvalidDataException("Invalid binary archive binding.");
        return binding;
    }
    internal static ArchiveBulkBinding ReadRequest(Guid operationId, JsonElement call, byte kind)
    {
        var binding = new ArchiveBulkBinding(kind, operationId, call.GetProperty("call_id").GetGuid(),
            call.GetProperty("token").GetGuid(), call.GetProperty("sequence").GetUInt64(),
            call.GetProperty("offset").GetUInt64(), call.GetProperty("length").GetUInt32());
        if (kind is not (2 or 3)) throw new InvalidDataException("Invalid binary archive request.");
        return binding;
    }
    internal static byte[] Pack(ArchiveBulkReply reply)
    {
        var binding = reply.Binding; var payload = reply.Payload;
        if (!binding.Valid || binding.Kind is not (2 or 3) || payload.Length > binding.Length)
            throw new InvalidDataException("Invalid binary archive reply.");
        var result = new byte[4 + HeaderLength + payload.Length]; var body = result.AsSpan(4);
        BinaryPrimitives.WriteUInt32LittleEndian(result, Discriminator | (uint)(HeaderLength + payload.Length));
        body[0] = 1; body[1] = binding.Kind;
        binding.OperationId.TryWriteBytes(body[4..20], bigEndian: true, out _);
        binding.CallId.TryWriteBytes(body[20..36], bigEndian: true, out _);
        binding.Token.TryWriteBytes(body[36..52], bigEndian: true, out _);
        BinaryPrimitives.WriteUInt64LittleEndian(body[52..], binding.Sequence);
        BinaryPrimitives.WriteUInt64LittleEndian(body[60..], binding.Offset);
        BinaryPrimitives.WriteUInt32LittleEndian(body[68..], binding.Length);
        BinaryPrimitives.WriteUInt32LittleEndian(body[72..], (uint)payload.Length);
        payload.CopyTo(body[HeaderLength..]); return result;
    }
}
