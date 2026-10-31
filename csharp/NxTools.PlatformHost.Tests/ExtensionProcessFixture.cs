using System.Diagnostics;
using System.Runtime.InteropServices;
using NxTools.PlatformHost.ExtensionHost;
using NxTools.PlatformHost.Windows.Extensions;

namespace NxTools.PlatformHost.Tests;

internal static class ExtensionProcessFixture
{
    internal static async Task<int> RunAsync(string[] args)
    {
        if (args[0] == "--extension-fixture-broker")
            return await ExtensionHostBroker.RunAsync(uint.Parse(args[1]), Console.OpenStandardInput(),
                Console.OpenStandardOutput(), policy: new ExtensionParentPolicy(args[2]), credentialStore: new CredentialFixtureStore());
        if (args[0] == "--extension-fixture-parent")
        {
            using var broker = Start("--extension-fixture-broker", Environment.ProcessId.ToString(), Environment.ProcessPath!);
            File.WriteAllText(args[1], broker.Id.ToString());
            await broker.StandardInput.WriteLineAsync(args[2]); await broker.StandardInput.FlushAsync();
            await Task.Delay(Timeout.Infinite); return 0;
        }
        var mode = args[1];
        if (mode is "protocol" or "protocol-host-call")
        {
            using var invoke = System.Text.Json.JsonDocument.Parse(Console.ReadLine()!);
            var root = invoke.RootElement;
            if (root.GetProperty("protocol_version").GetInt32() != 1 || root.GetProperty("op").GetString() != "invoke") return 2;
            var runId = root.GetProperty("run_id").GetGuid();
            if (mode == "protocol-host-call")
            {
                Console.WriteLine(System.Text.Json.JsonSerializer.Serialize(new { protocol_version = 1, op = "host_call", run_id = runId,
                    call_id = "fixture-call", method = "host.describe", @params = new { } }));
                using var reply = System.Text.Json.JsonDocument.Parse(Console.ReadLine()!);
                var response = reply.RootElement;
                if (response.GetProperty("protocol_version").GetInt32() != 1 || response.GetProperty("op").GetString() != "host_result" ||
                    response.GetProperty("run_id").GetGuid() != runId || response.GetProperty("call_id").GetString() != "fixture-call") return 2;
            }
            Console.WriteLine(System.Text.Json.JsonSerializer.Serialize(new { protocol_version = 1, op = "complete", run_id = runId,
                status = "success", value = new { text = "fixture" } }));
            return 0;
        }
        if (mode == "echo")
        {
            File.WriteAllText(args[2], "executed");
            Console.WriteLine("fixture-ready"); return 0;
        }
        if (mode == "duplex")
        {
            Console.ReadLine(); Console.WriteLine("host-call");
            var reply = Console.ReadLine(); Console.WriteLine(reply); return 0;
        }
        if (mode == "badutf8") { Console.OpenStandardOutput().Write(new byte[] { 0xc3, 0x28, 10 }); return 0; }
        if (mode == "oversized") { Console.WriteLine(new string('x', 256 * 1024 + 1)); return 0; }
        if (mode == "stdoutflood") { var count = args.Length > 2 ? int.Parse(args[2]) : 5000; for (var i = 0; i < count; i++) Console.WriteLine(new string('x', 1024)); return 0; }
        if (mode == "stderrflood") { for (var i = 0; i < 100; i++) Console.Error.WriteLine(new string('x', 1024)); return 0; }
        if (mode == "joblimits")
        {
            var limits = ExtensionNative.CurrentJobLimits();
            Console.WriteLine($"{limits.Basic.Flags}:{limits.JobMemory.ToUInt64()}:{limits.Basic.ActiveProcesses}"); return 0;
        }
        if (mode == "token")
        {
            var identity = ExtensionNative.Identity(ExtensionNative.CurrentProcess());
            Console.WriteLine($"{identity.Integrity}:{identity.Elevated}"); return 0;
        }
        if (mode == "tree" || mode == "pipeholder" || mode == "processlimit")
        {
            var count = mode == "processlimit" ? 10 : 1;
            for (var i = 0; i < count; i++)
            {
                try
                {
                    if (mode == "pipeholder")
                    {
                        var request = ExtensionProcessTests.Request("sleep");
                        var child = ExtensionNative.Spawn(request, BorrowStd(-10), BorrowStd(-11), BorrowStd(-12));
                        ExtensionNative.Resume(child.Thread); child.Thread.Dispose(); child.Process.Dispose();
                        File.WriteAllText(args[2], child.Pid.ToString());
                    }
                    else
                    {
                        using var child = Process.Start(new ProcessStartInfo(Environment.ProcessPath!)
                        { UseShellExecute = false, CreateNoWindow = true, ArgumentList = { "--extension-fixture-child", "sleep" } })!;
                        File.AppendAllText(args[2], child.Id + "\n");
                    }
                }
                catch (Exception) { Console.WriteLine("spawn-denied"); }
            }
            if (mode == "pipeholder") return 0;
        }
        if (mode == "consume") { while (Console.ReadLine() is not null) { } return 0; }
        await Task.Delay(Timeout.Infinite); return 0;
    }
    internal static Process Start(params string[] args)
    {
        var start = new ProcessStartInfo(Environment.ProcessPath!)
        { UseShellExecute = false, CreateNoWindow = true, RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true };
        foreach (var arg in args) start.ArgumentList.Add(arg);
        return Process.Start(start)!;
    }
    [DllImport("kernel32.dll", EntryPoint = "GetStdHandle", ExactSpelling = true)]
    private static extern IntPtr GetStd(int kind);
    private static Microsoft.Win32.SafeHandles.SafeFileHandle BorrowStd(int kind) => new(GetStd(kind), false);
}
