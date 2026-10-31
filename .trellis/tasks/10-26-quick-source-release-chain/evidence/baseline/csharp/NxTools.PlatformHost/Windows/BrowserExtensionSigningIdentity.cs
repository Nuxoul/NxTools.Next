using System.Security.Cryptography;

namespace NxTools.PlatformHost.Windows;

// Signing identity only. Export owns creation; registration calls the read-only path.
internal static class BrowserExtensionSigningIdentity
{
    internal static string ProductionKeyPath => KeyPathUnder(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData));

    internal static string KeyPathUnder(string localApplicationData) =>
        Path.Combine(localApplicationData, "NxTools", "browser_extension_key.pem");

    internal static byte[] CrxId(ReadOnlySpan<byte> subjectPublicKeyInfo) =>
        SHA256.HashData(subjectPublicKeyInfo).AsSpan(0, 16).ToArray();

    internal static string ChromiumId(ReadOnlySpan<byte> subjectPublicKeyInfo)
    {
        var id = CrxId(subjectPublicKeyInfo);
        var characters = new char[id.Length * 2];
        for (var index = 0; index < id.Length; index++)
        {
            characters[index * 2] = (char)('a' + (id[index] >> 4));
            characters[index * 2 + 1] = (char)('a' + (id[index] & 15));
        }
        return new string(characters);
    }

    internal static string? ReadExistingChromiumId(string keyPath)
    {
        if (!File.Exists(keyPath)) return null;
        using var rsa = RSA.Create();
        rsa.ImportFromPem(File.ReadAllText(keyPath));
        return ChromiumId(rsa.ExportSubjectPublicKeyInfo());
    }
}
