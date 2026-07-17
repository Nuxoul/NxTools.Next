using System.Diagnostics;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost;

internal static class Program
{
    public static async Task<int> Main(string[] args)
    {
        using var log = PlatformLog.CreateDefault();
        log.Write("info", "process.start", fields: new Dictionary<string, object?>
        {
            ["version"] = typeof(Program).Assembly.GetName().Version?.ToString(),
            ["is_64_bit"] = Environment.Is64BitProcess,
            ["os_version"] = Environment.OSVersion.VersionString
        });
        if (!Application.SetHighDpiMode(HighDpiMode.PerMonitorV2))
        {
            Console.Error.WriteLine("PlatformHost could not enable PerMonitorV2 DPI awareness.");
            log.Write("warning", "process.dpi_awareness_failed");
        }

        using var shutdown = new CancellationTokenSource();
        Console.CancelKeyPress += (_, eventArgs) =>
        {
            eventArgs.Cancel = true;
            shutdown.Cancel();
        };

        using var platformAdapter = new WindowsPlatformAdapter(new ForegroundContextProvider(), log);
        var client = new PlatformPipeClient(
            ProtocolConstants.DefaultPipeName,
            platformAdapter,
            log);
        var parentMonitor = MonitorParentAsync(ParseParentProcessId(args), shutdown);

        try
        {
            await client.RunAsync(shutdown.Token);
            return 0;
        }
        catch (OperationCanceledException) when (shutdown.IsCancellationRequested)
        {
            return 0;
        }
        finally
        {
            await shutdown.CancelAsync();
            await parentMonitor;
            log.Write("info", "process.stop");
        }
    }

    private static int? ParseParentProcessId(IReadOnlyList<string> args)
    {
        for (var index = 0; index < args.Count - 1; index++)
        {
            if (args[index] == "--parent-pid" && int.TryParse(args[index + 1], out var processId))
            {
                return processId;
            }
        }
        return null;
    }

    private static async Task MonitorParentAsync(
        int? parentProcessId,
        CancellationTokenSource shutdown)
    {
        if (parentProcessId is null)
        {
            return;
        }

        try
        {
            using var parent = Process.GetProcessById(parentProcessId.Value);
            await parent.WaitForExitAsync(shutdown.Token);
            await shutdown.CancelAsync();
        }
        catch (ArgumentException)
        {
            await shutdown.CancelAsync();
        }
        catch (OperationCanceledException) when (shutdown.IsCancellationRequested)
        {
        }
    }
}
