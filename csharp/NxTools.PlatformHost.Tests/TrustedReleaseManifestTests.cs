using System.ComponentModel;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Generated;
using System.Text.Json.Nodes;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// All artifacts and keys are synthetic GUID-owned temporary fixtures; no service or production PEM.
internal static class TrustedReleaseManifestTests
{
    internal static Task ValidSignatureAndLockedCopy()
    {
        using var fixture = new ReleaseFixture();
        fixture.Sign();
        using (var verified = fixture.Verify())
        {
            Require(verified.Manifest.ReleaseSequence == 10 && verified.Manifest.ReleaseId == fixture.Id, "Signed identity must survive verification.");
            Require(verified.ManifestSha256 == Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(fixture.ManifestPath))).ToLowerInvariant(), "Manifest hash covers exact raw bytes.");
            foreach (var path in new[] { "NxTools.exe", TrustedReleaseManifest.ManifestName, TrustedReleaseManifest.SignatureName })
            {
                Reject<IOException>(() => { using var write = File.Open(Path.Combine(fixture.Package, path), FileMode.Open, FileAccess.Write, FileShare.ReadWrite); });
                Reject<IOException>(() => File.Delete(Path.Combine(fixture.Package, path)));
            }
            Reject<IOException>(() => Directory.Move(fixture.Package, fixture.Package + "-moved"));
            var staging = Path.Combine(fixture.Root, "staging");
            Directory.CreateDirectory(staging);
            verified.CopyTo(staging);
            using var recovered = TrustedReleaseManifest.VerifyPackage(staging, fixture.Spki, 9);
            Require(recovered.ManifestSha256 == verified.ManifestSha256 && recovered.Files.Count == verified.Files.Count, "Copied staging must verify independently against old protected sequence.");
            foreach (var file in verified.Files)
                Require(File.ReadAllBytes(Path.Combine(staging, file.Path)).SequenceEqual(File.ReadAllBytes(Path.Combine(fixture.Package, file.Path))), "Copy must preserve every signed byte.");
            Reject<InvalidDataException>(() => verified.CopyTo(staging));
        }
        // Dispose releases original handles.
        using var released = File.Open(Path.Combine(fixture.Package, "NxTools.exe"), FileMode.Open, FileAccess.Write, FileShare.None);
        return Task.CompletedTask;
    }

    internal static Task TamperAndWrongKeyRejectedBeforeParse()
    {
        using var fixture = new ReleaseFixture();
        fixture.Sign();
        using var other = RSA.Create(2048);
        Reject<CryptographicException>(() => { using var rejected = TrustedReleaseManifest.VerifyPackage(fixture.Package, other.ExportSubjectPublicKeyInfo(), 0); });
        var original = File.ReadAllBytes(fixture.ManifestPath);
        File.WriteAllBytes(fixture.ManifestPath, Encoding.UTF8.GetBytes("{not authenticated JSON"));
        Reject<CryptographicException>(() => { using var rejected = fixture.Verify(); });
        File.WriteAllBytes(fixture.ManifestPath, original);
        var signature = File.ReadAllBytes(fixture.SignaturePath);
        signature[0] ^= 1;
        File.WriteAllBytes(fixture.SignaturePath, signature);
        Reject<CryptographicException>(() => { using var rejected = fixture.Verify(); });
        return Task.CompletedTask;
    }

    internal static Task LegacySnapshotIsRecoveryOnly()
    {
        var workspace = FindWorkspaceRoot();
        var package = Path.Combine(workspace, ".diagnostics", "packaging", "20261115-121004-bb6cb18d-published", "previous-package");
        Require(Directory.Exists(package), "Retained legacy release fixture is required.");
        var manifestPath = Path.Combine(package, TrustedReleaseManifest.ManifestName);
        var manifestBytes = File.ReadAllBytes(manifestPath);
        var manifest = JsonSerializer.Deserialize<ReleaseManifest>(manifestBytes)
            ?? throw new InvalidDataException("Legacy release manifest is missing.");
        var manifestSha = Convert.ToHexString(SHA256.HashData(manifestBytes)).ToLowerInvariant();
        var legacyKey = Convert.FromBase64String(LegacyUpdateTrust.PublicKeySpkiBase64);
        var currentKey = Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64);
        Require(manifest.PublicKeySha256 == Convert.ToHexString(SHA256.HashData(legacyKey)).ToLowerInvariant(), "Legacy fixture is bound to the historical trust anchor.");
        Require(!legacyKey.SequenceEqual(currentKey), "Historical and current trust anchors must differ.");
        using (var recovered = TrustedReleaseManifest.VerifyRecoverySnapshot(package, legacyKey,
            manifest.ReleaseSequence, manifest.ReleaseId, manifestSha))
            Require(recovered.RecoveryOnly, "Historical snapshot is accepted only through recovery verification.");
        Reject<CryptographicException>(() => { using var rejected = TrustedReleaseManifest.VerifyPackage(package, currentKey, 0); });
        return Task.CompletedTask;
    }

    private static string FindWorkspaceRoot()
    {
        foreach (var start in new[] { Directory.GetCurrentDirectory(), AppContext.BaseDirectory })
        {
            var directory = new DirectoryInfo(start);
            while (directory is not null)
            {
                if (Directory.Exists(Path.Combine(directory.FullName, ".diagnostics"))) return directory.FullName;
                directory = directory.Parent;
            }
        }
        throw new DirectoryNotFoundException("Workspace root not found.");
    }

    internal static Task SignedDuplicateTraversalAndSchemaRejected()
    {
        using var fixture = new ReleaseFixture();
        fixture.Sign();
        var original = File.ReadAllBytes(fixture.ManifestPath);
        void Mutation(Action<JsonObject> mutate)
        {
            var json = JsonNode.Parse(original)!.AsObject();
            mutate(json);
            fixture.WriteSigned(JsonSerializer.SerializeToUtf8Bytes(json));
            Reject<InvalidDataException>(() => { using var rejected = fixture.Verify(); });
        }
        Mutation(json => json["files"]!.AsArray().Add(json["files"]![0]!.DeepClone()));
        Mutation(json => json["files"]!.AsArray().Add(new JsonObject { ["path"] = "browser_extension/nxtools-interceptor/MANIFEST.JSON", ["sha256"] = new string('0', 64), ["size"] = 0 }));
        foreach (var path in new[] { "../NxTools.exe", "C:/NxTools.exe", "/NxTools.exe", "NxTools.exe:evil", "browser_extension\\nxtools-interceptor\\manifest.json", "browser_extension/nxtools-interceptor/../evil", "browser_extension/nxtools-interceptor/CON.txt", "browser_extension/nxtools-interceptor/evil.", "unapproved.dll" })
            Mutation(json => json["files"]![0]!["path"] = path);
        Mutation(json => json["files"]!.AsArray().RemoveAt(0));
        Mutation(json => json["product"] = "another-product");
        Mutation(json => json["schema_version"] = 2);
        Mutation(json => json["release_id"] = Guid.Empty.ToString());
        Mutation(json => json["public_key_sha256"] = new string('0', 64));
        Mutation(json => json["files"]![0]!["size"] = -1);
        Mutation(json => json["files"]![0]!["size"] = TrustedReleaseManifest.MaximumTotalBytes + 1);
        Mutation(json =>
        {
            json["files"]![0]!["size"] = TrustedReleaseManifest.MaximumTotalBytes;
            json["files"]![1]!["size"] = 1;
        });
        Mutation(json =>
        {
            var entries = json["files"]!.AsArray();
            while (entries.Count <= TrustedReleaseManifest.MaximumFiles) entries.Add(entries[0]!.DeepClone());
        });
        foreach (var field in new[] { "schema_version", "product", "release_sequence", "release_id", "public_key_sha256", "files" })
        {
            var json = JsonNode.Parse(original)!.AsObject();
            json.Remove(field);
            fixture.WriteSigned(JsonSerializer.SerializeToUtf8Bytes(json));
            Reject<JsonException>(() => { using var rejected = fixture.Verify(); });
        }
        var missingSize = JsonNode.Parse(original)!.AsObject();
        missingSize["files"]![0]!.AsObject().Remove("size");
        fixture.WriteSigned(JsonSerializer.SerializeToUtf8Bytes(missingSize));
        Reject<JsonException>(() => { using var rejected = fixture.Verify(); });
        var duplicateProperty = Encoding.UTF8.GetString(original).Replace("\"schema_version\":1", "\"schema_version\":1,\"schema_version\":1", StringComparison.Ordinal);
        fixture.WriteSigned(Encoding.UTF8.GetBytes(duplicateProperty));
        Reject<InvalidDataException>(() => { using var rejected = fixture.Verify(); });
        return Task.CompletedTask;
    }

    internal static Task MissingTamperedAndDowngradeRejected()
    {
        using var fixture = new ReleaseFixture();
        fixture.Sign();
        Reject<InvalidDataException>(() => { using var rejected = fixture.Verify(10); });
        Reject<InvalidDataException>(() => { using var rejected = fixture.Verify(11); });
        var binary = Path.Combine(fixture.Package, "NxTools.exe");
        var original = File.ReadAllBytes(binary);
        var sameSizeTamper = original.ToArray();
        sameSizeTamper[0] ^= 1;
        File.WriteAllBytes(binary, sameSizeTamper);
        Reject<InvalidDataException>(() => { using var rejected = fixture.Verify(); });
        File.WriteAllBytes(binary, original);
        File.WriteAllBytes(binary, Encoding.UTF8.GetBytes("tampered"));
        Reject<InvalidDataException>(() => { using var rejected = fixture.Verify(); });
        File.Delete(binary);
        Reject<Win32Exception>(() => { using var rejected = fixture.Verify(); });
        return Task.CompletedTask;
    }

    internal static Task ReparseComponentsRejected()
    {
        using var fixture = new ReleaseFixture();
        fixture.Sign();
        var extension = Path.Combine(fixture.Package, "browser_extension", "nxtools-interceptor");
        var original = Path.Combine(fixture.Root, "extension-original");
        Directory.Move(extension, original);
        Directory.CreateSymbolicLink(extension, original);
        try
        {
            Reject<InvalidDataException>(() => { using var rejected = fixture.Verify(); });
            var key = fixture.Key;
            File.Delete(fixture.ManifestPath); File.Delete(fixture.SignaturePath);
            Reject<InvalidDataException>(() => TrustedReleaseManifest.SignPackage(fixture.Package, key, 11, Guid.NewGuid()));
        }
        finally { Directory.Delete(extension); Directory.Move(original, extension); }
        fixture.Sign();
        var binary = Path.Combine(fixture.Package, "NxTools.exe");
        var moved = Path.Combine(fixture.Root, "binary-original");
        File.Move(binary, moved);
        File.CreateSymbolicLink(binary, moved);
        try { Reject<InvalidDataException>(() => { using var rejected = fixture.Verify(); }); }
        finally { File.Delete(binary); File.Move(moved, binary); }
        return Task.CompletedTask;
    }

    internal static Task BoundedInputsAndSecretSigningRejected()
    {
        using var fixture = new ReleaseFixture();
        fixture.Sign();
        var original = File.ReadAllBytes(fixture.ManifestPath);
        File.WriteAllBytes(fixture.ManifestPath, new byte[TrustedReleaseManifest.MaximumManifestBytes + 1]);
        Reject<InvalidDataException>(() => { using var rejected = fixture.Verify(); });
        File.WriteAllBytes(fixture.ManifestPath, original);
        File.WriteAllBytes(fixture.SignaturePath, new byte[1025]);
        Reject<InvalidDataException>(() => { using var rejected = fixture.Verify(); });
        File.Delete(fixture.ManifestPath); File.Delete(fixture.SignaturePath);
        var secret = Path.Combine(fixture.Package, "browser_extension", "nxtools-interceptor", "secret.pem");
        File.WriteAllText(secret, "synthetic");
        Reject<InvalidDataException>(() => fixture.Sign());
        Require(!File.Exists(fixture.ManifestPath), "Secret validation precedes manifest emission.");
        File.Delete(secret);
        var disguised = Path.Combine(fixture.Package, "browser_extension", "nxtools-interceptor", "secret.txt");
        File.WriteAllText(disguised, new string('x', 65530) + "-----BEGIN PRIVATE KEY-----\nsynthetic");
        Reject<InvalidDataException>(() => fixture.Sign());
        Require(!File.Exists(fixture.ManifestPath), "Chunk-boundary private material must not be signed.");
        File.Delete(disguised);
        File.WriteAllText(Path.Combine(fixture.Package, "unexpected.dll"), "synthetic");
        Reject<InvalidDataException>(() => fixture.Sign());
        return Task.CompletedTask;
    }

    internal static Task ArchiveMembersRequiredBeforeSigning()
    {
        // Exercise the existing signer, not a parallel package-completeness predicate.
        using var fixture = new ReleaseFixture(includeArchive: false);
        foreach (var name in ArchiveMembers) {
            Reject<InvalidDataException>(() => fixture.Sign());
            File.WriteAllText(Path.Combine(fixture.Package, name), "synthetic " + name);
        }
        fixture.Sign();
        using var verified = fixture.Verify();
        Require(verified.Files.Count == 14, "All five archive deployment members must be authenticated.");
        return Task.CompletedTask;
    }

    internal static readonly string[] ArchiveMembers = ["NxTools.ArchiveWorker.exe", "NxTools.ShellSelectionHelper.exe",
        "NxTools.ShellSelectionHelper.dll", "NxTools.ShellSelectionHelper.deps.json", "NxTools.ShellSelectionHelper.runtimeconfig.json"];
    private static void Require(bool condition, string message)
    { if (!condition) throw new InvalidOperationException(message); }
    private static void Reject<T>(Action action) where T : Exception
    {
        try { action(); }
        catch (T) { return; }
        throw new InvalidOperationException($"Expected {typeof(T).Name} rejection.");
    }

    private sealed class ReleaseFixture : IDisposable
    {
        internal string Root { get; } = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(), "NxTools-release-fixture-" + Guid.NewGuid().ToString("N"));
        internal string Package => Path.Combine(Root, "package");
        internal string Key => Path.Combine(Root, "fixture-private.pem");
        internal string ManifestPath => Path.Combine(Package, TrustedReleaseManifest.ManifestName);
        internal string SignaturePath => Path.Combine(Package, TrustedReleaseManifest.SignatureName);
        internal Guid Id { get; } = Guid.NewGuid();
        private readonly RSA rsa = RSA.Create(2048);
        internal byte[] Spki => rsa.ExportSubjectPublicKeyInfo();
        internal ReleaseFixture(bool includeArchive = true)
        {
            Directory.CreateDirectory(Path.Combine(Package, "browser_extension", "nxtools-interceptor"));
            File.WriteAllText(Key, rsa.ExportPkcs8PrivateKeyPem());
            foreach (var name in new[] { "NxTools.exe", "NxTools.PlatformHost.exe", "NxTools.Indexer.exe", "NxTools.BrowserHost.exe", "NxTools.Cli.exe", "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md", "browser_extension/nxtools-interceptor/manifest.json" })
                File.WriteAllBytes(Path.Combine(Package, name), Encoding.UTF8.GetBytes("synthetic release artifact: " + name));
            if (includeArchive) foreach (var name in ArchiveMembers)
                File.WriteAllText(Path.Combine(Package, name), "synthetic " + name);
        }
        internal void Sign() => TrustedReleaseManifest.SignPackage(Package, Key, 10, Id);
        internal VerifiedRelease Verify(long minimum = 9) => TrustedReleaseManifest.VerifyPackage(Package, Spki, minimum);
        internal void WriteSigned(byte[] bytes)
        {
            File.WriteAllBytes(ManifestPath, bytes);
            File.WriteAllBytes(SignaturePath, rsa.SignData(bytes, HashAlgorithmName.SHA256, RSASignaturePadding.Pss));
        }
        public void Dispose() { rsa.Dispose(); Directory.Delete(Root, true); }
    }
}
