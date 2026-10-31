using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

internal sealed record SystemProxyEndpoint(string Protocol, string Host, ushort Port);
internal sealed record SystemProxySnapshot(bool AutoDetect, bool PacConfigured, SystemProxyEndpoint[] Endpoints);

internal static class SystemProxySnapshotReader
{
    internal static WindowsCommandResult Read(Guid correlation, IPlatformLog log)
    {
        var started = System.Diagnostics.Stopwatch.GetTimestamp();
        SafeLog(log, "info", "system.proxy.snapshot.start", correlation, new Dictionary<string, object?> { ["stage"] = "native_read" });
        NativeConfiguration config = default;
        WindowsCommandResult result;
        try
        {
            if (!WinHttpGetIEProxyConfigForCurrentUser(out config))
                throw new Win32Exception(Marshal.GetLastWin32Error());
            var snapshot = Parse(Marshal.PtrToStringUni(config.Proxy), config.AutoDetect, config.AutoConfigUrl != IntPtr.Zero);
            result = new(true, Data: JsonSerializer.Serialize(snapshot, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower }));
        }
        catch (Exception failure) when (failure is Win32Exception or InvalidDataException)
        {
            result = new(false, "system_proxy_read_failed", NativeErrorCode: failure is Win32Exception native ? native.NativeErrorCode : 13);
        }
        finally
        {
            if (config.Proxy != IntPtr.Zero) _ = GlobalFree(config.Proxy);
            if (config.ProxyBypass != IntPtr.Zero) _ = GlobalFree(config.ProxyBypass);
            if (config.AutoConfigUrl != IntPtr.Zero) _ = GlobalFree(config.AutoConfigUrl);
        }
        SafeLog(log, result.Success ? "info" : "warning", "system.proxy.snapshot.terminal", correlation,
            new Dictionary<string, object?> { ["success"] = result.Success, ["native_error_code"] = result.NativeErrorCode,
                ["elapsed_ms"] = System.Diagnostics.Stopwatch.GetElapsedTime(started).TotalMilliseconds });
        return result;
    }

    private static void SafeLog(IPlatformLog log, string level, string name, Guid correlation, IReadOnlyDictionary<string, object?> fields)
    {
        try { log.Write(level, name, correlation, fields); } catch { /* observation cannot change result */ }
    }

    internal static SystemProxySnapshot Parse(string? proxy, bool autoDetect, bool pacConfigured)
    {
        if (proxy?.Length > 8192) throw new InvalidDataException("proxy_configuration_too_large");
        var endpoints = new List<SystemProxyEndpoint>();
        foreach (var raw in (proxy ?? string.Empty).Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            if (endpoints.Count >= 8) throw new InvalidDataException("too_many_proxy_endpoints");
            var pair = raw.Split('=', 2, StringSplitOptions.TrimEntries);
            var protocol = pair.Length == 2 ? pair[0].ToLowerInvariant() : "all";
            var address = pair[^1];
            if (protocol is not ("all" or "http" or "https" or "socks")) throw new InvalidDataException("unsupported_proxy_endpoint");
            if (!Uri.TryCreate(address.Contains("://", StringComparison.Ordinal) ? address : "http://" + address, UriKind.Absolute, out var uri)
                || uri.Scheme is not ("http" or "https" or "socks" or "socks4" or "socks5")
                || uri.UserInfo.Length != 0 || uri.AbsolutePath != "/" || uri.Query.Length != 0 || uri.Fragment.Length != 0
                || uri.Host.Length is < 1 or > 253 || uri.Port is < 1 or > 65535)
                throw new InvalidDataException("invalid_proxy_endpoint");
            endpoints.Add(new(protocol, uri.Host.Trim('[', ']'), checked((ushort)uri.Port)));
        }
        return new(autoDetect, pacConfigured, endpoints.ToArray());
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeConfiguration
    {
        [MarshalAs(UnmanagedType.Bool)] public bool AutoDetect;
        public IntPtr AutoConfigUrl;
        public IntPtr Proxy;
        public IntPtr ProxyBypass;
    }
    [DllImport("winhttp.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool WinHttpGetIEProxyConfigForCurrentUser(out NativeConfiguration configuration);
    [DllImport("kernel32.dll")] private static extern IntPtr GlobalFree(IntPtr memory);
}
