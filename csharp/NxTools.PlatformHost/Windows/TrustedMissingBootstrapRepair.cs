using System.ComponentModel;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Security.Principal;
using System.ServiceProcess;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Generated;

namespace NxTools.PlatformHost.Windows;

// Explicit recovery only: an existing updater tree (including an incomplete enrollment) is never repaired here.
internal static class TrustedMissingBootstrapRepair
{
    internal sealed record Registration(string? ImagePath, string? Account, string? KeyHash, string? BootstrapHash);

    internal sealed record Operations(
        Func<bool> IsAdministrator,
        Func<Registration> ReadRegistration,
        Action ValidateParent,
        Action Stop,
        Func<IDisposable> EnterDeploymentGate,
        Action<string, IPlatformLog, Guid> Enroll,
        Action ValidateEnrollment,
        Action<string, IPlatformLog, Guid> CompleteDeployment);

    internal static int Run(IPlatformLog log, Guid correlation)
    {
        var root = Path.Combine(IndexerServiceInstaller.ProtectedProductDirectory(), "Updater");
        var bootstrap = Path.Combine(root, "Bootstrap", "NxTools.PlatformHost.exe");
        Registration ReadRegistration()
        {
            using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{TrustedUpdateBroker.ServiceName}")
                ?? throw new InvalidDataException("Existing updater registration required.");
            return new Registration(key.GetValue("ImagePath") as string, key.GetValue("ObjectName") as string,
                key.GetValue("NxToolsUpdateKeySha256") as string, key.GetValue("NxToolsBootstrapSha256") as string);
        }
        var operations = new Operations(
            () => new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator),
            ReadRegistration,
            IndexerServiceInstaller.ValidateMissingProductParents,
            () =>
            {
                // Ownership is checked again immediately before this single SCM operation.
                ValidateRegistration(ReadRegistration(), bootstrap,
                    Convert.ToHexString(SHA256.HashData(Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64))));
                using var service = new ServiceController(TrustedUpdateBroker.ServiceName);
                service.Refresh();
                if (service.Status != ServiceControllerStatus.Stopped)
                {
                    service.Stop();
                    service.WaitForStatus(ServiceControllerStatus.Stopped, TimeSpan.FromMinutes(4) + TimeSpan.FromSeconds(15));
                }
            },
            TrustedDeploymentGate.Acquire,
            (package, sink, id) => TrustedUpdateBroker.EnrollMissingRoot(package, sink, id),
            () =>
            {
                var keyHash = Convert.ToHexString(SHA256.HashData(Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64)));
                IndexerServiceInstaller.ValidateProductChildDirectory("Updater", "Bootstrap");
                IndexerServiceInstaller.ValidateProductChildDirectory("Updater", "Releases");
                var registered = ReadRegistration();
                ValidateRegistration(registered, bootstrap, keyHash);
                using var binary = IndexerServiceInstaller.TrustedBinary.Open(bootstrap, registered.BootstrapHash!);
                if (!new TrustedUpdateEnrollment(root).IsComplete())
                    throw new InvalidDataException("Updater enrollment is not complete.");
            },
            (package, sink, id) =>
            {
                var result = TrustedUpdateBroker.RequestAsync(package, id, CancellationToken.None, sink).GetAwaiter().GetResult();
                if (!result.Success) throw new Win32Exception(result.NativeErrorCode ?? 1, "Initial protected deployment did not complete.");
            });
        return Run(log, correlation, AppContext.BaseDirectory, Environment.ProcessPath!, root, bootstrap,
            Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64), operations);
    }

    // Same signed-package, caller, registration and original deployment-gate boundary in fixtures and production.
    internal static int Run(IPlatformLog log, Guid correlation, string package, string caller, string root,
        string bootstrap, byte[] publicKey, Operations operations)
    {
        var started = Stopwatch.GetTimestamp();
        var stage = "administrator";
        Trace(log, correlation, "accepted", started);
        try
        {
            if (!operations.IsAdministrator()) throw new UnauthorizedAccessException("Administrator required.");
            stage = "package_identity";
            using var release = TrustedReleaseManifest.VerifyPackage(package, publicKey, 0);
            using var callerBinary = IndexerServiceInstaller.TrustedBinary.Open(caller,
                release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe").Sha256);
            stage = "registration";
            var keyHash = Convert.ToHexString(SHA256.HashData(publicKey));
            ValidateRegistration(operations.ReadRegistration(), bootstrap, keyHash);
            stage = "parent_validation";
            operations.ValidateParent();
            stage = "root_absence";
            RequireMissingRoot(root);
            stage = "service_stop";
            Trace(log, correlation, "service_stop_started", started);
            operations.Stop();
            stage = "deployment_gate";
            using (operations.EnterDeploymentGate())
            {
                stage = "revalidation";
                ValidateRegistration(operations.ReadRegistration(), bootstrap, keyHash);
                operations.ValidateParent();
                RequireMissingRoot(root);
                stage = "enrollment";
                Trace(log, correlation, "enrollment_started", started);
                operations.Enroll(package, log, correlation);
                stage = "enrollment_validation";
                operations.ValidateEnrollment();
                var registered = operations.ReadRegistration();
                ValidateRegistration(registered, bootstrap, keyHash);
                if (!string.Equals(registered.BootstrapHash,
                    release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe").Sha256, StringComparison.OrdinalIgnoreCase))
                    throw new InvalidDataException("Enrolled bootstrap does not match signed package.");
                stage = "protocol_state";
                TrustedBootstrapMaintenance.RecordRepairedEnrollment(root, release);
            }
            stage = "initial_deployment";
            Trace(log, correlation, "deployment_started", started);
            operations.CompleteDeployment(package, log, correlation);
            Trace(log, correlation, "completed", started);
            return 0;
        }
        catch (Exception failure) when (failure is not OutOfMemoryException)
        {
            Trace(log, correlation, "rejected", started, stage,
                failure is Win32Exception native ? native.NativeErrorCode : failure.HResult);
            return 1;
        }
    }

    internal static void RequireMissingRoot(string root)
    {
        try
        {
            // Unlike Exists(), attribute probing does not conflate access errors with absence.
            _ = File.GetAttributes(root);
        }
        catch (FileNotFoundException) { return; }
        catch (DirectoryNotFoundException) { return; }
        throw new InvalidDataException("Updater root already exists.");
    }

    internal static void ValidateRegistration(Registration registration, string bootstrap, string keyHash)
    {
        if (!string.Equals(registration.ImagePath, $"\"{bootstrap}\" --trusted-update-service", StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(registration.Account, "LocalSystem", StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(registration.KeyHash, keyHash, StringComparison.OrdinalIgnoreCase) ||
            registration.BootstrapHash is not { Length: 64 } hash ||
            !hash.All(Uri.IsHexDigit))
            throw new InvalidDataException("Updater registration does not match the signed package.");
    }

    private static void Trace(IPlatformLog log, Guid id, string state, long started, string? stage = null, int? native = null)
    {
        try
        {
            log.Write(state == "rejected" ? "error" : "info", "trusted_update.missing_bootstrap." + state, id,
                new Dictionary<string, object?> { ["stage"] = stage, ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                    ["native_error_code"] = native });
        }
        catch { /* Diagnostics never change repair outcome. */ }
    }
}
