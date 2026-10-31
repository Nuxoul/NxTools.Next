using System.Buffers;
using System.Text;

namespace NxTools.PlatformHost.Protocol;

internal sealed class BoundedFrameReader(Stream stream)
{
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);
    private readonly byte[] _readBuffer = new byte[8192];
    private int _offset;
    private int _count;

    public async Task<string?> ReadAsync(CancellationToken cancellationToken)
    {
        var frame = new ArrayBufferWriter<byte>(4096);

        while (true)
        {
            if (_offset == _count)
            {
                _count = await stream.ReadAsync(_readBuffer, cancellationToken);
                _offset = 0;
                if (_count == 0)
                {
                    if (frame.WrittenCount == 0)
                    {
                        return null;
                    }
                    throw new EndOfStreamException("Desktop closed the pipe mid-frame.");
                }
            }

            var newlineIndex = Array.IndexOf(_readBuffer, (byte)'\n', _offset, _count - _offset);
            var payloadLength = newlineIndex >= 0 ? newlineIndex - _offset : _count - _offset;
            if (frame.WrittenCount + payloadLength > ProtocolConstants.MaxFrameBytes)
            {
                throw new InvalidDataException(
                    $"Desktop sent a frame larger than {ProtocolConstants.MaxFrameBytes} bytes.");
            }

            frame.Write(new ReadOnlySpan<byte>(_readBuffer, _offset, payloadLength));
            _offset += payloadLength;
            if (newlineIndex < 0)
            {
                continue;
            }

            _offset++;
            var bytes = frame.WrittenMemory.ToArray();
            var length = bytes.Length;
            if (length > 0 && bytes[length - 1] == (byte)'\r')
            {
                length--;
            }
            return StrictUtf8.GetString(bytes, 0, length);
        }
    }
}
