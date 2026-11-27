// Owned Debug fixture relay only. No PlatformHost main, service, registry or formal IPC.
using System.Diagnostics;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.Loader;
using System.Text;
using System.Text.Json;

internal static class Program
{
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(nint window, out uint process);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(nint window, StringBuilder text, int count);
    [DllImport("user32.dll")] private static extern nint GetForegroundWindow();
    [DllImport("user32.dll")] private static extern nint GetDC(nint window);
    [DllImport("user32.dll")] private static extern int ReleaseDC(nint window, nint dc);
    [DllImport("gdi32.dll")] private static extern int GetDeviceCaps(nint dc, int index);

    private static void Owned(long handle, int process, string title)
    {
        GetWindowThreadProcessId((nint)handle, out var actual);
        var text = new StringBuilder(200);
        GetWindowText((nint)handle, text, 200);
        if (actual != process || text.ToString() != title)
            throw new InvalidOperationException("Unowned fixture HWND rejected.");
    }

    private static int Main(string[] args)
    {
        if (args.Length != 6 || args[0] != "--authorize-owned-native")
            throw new ArgumentException("Explicit owned fixture authorization required.");
        var exe = Path.GetFullPath(args[1]);
        var assemblyPath = Path.GetFullPath(args[2]);
        var directory = Path.GetFullPath(args[3]);
        var mode = args[4];
        var theme = args[5];
        var scratch = Path.GetFullPath(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new Exception("Scratch required."));
        if (!directory.StartsWith(scratch + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase)
            || !exe.EndsWith(@"target\debug\nxtools-desktop.exe", StringComparison.OrdinalIgnoreCase)
            || mode is not ("legacy" or "current" or "reduced") || theme is not ("light" or "dark"))
            throw new Exception("Fixture roots/mode rejected.");
        Directory.CreateDirectory(directory);
        AssemblyLoadContext.Default.Resolving += (_, name) =>
        {
            var path = Path.Combine(Path.GetDirectoryName(assemblyPath)!, name.Name + ".dll");
            return File.Exists(path) ? AssemblyLoadContext.Default.LoadFromAssemblyPath(path) : null;
        };
        var assembly = AssemblyLoadContext.Default.LoadFromAssemblyPath(assemblyPath);
        var adapter = assembly.GetType("NxTools.PlatformHost.Windows.WindowsPlatformAdapter", true)!;
        var commandType = assembly.GetType("NxTools.PlatformHost.Protocol.PrepareNotificationWindowCommand", true)!;
        var prepare = adapter.GetMethod("PrepareNotificationWindowForTest", BindingFlags.NonPublic | BindingFlags.Static)!;
        var visibility = adapter.GetMethod("SetNativeWindowVisibilityForTest", BindingFlags.NonPublic | BindingFlags.Static)!;
        var start = new ProcessStartInfo(exe, "--ui-preview foundation")
        {
            UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true,
            WorkingDirectory = Path.GetDirectoryName(exe)!
        };
        start.Environment["NXTOOLS_PREVIEW_SCREEN_NOTIFICATION_DIR"] = directory;
        start.Environment["NXTOOLS_PREVIEW_SCREEN_NOTIFICATION_POSITION"] = "top_right";
        start.Environment["NXTOOLS_PREVIEW_SCREEN_NOTIFICATION_EXIT"] = "1";
        start.Environment["NXTOOLS_PREVIEW_NOTIFICATION_CADENCE"] = "1";
        start.Environment["NXTOOLS_PREVIEW_NOTIFICATION_LEGACY_CAP"] = mode == "legacy" ? "1" : "0";
        start.Environment["NXTOOLS_PREVIEW_NOTIFICATION_REDUCED"] = mode == "reduced" ? "1" : "0";
        start.Environment["NXTOOLS_PREVIEW_THEME"] = theme;
        var materialType = assembly.GetType("NxTools.PlatformHost.Windows.WindowMaterialOwner", true)!;
        var materialOwner = Activator.CreateInstance(materialType,
            BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic, null, new object?[] { null, null }, null)!;
        var readerType = assembly.GetType("NxTools.PlatformHost.Windows.WindowsSystemMaterialReader", true)!;
        var reader = Activator.CreateInstance(readerType, true)!;
        var materialCommandType = assembly.GetType("NxTools.PlatformHost.Protocol.ApplyWindowMaterialCommand", true)!;
        var applyMaterial = adapter.GetMethod("ApplyWindowMaterialCoreAsync", BindingFlags.NonPublic | BindingFlags.Static)!;
        using var child = Process.Start(start) ?? throw new Exception("Fixture start failed.");
        var stdout = new StringBuilder();
        var stderr = new StringBuilder();
        child.OutputDataReceived += (_, e) => { if (e.Data != null) lock (stdout) stdout.AppendLine(e.Data); };
        child.ErrorDataReceived += (_, e) => { if (e.Data != null) lock (stderr) stderr.AppendLine(e.Data); };
        child.BeginOutputReadLine(); child.BeginErrorReadLine();
        var processed = new HashSet<string>();
        var operations = new List<object>();
        var clock = Stopwatch.StartNew();
        TimeSpan? revealed = null, hidden = null;
        bool dismissed = false, finish = false;
        try
        {
            materialType.GetMethod("BindPeer")!.Invoke(materialOwner, new object[] { (uint)child.Id });
            var materialFacts = readerType.GetMethod("Read")!.Invoke(reader, null)!;
            var snapshot = materialType.GetMethod("UpdateCapabilities")!.Invoke(materialOwner, new[] { materialFacts })!;
            File.WriteAllText(Path.Combine(directory, "material.facts.json"),
                JsonSerializer.Serialize(snapshot, snapshot.GetType(), new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower }));
            while (clock.Elapsed < TimeSpan.FromSeconds(18) && !child.HasExited)
            {
                foreach (var path in Directory.GetFiles(directory, "*.command.json"))
                {
                    if (!processed.Add(path)) continue;
                    using var json = JsonDocument.Parse(File.ReadAllText(path));
                    var root = json.RootElement;
                    var payload = root.GetProperty("payload");
                    var kind = payload.GetProperty("command").GetString();
                    var data = payload.GetProperty("data");
                    var hwnd = data.GetProperty("window_handle").GetInt64();
                    Owned(hwnd, child.Id, "NxTools.Notification");
                    var foreground = GetForegroundWindow();
                    object result;
                    if (kind == "prepare_notification_window")
                    {
                        var source = data.GetProperty("source_window_handle").GetInt64();
                        Owned(source, child.Id, "NxTools.ScreenNotificationFixture");
                        if (data.GetProperty("source_process_id").GetUInt32() != child.Id) throw new Exception("Foreign source PID.");
                        var command = Activator.CreateInstance(commandType, hwnd, source, (uint)child.Id,
                            data.GetProperty("position").GetString()!, data.GetProperty("width").GetInt32(),
                            data.GetProperty("height").GetInt32(), data.TryGetProperty("keep_visible", out var keep) && keep.GetBoolean())!;
                        result = prepare.Invoke(null, new[] { command })!;
                    }
                    else if (kind == "apply_window_material")
                    {
                        if (data.GetProperty("surface").GetString() != "transparent") throw new Exception("Ordinary material target rejected.");
                        var command = Activator.CreateInstance(materialCommandType,
                            data.GetProperty("session_id").GetGuid(), data.GetProperty("facts_revision").GetUInt64(),
                            hwnd, data.GetProperty("generation").GetUInt64(), data.GetProperty("style").GetString()!,
                            "transparent", data.GetProperty("dark").GetBoolean())!;
                        var pending = (Task)applyMaterial.Invoke(null, new object[] {
                            materialOwner, command, root.GetProperty("id").GetGuid(), CancellationToken.None })!;
                        pending.GetAwaiter().GetResult();
                        result = pending.GetType().GetProperty("Result")!.GetValue(pending)!;
                    }
                    else if (kind == "set_window_visibility")
                    {
                        var visible = data.GetProperty("visible").GetBoolean();
                        result = visibility.Invoke(null, new object[] { hwnd, visible })!;
                        if (visible) revealed = clock.Elapsed; else hidden = clock.Elapsed;
                    }
                    else throw new Exception("Unexpected command rejected.");
                    var type = result.GetType();
                    var success = (bool)type.GetProperty("Success")!.GetValue(result)!;
                    if (!success || GetForegroundWindow() != foreground) throw new Exception("Native command failed or changed foreground.");
                    var reply = new
                    {
                        success, error = (string?)type.GetProperty("Error")!.GetValue(result),
                        data = (string?)type.GetProperty("Data")!.GetValue(result),
                        native_error_code = (int?)type.GetProperty("NativeErrorCode")!.GetValue(result)
                    };
                    File.WriteAllText(Path.Combine(directory, root.GetProperty("id").GetString() + ".reply.json"), JsonSerializer.Serialize(reply));
                    var dc = GetDC((nint)hwnd);
                    var refreshHz = GetDeviceCaps(dc, 116);
                    ReleaseDC((nint)hwnd, dc);
                    operations.Add(new { kind, elapsed_ms = clock.Elapsed.TotalMilliseconds, success, foreground_unchanged = true, readback = reply.data, refresh_hz = refreshHz });
                }
                if (revealed.HasValue && !dismissed && clock.Elapsed - revealed.Value > TimeSpan.FromSeconds(2))
                {
                    File.WriteAllText(Path.Combine(directory, "dismiss.request"), "dismiss");
                    operations.Add(new { kind = "dismiss_requested", elapsed_ms = clock.Elapsed.TotalMilliseconds });
                    dismissed = true;
                }
                if (hidden.HasValue && !finish && clock.Elapsed - hidden.Value > TimeSpan.FromMilliseconds(600))
                {
                    File.WriteAllText(Path.Combine(directory, "cadence.finish"), "finish"); finish = true;
                }
                Thread.Sleep(10);
            }
            if (!child.WaitForExit(3000) || !File.Exists(Path.Combine(directory, "cadence.json"))) throw new Exception("Fixture did not produce bounded summary.");
            File.WriteAllText(Path.Combine(directory, "native.json"), JsonSerializer.Serialize(new { mode, theme, operations }));
            Console.WriteLine($"PASS owned notification relay mode={mode} operations={operations.Count} exit={child.ExitCode}");
            return child.ExitCode;
        }
        finally
        {
            materialType.GetMethod("BindPeer")!.Invoke(materialOwner, new object[] { 0u });
            if (!child.HasExited) { child.Kill(); child.WaitForExit(); }
            File.WriteAllText(Path.Combine(directory, "stdout.log"), stdout.ToString());
            File.WriteAllText(Path.Combine(directory, "stderr.log"), stderr.ToString());
        }
    }
}
