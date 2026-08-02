using System.Diagnostics;
using System.Text.Json;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

internal interface INativeMessagingRegistry
{
    void SetCurrentUserDefaultValue(string subKey, string value);
}

internal sealed class WindowsNativeMessagingRegistry : INativeMessagingRegistry
{
    public void SetCurrentUserDefaultValue(string subKey, string value)
    {
        using var key = Registry.CurrentUser.CreateSubKey(subKey, writable: true)
            ?? throw new InvalidOperationException($"Cannot create registry key HKCU\\{subKey}.");
        key.SetValue(null, value, RegistryValueKind.String);
    }
}

internal static class BrowserNativeMessagingRegistration
{
    internal const string HostName = "com.nxtools.next.browser";
    internal const string ChromiumExtensionId = "ajcjlgdnfcagknjcajhbiikagimgdnaf";
    internal const string FirefoxExtensionId = "nxtools-download-interceptor@nxtools.local";
    internal static readonly string[] ChromiumRegistryKeys =
    [
        $@"Software\Google\Chrome\NativeMessagingHosts\{HostName}",
        $@"Software\Microsoft\Edge\NativeMessagingHosts\{HostName}"
    ];
    internal static readonly string FirefoxRegistryKey =
        $@"Software\Mozilla\NativeMessagingHosts\{HostName}";

    public static bool RegisterForCurrentPackage(IPlatformLog log)
    {
        var correlationId = Guid.NewGuid();
        var startedAt = Stopwatch.GetTimestamp();
        log.Write("info", "browser_native_messaging.registration_started", correlationId);
        try
        {
            var platformHostPath = Environment.ProcessPath
                ?? throw new InvalidOperationException("Cannot locate NxTools.PlatformHost.exe.");
            var packageDirectory = Path.GetDirectoryName(platformHostPath)
                ?? throw new InvalidOperationException("Cannot locate the package directory.");
            var nativeHostPath = Path.GetFullPath(Path.Combine(packageDirectory, "NxTools.Cli.exe"));
            if (!File.Exists(nativeHostPath))
            {
                throw new FileNotFoundException(
                    "NxTools.Cli.exe is not packaged beside NxTools.PlatformHost.exe.",
                    nativeHostPath);
            }
            var localData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            var manifestDirectory = Path.Combine(
                localData,
                "NxTools",
                "NxTools.Next",
                "native-messaging");
            Register(nativeHostPath, manifestDirectory, new WindowsNativeMessagingRegistry());
            log.Write("info", "browser_native_messaging.registration_completed", correlationId,
                new Dictionary<string, object?>
                {
                    ["status"] = "success",
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(startedAt).TotalMilliseconds,
                    ["browser_count"] = 3
                });
            return true;
        }
        catch (Exception error)
        {
            log.Write("error", "browser_native_messaging.registration_completed", correlationId,
                new Dictionary<string, object?>
                {
                    ["status"] = "failed",
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(startedAt).TotalMilliseconds,
                    ["native_error_code"] = error.HResult,
                    ["exception_type"] = error.GetType().FullName,
                    ["message"] = error.Message
                });
            return false;
        }
    }

    internal static void Register(
        string nativeHostPath,
        string manifestDirectory,
        INativeMessagingRegistry registry)
    {
        var resolvedHost = Path.GetFullPath(nativeHostPath);
        if (!Path.GetFileName(resolvedHost).Equals("NxTools.Cli.exe", StringComparison.OrdinalIgnoreCase) ||
            !File.Exists(resolvedHost))
        {
            throw new InvalidOperationException("Native Messaging must use the packaged NxTools.Cli.exe.");
        }
        Directory.CreateDirectory(manifestDirectory);
        var chromiumManifestPath = Path.GetFullPath(Path.Combine(manifestDirectory, "chromium-native-host.json"));
        var firefoxManifestPath = Path.GetFullPath(Path.Combine(manifestDirectory, "firefox-native-host.json"));
        WriteAtomic(chromiumManifestPath, BuildChromiumManifest(resolvedHost));
        WriteAtomic(firefoxManifestPath, BuildFirefoxManifest(resolvedHost));
        foreach (var registryKey in ChromiumRegistryKeys)
        {
            registry.SetCurrentUserDefaultValue(registryKey, chromiumManifestPath);
        }
        registry.SetCurrentUserDefaultValue(FirefoxRegistryKey, firefoxManifestPath);
    }

    internal static string BuildChromiumManifest(string nativeHostPath) =>
        JsonSerializer.Serialize(new
        {
            name = HostName,
            description = "NxTools.Next browser download integration",
            path = Path.GetFullPath(nativeHostPath),
            type = "stdio",
            allowed_origins = new[] { $"chrome-extension://{ChromiumExtensionId}/" }
        }, new JsonSerializerOptions { WriteIndented = true });

    internal static string BuildFirefoxManifest(string nativeHostPath) =>
        JsonSerializer.Serialize(new
        {
            name = HostName,
            description = "NxTools.Next browser download integration",
            path = Path.GetFullPath(nativeHostPath),
            type = "stdio",
            allowed_extensions = new[] { FirefoxExtensionId }
        }, new JsonSerializerOptions { WriteIndented = true });

    private static void WriteAtomic(string path, string content)
    {
        var temporary = $"{path}.{Guid.NewGuid():N}.tmp";
        File.WriteAllText(temporary, content);
        File.Move(temporary, path, overwrite: true);
    }
}
