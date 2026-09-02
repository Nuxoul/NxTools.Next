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
        var started = Stopwatch.GetTimestamp();
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
                RemoveRegistryManifest($@"Software\Google\Chrome\NativeMessagingHosts\{HostName}");
                RemoveRegistryManifest($@"Software\Microsoft\Edge\NativeMessagingHosts\{HostName}");
                RemoveRegistryManifest($@"Software\Mozilla\NativeMessagingHosts\{HostName}");
                DeleteManifest(chromiumManifestPath);
                DeleteManifest(firefoxManifestPath);
                log.Write("info", "browser_native_messaging.registration_removed", correlationId,
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
            RemoveRegistryManifest(@"Software\Google\Chrome\NativeMessagingHosts\com.nxtools.next");
            RemoveRegistryManifest(@"Software\Microsoft\Edge\NativeMessagingHosts\com.nxtools.next");
            RemoveRegistryManifest(@"Software\Mozilla\NativeMessagingHosts\com.nxtools.next");

            Directory.CreateDirectory(manifestDirectory);
            var chromiumOrigins = ChromiumAllowedOrigins(command.ChromiumExtensionId);
            WriteManifestAtomically(chromiumManifestPath, new
            {
                name = HostName,
                description = "NxTools.Next browser download bridge",
                path = hostPath,
                type = "stdio",
                allowed_origins = chromiumOrigins
            });
            WriteManifestAtomically(firefoxManifestPath, new
            {
                name = HostName,
                description = "NxTools.Next browser download bridge",
                path = hostPath,
                type = "stdio",
                allowed_extensions = new[] { command.FirefoxExtensionId }
            });

            WriteRegistryManifest(
                $@"Software\Google\Chrome\NativeMessagingHosts\{HostName}",
                chromiumManifestPath);
            WriteRegistryManifest(
                $@"Software\Microsoft\Edge\NativeMessagingHosts\{HostName}",
                chromiumManifestPath);
            WriteRegistryManifest(
                $@"Software\Mozilla\NativeMessagingHosts\{HostName}",
                firefoxManifestPath);

            log.Write("info", "browser_native_messaging.registration_completed", correlationId,
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
            log.Write("error", "browser_native_messaging.registration_failed", correlationId,
                new Dictionary<string, object?>
                {
                    ["exception_type"] = error.GetType().Name,
                    ["native_error_code"] = error.HResult,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                });
            return new WindowsCommandResult(false, error.Message);
        }
    }

    private static void WriteManifestAtomically(string path, object manifest)
    {
        var temporaryPath = $"{path}.{Guid.NewGuid():N}.tmp";
        try
        {
            File.WriteAllText(temporaryPath, JsonSerializer.Serialize(manifest));
            File.Move(temporaryPath, path, true);
        }
        finally
        {
            if (File.Exists(temporaryPath))
            {
                File.Delete(temporaryPath);
            }
        }
    }

    private static void WriteRegistryManifest(string keyPath, string manifestPath)
    {
        using var key = Registry.CurrentUser.CreateSubKey(keyPath, true)
            ?? throw new InvalidOperationException($"Unable to create native messaging key '{keyPath}'.");
        key.SetValue(null, manifestPath, RegistryValueKind.String);
    }

    private static void RemoveRegistryManifest(string keyPath) =>
        Registry.CurrentUser.DeleteSubKeyTree(keyPath, false);

    private static void DeleteManifest(string path)
    {
        if (File.Exists(path))
        {
            File.Delete(path);
        }
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
