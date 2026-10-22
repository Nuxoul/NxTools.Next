using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class BrowserExtensionSigningTests
{
    internal static Task ExplicitSigningPathIsRequiredForIsolatedEntry()
    {
        var export = typeof(BrowserExtensionPackageService).GetMethod("ExportWithSigningKey",
            BindingFlags.Static | BindingFlags.NonPublic, null,
            new[] { typeof(string), typeof(string), typeof(string), typeof(string), typeof(CancellationToken) }, null);
        Require(export is not null, "Isolated export must accept an explicit signingKeyPath; source/output temp paths alone do not isolate PEM storage.");
        var origins = typeof(BrowserNativeMessagingRegistrar).GetMethod("ChromiumAllowedOrigins",
            BindingFlags.Static | BindingFlags.NonPublic, null,
            new[] { typeof(string), typeof(string), typeof(string) }, null);
        Require(origins is not null, "Read-only identity projection must accept explicit source and key paths without registration.");
        return Task.CompletedTask;
    }

    private const string PinnedKey = "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEArh9uoEGy5esC9y2AGR814h0iNb6783TgMeyJM3uQ7MxJPZfAPHYU/enNwMpko9xKi1g/F8i96Ix5+skDJgRY0d36H20P3W3qlCXSIXxLQycnKvJ3731748ZLrWvGKRewA7FMN/ha+CUFNDzFWmNTv/kTEonV/v2KQu2TCiCJKCo3BoHRSL8La03ZAhG4TiHA4881W3Gwas+WJn3BevHdN0hVcdh8IgFQsCdEBLpds3rO0IWa7k8pcZVgqSkMh+roJ1JudswvClAv3IyArZwCVEgm49WDjgCSiqAcZkqIqO/xmlvRCQfLYmOL+SrIoniE5YVA3s/2YqJHZGkN6PhxbQIDAQAB";

    internal static Task FixedSpkiVectorPreservesChromiumIdentity()
    {
        var spki = Convert.FromBase64String(PinnedKey);
        Require(BrowserExtensionSigningIdentity.ChromiumId(spki) == "haohhnpjfggpdadhcbjakjpkmkljkpif", "Pinned manifest ID must remain exact.");
        Require(Convert.ToHexString(BrowserExtensionSigningIdentity.CrxId(spki)) == "70E77DF9566F30372190A9FACAB9AF85", "CRX ID uses first16SHA256 bytes.");
        Require(BrowserExtensionSigningIdentity.KeyPathUnder("fixture-root") == Path.Combine("fixture-root", "NxTools", "browser_extension_key.pem"), "Legacy relative key location preserved without reading production storage.");
        return Task.CompletedTask;
    }

    internal static Task RegistrationReadsExistingPemWithoutCreatingOrRewriting()
    {
        using var fixture = new SigningFixture();
        var absent = Path.Combine(fixture.Root, "absent", "key.pem");
        Require(BrowserExtensionSigningIdentity.ReadExistingChromiumId(absent) is null, "Missing key has no exporter identity.");
        Require(!Directory.Exists(Path.GetDirectoryName(absent)), "Read-only path creates no directory.");
        using var rsa = RSA.Create(2048);
        File.WriteAllText(fixture.KeyPath, rsa.ExportPkcs8PrivateKeyPem());
        var bytes = File.ReadAllBytes(fixture.KeyPath);
        var timestamp = File.GetLastWriteTimeUtc(fixture.KeyPath);
        Require(BrowserExtensionSigningIdentity.ReadExistingChromiumId(fixture.KeyPath) == BrowserExtensionSigningIdentity.ChromiumId(rsa.ExportSubjectPublicKeyInfo()), "PEM public identity matches SPKI.");
        Require(File.ReadAllBytes(fixture.KeyPath).SequenceEqual(bytes) && File.GetLastWriteTimeUtc(fixture.KeyPath) == timestamp, "Registration identity does not rewrite key.");
        File.WriteAllText(fixture.KeyPath, rsa.ExportSubjectPublicKeyInfoPem());
        Require(BrowserExtensionSigningIdentity.ReadExistingChromiumId(fixture.KeyPath) == BrowserExtensionSigningIdentity.ChromiumId(rsa.ExportSubjectPublicKeyInfo()), "Existing public-only PEM retains original readable identity behavior.");
        File.WriteAllText(fixture.KeyPath, rsa.ExportPkcs8PrivateKeyPem());
        using (var locked = new FileStream(fixture.KeyPath, FileMode.Open, FileAccess.Read, FileShare.None))
        {
            var origins = BrowserNativeMessagingRegistrar.ChromiumAllowedOrigins("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", fixture.Source, fixture.KeyPath);
            Require(origins.SequenceEqual(new[] { "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/" }), "Existing IOException fallback preserves command identity without writing PEM.");
            try { BrowserExtensionPackageService.ExportWithSigningKey("chrome", Path.Combine(fixture.Root, "locked.crx"), fixture.Source, fixture.KeyPath); throw new InvalidOperationException("Expected locked PEM IO failure"); }
            catch (IOException) { }
        }
        Require(File.ReadAllBytes(fixture.KeyPath).SequenceEqual(bytes), "Locked existing key is not replaced.");
        return Task.CompletedTask;
    }

    internal static Task OriginsMergeCommandPinnedAndExportedWithoutRegistration()
    {
        using var fixture = new SigningFixture();
        using var rsa = RSA.Create(2048);
        File.WriteAllText(fixture.KeyPath, rsa.ExportPkcs8PrivateKeyPem());
        fixture.Manifest(PinnedKey);
        var exported = BrowserExtensionSigningIdentity.ChromiumId(rsa.ExportSubjectPublicKeyInfo());
        var origins = BrowserNativeMessagingRegistrar.ChromiumAllowedOrigins("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", fixture.Source, fixture.KeyPath);
        Require(origins.SequenceEqual(new[] { "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/", "chrome-extension://haohhnpjfggpdadhcbjakjpkmkljkpif/", $"chrome-extension://{exported}/" }), "Command/pinned/exported union and ordering preserved.");
        fixture.Manifest(Convert.ToBase64String(rsa.ExportSubjectPublicKeyInfo()));
        origins = BrowserNativeMessagingRegistrar.ChromiumAllowedOrigins(exported, fixture.Source, fixture.KeyPath);
        Require(origins.Length == 1, "Duplicate command/manifest/exported identity deduplicates.");
        fixture.Manifest("not base64!");
        origins = BrowserNativeMessagingRegistrar.ChromiumAllowedOrigins("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", fixture.Source, fixture.KeyPath);
        Require(origins.Length == 2 && origins[1] == $"chrome-extension://{exported}/", "Malformed manifest key keeps command/exported fallback.");
        File.Delete(Path.Combine(fixture.Source, "manifest.json"));
        origins = BrowserNativeMessagingRegistrar.ChromiumAllowedOrigins("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", fixture.Source, fixture.KeyPath);
        Require(origins.Length == 2 && origins[1] == $"chrome-extension://{exported}/", "Missing manifest preserves command/exported union.");
        File.WriteAllText(Path.Combine(fixture.Source, "manifest.json"), "{");
        try { BrowserNativeMessagingRegistrar.ChromiumAllowedOrigins("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", fixture.Source, fixture.KeyPath); throw new InvalidOperationException("Expected malformed JSON failure"); }
        catch (JsonException) { }
        fixture.Manifest("not base64!");
        File.Delete(fixture.KeyPath);
        origins = BrowserNativeMessagingRegistrar.ChromiumAllowedOrigins("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", fixture.Source, fixture.KeyPath);
        Require(origins.Length == 1 && !File.Exists(fixture.KeyPath), "Registration never creates missing key.");
        return Task.CompletedTask;
    }

    internal static Task IsolatedCrxSignatureAndKeyReuseMatchRegistrationIdentity()
    {
        using var fixture = new SigningFixture();
        fixture.Manifest(PinnedKey);
        var output = BrowserExtensionPackageService.ExportWithSigningKey("chrome", Path.Combine(fixture.Root, "first.crx"), fixture.Source, fixture.KeyPath);
        Require(File.Exists(fixture.KeyPath), "Explicit export owns fixture key creation.");
        var pem = File.ReadAllBytes(fixture.KeyPath);
        using var rsa = RSA.Create();
        rsa.ImportFromPem(Encoding.UTF8.GetString(pem));
        var crx = File.ReadAllBytes(output);
        Require(Encoding.ASCII.GetString(crx, 0, 4) == "Cr24" && BitConverter.ToUInt32(crx, 4) == 3, "CRX3 envelope retained.");
        var headerLength = checked((int)BitConverter.ToUInt32(crx, 8));
        var header = Fields(crx.AsSpan(12, headerLength).ToArray());
        var proof = Fields(header[2]);
        var signed = header[10000];
        var crxId = Fields(signed)[1];
        Require(proof[1].SequenceEqual(rsa.ExportSubjectPublicKeyInfo()), "Embedded key is the isolated PEM's SPKI.");
        Require(crxId.SequenceEqual(BrowserExtensionSigningIdentity.CrxId(proof[1])), "CRX signed ID matches shared identity.");
        using var input = new MemoryStream();
        input.Write(Encoding.ASCII.GetBytes("CRX3 SignedData\0"));
        input.Write(BitConverter.GetBytes((uint)signed.Length)); input.Write(signed); input.Write(crx.AsSpan(12 + headerLength));
        Require(rsa.VerifyData(input.ToArray(), proof[2], HashAlgorithmName.SHA256, RSASignaturePadding.Pkcs1), "Actual CRX signature verifies.");
        var origins = BrowserNativeMessagingRegistrar.ChromiumAllowedOrigins("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", fixture.Source, fixture.KeyPath);
        Require(origins.Contains($"chrome-extension://{BrowserExtensionSigningIdentity.ChromiumId(proof[1])}/"), "Registrar trusts actual exported identity.");
        BrowserExtensionPackageService.ExportWithSigningKey("chrome", Path.Combine(fixture.Root, "second.crx"), fixture.Source, fixture.KeyPath);
        Require(File.ReadAllBytes(fixture.KeyPath).SequenceEqual(pem), "Second export reuses existing key unchanged.");
        return Task.CompletedTask;
    }

    internal static Task MissingCorruptAndFirefoxKeyBehaviorRemainCompatible()
    {
        using var fixture = new SigningFixture();
        fixture.Manifest(PinnedKey);
        var untouched = Path.Combine(fixture.Root, "not-created", "key.pem");
        var xpi = BrowserExtensionPackageService.ExportWithSigningKey("firefox", Path.Combine(fixture.Root, "firefox.xpi"), fixture.Source, untouched);
        Require(File.Exists(xpi) && !Directory.Exists(Path.GetDirectoryName(untouched)), "Firefox never reads/creates RSA key.");
        File.WriteAllText(fixture.KeyPath, "-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----");
        var corrupt = File.ReadAllBytes(fixture.KeyPath);
        RejectInvalidPem(() => BrowserNativeMessagingRegistrar.ChromiumAllowedOrigins("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", fixture.Source, fixture.KeyPath));
        RejectInvalidPem(() => BrowserExtensionPackageService.ExportWithSigningKey("chrome", Path.Combine(fixture.Root, "bad.crx"), fixture.Source, fixture.KeyPath));
        Require(File.ReadAllBytes(fixture.KeyPath).SequenceEqual(corrupt), "Corrupt existing key is never replaced.");
        using var cancel = new CancellationTokenSource(); cancel.Cancel();
        try { BrowserExtensionPackageService.ExportWithSigningKey("chrome", Path.Combine(fixture.Root, "cancel.crx"), fixture.Source, untouched, cancel.Token); throw new InvalidOperationException("Expected cancellation"); }
        catch (OperationCanceledException) { }
        Require(!Directory.Exists(Path.GetDirectoryName(untouched)), "Cancelled export creates no key.");
        return Task.CompletedTask;
    }

    internal static Task SourceDirectoryFallbacksStayDistinct()
    {
        using var fixture = new SigningFixture();
        var root = Path.Combine(fixture.Root, "package", "host");
        var current = Path.Combine(fixture.Root, "working");
        var first = Path.Combine(root, "browser_extension", "nxtools-interceptor");
        var parent = Path.Combine(root, "..", "browser_extension", "nxtools-interceptor");
        var working = Path.Combine(current, "browser_extension", "nxtools-interceptor");
        Directory.CreateDirectory(working);
        Require(BrowserExtensionPackageService.ResolveExtensionDirectory(root, current) == working, "Exporter retains working-directory fallback.");
        Require(BrowserNativeMessagingRegistrar.ResolveExtensionSourceDirectory(root) == first, "Registrar does not adopt working-directory fallback.");
        Directory.CreateDirectory(parent);
        Require(BrowserExtensionPackageService.ResolveExtensionDirectory(root, current) == parent && BrowserNativeMessagingRegistrar.ResolveExtensionSourceDirectory(root) == parent, "Both retain parent fallback.");
        Directory.CreateDirectory(first);
        Require(BrowserExtensionPackageService.ResolveExtensionDirectory(root, current) == first && BrowserNativeMessagingRegistrar.ResolveExtensionSourceDirectory(root) == first, "Packaged sibling remains first.");
        return Task.CompletedTask;
    }

    private static Dictionary<int, byte[]> Fields(byte[] bytes)
    {
        var result = new Dictionary<int, byte[]>(); var position = 0;
        ulong ReadVarint() { ulong value = 0; var shift = 0; while (position < bytes.Length && shift < 64) { var next = bytes[position++]; value |= (ulong)(next & 127) << shift; if ((next & 128) == 0) return value; shift += 7; } throw new InvalidDataException("invalid fixture CRX protobuf"); }
        while (position < bytes.Length) { var tag = ReadVarint(); Require((tag & 7) == 2, "CRX expected length-delimited fields"); var length = checked((int)ReadVarint()); Require(length <= bytes.Length - position, "CRX field bounds"); result.Add(checked((int)(tag >> 3)), bytes.AsSpan(position, length).ToArray()); position += length; }
        return result;
    }
    private static void RejectInvalidPem(Action action)
    {
        try { action(); }
        catch (Exception error) when (error is CryptographicException or ArgumentException) { return; }
        throw new InvalidOperationException("Malformed key must preserve original failure category.");
    }
    private sealed class SigningFixture : IDisposable
    {
        internal string Root { get; } = Path.Combine(Path.GetTempPath(), "nxtools-signing-fixture-" + Guid.NewGuid().ToString("N"));
        internal string Source => Path.Combine(Root, "source");
        internal string KeyPath => Path.Combine(Root, "isolated.pem");
        internal SigningFixture() { Directory.CreateDirectory(Source); File.WriteAllText(Path.Combine(Source, "background.js"), "// fixture"); }
        internal void Manifest(string key) => File.WriteAllText(Path.Combine(Source, "manifest.json"), JsonSerializer.Serialize(new { manifest_version = 3, name = "fixture", version = "0.0.1", key, permissions = new[] { "declarativeNetRequest", "storage" } }));
        public void Dispose() { if (Directory.Exists(Root)) Directory.Delete(Root, true); }
    }

    private static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
