using System.Diagnostics;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost;

internal static class Program
{
    public static async Task<int> Main(string[] args)
    {
        if (ExtensionHost.ExtensionHostBroker.IsMode(args))
        {
            if (!ExtensionHost.ExtensionHostBroker.TryParent(args, out var extensionParent)) return 2;
            using var extensionLog = PlatformLog.CreateDefault();
            return await ExtensionHost.ExtensionHostBroker.RunAsync(extensionParent,
                Console.OpenStandardInput(), Console.OpenStandardOutput(), extensionLog);
        }
        IndexerServiceInstaller.PinCurrentProcessImage();
        if (TryParseElevatedDirectoryHelper(
                args,
                out var helperPipeName,
                out var helperAuthenticationToken))
        {
            return RunProcessRole(
                "elevated_directory_helper",
                () => ElevatedDirectoryNavigationHelper.Run(
                    helperPipeName!,
                    helperAuthenticationToken!));
        }
        if (TryParseElevatedDirectoryHelperBroker(
                args,
                out var helperSessionId,
                out helperPipeName,
                out helperAuthenticationToken))
        {
            return RunProcessRole(
                "elevated_directory_helper_broker",
                () => ElevatedHelperProcessBroker.Launch(
                    helperSessionId,
                    helperPipeName!,
                    helperAuthenticationToken!));
        }
        if (ErrorDialog.TryParseArguments(args, out var errorReportPath))
        {
            return errorReportPath is null
                ? 0
                : ErrorDialog.Show(errorReportPath!);
        }
        using var log = PlatformLog.CreateDefault();
        if (args.Length == 1 && args[0] == "--trusted-update-service")
            return TrustedUpdateBroker.RunService(log);
        if (args.Length == 1 && args[0] == "--prepare-trusted-update")
        {
            if (!TrustedUpdateBroker.IsEnrolled()) return 0;
            var outcome = await TrustedUpdateBroker.RequestAsync(AppContext.BaseDirectory, Guid.NewGuid(), CancellationToken.None, log);
            return outcome.Success ? 0 : 1;
        }
        if (args.Length == 3 && args[0] == "--deploy-trusted-release" && Guid.TryParse(args[2], out var trustedCorrelation))
        {
            if (!System.Security.Principal.WindowsIdentity.GetCurrent().IsSystem) return 1;
            return IndexerServiceInstaller.InstallElevated(args[1], log, trustedCorrelation, enrollUpdater: false);
        }
        if (IndexerServiceInstaller.TryParseInstallArguments(args, out var indexerPath, out var installCorrelation))
        {
            return IndexerServiceInstaller.InstallElevated(indexerPath!, log, installCorrelation);
        }
        log.Write("info", "process.start", fields: new Dictionary<string, object?>
        {
            ["version"] = typeof(Program).Assembly.GetName().Version?.ToString(),
            ["is_64_bit"] = Environment.Is64BitProcess,
            ["os_version"] = Environment.OSVersion.VersionString,
            ["process_role"] = "platform_host",
        });
        if (!Application.SetHighDpiMode(HighDpiMode.PerMonitorV2))
        {
            Console.Error.WriteLine("PlatformHost could not enable PerMonitorV2 DPI awareness.");
            log.Write("warning", "process.dpi_awareness_failed");
        }

        using var shutdown = new CancellationTokenSource();
        PlatformExceptionDiagnostics.Register(log, shutdown);
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
        catch (Exception error)
        {
            PlatformExceptionDiagnostics.Record(log, "process.pipe_loop_failed", error);
            return 1;
        }
        finally
        {
            await shutdown.CancelAsync();
            await parentMonitor;
            log.Write("info", "process.stop", fields: new Dictionary<string, object?>
            {
                ["process_role"] = "platform_host"
            });
        }
    }

    private static int RunProcessRole(string processRole, Func<int> run)
    {
        using var log = PlatformLog.CreateDefault();
        log.Write("info", "process.start", fields: new Dictionary<string, object?>
        {
            ["process_role"] = processRole
        });
        int? exitCode = null;
        try
        {
            exitCode = run();
            return exitCode.Value;
        }
        catch (Exception error)
        {
            PlatformExceptionDiagnostics.Record(
                log,
                "process.role_failed",
                error,
                new Dictionary<string, object?>
                {
                    ["process_role"] = processRole
                });
            throw;
        }
        finally
        {
            log.Write("info", "process.stop", fields: new Dictionary<string, object?>
            {
                ["process_role"] = processRole,
                ["exit_code"] = exitCode
            });
        }
    }


    private static bool TryParseElevatedDirectoryHelper(
        IReadOnlyList<string> args,
        out string? pipeName,
        out string? authenticationToken)
    {
        pipeName = null;
        authenticationToken = null;
        if (args.Count != 3 ||
            args[0] != "--elevated-directory-helper")
        {
            return false;
        }
        pipeName = args[1];
        authenticationToken = args[2];
        return true;
    }

    private static bool TryParseElevatedDirectoryHelperBroker(
        IReadOnlyList<string> args,
        out uint sessionId,
        out string? pipeName,
        out string? authenticationToken)
    {
        sessionId = 0;
        pipeName = null;
        authenticationToken = null;
        if (args.Count != 4 ||
            args[0] != "--launch-elevated-directory-helper" ||
            !uint.TryParse(args[1], out sessionId))
        {
            return false;
        }
        pipeName = args[2];
        authenticationToken = args[3];
        return true;
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
