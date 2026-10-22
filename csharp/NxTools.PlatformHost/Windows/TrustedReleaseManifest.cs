using System.Collections.ObjectModel;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows;

public sealed record ReleaseFile(
    [property: JsonPropertyName("path"), JsonRequired] string Path,
    [property: JsonPropertyName("sha256"), JsonRequired] string Sha256,
    [property: JsonPropertyName("size"), JsonRequired] long Size);

public sealed record ReleaseManifest(
    [property: JsonPropertyName("schema_version"), JsonRequired] int SchemaVersion,
    [property: JsonPropertyName("product"), JsonRequired] string Product,
    [property: JsonPropertyName("release_sequence"), JsonRequired] long ReleaseSequence,
    [property: JsonPropertyName("release_id"), JsonRequired] Guid ReleaseId,
    [property: JsonPropertyName("public_key_sha256"), JsonRequired] string PublicKeySha256,
    [property: JsonPropertyName("files"), JsonRequired] IReadOnlyList<ReleaseFile> Files);

/// <summary>Independent release authentication; caller supplies the protected trust anchor and sequence.</summary>
public static class TrustedReleaseManifest
{
    public const string ManifestName = "NxTools.release.json";
    public const string SignatureName = "NxTools.release.sig";
    public const int MaximumManifestBytes = 1024 * 1024;
    public const int MaximumFiles = 4096;
    public const long MaximumTotalBytes = 2L * 1024 * 1024 * 1024;
    private const string ExtensionPrefix = "browser_extension/nxtools-interceptor/";
    private static readonly string[] RequiredPaths =
    [
        "NxTools.exe", "NxTools.PlatformHost.exe", "NxTools.Indexer.exe", "NxTools.BrowserHost.exe", "NxTools.Cli.exe",
        "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md", ExtensionPrefix + "manifest.json"
    ];
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow,
        MaxDepth = 8
    };

    public static VerifiedRelease VerifyPackage(string root, byte[] expectedPublicKey, long minimumSequence)
    {
        ArgumentNullException.ThrowIfNull(expectedPublicKey);
        if (minimumSequence < 0) throw new ArgumentOutOfRangeException(nameof(minimumSequence));
        expectedPublicKey = expectedPublicKey.ToArray();
        using var rsa = RSA.Create();
        rsa.ImportSubjectPublicKeyInfo(expectedPublicKey, out var consumed);
        if (consumed != expectedPublicKey.Length || rsa.KeySize < 2048 || rsa.KeySize > 8192)
            throw new InvalidDataException("Invalid release trust anchor.");
        var handles = new PackageHandles(root);
        try
        {
            var manifestStream = handles.OpenFile(ManifestName);
            var signatureStream = handles.OpenFile(SignatureName);
            var bytes = ReadBounded(manifestStream, MaximumManifestBytes);
            var signature = ReadBounded(signatureStream, 1024);
            // Authenticate the exact raw bytes before interpreting any manifest field.
            if (!rsa.VerifyData(bytes, signature, HashAlgorithmName.SHA256, RSASignaturePadding.Pss))
                throw new CryptographicException("Release signature rejected.");
            using var document = JsonDocument.Parse(bytes, new JsonDocumentOptions { MaxDepth = 8 });
            RejectDuplicateProperties(document.RootElement);
            var manifest = JsonSerializer.Deserialize<ReleaseManifest>(bytes, JsonOptions)
                ?? throw new InvalidDataException("Missing release manifest.");
            ValidateManifest(manifest, expectedPublicKey, minimumSequence);
            manifest = manifest with { Files = new ReadOnlyCollection<ReleaseFile>(manifest.Files.ToArray()) };
            var files = new List<(ReleaseFile File, FileStream Stream)>();
            foreach (var file in manifest.Files)
            {
                var stream = handles.OpenFile(file.Path);
                VerifyFile(stream, file);
                files.Add((file, stream));
            }
            return new VerifiedRelease(manifest, Hash(bytes), handles, files, manifestStream, signatureStream);
        }
        catch { handles.Dispose(); throw; }
    }

    /// <summary>Uses an existing, explicitly supplied private PEM. Never creates or enrolls a key.</summary>
    public static byte[] PublicKeySpki(string signingKeyPath)
    {
        using var rsa = ReadSigningKey(signingKeyPath);
        return rsa.ExportSubjectPublicKeyInfo();
    }

    public static ReleaseManifest SignPackage(string root, string signingKeyPath, long releaseSequence, Guid releaseId)
    {
        if (releaseSequence <= 0 || releaseId == Guid.Empty) throw new ArgumentException("Invalid release identity.");
        var packageRoot = Path.GetFullPath(root);
        var keyPath = Path.GetFullPath(signingKeyPath);
        if (keyPath.StartsWith(Path.TrimEndingDirectorySeparator(packageRoot) + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
            throw new InvalidDataException("Release key must be outside the package.");
        using var rsa = ReadSigningKey(keyPath);
        using var handles = new PackageHandles(packageRoot);
        var files = new List<ReleaseFile>();
        var pending = new Stack<string>();
        pending.Push(packageRoot);
        var directoryCount = 0;
        long total = 0;
        while (pending.TryPop(out var directory))
        {
            if (++directoryCount > MaximumFiles) throw new InvalidDataException("Too many package directories.");
            handles.LockDirectory(directory);
            foreach (var entry in Directory.EnumerateFileSystemEntries(directory))
            {
                var attributes = File.GetAttributes(entry);
                if ((attributes & FileAttributes.ReparsePoint) != 0) throw new InvalidDataException("Reparse package entry rejected.");
                var relative = Path.GetRelativePath(packageRoot, entry).Replace('\\', '/');
                if ((attributes & FileAttributes.Directory) != 0)
                {
                    if (relative != "browser_extension" && relative != "browser_extension/nxtools-interceptor" && !relative.StartsWith(ExtensionPrefix, StringComparison.Ordinal))
                        throw new InvalidDataException("Unexpected package directory.");
                    ValidateComponents(relative);
                    pending.Push(entry);
                    continue;
                }
                if (relative is ManifestName or SignatureName) continue;
                ValidatePath(relative);
                if (files.Count >= MaximumFiles) throw new InvalidDataException("Too many package files.");
                var stream = handles.OpenFile(relative);
                total = checked(total + stream.Length);
                if (total > MaximumTotalBytes) throw new InvalidDataException("Package too large.");
                RejectSecrets(relative, stream);
                stream.Position = 0;
                files.Add(new ReleaseFile(relative, Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant(), stream.Length));
            }
        }
        files.Sort((a, b) => StringComparer.Ordinal.Compare(a.Path, b.Path));
        var spki = rsa.ExportSubjectPublicKeyInfo();
        var manifest = new ReleaseManifest(1, "nxtools-next", releaseSequence, releaseId, Hash(spki), files.AsReadOnly());
        ValidateManifest(manifest, spki, 0);
        var raw = JsonSerializer.SerializeToUtf8Bytes(manifest, JsonOptions);
        if (raw.Length > MaximumManifestBytes) throw new InvalidDataException("Manifest too large.");
        var signature = rsa.SignData(raw, HashAlgorithmName.SHA256, RSASignaturePadding.Pss);
        // Refuse to overwrite existing output or follow an output symlink. Package wrapper clears old sidecars explicitly.
        using (var output = new FileStream(Path.Combine(packageRoot, ManifestName), FileMode.CreateNew, FileAccess.Write, FileShare.None))
        { output.Write(raw); output.Flush(true); }
        using (var output = new FileStream(Path.Combine(packageRoot, SignatureName), FileMode.CreateNew, FileAccess.Write, FileShare.None))
        { output.Write(signature); output.Flush(true); }
        return manifest;
    }

    private static RSA ReadSigningKey(string path)
    {
        using var handles = new PackageHandles(Path.GetDirectoryName(Path.GetFullPath(path))!);
        using var stream = handles.OpenFile(Path.GetFileName(path));
        var pem = ReadBounded(stream, 64 * 1024);
        var rsa = RSA.Create();
        try
        {
            rsa.ImportFromPem(Encoding.UTF8.GetString(pem));
            if (rsa.KeySize < 2048 || rsa.KeySize > 8192) throw new CryptographicException("Invalid signing key size.");
            // Public-only PEM is not valid signing input.
            var secret = rsa.ExportPkcs8PrivateKey();
            CryptographicOperations.ZeroMemory(secret);
            return rsa;
        }
        catch { rsa.Dispose(); throw; }
        finally { CryptographicOperations.ZeroMemory(pem); }
    }

    private static void ValidateManifest(ReleaseManifest manifest, byte[] spki, long minimumSequence)
    {
        if (manifest.SchemaVersion != 1 || manifest.Product != "nxtools-next" || manifest.ReleaseId == Guid.Empty || manifest.ReleaseSequence <= minimumSequence)
            throw new InvalidDataException("Unsupported, invalid or replayed release.");
        if (!IsHash(manifest.PublicKeySha256) || !StringComparer.OrdinalIgnoreCase.Equals(manifest.PublicKeySha256, Hash(spki)))
            throw new InvalidDataException("Release key identity mismatch.");
        if (manifest.Files is null || manifest.Files.Count == 0 || manifest.Files.Count > MaximumFiles)
            throw new InvalidDataException("Invalid release file count.");
        var paths = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        long total = 0;
        foreach (var file in manifest.Files)
        {
            if (file is null) throw new InvalidDataException("Invalid release file.");
            ValidatePath(file.Path);
            if (!paths.Add(file.Path) || !IsHash(file.Sha256) || file.Size < 0 || file.Size > MaximumTotalBytes)
                throw new InvalidDataException("Invalid or duplicate release file.");
            total = checked(total + file.Size);
            if (total > MaximumTotalBytes) throw new InvalidDataException("Release too large.");
        }
        if (RequiredPaths.Any(path => !paths.Contains(path))) throw new InvalidDataException("Incomplete release.");
    }

    private static bool IsHash(string? value) => value is { Length: 64 } && value.All(Uri.IsHexDigit);
    internal static string Hash(byte[] bytes) => Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();

    private static void ValidatePath(string path)
    {
        ValidateComponents(path);
        if (!RequiredPaths.Contains(path, StringComparer.Ordinal) && !path.StartsWith(ExtensionPrefix, StringComparison.Ordinal))
            throw new InvalidDataException("Unapproved release path.");
    }

    private static void ValidateComponents(string path)
    {
        if (string.IsNullOrEmpty(path) || path.Length > 1024 || path.Contains('\\') || path.Contains(':') || path.StartsWith('/') || path.EndsWith('/'))
            throw new InvalidDataException("Unsafe release path.");
        foreach (var component in path.Split('/'))
        {
            if (component.Length == 0 || component is "." or ".." || component.EndsWith('.') || component.EndsWith(' ') || component.Any(c => c < 32 || "<>\"|?*".Contains(c)))
                throw new InvalidDataException("Unsafe release component.");
            var stem = component.Split('.')[0];
            if (new[] { "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$" }.Contains(stem, StringComparer.OrdinalIgnoreCase) ||
                (stem.Length == 4 && (stem.StartsWith("COM", StringComparison.OrdinalIgnoreCase) || stem.StartsWith("LPT", StringComparison.OrdinalIgnoreCase)) && "123456789¹²³".Contains(stem[3])))
                throw new InvalidDataException("Reserved release component.");
        }
    }

    private static void RejectSecrets(string path, FileStream stream)
    {
        var name = Path.GetFileName(path);
        if (new[] { ".pem", ".key", ".pfx", ".p12", ".snk" }.Contains(Path.GetExtension(name), StringComparer.OrdinalIgnoreCase) ||
            name.StartsWith(".env", StringComparison.OrdinalIgnoreCase) || name.Equals("credentials", StringComparison.OrdinalIgnoreCase))
            throw new InvalidDataException("Secret artifact rejected.");
        var buffer = new byte[65536 + 128];
        var carry = 0;
        int count;
        while ((count = stream.Read(buffer, carry, 65536)) != 0)
        {
            var text = Encoding.Latin1.GetString(buffer, 0, carry + count);
            if (text.Contains("PRIVATE KEY-----", StringComparison.Ordinal) || text.Contains("PuTTY-User-Key-File-", StringComparison.Ordinal))
                throw new InvalidDataException("Private key material rejected.");
            carry = Math.Min(128, carry + count);
            Buffer.BlockCopy(buffer, text.Length - carry, buffer, 0, carry);
        }
    }

    private static void RejectDuplicateProperties(JsonElement element)
    {
        if (element.ValueKind == JsonValueKind.Object)
        {
            var names = new HashSet<string>(StringComparer.Ordinal);
            foreach (var property in element.EnumerateObject())
            {
                if (!names.Add(property.Name)) throw new InvalidDataException("Duplicate manifest property.");
                RejectDuplicateProperties(property.Value);
            }
        }
        else if (element.ValueKind == JsonValueKind.Array)
            foreach (var item in element.EnumerateArray()) RejectDuplicateProperties(item);
    }

    private static byte[] ReadBounded(FileStream stream, int maximum)
    {
        if (stream.Length <= 0 || stream.Length > maximum) throw new InvalidDataException("Release input size rejected.");
        var bytes = new byte[checked((int)stream.Length)];
        stream.Position = 0;
        stream.ReadExactly(bytes);
        return bytes;
    }

    internal static void VerifyFile(FileStream stream, ReleaseFile file)
    {
        stream.Position = 0;
        if (stream.Length != file.Size || !StringComparer.OrdinalIgnoreCase.Equals(Convert.ToHexString(SHA256.HashData(stream)), file.Sha256))
            throw new InvalidDataException("Release artifact integrity mismatch.");
        stream.Position = 0;
    }

    // Lock every ancestor directory against rename/delete, open the final component without following
    // a reparse point, then inspect the opened handle (not a race-prone pre-open path check).
    internal sealed class PackageHandles : IDisposable
    {
        private readonly string root;
        private readonly Dictionary<string, SafeFileHandle> directories = new(StringComparer.OrdinalIgnoreCase);
        private readonly List<FileStream> streams = new();
        internal PackageHandles(string root)
        {
            this.root = Path.GetFullPath(root);
            try { LockDirectory(this.root); }
            catch { Dispose(); throw; }
        }
        internal void LockDirectory(string path)
        {
            path = Path.GetFullPath(path);
            if (directories.ContainsKey(path)) return;
            var parent = Directory.GetParent(path);
            if (parent is not null) LockDirectory(parent.FullName);
            var handle = OpenHandle(path, true);
            directories.Add(path, handle);
        }
        internal FileStream OpenFile(string relative)
        {
            ValidateComponents(relative);
            var path = Path.Combine(root, relative.Replace('/', Path.DirectorySeparatorChar));
            LockDirectory(Path.GetDirectoryName(path)!);
            var handle = OpenHandle(path, false);
            try
            {
                var stream = new FileStream(handle, FileAccess.Read);
                streams.Add(stream);
                return stream;
            }
            catch { handle.Dispose(); throw; }
        }
        private static SafeFileHandle OpenHandle(string path, bool directory)
        {
            var handle = CreateFileW(path, directory ? 0u : 0x80000000u, 1, IntPtr.Zero, 3, 0x00200000u | (directory ? 0x02000000u : 0u), IntPtr.Zero);
            if (handle.IsInvalid) { var code = Marshal.GetLastWin32Error(); handle.Dispose(); throw new System.ComponentModel.Win32Exception(code, "Release handle open failed."); }
            if (!GetFileInformationByHandle(handle, out var info))
            { var code = Marshal.GetLastWin32Error(); handle.Dispose(); throw new System.ComponentModel.Win32Exception(code, "Release handle inspection failed."); }
            if ((info.Attributes & 0x400) != 0 || ((info.Attributes & 0x10) != 0) != directory)
            { handle.Dispose(); throw new InvalidDataException("Reparse or wrong-kind release entry rejected."); }
            return handle;
        }
        public void Dispose()
        {
            foreach (var stream in streams) stream.Dispose();
            foreach (var handle in directories.Values.Reverse()) handle.Dispose();
        }
        [StructLayout(LayoutKind.Sequential)]
        private struct FileInformation
        {
            public uint Attributes;
            public System.Runtime.InteropServices.ComTypes.FILETIME CreationTime, AccessTime, WriteTime;
            public uint VolumeSerial, SizeHigh, SizeLow, Links, IndexHigh, IndexLow;
        }
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true, ExactSpelling = true)]
        private static extern SafeFileHandle CreateFileW(string path, uint access, uint share, IntPtr security, uint creation, uint flags, IntPtr template);
        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetFileInformationByHandle(SafeFileHandle handle, out FileInformation information);
    }
}

public sealed class VerifiedRelease : IDisposable
{
    public ReleaseManifest Manifest { get; }
    public string ManifestSha256 { get; }
    public IReadOnlyList<ReleaseFile> Files => Manifest.Files;
    private readonly TrustedReleaseManifest.PackageHandles handles;
    private readonly IReadOnlyList<(ReleaseFile File, FileStream Stream)> files;
    private readonly FileStream manifestStream, signatureStream;
    private bool disposed;
    internal VerifiedRelease(ReleaseManifest manifest, string sha256, TrustedReleaseManifest.PackageHandles handles,
        IReadOnlyList<(ReleaseFile File, FileStream Stream)> files, FileStream manifestStream, FileStream signatureStream)
    {
        Manifest = manifest; ManifestSha256 = sha256; this.handles = handles; this.files = files;
        this.manifestStream = manifestStream; this.signatureStream = signatureStream;
    }

    /// <summary>Copies only authenticated artifacts and sidecars from the original locked handles.
    /// Destination must be an existing empty, caller-owned staging directory with protected ACLs.
    /// A failed copy remains unpublished; caller owns staging cleanup/transaction recovery.</summary>
    public void CopyTo(string destination)
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        destination = Path.GetFullPath(destination);
        using var targetLocks = new TrustedReleaseManifest.PackageHandles(destination);
        if (Directory.EnumerateFileSystemEntries(destination).Any()) throw new InvalidDataException("Staging directory must be empty.");
        foreach (var (file, source) in files)
        {
            var target = Path.Combine(destination, file.Path.Replace('/', Path.DirectorySeparatorChar));
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            targetLocks.LockDirectory(Path.GetDirectoryName(target)!);
            using var output = new FileStream(target, FileMode.CreateNew, FileAccess.ReadWrite, FileShare.None);
            source.Position = 0;
            source.CopyTo(output, 65536);
            output.Flush(true);
            TrustedReleaseManifest.VerifyFile(output, file);
        }
        CopySidecar(manifestStream, Path.Combine(destination, TrustedReleaseManifest.ManifestName));
        CopySidecar(signatureStream, Path.Combine(destination, TrustedReleaseManifest.SignatureName));
    }
    private static void CopySidecar(FileStream source, string destination)
    {
        using var output = new FileStream(destination, FileMode.CreateNew, FileAccess.Write, FileShare.None);
        source.Position = 0; source.CopyTo(output, 65536); output.Flush(true);
    }
    public void Dispose() { if (!disposed) { disposed = true; handles.Dispose(); } }
}
