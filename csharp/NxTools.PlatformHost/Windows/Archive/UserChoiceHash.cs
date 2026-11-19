using System.Security.Cryptography;
using System.Text;

namespace NxTools.PlatformHost.Windows.Archive;

/// <summary>
/// Explorer's per-user default-program proof stored beside
/// <c>FileExts\.ext\UserChoice\ProgId</c>. Explorer recomputes it from the extension, the user
/// SID, the ProgID, the choice key's last-write minute and the Shell experience string; a
/// mismatching value makes Explorer ignore the choice. Pure: no Registry or file access.
/// </summary>
internal static class UserChoiceHash
{
    internal const string ExperiencePrefix = "User Choice set via Windows User Experience";
    internal const string DefaultExperience = ExperiencePrefix + " {D18B6DD5-6124-4341-9318-804003BAFA0B}";

    internal static string Compute(string extension, string userSid, string progId, DateTime lastWriteUtc, string experience)
    {
        var utc = lastWriteUtc.Kind == DateTimeKind.Utc ? lastWriteUtc : lastWriteUtc.ToUniversalTime();
        var minute = new DateTime(utc.Year, utc.Month, utc.Day, utc.Hour, utc.Minute, 0, DateTimeKind.Utc);
        var fileTime = minute.ToFileTimeUtc();
        var stamp = ((uint)(fileTime >> 32)).ToString("x8") + ((uint)fileTime).ToString("x8");
        return Hash((extension + userSid + progId + stamp + experience).ToLowerInvariant());
    }

    /// <summary>Finds the experience string inside shell32.dll's UTF-16 resources.</summary>
    internal static string? FindExperience(ReadOnlySpan<byte> shell32)
    {
        var text = Encoding.Unicode.GetString(shell32);
        var start = text.IndexOf(ExperiencePrefix, StringComparison.Ordinal);
        if (start < 0) return null;
        var end = text.IndexOf('}', start);
        return end < 0 || end - start > 128 ? null : text[start..(end + 1)];
    }

    internal static string Hash(string baseInfo)
    {
        var data = new byte[Encoding.Unicode.GetByteCount(baseInfo) + 2];
        Encoding.Unicode.GetBytes(baseInfo, data);
        var md5 = MD5.HashData(data);
        var length = ((data.Length & 4) == 0 ? 1 : 0) + (data.Length >> 2) - 1;
        if (length <= 1) return "";
        var rounds = ((length - 2) >> 1) + 1;
        uint Read(int offset) => BitConverter.ToUInt32(data, offset);
        var md0 = BitConverter.ToUInt32(md5, 0);
        var md1 = BitConverter.ToUInt32(md5, 4);
        var output = new byte[16];

        unchecked
        {
            uint cache = 0, out1 = 0, out2 = 0;
            uint m1 = (md0 | 1) + 0x69FB0000u, m2 = (md1 | 1) + 0x13DB0000u;
            for (int round = 0, p = 0; round < rounds; round++, p += 8)
            {
                var r0 = Read(p) + out1;
                var r1 = Read(p + 4);
                var r2a = r0 * m1 - 0x10FA9605u * (r0 >> 16);
                var r2b = 0x79F8A395u * r2a + 0x689B6B9Fu * (r2a >> 16);
                var r3 = 0xEA970001u * r2b - 0x3C101569u * (r2b >> 16);
                var r4 = r3 + r1;
                var r5 = cache + r3;
                var r6a = r4 * m2 - 0x3CE8EC25u * (r4 >> 16);
                var r6b = 0x59C3AF2Du * r6a - 0x2232E0F1u * (r6a >> 16);
                out1 = 0x1EC90001u * r6b + 0x35BD1EC9u * (r6b >> 16);
                out2 = r5 + out1;
                cache = out2;
            }
            BitConverter.TryWriteBytes(output.AsSpan(0, 4), out1);
            BitConverter.TryWriteBytes(output.AsSpan(4, 4), out2);

            cache = 0; out1 = 0; out2 = 0;
            m1 = md0 | 1; m2 = md1 | 1;
            for (int round = 0, p = 0; round < rounds; round++, p += 8)
            {
                var r0 = Read(p) + out1;
                var r1a = r0 * m1;
                var r1b = 0xB1110000u * r1a - 0x30674EEFu * (r1a >> 16);
                var r2a = 0x5B9F0000u * r1b - 0x78F7A461u * (r1b >> 16);
                var r2b = 0x12CEB96Du * (r2a >> 16) - 0x46930000u * r2a;
                var r3 = 0x1D830000u * r2b + 0x257E1D83u * (r2b >> 16);
                var r4a = m2 * (r3 + Read(p + 4));
                var r4b = 0x16F50000u * r4a - 0x5D8BE90Bu * (r4a >> 16);
                var r5a = 0x96FF0000u * r4b - 0x2C7C6901u * (r4b >> 16);
                var r5b = 0x2B890000u * r5a + 0x7C932B89u * (r5a >> 16);
                out1 = 0x9F690000u * r5b - 0x405B6097u * (r5b >> 16);
                out2 = out1 + cache + r3;
                cache = out2;
            }
            BitConverter.TryWriteBytes(output.AsSpan(8, 4), out1);
            BitConverter.TryWriteBytes(output.AsSpan(12, 4), out2);
        }

        var proof = new byte[8];
        BitConverter.TryWriteBytes(proof.AsSpan(0, 4), BitConverter.ToUInt32(output, 8) ^ BitConverter.ToUInt32(output, 0));
        BitConverter.TryWriteBytes(proof.AsSpan(4, 4), BitConverter.ToUInt32(output, 12) ^ BitConverter.ToUInt32(output, 4));
        return Convert.ToBase64String(proof);
    }
}
