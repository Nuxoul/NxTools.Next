using NxTools.PlatformHost.Windows;
namespace NxTools.PlatformHost.Tests;

// Pure parser fixtures: no registry/native system proxy reads, external network or credentials.
internal static class SystemProxySnapshotTests
{
    internal static Task StaticAutomaticAndInvalidConfigurations()
    {
        var direct = SystemProxySnapshotReader.Parse(null, false, false);
        Assert(direct.Endpoints.Length == 0 && !direct.AutoDetect && !direct.PacConfigured, "direct config");
        var single = SystemProxySnapshotReader.Parse("127.0.0.1:7890", false, false);
        Assert(single.Endpoints.Single() == new SystemProxyEndpoint("all", "127.0.0.1", 7890), "single endpoint");
        var split = SystemProxySnapshotReader.Parse("http=proxy.test:8080;https=[::1]:8443", true, true);
        Assert(split.AutoDetect && split.PacConfigured && split.Endpoints.Length == 2, "automatic flags separate from static endpoints");
        Assert(split.Endpoints[1] == new SystemProxyEndpoint("https", "::1", 8443), "IPv6 endpoint preserved");
        foreach (var invalid in new[] { "user:secret@proxy.test:80", "proxy.test:0", "proxy.test:70000", "http://proxy.test/path", new string('a', 8193) })
        {
            try { _ = SystemProxySnapshotReader.Parse(invalid, false, false); }
            catch (InvalidDataException) { continue; }
            throw new InvalidOperationException("invalid proxy config admitted");
        }
        return Task.CompletedTask;
    }
    private static void Assert(bool value, string reason) { if (!value) throw new InvalidOperationException(reason); }
}
