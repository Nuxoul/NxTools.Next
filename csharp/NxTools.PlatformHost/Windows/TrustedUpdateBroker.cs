using System.ComponentModel;
using System.Diagnostics;
using System.IO.Pipes;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Principal;
using System.ServiceProcess;
using TimeoutException = System.TimeoutException;
using System.Text;
using System.Text.Json;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Generated;

namespace NxTools.PlatformHost.Windows;

internal static class TrustedUpdateBroker
{
    internal const string ServiceName = "NxToolsUpdateV1";
    private const string PipeName = "NxTools.TrustedUpdate.v1";
    private const int MaximumFrame = 16 * 1024;
    private static string Root => Path.Combine(IndexerServiceInstaller.ProtectedProductDirectory(), "Updater");
    private static string Bootstrap => Path.Combine(Root, "Bootstrap", "NxTools.PlatformHost.exe");
    private static string Store => Path.Combine(Root, "Releases");
    private sealed record Request(string PackageDirectory, Guid CorrelationId, bool MaintenanceProbe = false);
    private sealed record Reply(Guid CorrelationId, bool Success, string Stage, int? NativeErrorCode = null);

    // Incomplete enrollment resumes only under its pinned explicit authorization record.
    internal static bool IsEnrolled()
    {
        if (File.Exists(Path.Combine(Root, "enrollment.json")))
        {
            IndexerServiceInstaller.ValidateProductChildDirectory("Updater");
            return new TrustedUpdateEnrollment(Root).IsComplete();
        }
        using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{ServiceName}");
        return key is not null;
    }

    internal static void ValidateEnrollmentCandidate(VerifiedRelease release)
    {
        if (!Directory.Exists(Root)) return;
        IndexerServiceInstaller.ValidateProductChildDirectory("Updater");
        new TrustedUpdateEnrollment(Root).ValidateCandidate(EnrollmentIdentity(release));
    }

    private static TrustedUpdateEnrollment.Identity EnrollmentIdentity(VerifiedRelease release) => new(
        release.Manifest.PublicKeySha256.ToUpperInvariant(),
        release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe").Sha256.ToUpperInvariant(),
        release.Manifest.ReleaseSequence, release.Manifest.ReleaseId);

    internal static async Task<WindowsCommandResult> RequestAsync(string packageDirectory, Guid correlationId,
        CancellationToken cancellationToken, IPlatformLog? log = null)
    {
        var started = Stopwatch.GetTimestamp();
        var stage = "enrollment_validation";
        Trace(log, correlationId, "client_accepted", started);
        try
        {
            ValidateEnrollment();
            stage = "package_validation";
            using var requestedRelease = TrustedReleaseManifest.VerifyPackage(packageDirectory,
                Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64), 0);
            stage = "pipe_connect";
            using var deadline = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            deadline.CancelAfter(TimeSpan.FromMinutes(5));
            using var pipe = new NamedPipeClientStream(".", PipeName, PipeDirection.InOut, PipeOptions.Asynchronous,
                TokenImpersonationLevel.Impersonation);
            await pipe.ConnectAsync(10_000, deadline.Token);
            stage = "peer_service_validation";
            TrustedUpdatePeer.Verify(pipe.SafePipeHandle);
            Trace(log, correlationId, "client_peer_verified", started);
            stage = "request_reply";
            await WriteFrameAsync(pipe, new Request(Path.GetFullPath(packageDirectory), correlationId), deadline.Token);
            var reply = await ReadFrameAsync<Reply>(pipe, deadline.Token);
            if (reply.CorrelationId != correlationId) throw new InvalidDataException("Updater correlation mismatch.");
            stage = "published_verification";
            if (reply.Success)
            {
                foreach (var file in requestedRelease.Files)
                {
                    var deployed = Path.Combine(IndexerServiceInstaller.ProtectedServiceDirectory(), file.Path.Replace('/', Path.DirectorySeparatorChar));
                    if (!File.Exists(deployed) || new FileInfo(deployed).Length != file.Size ||
                        !string.Equals(IndexerServiceInstaller.ComputeBinaryHash(deployed), file.Sha256, StringComparison.OrdinalIgnoreCase))
                        throw new InvalidDataException("Published release verification failed.");
                }
            }
            Trace(log, correlationId, reply.Success ? "client_completed" : "client_" + reply.Stage, started, reply.NativeErrorCode);
            return new WindowsCommandResult(reply.Success,
                reply.Success ? null : "可信更新未完成，请使用维护安装程序检查恢复状态。",
                Data: reply.Stage, NativeErrorCode: reply.NativeErrorCode);
        }
        catch (Exception error) when (error is not OutOfMemoryException)
        {
            Trace(log, correlationId, "client_failed_" + stage, started,
                error is Win32Exception nativeError ? nativeError.NativeErrorCode : error.HResult);
            return new WindowsCommandResult(false, "无法连接或验证可信更新服务；未请求额外授权。请重试或使用维护安装程序修复。",
                Data: cancellationToken.IsCancellationRequested ? "cancelled" : "trusted_update_unavailable",
                NativeErrorCode: error is Win32Exception native ? native.NativeErrorCode : error.HResult);
        }
    }

    internal static void EnrollIfConfigured(string package, IPlatformLog log, Guid? correlationId)
    {
        if (string.IsNullOrEmpty(UpdateTrust.PublicKeySpkiBase64)) return;
        var publicKey = Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64);
        using var verified = TrustedReleaseManifest.VerifyPackage(package, publicKey, 0);
        if (IsEnrolled())
        {
            ValidateEnrollment();
            // Trust rotation/bootstrap replacement is explicit maintenance, not implicit enrollment.
            using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{ServiceName}");
            if (!string.Equals(key?.GetValue("NxToolsUpdateKeySha256") as string,
                    Convert.ToHexString(SHA256.HashData(publicKey)), StringComparison.OrdinalIgnoreCase))
                throw new InvalidDataException("Update trust rotation requires maintenance.");
            return;
        }
        using var rootLease = IndexerServiceInstaller.HardenProductChildDirectory("Updater");
        using var bootstrapLease = IndexerServiceInstaller.HardenProductChildDirectory("Updater", "Bootstrap");
        using var storeLease = IndexerServiceInstaller.HardenProductChildDirectory("Updater", "Releases");
        new TrustedUpdateEnrollment(Root).Run(EnrollmentIdentity(verified),
            () => { rootLease.Validate(); bootstrapLease.Validate(); storeLease.Validate(); },
            () =>
            {
                if (!File.Exists(Bootstrap))
                {
                    var temporary = Bootstrap + "." + Guid.NewGuid().ToString("N") + ".tmp";
                    using (var input = new FileStream(Path.Combine(package, "NxTools.PlatformHost.exe"), FileMode.Open, FileAccess.Read, FileShare.Read))
                    using (var output = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None))
                    {
                        input.CopyTo(output);
                        output.Flush(true);
                    }
                    File.Move(temporary, Bootstrap);
                }
                if (!string.Equals(IndexerServiceInstaller.ComputeBinaryHash(Bootstrap), EnrollmentIdentity(verified).BootstrapSha256, StringComparison.OrdinalIgnoreCase))
                    throw new InvalidDataException("Bootstrap recovery identity mismatch.");
                new TrustedUpdateTransaction(Store, publicKey, new NativeDeployment(log), log).Initialize(package);
            },
            () => RegisterEnrollment(publicKey), ValidateEnrollment);
        Trace(log, correlationId, "enrolled", Stopwatch.GetTimestamp());
    }

    private static void RegisterEnrollment(byte[] publicKey)
    {
        bool exists;
        using (var existing = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{ServiceName}"))
        {
            exists = existing is not null;
            if (exists && (!string.Equals(existing!.GetValue("ImagePath") as string,
                    $"\"{Bootstrap}\" --trusted-update-service", StringComparison.OrdinalIgnoreCase) ||
                !string.Equals(existing.GetValue("ObjectName") as string, "LocalSystem", StringComparison.OrdinalIgnoreCase)))
                throw new InvalidDataException("Existing updater service is not the enrollment target.");
        }
        RunSc([exists ? "config" : "create", ServiceName, "binPath=", $"\"{Bootstrap}\" --trusted-update-service",
            "start=", "auto", "obj=", "LocalSystem", "DisplayName=", "NxTools Trusted Update"]);
        using (var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{ServiceName}", true)
            ?? throw new InvalidOperationException("Updater registration unavailable."))
        {
            key.SetValue("NxToolsBootstrapSha256", IndexerServiceInstaller.ComputeBinaryHash(Bootstrap));
            key.SetValue("NxToolsUpdateKeySha256", Convert.ToHexString(SHA256.HashData(publicKey)));
        }
        RunSc(["sdset", ServiceName, "D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWRPLORC;;;AU)"]);
        RunSc(["failure", ServiceName, "reset=", "86400", "actions=", "restart/3000/restart/10000"]);
        RunSc(["start", ServiceName]);
        using var service = new ServiceController(ServiceName);
        service.WaitForStatus(ServiceControllerStatus.Running, TimeSpan.FromSeconds(30));
    }

    private static void ValidateEnrollment()
    {
        using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{ServiceName}")
            ?? throw new InvalidDataException("Updater is not registered.");
        if (!string.Equals(key.GetValue("ImagePath") as string, $"\"{Bootstrap}\" --trusted-update-service", StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(key.GetValue("ObjectName") as string, "LocalSystem", StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(key.GetValue("NxToolsBootstrapSha256") as string,
                IndexerServiceInstaller.ComputeBinaryHash(Bootstrap), StringComparison.OrdinalIgnoreCase))
            throw new InvalidDataException("Updater protected registration mismatch.");
        IndexerServiceInstaller.ValidateProductChildDirectory("Updater");
        IndexerServiceInstaller.ValidateProductChildDirectory("Updater", "Bootstrap");
        IndexerServiceInstaller.ValidateProductChildDirectory("Updater", "Releases");
    }

    internal static int RunService(IPlatformLog log)
    {
        if (!new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator) ||
            !WindowsIdentity.GetCurrent().IsSystem ||
            !string.Equals(Path.GetFullPath(Environment.ProcessPath!), Path.GetFullPath(Bootstrap), StringComparison.OrdinalIgnoreCase))
            return 1;
        ValidateEnrollment();
        if (string.IsNullOrEmpty(UpdateTrust.PublicKeySpkiBase64)) return 1;
        using var service = new BrokerService(log);
        ServiceBase.Run(service);
        return 0;
    }

    private sealed class BrokerService : ServiceBase
    {
        private readonly IPlatformLog log;
        private readonly CancellationTokenSource shutdown = new();
        private Task? loop;
        internal BrokerService(IPlatformLog log)
        {
            this.log = log;
            ServiceName = TrustedUpdateBroker.ServiceName;
        }
        protected override void OnStart(string[] args)
        {
            loop = Task.Run(async () =>
            {
                try { await ServeAsync(log, shutdown.Token); }
                catch (Exception failure) when (failure is not OutOfMemoryException)
                {
                    Trace(log, null, "service_loop_failed", Stopwatch.GetTimestamp(), failure.HResult);
                    Environment.Exit(1); // SCM recovery restarts and replays the protected journal.
                }
            });
        }
        protected override void OnStop()
        {
            shutdown.Cancel();
            // A prepared transaction finishes/rolls back independently of a client disconnect.
            if (loop is not null && !loop.Wait(TimeSpan.FromMinutes(4)))
                throw new TimeoutException("Updater transaction did not stop safely.");
        }
    }

    private static async Task ServeAsync(IPlatformLog log, CancellationToken cancellationToken)
    {
        var transaction = new TrustedUpdateTransaction(Store, Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64),
            new NativeDeployment(log), log);
        await transaction.RecoverAsync(Guid.NewGuid(), CancellationToken.None);
        while (!cancellationToken.IsCancellationRequested)
        {
            var security = new PipeSecurity();
            security.SetAccessRuleProtection(true, false);
            foreach (var sid in new[] { WellKnownSidType.LocalSystemSid, WellKnownSidType.BuiltinAdministratorsSid })
                security.AddAccessRule(new PipeAccessRule(new SecurityIdentifier(sid, null), PipeAccessRights.FullControl, AccessControlType.Allow));
            security.AddAccessRule(new PipeAccessRule(new SecurityIdentifier(WellKnownSidType.AuthenticatedUserSid, null),
                PipeAccessRights.ReadWrite, AccessControlType.Allow));
            // Network clients cannot ask SYSTEM to read a remote/unowned package.
            security.AddAccessRule(new PipeAccessRule(new SecurityIdentifier(WellKnownSidType.NetworkSid, null),
                PipeAccessRights.FullControl, AccessControlType.Deny));
            using var pipe = NamedPipeServerStreamAcl.Create(PipeName, PipeDirection.InOut, 1,
                PipeTransmissionMode.Byte, PipeOptions.Asynchronous, MaximumFrame, MaximumFrame, security);
            try
            {
                await pipe.WaitForConnectionAsync(cancellationToken);
                using var readDeadline = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
                readDeadline.CancelAfter(TimeSpan.FromSeconds(10));
                var request = await ReadFrameAsync<Request>(pipe, readDeadline.Token);
                if (request.MaintenanceProbe && request.CorrelationId != Guid.Empty && request.PackageDirectory == "")
                {
                    await WriteFrameAsync(pipe, new Reply(request.CorrelationId, true, "maintenance_ready"), readDeadline.Token);
                    continue;
                }
                if (request.CorrelationId == Guid.Empty || request.PackageDirectory.Length > 1024 ||
                    !Path.IsPathFullyQualified(request.PackageDirectory) || request.PackageDirectory.StartsWith("\\\\", StringComparison.Ordinal))
                    throw new InvalidDataException("Invalid update request.");
                FileStream updateAdmission;
                try { updateAdmission = new FileStream(Path.Combine(Root, "bootstrap-maintenance.lock"), FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None); }
                catch (IOException)
                {
                    await WriteFrameAsync(pipe, new Reply(request.CorrelationId, false, "bootstrap_maintenance_busy"), readDeadline.Token);
                    continue;
                }
                using var admission = updateAdmission;
                // Open the source under the caller token, not arbitrary SYSTEM read privileges.
                // Locked verified handles can then be copied into SYSTEM-owned staging.
                VerifiedRelease? sourceRelease = null;
                pipe.RunAsClient(() => sourceRelease = TrustedReleaseManifest.VerifyPackage(request.PackageDirectory,
                    Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64), 0));
                using var callerRelease = sourceRelease ?? throw new InvalidDataException("Updater caller unavailable.");
                var started = Stopwatch.GetTimestamp();
                Trace(log, request.CorrelationId, "accepted", started);
                Reply reply;
                try
                {
                    await transaction.ApplyVerifiedAsync(callerRelease, request.CorrelationId, cancellationToken);
                    reply = new(request.CorrelationId, true, "trusted_updated");
                }
                catch (Exception error) when (error is not OutOfMemoryException)
                {
                    reply = new(request.CorrelationId, false, "trusted_update_rejected",
                        error is Win32Exception native ? native.NativeErrorCode : error.HResult);
                }
                Trace(log, request.CorrelationId, reply.Stage, started, reply.NativeErrorCode);
                using var writeDeadline = new CancellationTokenSource(TimeSpan.FromSeconds(5));
                await WriteFrameAsync(pipe, reply, writeDeadline.Token);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { break; }
            catch (Exception error) when (error is not OutOfMemoryException)
            {
                Trace(log, null, "request_failed", Stopwatch.GetTimestamp(), error.HResult);
            }
        }
    }

    private sealed class NativeDeployment(IPlatformLog log) : TrustedUpdateTransaction.IDeployment
    {
        public async Task DeployAsync(string verifiedPackage, Guid correlationId, CancellationToken cancellationToken)
        {
            using var expected = TrustedReleaseManifest.VerifyPackage(verifiedPackage,
                Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64), 0);
            await DeployAsync(verifiedPackage, expected, correlationId, cancellationToken);
        }

        public async Task DeployAsync(string verifiedPackage, VerifiedRelease expected, Guid correlationId, CancellationToken cancellationToken)
        {
            var startInfo = new ProcessStartInfo(Path.Combine(verifiedPackage, "NxTools.PlatformHost.exe"))
            { UseShellExecute = false, CreateNoWindow = true };
            startInfo.ArgumentList.Add("--deploy-trusted-release");
            startInfo.ArgumentList.Add(Path.Combine(verifiedPackage, "NxTools.Indexer.exe"));
            startInfo.ArgumentList.Add(correlationId.ToString("D"));
            using var process = Process.Start(startInfo) ?? throw new InvalidOperationException("Verified deployment could not start.");
            // Never roll back while the mutating child is still alive. Bound it by killing exactly
            // this owned child and waiting for exit before reporting failure to the transaction.
            using var deadline = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            deadline.CancelAfter(TimeSpan.FromMinutes(2));
            try { await process.WaitForExitAsync(deadline.Token); }
            catch (OperationCanceledException)
            {
                process.Kill(entireProcessTree: true);
                await process.WaitForExitAsync(CancellationToken.None);
                throw;
            }
            if (process.ExitCode != 0) throw new Win32Exception(process.ExitCode, "Verified deployment failed.");
            if (!expected.RecoveryOnly && !IndexerServiceInstaller.IsProtectedServiceHealthy())
                throw new InvalidDataException("Protected service tuple failed post-deployment verification.");
            foreach (var file in expected.Files)
            {
                var destination = Path.Combine(IndexerServiceInstaller.ProtectedServiceDirectory(), file.Path.Replace('/', Path.DirectorySeparatorChar));
                if (!File.Exists(destination) || new FileInfo(destination).Length != file.Size ||
                    !string.Equals(IndexerServiceInstaller.ComputeBinaryHash(destination), file.Sha256, StringComparison.OrdinalIgnoreCase))
                    throw new InvalidDataException("Protected release file failed post-deployment verification.");
            }
            Trace(log, correlationId, "deployment_completed", Stopwatch.GetTimestamp());
        }
        public Task RemoveSupersededAsync(VerifiedRelease previous, VerifiedRelease failed, CancellationToken cancellationToken)
        {
            using var gate = TrustedDeploymentGate.Acquire();
            TrustedReleaseManifest.RemoveSupersededArchiveFiles(IndexerServiceInstaller.ProtectedServiceDirectory(), previous, failed);
            if (!previous.Files.Any(file => file.Path == "NxTools.ArchiveWorker.exe"))
            {
                using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{IndexerServiceInstaller.ServiceName}", true)
                    ?? throw new InvalidDataException("Recovered service registration missing.");
                var json = key.GetValue("NxToolsArchiveReleaseSha256") as string;
                if (json is not null)
                {
                    var registered = JsonSerializer.Deserialize<Dictionary<string, string>>(json) ?? throw new InvalidDataException("Invalid archive registration.");
                    var expected = failed.Files.Where(file => file.Path == "NxTools.ArchiveWorker.exe" || file.Path.StartsWith("NxTools.ShellSelectionHelper.", StringComparison.Ordinal)).ToDictionary(file => file.Path, file => file.Sha256);
                    if (registered.Count != expected.Count || registered.Any(pair => !expected.TryGetValue(pair.Key, out var hash) || !string.Equals(hash, pair.Value, StringComparison.OrdinalIgnoreCase)))
                        throw new InvalidDataException("Archive approval changed during recovery.");
                    key.DeleteValue("NxToolsArchiveReleaseSha256", false);
                }
            }
            return Task.CompletedTask;
        }
    }

    private static void RunSc(string[] arguments)
    {
        var startInfo = new ProcessStartInfo(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), "sc.exe"))
        { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true };
        foreach (var argument in arguments) startInfo.ArgumentList.Add(argument);
        using var process = Process.Start(startInfo) ?? throw new InvalidOperationException("Service command unavailable.");
        var output = process.StandardOutput.ReadToEndAsync();
        var errors = process.StandardError.ReadToEndAsync();
        if (!process.WaitForExit(30_000)) { process.Kill(); process.WaitForExit(); throw new TimeoutException("Service command timed out."); }
        Task.WhenAll(output, errors).GetAwaiter().GetResult();
        if (process.ExitCode != 0 && !(arguments[0] == "start" && process.ExitCode == 1056))
            throw new Win32Exception(process.ExitCode, "Updater service command failed.");
    }

    internal static async Task WriteFrameAsync<T>(Stream stream, T value, CancellationToken cancellationToken)
    {
        var bytes = JsonSerializer.SerializeToUtf8Bytes(value);
        if (bytes.Length > MaximumFrame) throw new InvalidDataException("Updater frame oversized.");
        await stream.WriteAsync(BitConverter.GetBytes(bytes.Length), cancellationToken);
        await stream.WriteAsync(bytes, cancellationToken);
        await stream.FlushAsync(cancellationToken);
    }
    internal static async Task<T> ReadFrameAsync<T>(Stream stream, CancellationToken cancellationToken)
    {
        var header = new byte[4];
        await stream.ReadExactlyAsync(header, cancellationToken);
        var size = BitConverter.ToInt32(header);
        if (size <= 0 || size > MaximumFrame) throw new InvalidDataException("Updater frame invalid.");
        var bytes = new byte[size];
        await stream.ReadExactlyAsync(bytes, cancellationToken);
        return JsonSerializer.Deserialize<T>(bytes) ?? throw new InvalidDataException("Updater frame missing.");
    }
    private static void Trace(IPlatformLog? log, Guid? correlationId, string stage, long started, int? nativeCode = null)
    {
        if (log is null) return;
        try
        {
            log.Write("info", "trusted_update.broker", correlationId, new Dictionary<string, object?>
            {
                ["stage"] = stage, ["native_error_code"] = nativeCode,
                ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
            });
        }
        catch { /* Diagnostics cannot authorize/reject a release. */ }
    }
}
