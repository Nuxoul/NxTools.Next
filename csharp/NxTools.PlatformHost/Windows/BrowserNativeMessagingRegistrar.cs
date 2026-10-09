using System.Diagnostics;
using System.Globalization;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Generated;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static partial class BrowserNativeMessagingRegistrar
{
    private const string HostName = "com.nxtools.next.browser";
    private static readonly object RegistrationGate = new();
    private static readonly string[] BrowserKeys = [
        @"Software\Google\Chrome\NativeMessagingHosts\",
        @"Software\Microsoft\Edge\NativeMessagingHosts\",
        @"Software\Mozilla\NativeMessagingHosts\" ];


    /// <summary>
    /// Builds the allowed-origins list from the packaged extension's pinned
    /// key (its stable extension id) merged with the command-provided id, so
    /// unpacked loads whose manifest predates the key still connect.
    /// </summary>
    private static string[] ChromiumAllowedOrigins(string commandExtensionId)
    {
        var origins = new List<string> { $"chrome-extension://{commandExtensionId}/" };
        try
        {
            var manifestPath = Path.Combine(
                ResolveExtensionSourceDirectory(),
                "manifest.json");
            if (File.Exists(manifestPath))
            {
                var manifest = JsonNode.Parse(File.ReadAllText(manifestPath))?.AsObject();
                if (manifest?["key"] is JsonValue key &&
                    key.GetValue<string>() is { Length: > 0 } keyText)
                {
                    var digest = SHA256.HashData(Convert.FromBase64String(keyText));
                    var mapped = new StringBuilder();
                    foreach (var digit in Convert.ToHexString(digest))
                    {
                        mapped.Append((char)('a' + byte.Parse(digit.ToString(), NumberStyles.HexNumber)));
                    }

                    var derived = mapped.ToString()[..32];
                    if (!origins.Contains($"chrome-extension://{derived}/"))
                    {
                        origins.Add($"chrome-extension://{derived}/");
                    }
                }
            }
        }
        catch (Exception error) when (error is IOException or FormatException or InvalidOperationException)
        {
            // No pinned key in the manifest — fall through to the exporter-key origin below.
        }

        // Exported CRX packages are signed with the per-machine key persisted at
        // LocalApplicationData/NxTools/browser_extension_key.pem. Chrome derives the
        // installed extension's ID from that signature, so a user who re-imports the
        // exported package on this machine gets an ID that differs from the pinned one.
        // Trust it as well, or the imported extension is locked out with
        // "Access to the specified native messaging host" errors.
        try
        {
            var keyPath = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "NxTools",
                "browser_extension_key.pem");
            if (File.Exists(keyPath))
            {
                using var rsa = System.Security.Cryptography.RSA.Create();
                rsa.ImportFromPem(File.ReadAllText(keyPath));
                var spki = rsa.ExportSubjectPublicKeyInfo();
                var digest = SHA256.HashData(spki);
                var mapped = new StringBuilder();
                foreach (var digit in Convert.ToHexString(digest))
                {
                    mapped.Append((char)('a' + byte.Parse(digit.ToString(), NumberStyles.HexNumber)));
                }

                var exportedOrigin = $"chrome-extension://{mapped.ToString()[..32]}/";
                if (!origins.Contains(exportedOrigin))
                {
                    origins.Add(exportedOrigin);
                }
            }
        }
        catch (Exception error) when (error is IOException or InvalidOperationException or FormatException)
        {
            // A missing or malformed manifest keeps the command-provided id.
        }
        return origins.ToArray();
    }

    private static string ResolveExtensionSourceDirectory()
    {
        var root = AppContext.BaseDirectory;
        var candidates = new[]
        {
            Path.Combine(root, "browser_extension", "nxtools-interceptor"),
            Path.Combine(root, "..", "browser_extension", "nxtools-interceptor"),
        };
        return candidates.FirstOrDefault(Directory.Exists) ?? candidates[0];
    }

    public static WindowsCommandResult Ensure(
        EnsureBrowserNativeMessagingCommand command,
        IPlatformLog log,
        Guid correlationId)
    {
        lock (RegistrationGate) return EnsureTransaction(command, log, correlationId);
    }

    private static WindowsCommandResult EnsureTransaction(
        EnsureBrowserNativeMessagingCommand command, IPlatformLog log, Guid correlationId)
    {
        var started = Stopwatch.GetTimestamp();
        WriteRegistrationLog(log, "info", "browser_native_messaging.registration_started", correlationId,
            new Dictionary<string, object?> { ["enabled"] = command.Enabled, ["browser_count"] = 3 });
        try
        {
            ValidateExtensionId(command.ChromiumExtensionId, ChromiumExtensionIdPattern());
            ValidateExtensionId(command.FirefoxExtensionId, FirefoxExtensionIdPattern());

            var manifestDirectory = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "NxTools",
                "NxTools.Next",
                "browser-native-host");
            var chromiumManifestPath = Path.Combine(manifestDirectory, "chromium-host.json");
            var firefoxManifestPath = Path.Combine(manifestDirectory, "firefox-host.json");
            if (!command.Enabled)
            {
                BrowserRegistrationTransaction.Apply(new WindowsBrowserRegistrationStore(),
                    BrowserKeys.Select(key => new BrowserRegistrationChange("key:" + key + HostName, null))
                        .Concat(new[] { new BrowserRegistrationChange("manifest:" + chromiumManifestPath, null),
                            new BrowserRegistrationChange("manifest:" + firefoxManifestPath, null) }).ToArray());
                WriteRegistrationLog(log, "info", "browser_native_messaging.registration_removed", correlationId,
                    new Dictionary<string, object?>
                    {
                        ["browser_count"] = 3,
                        ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                    });
                return new WindowsCommandResult(true);
            }

            var hostPath = Path.GetFullPath(Path.Combine(
                AppContext.BaseDirectory,
                "NxTools.BrowserHost.exe"));
            if (!File.Exists(hostPath))
            {
                throw new FileNotFoundException(
                    "Packaged browser native host was not found.",
                    hostPath);
            }
            using (var stream = File.OpenRead(hostPath))
            {
                var actualHash = Convert.ToHexString(SHA256.HashData(stream));
                if (string.IsNullOrEmpty(ReleaseBinaryManifest.BrowserHostSha256) ||
                    !actualHash.Equals(
                        ReleaseBinaryManifest.BrowserHostSha256,
                        StringComparison.OrdinalIgnoreCase))
                {
                    throw new InvalidDataException(
                        "Packaged browser native host failed the release integrity check.");
                }
            }

            // Migration: the pre-0.7 extension generation registered its host
            // under "com.nxtools.next"; machines upgraded from that era keep
            // stale registrations pointing at dead paths, and browsers report
            // "forbidden" on them. The current extension only speaks to
            // "com.nxtools.next.browser", so retire the legacy keys.

            Directory.CreateDirectory(manifestDirectory);
            var chromiumOrigins = ChromiumAllowedOrigins(command.ChromiumExtensionId);
            var chromiumManifest = JsonSerializer.Serialize(new
            {
                name = HostName,
                description = "NxTools.Next browser download bridge",
                path = hostPath,
                type = "stdio",
                allowed_origins = chromiumOrigins
            });
            var firefoxManifest = JsonSerializer.Serialize(new
            {
                name = HostName,
                description = "NxTools.Next browser download bridge",
                path = hostPath,
                type = "stdio",
                allowed_extensions = new[] { command.FirefoxExtensionId }
            });

            BrowserRegistrationTransaction.Apply(new WindowsBrowserRegistrationStore(), new[] {
                new BrowserRegistrationChange("manifest:" + chromiumManifestPath, chromiumManifest),
                new BrowserRegistrationChange("manifest:" + firefoxManifestPath, firefoxManifest),
                new BrowserRegistrationChange("key:" + BrowserKeys[0] + HostName, chromiumManifestPath),
                new BrowserRegistrationChange("key:" + BrowserKeys[1] + HostName, chromiumManifestPath),
                new BrowserRegistrationChange("key:" + BrowserKeys[2] + HostName, firefoxManifestPath)
            }.Concat(BrowserKeys.Select(key => new BrowserRegistrationChange("key:" + key + "com.nxtools.next", null))).ToArray());

            WriteRegistrationLog(log, "info", "browser_native_messaging.registration_completed", correlationId,
                new Dictionary<string, object?>
                {
                    ["browser_count"] = 3,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                });
            return new WindowsCommandResult(true, Data: JsonSerializer.Serialize(new
            {
                host_name = HostName,
                browser_count = 3
            }));
        }
        catch (Exception error)
        {
            WriteRegistrationLog(log, "error", "browser_native_messaging.registration_failed", correlationId,
                new Dictionary<string, object?>
                {
                    ["exception_type"] = error.GetType().Name,
                    ["native_error_code"] = error.HResult,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                });
            return new WindowsCommandResult(false, error.Message);
        }
    }


    private static void WriteRegistrationLog(IPlatformLog log, string level, string eventName,
        Guid correlationId, IReadOnlyDictionary<string, object?> fields)
    {
        try { log.Write(level, eventName, correlationId, fields); }
        catch { /* Observing a transaction must not change its outcome. */ }
    }

    private static void ValidateExtensionId(string value, Regex pattern)
    {
        if (!pattern.IsMatch(value))
        {
            throw new InvalidDataException("Browser extension identity is invalid.");
        }
    }

    [GeneratedRegex("^[a-p]{32}$", RegexOptions.CultureInvariant)]
    private static partial Regex ChromiumExtensionIdPattern();

    [GeneratedRegex("^[A-Za-z0-9._@-]{3,128}$", RegexOptions.CultureInvariant)]
    private static partial Regex FirefoxExtensionIdPattern();
}
