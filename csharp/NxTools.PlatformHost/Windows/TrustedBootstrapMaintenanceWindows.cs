using System.ComponentModel;
using System.Diagnostics;
using System.IO.Pipes;
using System.Security.Cryptography;
using System.Security.Principal;
using System.ServiceProcess;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Generated;

namespace NxTools.PlatformHost.Windows;

internal static class TrustedBootstrapMaintenanceWindows
{
    private static string Root => Path.Combine(IndexerServiceInstaller.ProtectedProductDirectory(), "Updater");
    private static string Bootstrap => Path.Combine(Root, "Bootstrap", "NxTools.PlatformHost.exe");
    internal static int Run(IPlatformLog log)
    {
        var correlation = Guid.NewGuid();
        if (!new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator)) return 5;
        try
        {
            // Explicit maintenance never re-enrolls or installs a missing service.
            IndexerServiceInstaller.ValidateProductChildDirectory("Updater");
            IndexerServiceInstaller.ValidateProductChildDirectory("Updater", "Bootstrap");
            IndexerServiceInstaller.ValidateProductChildDirectory("Updater", "Releases");
            var publicKey = Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64);
            // Pin the exact maintenance caller from this complete, same-key signed package.
            using (var release = TrustedReleaseManifest.VerifyPackage(AppContext.BaseDirectory, publicKey, 0))
            using (var caller = IndexerServiceInstaller.TrustedBinary.Open(Environment.ProcessPath!,
                release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe").Sha256)) { }
            new TrustedBootstrapMaintenance(Root, publicKey, new Service(), log).Apply(AppContext.BaseDirectory, correlation);
            return 0;
        }
        catch (Exception error)
        {
            try { log.Write("error", "trusted_update.maintenance.rejected", correlation,
                new Dictionary<string, object?> { ["native_error_code"] = error is Win32Exception native ? native.NativeErrorCode : error.HResult,
                    ["exception_type"] = error.GetType().Name }); } catch { }
            return 1;
        }
    }

    private sealed class Service : TrustedBootstrapMaintenance.IService
    {
        private string? approvedKey;
        public void Validate(string expectedBootstrapHash, string expectedPublicKeyHash)
        {
            using var key = Open(false);
            if (!string.Equals(key.GetValue("ImagePath") as string, $"\"{Bootstrap}\" --trusted-update-service", StringComparison.OrdinalIgnoreCase) ||
                !string.Equals(key.GetValue("ObjectName") as string, "LocalSystem", StringComparison.OrdinalIgnoreCase) ||
                !string.Equals(key.GetValue("NxToolsBootstrapSha256") as string, expectedBootstrapHash, StringComparison.OrdinalIgnoreCase) ||
                !string.Equals(key.GetValue("NxToolsUpdateKeySha256") as string, expectedPublicKeyHash, StringComparison.OrdinalIgnoreCase))
                throw new InvalidDataException("Maintenance service ownership rejected.");
            using var file = IndexerServiceInstaller.TrustedBinary.Open(Bootstrap, expectedBootstrapHash);
            approvedKey = expectedPublicKeyHash;
        }
        public void Stop()
        {
            ValidateFixedRegistration();
            using var service = new ServiceController(TrustedUpdateBroker.ServiceName);
            service.Refresh();
            if (service.Status != ServiceControllerStatus.Stopped) { service.Stop(); service.WaitForStatus(ServiceControllerStatus.Stopped, TimeSpan.FromMinutes(4) + TimeSpan.FromSeconds(15)); }
        }
        public IDisposable EnterDeploymentGate() => TrustedDeploymentGate.Acquire();
        public void SetBootstrapHash(string hash)
        {
            ValidateFixedRegistration();
            using var file = IndexerServiceInstaller.TrustedBinary.Open(Bootstrap, hash);
            using var key = Open(true); key.SetValue("NxToolsBootstrapSha256", hash, RegistryValueKind.String);
        }
        public void Start()
        {
            ValidateFixedRegistration();
            using var service = new ServiceController(TrustedUpdateBroker.ServiceName);
            service.Refresh();
            if (service.Status != ServiceControllerStatus.Running) { service.Start(); service.WaitForStatus(ServiceControllerStatus.Running, TimeSpan.FromSeconds(45)); }
        }
        public void ConfirmReady()
        {
            using var service = new ServiceController(TrustedUpdateBroker.ServiceName);
            service.Refresh(); if (service.Status != ServiceControllerStatus.Running) throw new InvalidDataException("Maintenance service not running.");
            // Dedicated read-only health frame proves new broker Serve loop/recovery reached readiness.
            using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(30));
            using var pipe = new NamedPipeClientStream(".", "NxTools.TrustedUpdate.v1", PipeDirection.InOut, PipeOptions.Asynchronous,
                TokenImpersonationLevel.Impersonation);
            pipe.ConnectAsync(30_000, deadline.Token).GetAwaiter().GetResult();
            TrustedUpdatePeer.Verify(pipe.SafePipeHandle);
            var id = Guid.NewGuid();
            TrustedUpdateBroker.WriteFrameAsync(pipe, new { PackageDirectory = "", CorrelationId = id, MaintenanceProbe = true }, deadline.Token).GetAwaiter().GetResult();
            var reply = TrustedUpdateBroker.ReadFrameAsync<System.Text.Json.JsonElement>(pipe, deadline.Token).GetAwaiter().GetResult();
            if (reply.GetProperty("CorrelationId").GetGuid() != id || !reply.GetProperty("Success").GetBoolean() ||
                reply.GetProperty("Stage").GetString() != "maintenance_ready") throw new InvalidDataException("Maintenance broker readiness rejected.");
        }
        private void ValidateFixedRegistration()
        {
            using var key = Open(false);
            var currentKey = key.GetValue("NxToolsUpdateKeySha256") as string;
            var embeddedKey = Convert.ToHexString(SHA256.HashData(Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64)));
            if (!string.Equals(key.GetValue("ImagePath") as string, $"\"{Bootstrap}\" --trusted-update-service", StringComparison.OrdinalIgnoreCase) ||
                !string.Equals(key.GetValue("ObjectName") as string, "LocalSystem", StringComparison.OrdinalIgnoreCase) ||
                !string.Equals(currentKey, embeddedKey, StringComparison.OrdinalIgnoreCase) ||
                approvedKey is not null && !string.Equals(currentKey, approvedKey, StringComparison.OrdinalIgnoreCase))
                throw new InvalidDataException("Updater registration changed during maintenance.");
            IndexerServiceInstaller.ValidateProductChildDirectory("Updater", "Bootstrap");
        }
        private static RegistryKey Open(bool writable) => Registry.LocalMachine.OpenSubKey(
            $@"SYSTEM\CurrentControlSet\Services\{TrustedUpdateBroker.ServiceName}", writable) ?? throw new InvalidDataException("Existing updater service required.");
    }
}
