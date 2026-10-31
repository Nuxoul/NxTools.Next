using System.Diagnostics;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Text;
using System.Xml;
using System.Xml.Linq;

namespace NxTools.PlatformHost.Windows;

internal sealed record DirectoryOpusObservationHost(IntPtr Root, int ProcessId, long StartedAt, string Helper);

internal interface IDirectoryOpusTargetPlatform
{
    IReadOnlyList<DirectoryOpusObservationHost> CaptureHosts();
    bool IsCurrent(DirectoryOpusObservationHost host, IntPtr tab);
    string? Export(DirectoryOpusObservationHost host, string command);
}

// Official DOpusRT /info paths + listsel,<exact tab> only. Never open/select/Go commands.
// Reuses the shared target matcher; a helper exit code or matching folder alone is NOT success.
internal sealed class DirectoryOpusTargetObservation(
    string target, bool isDirectory, IDirectoryOpusTargetPlatform platform,
    Action<string>? stageChanged = null, Action<int>? nativeFailure = null) : IExplorerTargetObservationPlatform
{
    internal const int MaxXmlSize = 2 * 1024 * 1024;
    private sealed record TabState(IntPtr Root, IntPtr Tab, string Directory);

    public IEnumerable<ExplorerTargetView> ReadActiveViews()
    {
        var views = new List<ExplorerTargetView>();
        try
        {
            var fullTarget = ExplorerShellActivation.NormalizeDirectory(target);
            var parent = Path.GetDirectoryName(fullTarget);
            var revealRoot = isDirectory && string.IsNullOrEmpty(parent);
            var directory = revealRoot ? fullTarget : parent!;
            var hosts = platform.CaptureHosts();
            if (hosts.Count == 0) return views;
            Report("observation_dopus_hosts_present");
            if (hosts.Count > 256) return views;
            var candidates = 0;
            // /info paths enumerates all listers in one running DOpus instance.
            foreach (var instance in hosts.GroupBy(host => (host.ProcessId, host.StartedAt, host.Helper)))
            {
                var firstHost = instance.First();
                var tabs = ReadPaths(platform.Export(firstHost, "paths"));
                if (tabs is null) { Report("observation_dopus_paths_unavailable"); continue; }
                foreach (var tab in tabs)
                {
                    var host = instance.SingleOrDefault(value => value.Root == tab.Root);
                    if (host is null || !SamePath(tab.Directory, directory) || !platform.IsCurrent(host, tab.Tab)) continue;
                    if (++candidates > 4) return views;
                    Report("observation_dopus_active_tab_matched");
                    var command = "listsel,0x" + tab.Tab.ToInt64().ToString("x", CultureInfo.InvariantCulture);
                    var selected = ReadSelection(platform.Export(host, command), command, tab);
                    if (selected is null) { Report("observation_dopus_selection_unavailable"); continue; }
                    if (!revealRoot && !selected.Any(path => SamePath(path, fullTarget)))
                    { Report("observation_dopus_target_not_selected"); continue; }
                    // A tab switch/navigation during export must not reuse the old selection.
                    var current = ReadPaths(platform.Export(host, "paths"));
                    if (current is null || !current.Any(value => value.Root == tab.Root && value.Tab == tab.Tab &&
                        SamePath(value.Directory, tab.Directory)) || !platform.IsCurrent(host, tab.Tab))
                    { Report("observation_dopus_tab_changed"); continue; }
                    Report("observation_dopus_exact_target_ready");
                    views.Add(new(tab.Root, tab.Tab, true, tab.Directory, selected));
                    return views;
                }
            }
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or XmlException or
            ArgumentException or InvalidOperationException or System.ComponentModel.Win32Exception or NotSupportedException)
        {
            Report("observation_dopus_read_failed");
            try { nativeFailure?.Invoke(error.HResult); }
            catch { /* Diagnostics do not determine observation. */ }
        }
        return views;
    }

    private void Report(string stage)
    {
        try { stageChanged?.Invoke(stage); }
        catch { /* A diagnostic sink cannot change the result. */ }
    }

    private static bool SamePath(string left, string right) => string.Equals(
        ExplorerShellActivation.NormalizeDirectory(left), ExplorerShellActivation.NormalizeDirectory(right), StringComparison.OrdinalIgnoreCase);

    private static XElement? ReadDocument(string? xml, string command)
    {
        if (string.IsNullOrEmpty(xml) || xml.Length > MaxXmlSize) return null;
        using var input = new StringReader(xml);
        using var reader = XmlReader.Create(input, new XmlReaderSettings
        {
            DtdProcessing = DtdProcessing.Prohibit,
            XmlResolver = null,
            MaxCharactersInDocument = MaxXmlSize,
            MaxCharactersFromEntities = 0,
        });
        var root = XDocument.Load(reader).Root;
        return root?.Name == "results" && (string?)root.Attribute("result") == "1" &&
            (string?)root.Attribute("command") == command ? root : null;
    }

    private static List<TabState>? ReadPaths(string? xml)
    {
        var root = ReadDocument(xml, "paths");
        if (root is null) return null;
        var elements = root.Elements().Take(257).ToArray();
        if (elements.Length > 256 || elements.Any(element => element.Name != "path")) return null;
        var tabs = new List<TabState>();
        var seen = new HashSet<IntPtr>();
        foreach (var element in elements)
        {
            if (!TryHandle((string?)element.Attribute("lister"), out var lister) ||
                !TryHandle((string?)element.Attribute("tab"), out var tab) || !seen.Add(tab)) return null;
            if ((string?)element.Attribute("active_tab") != "1" || (string?)element.Attribute("tab_state") != "1") continue;
            var path = element.Value;
            if (element.HasElements || !Path.IsPathFullyQualified(path) || path.Length > 32767) return null;
            tabs.Add(new(lister, tab, path));
        }
        return tabs;
    }

    private static List<string>? ReadSelection(string? xml, string command, TabState tab)
    {
        var root = ReadDocument(xml, command);
        if (root is null) return null;
        var groups = root.Elements().Take(2).ToArray();
        if (groups.Length != 1 || groups[0].Name != "items") return null;
        var items = groups[0];
        var directory = (string?)items.Attribute("path");
        if (!TryHandle((string?)items.Attribute("lister"), out var lister) || lister != tab.Root ||
            !TryHandle((string?)items.Attribute("tab"), out var currentTab) || currentTab != tab.Tab ||
            string.IsNullOrEmpty(directory) || directory.Length > 32767 || !Path.IsPathFullyQualified(directory) || !SamePath(directory, tab.Directory)) return null;
        var elements = items.Elements().Take(4097).ToArray();
        if (elements.Length > 4096) return null;
        var selected = new List<string>(elements.Length);
        foreach (var element in elements)
        {
            var path = (string?)element.Attribute("path");
            if (element.Name != "item" || element.HasElements || string.IsNullOrEmpty(path) ||
                path.Length > 32767 || !Path.IsPathFullyQualified(path)) return null;
            selected.Add(path);
        }
        return selected;
    }

    private static bool TryHandle(string? value, out IntPtr handle)
    {
        handle = IntPtr.Zero;
        if (value is null || !value.StartsWith("0x", StringComparison.Ordinal) ||
            !long.TryParse(value.AsSpan(2), NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture, out var number) || number <= 0) return false;
        handle = new IntPtr(number);
        return true;
    }
}

// Runs on the existing detached Shell STA, never on the serial pipe reader/event thread.
internal sealed class NativeDirectoryOpusTargetPlatform(Action<string>? stageChanged = null) : IDirectoryOpusTargetPlatform
{
    private readonly Stopwatch _budget = Stopwatch.StartNew();
    private const int TotalBudgetMs = 1500;
    private const int ExportBudgetMs = 250;

    public IReadOnlyList<DirectoryOpusObservationHost> CaptureHosts()
    {
        var hosts = new List<DirectoryOpusObservationHost>();
        EnumWindows((window, _) =>
        {
            if (hosts.Count >= 256 || _budget.ElapsedMilliseconds >= TotalBudgetMs) return false;
            if (!IsWindowVisible(window) || !Class(window).Equals("dopus.lister", StringComparison.OrdinalIgnoreCase)) return true;
            try
            {
                GetWindowThreadProcessId(window, out var processId);
                using var process = Process.GetProcessById(checked((int)processId));
                if (!process.ProcessName.Equals("dopus", StringComparison.OrdinalIgnoreCase) || process.SessionId != Process.GetCurrentProcess().SessionId) return true;
                var image = process.MainModule?.FileName;
                if (image is null || !Path.GetFileName(image).Equals("dopus.exe", StringComparison.OrdinalIgnoreCase)) return true;
                var helper = Path.Combine(Path.GetDirectoryName(image)!, "dopusrt.exe");
                if (!File.Exists(helper) || (File.GetAttributes(helper) & FileAttributes.ReparsePoint) != 0) return true;
                hosts.Add(new(window, process.Id, process.StartTime.ToUniversalTime().Ticks, helper));
            }
            catch (Exception error) when (error is System.ComponentModel.Win32Exception or InvalidOperationException or
                ArgumentException or IOException or UnauthorizedAccessException) { Report("observation_dopus_host_identity_failed"); }
            return true;
        }, IntPtr.Zero);
        return hosts;
    }

    public bool IsCurrent(DirectoryOpusObservationHost host, IntPtr tab)
    {
        if (_budget.ElapsedMilliseconds >= TotalBudgetMs || !IsWindowVisible(host.Root) ||
            !Class(host.Root).Equals("dopus.lister", StringComparison.OrdinalIgnoreCase) || !IsWindowVisible(tab) ||
            !Class(tab).Equals("dopus.filedisplaycontainer", StringComparison.OrdinalIgnoreCase) || !IsChild(host.Root, tab)) return false;
        GetWindowThreadProcessId(host.Root, out var rootProcess);
        GetWindowThreadProcessId(tab, out var tabProcess);
        if (rootProcess != host.ProcessId || tabProcess != rootProcess) return false;
        using var process = Process.GetProcessById(host.ProcessId);
        return !process.HasExited && process.StartTime.ToUniversalTime().Ticks == host.StartedAt &&
            process.SessionId == Process.GetCurrentProcess().SessionId &&
            string.Equals(Path.Combine(Path.GetDirectoryName(process.MainModule!.FileName)!, "dopusrt.exe"), host.Helper, StringComparison.OrdinalIgnoreCase);
    }

    public string? Export(DirectoryOpusObservationHost host, string command)
    {
        var remaining = Math.Min(ExportBudgetMs, TotalBudgetMs - (int)_budget.ElapsedMilliseconds);
        if (remaining <= 0 || (command != "paths" && (!command.StartsWith("listsel,0x", StringComparison.Ordinal) ||
            !long.TryParse(command.AsSpan(10), NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture, out var tab) || tab <= 0))) return null;
        var temporary = Path.Combine(Path.GetTempPath(), "nxtools-dopus-observe-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(temporary);
        var output = Path.Combine(temporary, "result.xml");
        var clock = Stopwatch.StartNew();
        try
        {
            var info = new ProcessStartInfo(host.Helper) { UseShellExecute = false, CreateNoWindow = true };
            info.ArgumentList.Add("/info");
            info.ArgumentList.Add(output + "," + command);
            using var process = Process.Start(info);
            if (process is null) return null;
            if (!process.WaitForExit(remaining))
            {
                // This is exactly our helper child, never the user's DOpus process.
                process.Kill();
                var cleanupBudget = Math.Clamp(TotalBudgetMs - (int)_budget.ElapsedMilliseconds, 0, 25);
                if (!process.WaitForExit(cleanupBudget)) Report("observation_dopus_helper_exit_unconfirmed");
                Report("observation_dopus_helper_timeout");
                return null;
            }
            if (process.ExitCode != 0) { Report("observation_dopus_helper_failed"); return null; }
            // DOpusRT exits before DOpus writes the response. Fresh GUID path prevents stale evidence.
            while (clock.ElapsedMilliseconds < remaining)
            {
                if (File.Exists(output))
                {
                    try
                    {
                        using var stream = new FileStream(output, FileMode.Open, FileAccess.Read, FileShare.Read);
                        if (stream.Length > DirectoryOpusTargetObservation.MaxXmlSize) { Report("observation_dopus_output_budget"); return null; }
                        using var reader = new StreamReader(stream, new UTF8Encoding(false, true), true);
                        return reader.ReadToEnd();
                    }
                    catch (IOException) { /* A still-writing producer cannot provide evidence yet. */ }
                }
                Thread.Sleep(5);
            }
            Report("observation_dopus_output_timeout");
            return null;
        }
        finally
        {
            try { File.Delete(output); Directory.Delete(temporary); }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException)
            { Report("observation_dopus_cleanup_failed"); }
        }
    }

    private void Report(string stage)
    {
        try { stageChanged?.Invoke(stage); }
        catch { /* Diagnostics cannot alter behavior. */ }
    }
    private static string Class(IntPtr window)
    {
        var text = new StringBuilder(256);
        GetClassNameW(window, text, text.Capacity);
        return text.ToString();
    }
    private delegate bool WindowCallback(IntPtr window, IntPtr context);
    [DllImport("user32.dll")] private static extern bool EnumWindows(WindowCallback callback, IntPtr context);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] private static extern bool IsChild(IntPtr parent, IntPtr child);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, ExactSpelling = true)] private static extern int GetClassNameW(IntPtr window, StringBuilder text, int capacity);
}
