using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json.Nodes;

namespace NxTools.PlatformHost.Windows;

/// <summary>
/// Packages the browser extension that ships beside the host executable into
/// installable artifacts: a signed CRX3 for Chrome/Edge and an XPI for
/// Firefox. Ported from the original NxTools packaging service so the
/// exported bytes stay compatible with the same signing key.
/// </summary>
public static class BrowserExtensionPackageService
{
    public const string ChromeFormat = "chrome";
    public const string FirefoxFormat = "firefox";

    private static readonly byte[] CrxMagic = "Cr24"u8.ToArray();
    private static readonly byte[] CrxSignatureContext = Encoding.ASCII.GetBytes("CRX3 SignedData\0");

    public static string DefaultExportFileName =>
        BuildTimestampedExportFileName("NxTools_Chrome_Edge_Extension", ".crx");

    public static string DefaultFirefoxExportFileName =>
        BuildTimestampedExportFileName("NxTools_Firefox_Extension", ".xpi");

    public static string ResolveExtensionDirectory() =>
        ResolveExtensionDirectory(AppContext.BaseDirectory, Directory.GetCurrentDirectory());

    internal static string ResolveExtensionDirectory(string root, string currentDirectory)
    {
        var candidates = new[]
        {
            Path.Combine(root, "browser_extension", "nxtools-interceptor"),
            Path.Combine(root, "..", "browser_extension", "nxtools-interceptor"),
            Path.Combine(currentDirectory, "browser_extension", "nxtools-interceptor"),
        };

        return candidates.FirstOrDefault(Directory.Exists) ?? candidates[0];
    }

    public static string Export(
        string format,
        string destinationPath,
        CancellationToken cancellationToken = default) =>
        Export(format, destinationPath, ResolveExtensionDirectory(), cancellationToken);

    internal static string Export(
        string format,
        string destinationPath,
        string sourceDirectory,
        CancellationToken cancellationToken = default) =>
        ExportWithSigningKey(format, destinationPath, sourceDirectory,
            BrowserExtensionSigningIdentity.ProductionKeyPath, cancellationToken);

    internal static string ExportWithSigningKey(
        string format,
        string destinationPath,
        string sourceDirectory,
        string signingKeyPath,
        CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (string.IsNullOrWhiteSpace(destinationPath))
        {
            var desktop = Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
            destinationPath = Path.Combine(
                string.IsNullOrWhiteSpace(desktop) ? "." : desktop,
                string.Equals(format, FirefoxFormat, StringComparison.OrdinalIgnoreCase)
                    ? DefaultFirefoxExportFileName
                    : DefaultExportFileName);
        }

        var extension = Path.GetExtension(destinationPath);
        if (string.Equals(format, FirefoxFormat, StringComparison.OrdinalIgnoreCase) &&
            !extension.Equals(".xpi", StringComparison.OrdinalIgnoreCase))
        {
            destinationPath = Path.ChangeExtension(destinationPath, ".xpi");
        }

        if (!Directory.Exists(sourceDirectory))
        {
            throw new DirectoryNotFoundException($"未找到浏览器扩展目录：{sourceDirectory}");
        }

        var destinationDirectory = Path.GetDirectoryName(destinationPath);
        if (!string.IsNullOrWhiteSpace(destinationDirectory))
        {
            Directory.CreateDirectory(destinationDirectory);
        }

        var tempExtension = string.Equals(format, FirefoxFormat, StringComparison.OrdinalIgnoreCase)
            ? ".xpi"
            : ".crx";
        var tempFile = Path.Combine(Path.GetTempPath(), $"nxtools-extension-{Guid.NewGuid():N}{tempExtension}");

        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (string.Equals(format, FirefoxFormat, StringComparison.OrdinalIgnoreCase))
            {
                var manifest = CreateFirefoxManifest(sourceDirectory);
                CreateZipFile(sourceDirectory, tempFile, manifest);
            }
            else
            {
                var zipBytes = CreateZipBytesForCrx(sourceDirectory);
                cancellationToken.ThrowIfCancellationRequested();
                var crxBytes = CreateCrx3Bytes(zipBytes, signingKeyPath);
                cancellationToken.ThrowIfCancellationRequested();
                File.WriteAllBytes(tempFile, crxBytes);
            }

            cancellationToken.ThrowIfCancellationRequested();
            return MovePackageToDestination(tempFile, destinationPath);
        }
        finally
        {
            TryDeleteFile(tempFile);
        }
    }

    private static string BuildTimestampedExportFileName(string prefix, string extension) =>
        $"{prefix}_{DateTime.Now:yyyyMMdd_HHmmss}{extension}";

    private static string MovePackageToDestination(string tempFile, string destinationPath)
    {
        try
        {
            ReplacePackageFile(tempFile, destinationPath);
            return destinationPath;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            var fallbackPath = BuildNumberedPackagePath(destinationPath);
            File.Move(tempFile, fallbackPath);
            return fallbackPath;
        }
    }

    private static void ReplacePackageFile(string tempFile, string destinationPath)
    {
        if (File.Exists(destinationPath))
        {
            try
            {
                File.SetAttributes(destinationPath, FileAttributes.Normal);
            }
            catch (IOException)
            {
                // The next delete/move call will report the meaningful failure.
            }
            catch (UnauthorizedAccessException)
            {
                // The next delete/move call will report the meaningful failure.
            }

            File.Delete(destinationPath);
        }

        File.Move(tempFile, destinationPath);
    }

    private static string BuildNumberedPackagePath(string destinationPath)
    {
        var directory = Path.GetDirectoryName(destinationPath);
        var fileName = Path.GetFileNameWithoutExtension(destinationPath);
        var extension = Path.GetExtension(destinationPath);
        if (string.IsNullOrWhiteSpace(directory))
        {
            directory = Directory.GetCurrentDirectory();
        }

        for (var index = 1; index < 1000; index++)
        {
            var candidate = Path.Combine(directory, $"{fileName} ({index}){extension}");
            if (!File.Exists(candidate))
            {
                return candidate;
            }
        }

        return Path.Combine(directory, $"{fileName} ({DateTimeOffset.Now:yyyyMMddHHmmss}){extension}");
    }

    private static byte[] CreateZipBytesForCrx(string sourceDirectory)
    {
        var tempZip = Path.Combine(Path.GetTempPath(), $"nxtools-extension-{Guid.NewGuid():N}.zip");
        try
        {
            CreateZipFile(sourceDirectory, tempZip, manifestOverride: null);
            return File.ReadAllBytes(tempZip);
        }
        finally
        {
            TryDeleteFile(tempZip);
        }
    }

    private static void CreateZipFile(
        string sourceDirectory,
        string destinationPath,
        string? manifestOverride)
    {
        using var packageStream = new FileStream(
            destinationPath,
            FileMode.Create,
            FileAccess.ReadWrite,
            FileShare.None,
            bufferSize: 64 * 1024);
        using var archive = new ZipArchive(packageStream, ZipArchiveMode.Create, leaveOpen: false);
        foreach (var file in Directory
                     .EnumerateFiles(sourceDirectory, "*", SearchOption.AllDirectories)
                     .Order(StringComparer.OrdinalIgnoreCase))
        {
            var relativePath = Path.GetRelativePath(sourceDirectory, file).Replace('\\', '/');
            var entry = archive.CreateEntry(relativePath, CompressionLevel.Fastest);

            if (string.Equals(relativePath, "manifest.json", StringComparison.OrdinalIgnoreCase) &&
                !string.IsNullOrWhiteSpace(manifestOverride))
            {
                using var manifestTarget = entry.Open();
                manifestTarget.Write(Encoding.UTF8.GetBytes(manifestOverride));
                continue;
            }

            using var source = new FileStream(
                file,
                FileMode.Open,
                FileAccess.Read,
                FileShare.Read,
                bufferSize: 64 * 1024);
            using var target = entry.Open();
            source.CopyTo(target);
        }
    }

    private static string CreateFirefoxManifest(string sourceDirectory)
    {
        var manifestPath = Path.Combine(sourceDirectory, "manifest.json");
        var manifest = JsonNode.Parse(File.ReadAllText(manifestPath))?.AsObject()
                       ?? throw new InvalidDataException("浏览器扩展 manifest.json 无法解析。");

        if (manifest["permissions"] is JsonArray permissions)
        {
            RemoveArrayValue(permissions, "declarativeNetRequest");
        }

        manifest["background"] = new JsonObject
        {
            ["scripts"] = new JsonArray("background.js")
        };
        manifest["browser_specific_settings"] = new JsonObject
        {
            ["gecko"] = new JsonObject
            {
                ["id"] = "nxtools-download-interceptor@nxtools.local",
                ["strict_min_version"] = "140.0",
                ["data_collection_permissions"] = new JsonObject
                {
                    ["required"] = new JsonArray("none"),
                    ["optional"] = new JsonArray()
                }
            }
        };

        return manifest.ToJsonString(new()
        {
            WriteIndented = true
        });
    }

    private static void RemoveArrayValue(JsonArray array, string value)
    {
        for (var index = array.Count - 1; index >= 0; index--)
        {
            if (string.Equals(array[index]?.GetValue<string>(), value, StringComparison.OrdinalIgnoreCase))
            {
                array.RemoveAt(index);
            }
        }
    }

    private static byte[] CreateCrx3Bytes(byte[] zipBytes, string signingKeyPath)
    {
        using var rsa = LoadOrCreateSigningKey(signingKeyPath);
        var publicKey = rsa.ExportSubjectPublicKeyInfo();
        var crxId = BrowserExtensionSigningIdentity.CrxId(publicKey);
        var signedHeaderData = EncodeMessage([(1, crxId)]);
        var signatureInput = BuildCrx3SignatureInput(signedHeaderData, zipBytes);
        var signature = rsa.SignData(signatureInput, HashAlgorithmName.SHA256, RSASignaturePadding.Pkcs1);
        var proof = EncodeMessage([(1, publicKey), (2, signature)]);
        var header = EncodeMessage([(2, proof), (10000, signedHeaderData)]);

        using var stream = new MemoryStream();
        stream.Write(CrxMagic);
        WriteUInt32LittleEndian(stream, 3);
        WriteUInt32LittleEndian(stream, (uint)header.Length);
        stream.Write(header);
        stream.Write(zipBytes);
        return stream.ToArray();
    }

    private static byte[] BuildCrx3SignatureInput(byte[] signedHeaderData, byte[] zipBytes)
    {
        using var stream = new MemoryStream();
        stream.Write(CrxSignatureContext);
        WriteUInt32LittleEndian(stream, (uint)signedHeaderData.Length);
        stream.Write(signedHeaderData);
        stream.Write(zipBytes);
        return stream.ToArray();
    }

    private static RSA LoadOrCreateSigningKey(string keyPath)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(keyPath)!);

        var rsa = RSA.Create(2048);
        if (File.Exists(keyPath))
        {
            rsa.ImportFromPem(File.ReadAllText(keyPath));
            return rsa;
        }

        File.WriteAllText(keyPath, rsa.ExportPkcs8PrivateKeyPem());
        return rsa;
    }


    private static byte[] EncodeMessage(IEnumerable<(int FieldNumber, byte[] Value)> fields)
    {
        using var stream = new MemoryStream();
        foreach (var (fieldNumber, value) in fields)
        {
            WriteVarint(stream, (ulong)((fieldNumber << 3) | 2));
            WriteVarint(stream, (ulong)value.Length);
            stream.Write(value);
        }

        return stream.ToArray();
    }

    private static void WriteVarint(Stream stream, ulong value)
    {
        while (value >= 0x80)
        {
            stream.WriteByte((byte)(value | 0x80));
            value >>= 7;
        }

        stream.WriteByte((byte)value);
    }

    private static void WriteUInt32LittleEndian(Stream stream, uint value)
    {
        stream.WriteByte((byte)(value & 0xff));
        stream.WriteByte((byte)((value >> 8) & 0xff));
        stream.WriteByte((byte)((value >> 16) & 0xff));
        stream.WriteByte((byte)((value >> 24) & 0xff));
    }

    private static void TryDeleteFile(string path)
    {
        try
        {
            if (File.Exists(path))
            {
                File.Delete(path);
            }
        }
        catch (IOException)
        {
        }
        catch (UnauthorizedAccessException)
        {
        }
    }
}
