using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Principal;
using Microsoft.Win32.SafeHandles;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Generated;

namespace NxTools.PlatformHost.Windows;

internal static partial class IndexerServiceInstaller
{
    internal const string ServiceName = "NxToolsIndexerV1";
    private const int OperationTimeoutMs = 30_000;
    private const int ProtectedProcessExitTimeoutMs = 5_000;
    private const string BinaryHashValueName = "NxToolsBinarySha256";
    private const string DesktopHashValueName = "NxToolsDesktopSha256";
    private const string PlatformHostHashValueName = "NxToolsPlatformHostSha256";
    private const string DesktopPathValueName = "NxToolsDesktopPath";
    private const string ArchiveHashesValueName = "NxToolsArchiveReleaseSha256";
    private const string ServiceSddl =
        "D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)" +
        "(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)" +
        "(A;;CCLCSWLOCRRC;;;IU)(A;;CCLCSWLOCRRC;;;SU)" +
        "(A;;CCLCSWRPWPLORC;;;AU)";
    private const string ServiceInstallMutexName = @"Local\NxToolsIndexerServiceInstallV1";
    private static readonly object ProcessImageGate = new();
    private static TrustedBinary? _processImage;

    private sealed class ServiceInstallGate(Semaphore semaphore) : IDisposable
    {
        public void Dispose()
        {
            try
            {
                semaphore.Release();
            }
            finally
            {
                semaphore.Dispose();
            }
        }
    }

    internal static void PinCurrentProcessImage()
    {
        lock (ProcessImageGate)
        {
            if (_processImage is not null)
            {
                return;
            }
            var processPath = Environment.ProcessPath
                ?? throw new InvalidOperationException(
                    "Cannot locate NxTools.PlatformHost.exe.");
            _processImage = TrustedBinary.Open(processPath);
        }
    }

    public static async Task<WindowsCommandResult> EnsureAsync(
        IPlatformLog log, Guid correlationId, CancellationToken cancellationToken)
    {
        var started = Stopwatch.GetTimestamp();
        WriteDeploymentLog(log, correlationId, "gate_wait_started", started);
        try
        {
            using var installGate = await AcquireServiceInstallGateAsync(cancellationToken);
            return await EnsureCoreAsync(new NativeDeploymentPlatform(log, correlationId), log, correlationId, cancellationToken);
        }
        catch (Exception error) when (error is OperationCanceledException or TimeoutException)
        {
            var result = new WindowsCommandResult(false, "等待索引服务操作已取消或超时；请重试。",
                Data: cancellationToken.IsCancellationRequested ? "cancelled" : "authorization_timeout");
            WriteDeploymentLog(log, correlationId, result.Data!, started, result);
            return result;
        }
        catch (Exception error)
        {
            var result = new WindowsCommandResult(false, "无法准备索引服务升级，请重试或使用正式安装程序修复。",
                Data: "deployment_failed", NativeErrorCode: error is Win32Exception native ? native.NativeErrorCode : error.HResult);
            WriteDeploymentLog(log, correlationId, result.Data!, started, result);
            return result;
        }
    }

    internal enum ServiceDeploymentDecision { Install, Reuse, Upgrade, RepairRequired }
    internal sealed record ReleaseTuple(string Desktop, string PlatformHost, string Indexer);
    internal sealed record DeploymentState(bool Exists, bool Healthy, ReleaseTuple? Approved, ReleaseTuple Current);
    internal interface IDeploymentPlatform
    {
        DeploymentState ReadState();
        bool StartService();
        Task<WindowsCommandResult> InstallAsync(Guid correlationId, CancellationToken cancellationToken);
        bool HasTrustedUpdater() => false;
        bool NeedsTrustEnrollment() => false;
        bool NeedsLifetimeMigration() => false;
        Task<WindowsCommandResult> ConfigureLifetimeAsync(Guid correlationId, CancellationToken cancellationToken) =>
            throw new InvalidOperationException("Service lifetime configuration is unavailable.");
        Task<WindowsCommandResult> UpdateTrustedAsync(Guid correlationId, CancellationToken cancellationToken) =>
            throw new InvalidOperationException("Trusted updater is unavailable.");
    }

    internal static ServiceDeploymentDecision DecideDeployment(
        bool exists, bool healthy, ReleaseTuple? approved, ReleaseTuple current)
    {
        static bool Valid(ReleaseTuple? tuple) => tuple is not null &&
            new[] { tuple.Desktop, tuple.PlatformHost, tuple.Indexer }.All(hash =>
                hash is { Length: 64 } && hash.All(Uri.IsHexDigit));
        if (!Valid(current)) return ServiceDeploymentDecision.RepairRequired;
        if (!exists) return ServiceDeploymentDecision.Install;
        if (!healthy || !Valid(approved)) return ServiceDeploymentDecision.RepairRequired;
        return string.Equals(approved!.Desktop, current.Desktop, StringComparison.OrdinalIgnoreCase) &&
            string.Equals(approved.PlatformHost, current.PlatformHost, StringComparison.OrdinalIgnoreCase) &&
            string.Equals(approved.Indexer, current.Indexer, StringComparison.OrdinalIgnoreCase)
                ? ServiceDeploymentDecision.Reuse : ServiceDeploymentDecision.Upgrade;
    }

    internal static async Task<WindowsCommandResult> EnsureCoreAsync(
        IDeploymentPlatform platform, IPlatformLog log, Guid correlationId, CancellationToken cancellationToken)
    {
        var started = Stopwatch.GetTimestamp();
        void Trace(string stage, WindowsCommandResult? outcome = null) =>
            WriteDeploymentLog(log, correlationId, stage, started, outcome);
        WindowsCommandResult result;
        Trace("accepted");
        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            var state = platform.ReadState();
            var decision = DecideDeployment(state.Exists, state.Healthy, state.Approved, state.Current);
            if (decision == ServiceDeploymentDecision.Reuse && platform.NeedsTrustEnrollment())
                decision = ServiceDeploymentDecision.Install;
            Trace(decision.ToString());
            if (decision == ServiceDeploymentDecision.Reuse && platform.NeedsLifetimeMigration())
            {
                Trace("lifetime_authorization_requested");
                result = await platform.ConfigureLifetimeAsync(correlationId, cancellationToken);
                if (result.Success)
                    result = platform.StartService()
                        ? new WindowsCommandResult(true, Data: "running")
                        : new WindowsCommandResult(false, "按需启动设置已保存，但索引服务启动失败，请重试。", Data: "start_failed");
                Trace(result.Data ?? "lifetime_configuration_failed", result);
                return result;
            }
            switch (decision)
            {
                case ServiceDeploymentDecision.RepairRequired:
                    result = new WindowsCommandResult(false,
                        "受保护索引服务或当前安装包完整性检查未通过。请使用正式安装程序修复；未启动服务或请求升级授权。",
                        Data: "repair_required");
                    break;
                case ServiceDeploymentDecision.Reuse:
                    result = platform.StartService()
                        ? new WindowsCommandResult(true, Data: "running")
                        : new WindowsCommandResult(false, "当前版本索引服务启动失败，请重试或使用正式安装程序修复。", Data: "start_failed");
                    break;
                default:
                    if (decision == ServiceDeploymentDecision.Upgrade && platform.HasTrustedUpdater())
                    {
                        Trace("trusted_update_requested");
                        result = await platform.UpdateTrustedAsync(correlationId, cancellationToken);
                        if (result.Success) result = result with { Data = "upgraded" };
                        break;
                    }
                    Trace(decision == ServiceDeploymentDecision.Upgrade ? "upgrade_authorization_requested" : "initial_authorization_requested");
                    result = await platform.InstallAsync(correlationId, cancellationToken);
                    if (result.Success && decision == ServiceDeploymentDecision.Upgrade)
                        result = result with { Data = "upgraded" };
                    break;
            }
        }
        catch (Win32Exception error) when (error.NativeErrorCode == 1223)
        {
            result = new WindowsCommandResult(false, "已取消索引服务安装或升级授权；原服务未获准更新。",
                Data: "authorization_cancelled", NativeErrorCode: 1223);
        }
        catch (OperationCanceledException)
        {
            result = new WindowsCommandResult(false, "索引服务授权等待已取消或超时，请检查授权结果后重试。",
                Data: cancellationToken.IsCancellationRequested ? "cancelled" : "authorization_timeout");
        }
        catch (Exception error)
        {
            result = new WindowsCommandResult(false, "索引服务操作失败，请重试或使用正式安装程序修复。",
                Data: "deployment_failed", NativeErrorCode: error is Win32Exception native ? native.NativeErrorCode : error.HResult);
        }
        Trace(result.Data ?? "deployment_failed", result);
        return result;
    }

    private static void WriteDeploymentLog(IPlatformLog log, Guid? correlationId, string stage,
        long started, WindowsCommandResult? outcome = null)
    {
        try
        {
            log.Write("info", "indexer_service.deployment", correlationId, new Dictionary<string, object?>
            {
                ["stage"] = stage, ["success"] = outcome?.Success,
                ["native_error_code"] = outcome?.NativeErrorCode,
                ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
            });
        }
        catch { /* Logging cannot change deployment decisions. */ }
    }

    // Parent Program routes this before starting ordinary PlatformHost IPC.
    internal static bool TryParseInstallArguments(IReadOnlyList<string> args,
        out string? indexerPath, out Guid? correlationId)
    {
        indexerPath = null;
        correlationId = null;
        if (args.Count is not (2 or 3) || args[0] != "--install-indexer-service") return false;
        if (args.Count == 3)
        {
            if (!Guid.TryParse(args[2], out var parsed)) return false;
            correlationId = parsed;
        }
        indexerPath = args[1];
        return true;
    }

    private sealed class NativeDeploymentPlatform(IPlatformLog log, Guid correlationId) : IDeploymentPlatform
    {
        public DeploymentState ReadState()
        {
            PinCurrentProcessImage();
            var hostPath = Environment.ProcessPath ?? throw new InvalidOperationException("PlatformHost image unavailable.");
            using var desktop = TrustedBinary.Open(ValidatePackagedSibling(hostPath, "NxTools.exe"), ReleaseBinaryManifest.DesktopSha256);
            using var indexer = TrustedBinary.Open(ValidatePackagedIndexerPath(hostPath,
                Path.Combine(AppContext.BaseDirectory, "NxTools.Indexer.exe")), ReleaseBinaryManifest.IndexerSha256);
            using var archive = OpenArchiveRelease(AppContext.BaseDirectory);
            string hostHash;
            lock (ProcessImageGate) hostHash = _processImage!.Sha256;
            var exists = ServiceExists();
            var healthy = exists && IsProtectedServiceHealthy();
            ReleaseTuple? approved = null;
            if (healthy)
            {
                using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{ServiceName}");
                approved = new ReleaseTuple(key?.GetValue(DesktopHashValueName) as string ?? "",
                    key?.GetValue(PlatformHostHashValueName) as string ?? "", key?.GetValue(BinaryHashValueName) as string ?? "");
                // Revalidate after capturing approval; unknown/changing state fails closed.
                healthy = IsProtectedServiceHealthy();
            }
            return new DeploymentState(exists, healthy, approved, new ReleaseTuple(desktop.Sha256, hostHash, indexer.Sha256));
        }
        public bool StartService() => TryStartService(log, correlationId);
        public bool NeedsLifetimeMigration()
        {
            using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{ServiceName}");
            return key?.GetValue("Start") is not int start || start != 3;
        }
        public Task<WindowsCommandResult> ConfigureLifetimeAsync(Guid correlationId, CancellationToken cancellationToken) =>
            ConfigureLifetimeWithAuthorizationAsync(log, correlationId, cancellationToken);
        public bool HasTrustedUpdater() => TrustedUpdateBroker.IsEnrolled();
        public bool NeedsTrustEnrollment() => !string.IsNullOrEmpty(NxTools.PlatformHost.Generated.UpdateTrust.PublicKeySpkiBase64)
            && !TrustedUpdateBroker.IsEnrolled();
        public async Task<WindowsCommandResult> UpdateTrustedAsync(Guid correlationId, CancellationToken cancellationToken)
        {
            var result = await TrustedUpdateBroker.RequestAsync(AppContext.BaseDirectory, correlationId, cancellationToken, log);
            if (result.Success && !IsInstalledAt(Path.Combine(AppContext.BaseDirectory, "NxTools.Indexer.exe")))
                return new WindowsCommandResult(false, "更新服务结束，但当前受保护程序组未通过验证。", Data: "tuple_verification_failed");
            return result;
        }
        public async Task<WindowsCommandResult> InstallAsync(Guid correlationId, CancellationToken cancellationToken)
        {
            var hostPath = Environment.ProcessPath ?? throw new InvalidOperationException("PlatformHost image unavailable.");
            var indexerPath = ValidatePackagedIndexerPath(hostPath, Path.Combine(AppContext.BaseDirectory, "NxTools.Indexer.exe"));
            var startInfo = new ProcessStartInfo
            {
                FileName = hostPath, Verb = "runas", UseShellExecute = true, WindowStyle = ProcessWindowStyle.Hidden
            };
            startInfo.ArgumentList.Add("--install-indexer-service");
            startInfo.ArgumentList.Add(indexerPath);
            startInfo.ArgumentList.Add(correlationId.ToString("D"));
            using var installer = Process.Start(startInfo) ?? throw new InvalidOperationException("Elevated installer unavailable.");
            using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            timeout.CancelAfter(OperationTimeoutMs);
            await installer.WaitForExitAsync(timeout.Token);
            if (installer.ExitCode != 0)
                return new WindowsCommandResult(false, "索引服务安装或升级失败，请使用正式安装程序修复。",
                    Data: "install_failed", NativeErrorCode: installer.ExitCode);
            return IsInstalledAt(indexerPath)
                ? new WindowsCommandResult(true, Data: "installed")
                : new WindowsCommandResult(false, "安装程序结束，但当前版本受保护程序组未通过验证。", Data: "tuple_verification_failed");
        }
    }



    public static int InstallElevated(string indexerPath, IPlatformLog log, Guid? correlationId = null, bool enrollUpdater = true)
    {
        var started = Stopwatch.GetTimestamp();
        WriteDeploymentLog(log, correlationId, "elevated_started", started);
        try
        {
            var hostPath = Environment.ProcessPath
                ?? throw new InvalidOperationException("Cannot locate NxTools.PlatformHost.exe.");
            indexerPath = ValidatePackagedIndexerPath(hostPath, indexerPath);
            var desktopPath = ValidatePackagedSibling(hostPath, "NxTools.exe");
            var browserHostPath = ValidatePackagedSibling(hostPath, "NxTools.BrowserHost.exe");
            using var deploymentGate = TrustedDeploymentGate.Acquire();
            if (enrollUpdater && TrustedUpdateBroker.IsEnrolled())
                throw new InvalidOperationException("Use the enrolled trusted updater; explicit maintenance is required for repair.");
            using var signedRelease = string.IsNullOrEmpty(NxTools.PlatformHost.Generated.UpdateTrust.PublicKeySpkiBase64)
                ? null
                : TrustedReleaseManifest.VerifyPackage(Path.GetDirectoryName(indexerPath)!,
                    Convert.FromBase64String(NxTools.PlatformHost.Generated.UpdateTrust.PublicKeySpkiBase64), 0);
            if (enrollUpdater && signedRelease is not null) TrustedUpdateBroker.ValidateEnrollmentCandidate(signedRelease);
            PinCurrentProcessImage();
            string trustedPlatformHostHash;
            lock (ProcessImageGate)
            {
                trustedPlatformHostHash = _processImage?.Sha256
                    ?? throw new InvalidOperationException(
                        "The authorized PlatformHost image is unavailable.");
            }
            using var indexerSource = TrustedBinary.Open(
                indexerPath,
                ReleaseBinaryManifest.IndexerSha256);
            using var platformHostSource = TrustedBinary.Open(
                hostPath,
                trustedPlatformHostHash);
            using var desktopSource = TrustedBinary.Open(
                desktopPath,
                ReleaseBinaryManifest.DesktopSha256);
            using var browserHostSource = TrustedBinary.Open(
                browserHostPath,
                ReleaseBinaryManifest.BrowserHostSha256);
            using var archiveSources = OpenArchiveRelease(Path.GetDirectoryName(indexerPath)!);

            var serviceExists = ServiceExists();
            WriteDeploymentLog(log, correlationId, "source_tuple_verified", started);
            if (serviceExists)
            {
                RunSc(["stop", ServiceName], [0, 1062]);
                WaitForServiceStopped();
            }
            var protectedDirectory = ProtectedServiceDirectory();
            var protectedPlatformHostPath =
                Path.Combine(protectedDirectory, "NxTools.PlatformHost.exe");
            var protectedDesktopPath =
                Path.Combine(protectedDirectory, "NxTools.exe");
            var protectedBrowserHostPath =
                Path.Combine(protectedDirectory, "NxTools.BrowserHost.exe");
            StopProtectedProcesses(
                [protectedDesktopPath, protectedPlatformHostPath, protectedBrowserHostPath,
                    Path.Combine(protectedDirectory, "NxTools.ShellSelectionHelper.exe"),
                    Path.Combine(protectedDirectory, "NxTools.ArchiveWorker.exe")],
                Environment.ProcessId,
                log, correlationId);
            using var protectedDirectories =
                HardenServiceDirectory(protectedDirectory);
            var protectedIndexerPath = DeployProtectedBinary(
                indexerSource,
                protectedDirectory,
                "NxTools.Indexer.exe");
            protectedPlatformHostPath = DeployProtectedBinary(
                platformHostSource,
                protectedDirectory,
                "NxTools.PlatformHost.exe");
            protectedDesktopPath = DeployProtectedBinary(
                desktopSource,
                protectedDirectory,
                "NxTools.exe");
            if (signedRelease is null) CopyBrowserExtensionInto(protectedDirectory);
            else CopySignedAssets(signedRelease, Path.GetDirectoryName(indexerPath)!, protectedDirectory);
            protectedBrowserHostPath = DeployProtectedBinary(
                browserHostSource,
                protectedDirectory,
                "NxTools.BrowserHost.exe");
            foreach (var source in archiveSources.Sources)
                DeployProtectedBinary(source, protectedDirectory, Path.GetFileName(source.FullPath));
            using var verifiedArchive = OpenArchiveRelease(protectedDirectory);
            protectedDirectories.Validate();
            ValidateDeployedBinary(
                protectedIndexerPath,
                indexerSource.Sha256);
            ValidateDeployedBinary(
                protectedPlatformHostPath,
                platformHostSource.Sha256);
            ValidateDeployedBinary(
                protectedDesktopPath,
                desktopSource.Sha256);
            ValidateDeployedBinary(
                protectedBrowserHostPath,
                browserHostSource.Sha256);
            RunSc(RegistrationArguments(serviceExists, protectedIndexerPath), [0]);
            RunSc(["description", ServiceName, "NxTools NTFS MFT and USN index service"], [0]);
            RunSc(["sdset", ServiceName, ServiceSddl], [0]);
            RunSc(["failure", ServiceName, "reset=", "86400", "actions=", "restart/3000/restart/10000"], [0]);
            using (var key = Registry.LocalMachine.OpenSubKey(
                $@"SYSTEM\CurrentControlSet\Services\{ServiceName}",
                writable: true) ?? throw new InvalidOperationException("索引服务注册表项不存在。"))
            {
                key.SetValue(
                    BinaryHashValueName,
                    indexerSource.Sha256,
                    RegistryValueKind.String);
                key.SetValue(
                    DesktopHashValueName,
                    desktopSource.Sha256,
                    RegistryValueKind.String);
                key.SetValue(
                    PlatformHostHashValueName,
                    platformHostSource.Sha256,
                    RegistryValueKind.String);
                key.SetValue(ArchiveHashesValueName,
                    System.Text.Json.JsonSerializer.Serialize(ArchiveReleaseManifest.Files), RegistryValueKind.String);
                key.SetValue("NxToolsBrowserHostSha256", browserHostSource.Sha256, RegistryValueKind.String);
                key.SetValue(
                    DesktopPathValueName,
                    protectedDesktopPath,
                    RegistryValueKind.String);
            }
            protectedDirectories.Validate();
            RunSc(["start", ServiceName], [0, 1056]);
            WaitForServiceRunning();
            if (enrollUpdater) TrustedUpdateBroker.EnrollIfConfigured(Path.GetDirectoryName(indexerPath)!, log, correlationId);
            WriteDeploymentLog(log, correlationId, "elevated_installed", started,
                new WindowsCommandResult(true, Data: "installed"));
            return 0;
        }
        catch (Exception error)
        {
            WriteDeploymentLog(log, correlationId, "elevated_failed", started,
                new WindowsCommandResult(false, NativeErrorCode:
                    error is Win32Exception native ? native.NativeErrorCode : error.HResult));
            return 1;
        }
    }

    internal static string ValidatePackagedIndexerPath(string hostPath, string indexerPath)
    {
        var fullHostPath = Path.GetFullPath(hostPath);
        var fullIndexerPath = Path.GetFullPath(indexerPath);
        if (!File.Exists(fullIndexerPath) ||
            !string.Equals(Path.GetFileName(fullIndexerPath), "NxTools.Indexer.exe", StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(Path.GetDirectoryName(fullHostPath), Path.GetDirectoryName(fullIndexerPath), StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException("索引服务程序必须是当前安装目录中的 NxTools.Indexer.exe。");
        }
        return fullIndexerPath;
    }

    internal static string ValidatePackagedSibling(
        string hostPath,
        string siblingFileName)
    {
        var fullHostPath = Path.GetFullPath(hostPath);
        var siblingPath = Path.GetFullPath(
            Path.Combine(
                Path.GetDirectoryName(fullHostPath)
                    ?? throw new InvalidDataException(
                        "Windows 适配器安装目录不可用。"),
                siblingFileName));
        if (!File.Exists(siblingPath) ||
            !string.Equals(
                Path.GetFileName(siblingPath),
                siblingFileName,
                StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException(
                $"安装目录中缺少 {siblingFileName}。");
        }
        return siblingPath;
    }

    internal static bool IsProtectedServiceHealthy()
    {
        try
        {
            var protectedDirectory = ProtectedServiceDirectory();
            var protectedIndexerPath = Path.Combine(protectedDirectory, "NxTools.Indexer.exe");
            var protectedPlatformHostPath = Path.Combine(protectedDirectory, "NxTools.PlatformHost.exe");
            var protectedDesktopPath = Path.Combine(protectedDirectory, "NxTools.exe");
            if (!File.Exists(protectedIndexerPath) ||
                !File.Exists(protectedPlatformHostPath) ||
                !File.Exists(protectedDesktopPath))
            {
                return false;
            }

            using var key = Registry.LocalMachine.OpenSubKey(
                $@"SYSTEM\CurrentControlSet\Services\{ServiceName}");
            var imagePath = key?.GetValue("ImagePath") as string;
            var objectName = key?.GetValue("ObjectName") as string;
            var startMode = key?.GetValue("Start");
            var binaryHash = key?.GetValue(BinaryHashValueName) as string;
            var desktopHash = key?.GetValue(DesktopHashValueName) as string;
            var platformHostHash = key?.GetValue(PlatformHostHashValueName) as string;
            var registeredDesktopPath = key?.GetValue(DesktopPathValueName) as string;
            if (imagePath is null ||
                objectName is null ||
                startMode is null ||
                binaryHash is null ||
                desktopHash is null ||
                platformHostHash is null ||
                registeredDesktopPath is null)
            {
                return false;
            }

            var descriptor = RunSc(["sdshow", ServiceName], [0]);
            return string.Equals(
                    Path.GetFullPath(registeredDesktopPath),
                    Path.GetFullPath(protectedDesktopPath),
                    StringComparison.OrdinalIgnoreCase) &&
                BinaryHashMatches(binaryHash, protectedIndexerPath) &&
                BinaryHashMatches(desktopHash, protectedDesktopPath) &&
                BinaryHashMatches(platformHostHash, protectedPlatformHostPath) &&
                RegisteredArchiveReleaseMatches(key, protectedDirectory, platformHostHash) &&
                RegistrationMatches(
                    new IndexerServiceRegistration(
                        imagePath,
                        objectName,
                        Convert.ToInt32(startMode),
                        descriptor),
                    protectedIndexerPath);
        }
        catch
        {
            return false;
        }
    }

    private static bool IsInstalledAt(string indexerPath)
    {
        using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{ServiceName}");
        var imagePath = key?.GetValue("ImagePath") as string;
        var objectName = key?.GetValue("ObjectName") as string;
        var startMode = key?.GetValue("Start");
        var binaryHash = key?.GetValue(BinaryHashValueName) as string;
        var desktopHash = key?.GetValue(DesktopHashValueName) as string;
        var platformHostHash = key?.GetValue(PlatformHostHashValueName) as string;
        var registeredDesktopPath = key?.GetValue(DesktopPathValueName) as string;
        var sourceDirectory = Path.GetDirectoryName(indexerPath);
        var desktopPath = sourceDirectory is null
            ? null
            : Path.Combine(sourceDirectory, "NxTools.exe");
        var sourcePlatformHostPath = sourceDirectory is null
            ? null
            : Path.Combine(sourceDirectory, "NxTools.PlatformHost.exe");
        var protectedDirectory = ProtectedServiceDirectory();
        var protectedIndexerPath =
            Path.Combine(protectedDirectory, "NxTools.Indexer.exe");
        var protectedPlatformHostPath =
            Path.Combine(protectedDirectory, "NxTools.PlatformHost.exe");
        var protectedDesktopPath =
            Path.Combine(protectedDirectory, "NxTools.exe");
        if (imagePath is null ||
            objectName is null ||
            startMode is null ||
            binaryHash is null ||
            desktopHash is null ||
            platformHostHash is null ||
            registeredDesktopPath is null ||
            desktopPath is null ||
            sourcePlatformHostPath is null ||
            !File.Exists(desktopPath) ||
            !File.Exists(sourcePlatformHostPath) ||
            !File.Exists(protectedIndexerPath) ||
            !File.Exists(protectedPlatformHostPath) ||
            !File.Exists(protectedDesktopPath))
        {
            return false;
        }
        try
        {
            var descriptor = RunSc(["sdshow", ServiceName], [0]);
            return string.Equals(
                    Path.GetFullPath(registeredDesktopPath),
                    Path.GetFullPath(protectedDesktopPath),
                    StringComparison.OrdinalIgnoreCase) &&
                BinaryHashMatches(binaryHash, indexerPath) &&
                BinaryHashMatches(binaryHash, protectedIndexerPath) &&
                BinaryHashMatches(desktopHash, desktopPath) &&
                BinaryHashMatches(desktopHash, protectedDesktopPath) &&
                BinaryHashMatches(platformHostHash, sourcePlatformHostPath) &&
                BinaryHashMatches(platformHostHash, protectedPlatformHostPath) &&
                ArchiveReleaseMatches(sourceDirectory!) && ArchiveReleaseMatches(protectedDirectory) &&
                RegistrationMatches(
                new IndexerServiceRegistration(
                    imagePath,
                    objectName,
                    Convert.ToInt32(startMode),
                    descriptor),
                protectedIndexerPath);
        }
        catch
        {
            return false;
        }
    }

    internal sealed record IndexerServiceRegistration(
        string ImagePath,
        string ObjectName,
        int StartMode,
        string SecurityDescriptor);

    internal static bool RegistrationMatches(
        IndexerServiceRegistration registration,
        string expectedIndexerPath)
    {
        return RegisteredImagePathMatches(registration.ImagePath, expectedIndexerPath) &&
            string.Equals(registration.ObjectName, "LocalSystem", StringComparison.OrdinalIgnoreCase) &&
            registration.StartMode is 2 or 3 &&
            registration.SecurityDescriptor.Contains(
                "(A;;CCLCSWRPWPLORC;;;AU)",
                StringComparison.Ordinal);
    }

    internal static string RegistrationVerb(bool serviceExists) =>
        serviceExists ? "config" : "create";

    internal static string ComputeBinaryHash(string path)
    {
        using var stream = File.OpenRead(path);
        return Convert.ToHexString(SHA256.HashData(stream));
    }

    internal static bool VerifiedPackagedBinaryHashMatches(string path, string expectedHash)
    {
        using var binary = TrustedBinary.Open(path, expectedHash);
        return true;
    }

    internal static bool BinaryHashMatches(string registeredHash, string indexerPath) =>
        string.Equals(
            registeredHash,
            ComputeBinaryHash(indexerPath),
            StringComparison.OrdinalIgnoreCase);

    internal static bool IsApprovedProtectedDesktop(string processPath)
    {
        try
        {
            var expectedPath = Path.Combine(
                ProtectedServiceDirectory(),
                "NxTools.exe");
            if (!string.Equals(
                    Path.GetFullPath(processPath),
                    Path.GetFullPath(expectedPath),
                    StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }
            using var key = Registry.LocalMachine.OpenSubKey(
                $@"SYSTEM\CurrentControlSet\Services\{ServiceName}");
            var registeredDesktopPath = key?.GetValue(DesktopPathValueName) as string;
            var registeredIndexerHash = key?.GetValue(BinaryHashValueName) as string;
            var registeredDesktopHash = key?.GetValue(DesktopHashValueName) as string;
            var registeredPlatformHostHash = key?.GetValue(PlatformHostHashValueName) as string;
            var currentPlatformHostPath = Environment.ProcessPath;
            var protectedDirectory = ProtectedServiceDirectory();
            var protectedIndexerPath = Path.Combine(protectedDirectory, "NxTools.Indexer.exe");
            var protectedPlatformHostPath = Path.Combine(
                protectedDirectory,
                "NxTools.PlatformHost.exe");
            return registeredDesktopPath is not null &&
                string.Equals(
                    Path.GetFullPath(registeredDesktopPath),
                    Path.GetFullPath(expectedPath),
                    StringComparison.OrdinalIgnoreCase) &&
                registeredIndexerHash is not null &&
                registeredDesktopHash is not null &&
                registeredPlatformHostHash is not null &&
                currentPlatformHostPath is not null &&
                File.Exists(expectedPath) &&
                File.Exists(protectedIndexerPath) &&
                File.Exists(protectedPlatformHostPath) &&
                ArchiveReleaseMatches(protectedDirectory) &&
                BinaryHashMatches(registeredIndexerHash, protectedIndexerPath) &&
                BinaryHashMatches(ReleaseBinaryManifest.IndexerSha256, protectedIndexerPath) &&
                BinaryHashMatches(registeredDesktopHash, expectedPath) &&
                BinaryHashMatches(ReleaseBinaryManifest.DesktopSha256, expectedPath) &&
                BinaryHashMatches(registeredPlatformHostHash, protectedPlatformHostPath) &&
                BinaryHashMatches(registeredPlatformHostHash, currentPlatformHostPath);
        }
        catch (Exception error) when (
            error is IOException or
            UnauthorizedAccessException or
            System.Security.SecurityException or
            InvalidOperationException)
        {
            return false;
        }
    }

    internal static bool IsApprovedProtectedPlatformHost(string processPath)
    {
        try
        {
            var expectedPath = Path.Combine(
                ProtectedServiceDirectory(),
                "NxTools.PlatformHost.exe");
            if (!string.Equals(
                    Path.GetFullPath(processPath),
                    Path.GetFullPath(expectedPath),
                    StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }
            using var key = Registry.LocalMachine.OpenSubKey(
                $@"SYSTEM\CurrentControlSet\Services\{ServiceName}");
            var registeredHash =
                key?.GetValue(PlatformHostHashValueName) as string;
            return registeredHash is not null &&
                File.Exists(expectedPath) &&
                ArchiveReleaseMatches(ProtectedServiceDirectory()) &&
                BinaryHashMatches(registeredHash, expectedPath);
        }
        catch (Exception error) when (
            error is IOException or
            UnauthorizedAccessException or
            System.Security.SecurityException)
        {
            return false;
        }
    }


    private static void CopySignedAssets(VerifiedRelease release, string sourceRoot, string protectedDirectory)
    {
        var extensionRoot = Path.Combine(protectedDirectory, "browser_extension");
        if (Directory.Exists(extensionRoot))
        {
            TrustedUpdateTransaction.RejectReparseTree(extensionRoot);
            Directory.Delete(extensionRoot, true);
        }
        foreach (var file in release.Files)
        {
            if (file.Path is "NxTools.exe" or "NxTools.PlatformHost.exe" or "NxTools.Indexer.exe" or "NxTools.BrowserHost.exe" || ArchiveReleaseManifest.Files.ContainsKey(file.Path)) continue;
            var relative = file.Path.Replace('/', Path.DirectorySeparatorChar);
            var destination = Path.Combine(protectedDirectory, relative);
            Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
            File.Copy(Path.Combine(sourceRoot, relative), destination, true);
            if (!string.Equals(ComputeBinaryHash(destination), file.Sha256, StringComparison.OrdinalIgnoreCase))
                throw new InvalidDataException("Signed deployment asset mismatch.");
        }
    }

    private static void CopyBrowserExtensionInto(string protectedDirectory)
    {
        var sourceRoot = AppContext.BaseDirectory;
        var candidates = new[]
        {
            Path.Combine(sourceRoot, "browser_extension"),
            Path.Combine(sourceRoot, "..", "browser_extension"),
            Path.Combine(Directory.GetCurrentDirectory(), "browser_extension"),
        };
        var source = candidates.FirstOrDefault(Directory.Exists);
        if (source is null)
        {
            return;
        }

        var destination = Path.Combine(protectedDirectory, "browser_extension");
        if (Directory.Exists(destination))
        {
            Directory.Delete(destination, recursive: true);
        }
        foreach (var directory in Directory.GetDirectories(source, "*", SearchOption.AllDirectories))
        {
            Directory.CreateDirectory(
                directory.Replace(source, destination, StringComparison.OrdinalIgnoreCase));
        }
        foreach (var file in Directory.GetFiles(source, "*", SearchOption.AllDirectories))
        {
            File.Copy(file, file.Replace(source, destination, StringComparison.OrdinalIgnoreCase), overwrite: true);
        }
    }

    internal static string ProtectedVendorDirectory() =>
        Path.Combine(
            Environment.GetFolderPath(
                Environment.SpecialFolder.CommonApplicationData),
            "NxTools");

    internal static string ProtectedProductDirectory() =>
        Path.Combine(ProtectedVendorDirectory(), "NxTools.Next");

    internal static string ProtectedServiceDirectory() =>
        Path.Combine(ProtectedProductDirectory(), "Service");

    private static string DeployProtectedBinary(
        TrustedBinary source,
        string protectedDirectory,
        string fileName)
    {
        var destination = Path.Combine(protectedDirectory, fileName);
        if (DeploymentSourceMatchesDestination(
                source.FullPath,
                destination))
        {
            return destination;
        }
        var temporary = Path.Combine(
            protectedDirectory,
            $".{fileName}.{Guid.NewGuid():N}.tmp");
        try
        {
            using (var destinationStream = new FileStream(
                       temporary,
                       FileMode.CreateNew,
                       FileAccess.Write,
                       FileShare.None,
                       64 * 1024,
                       FileOptions.SequentialScan))
            {
                source.CopyTo(destinationStream);
                destinationStream.Flush(flushToDisk: true);
            }
            File.Delete(destination);
            File.Move(temporary, destination);
            return destination;
        }
        finally
        {
            File.Delete(temporary);
        }
    }

    internal static bool DeploymentSourceMatchesDestination(
        string sourcePath,
        string destinationPath) =>
        string.Equals(
            NormalizeFinalPath(sourcePath),
            NormalizeFinalPath(destinationPath),
            StringComparison.OrdinalIgnoreCase);

    internal static void StopProtectedProcesses(
        IReadOnlyCollection<string> protectedExecutablePaths,
        int installerProcessId,
        IPlatformLog log,
        Guid? correlationId = null)
    {
        var protectedPaths = protectedExecutablePaths
            .Select(NormalizeFinalPath)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        var processNames = protectedPaths
            .Select(Path.GetFileNameWithoutExtension)
            .Where(name => !string.IsNullOrWhiteSpace(name))
            .Distinct(StringComparer.OrdinalIgnoreCase);

        foreach (var processName in processNames)
        {
            foreach (var process in Process.GetProcessesByName(processName!))
            {
                using (process)
                {
                    if (process.Id == installerProcessId)
                    {
                        continue;
                    }

                    string? processPath;
                    try
                    {
                        processPath = process.MainModule?.FileName;
                    }
                    catch (Exception error) when (
                        error is Win32Exception or
                        InvalidOperationException or
                        NotSupportedException)
                    {
                        try
                        {
                            log.Write("warning", "indexer_service.protected_process_inspection_failed", correlationId,
                                new Dictionary<string, object?>
                                {
                                    ["process_id"] = process.Id,
                                    ["native_error_code"] = error is Win32Exception native ? native.NativeErrorCode : error.HResult
                                });
                        }
                        catch { /* Inspection diagnostics are best effort. */ }
                        continue;
                    }

                    if (processPath is null ||
                        !protectedPaths.Contains(NormalizeFinalPath(processPath)))
                    {
                        continue;
                    }

                    try
                    {
                        process.Kill();
                        if (!process.WaitForExit(ProtectedProcessExitTimeoutMs))
                        {
                            throw new TimeoutException(
                                $"等待旧程序退出超时：{processPath}");
                        }
                        try
                        {
                            log.Write("info", "indexer_service.protected_process_stopped", correlationId,
                                new Dictionary<string, object?> { ["process_id"] = process.Id });
                        }
                        catch { /* Stop diagnostics cannot change the deployment outcome. */ }
                    }
                    catch (Exception error) when (
                        error is Win32Exception or
                        InvalidOperationException or
                        NotSupportedException)
                    {
                        throw new InvalidOperationException(
                            $"无法关闭旧程序 {processPath}：{error.Message}",
                            error);
                    }
                }
            }
        }
    }

    internal static ProtectedDirectoryLease HardenProductChildDirectory(params string[] segments)
    {
        if (segments.Length is < 1 or > 3 || segments[0] != "Updater" ||
            segments.Any(segment => segment is not ("Updater" or "Bootstrap" or "Releases")))
            throw new InvalidDataException("Unknown protected updater directory.");
        var lease = new ProtectedDirectoryLease();
        try
        {
            PrepareProtectedDirectory(ProtectedVendorDirectory(), lease);
            PrepareProtectedDirectory(ProtectedProductDirectory(), lease);
            var path = ProtectedProductDirectory();
            foreach (var segment in segments)
            {
                path = Path.Combine(path, segment);
                PrepareProtectedDirectory(path, lease);
            }
            lease.Validate();
            return lease;
        }
        catch { lease.Dispose(); throw; }
    }

    internal static void ValidateProductChildDirectory(params string[] segments)
    {
        using var lease = new ProtectedDirectoryLease();
        var path = ProtectedProductDirectory();
        lease.Add(ProtectedVendorDirectory());
        lease.Add(path);
        foreach (var segment in segments)
        {
            path = Path.Combine(path, segment);
            lease.Add(path);
        }
        lease.Validate();
    }

    internal static void ValidateMissingProductParents() => ValidateMissingProductParents(
        Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData));

    internal static void ValidateMissingProductParents(string commonData)
    {
        // ProgramData must exist and be a stable directory; product ancestors may be absent.
        using var baseLease = new ProtectedDirectoryLease();
        baseLease.Add(commonData);
        using var productLease = new ProtectedDirectoryLease();
        foreach (var path in new[] { Path.Combine(commonData, "NxTools"), Path.Combine(commonData, "NxTools", "NxTools.Next") })
        {
            try { _ = File.GetAttributes(path); }
            catch (FileNotFoundException) { productLease.Validate(); return; }
            catch (DirectoryNotFoundException) { productLease.Validate(); return; }
            productLease.Add(path);
        }
        productLease.Validate();
    }

    private static ProtectedDirectoryLease HardenServiceDirectory(
        string directory)
    {
        var vendorDirectory = ProtectedVendorDirectory();
        var productDirectory = ProtectedProductDirectory();
        if (!string.Equals(
                Path.GetFullPath(directory),
                Path.GetFullPath(Path.Combine(productDirectory, "Service")),
                StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException(
                "受保护的索引服务目录不在固定的 ProgramData 产品目录中。");
        }
        var lease = new ProtectedDirectoryLease();
        try
        {
            PrepareProtectedDirectory(vendorDirectory, lease);
            PrepareProtectedDirectory(productDirectory, lease);
            PrepareProtectedDirectory(directory, lease);
            lease.Validate();
            return lease;
        }
        catch
        {
            lease.Dispose();
            throw;
        }
    }

    private static void PrepareProtectedDirectory(
        string directory,
        ProtectedDirectoryLease lease)
    {
        Directory.CreateDirectory(directory);
        lease.Add(directory);
        RunIcacls([
            directory,
            "/setowner",
            "*S-1-5-32-544",
            "/C"
        ]);
        RunIcacls([
            directory,
            "/reset",
            "/C"
        ]);
        RunIcacls([
            directory,
            "/inheritance:r",
            "/grant:r",
            "*S-1-5-18:(OI)(CI)(F)",
            "*S-1-5-32-544:(OI)(CI)(F)",
            "*S-1-5-32-545:(OI)(CI)(RX)",
            "/C"
        ]);
        VerifyProtectedDirectoryAcl(directory);
    }

    private static void ValidateDeployedBinary(
        string path,
        string expectedHash)
    {
        using var binary = TrustedBinary.Open(path);
        if (!string.Equals(
                binary.Sha256,
                expectedHash,
                StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException(
                $"受保护的服务文件 {Path.GetFileName(path)} 在发布后发生变化。");
        }
    }

    private static void VerifyProtectedDirectoryAcl(string directory)
    {
        var result = GetNamedSecurityInfo(
            directory,
            SeFileObject,
            OwnerSecurityInformation | DaclSecurityInformation,
            out _,
            out _,
            out _,
            out _,
            out var descriptor);
        if (result != 0)
        {
            throw new Win32Exception(
                checked((int)result),
                $"无法验证受保护目录 {directory} 的 ACL。");
        }
        try
        {
            var length = checked((int)GetSecurityDescriptorLength(descriptor));
            var binary = new byte[length];
            Marshal.Copy(descriptor, binary, 0, length);
            var security = new RawSecurityDescriptor(binary, 0);
            var administrators = new SecurityIdentifier(
                WellKnownSidType.BuiltinAdministratorsSid,
                null);
            if (security.Owner is null ||
                !administrators.Equals(security.Owner) ||
                !security.ControlFlags.HasFlag(
                    ControlFlags.DiscretionaryAclProtected) ||
                security.DiscretionaryAcl is null)
            {
                throw new InvalidDataException(
                    $"受保护目录 {directory} 的所有者或继承状态无效。");
            }

            var expectedMasks = new Dictionary<string, int>(
                StringComparer.Ordinal)
            {
                ["S-1-5-18"] = (int)FileSystemRights.FullControl,
                ["S-1-5-32-544"] = (int)FileSystemRights.FullControl,
                ["S-1-5-32-545"] =
                    (int)(FileSystemRights.ReadAndExecute |
                          FileSystemRights.Synchronize)
            };
            if (security.DiscretionaryAcl.Count != expectedMasks.Count)
            {
                throw new InvalidDataException(
                    $"受保护目录 {directory} 仍包含未知的显式 ACL。");
            }
            foreach (GenericAce genericAce in security.DiscretionaryAcl)
            {
                if (genericAce is not CommonAce ace ||
                    ace.AceQualifier != AceQualifier.AccessAllowed ||
                    ace.SecurityIdentifier is null ||
                    ace.AceFlags.HasFlag(AceFlags.Inherited) ||
                    !ace.AceFlags.HasFlag(AceFlags.ContainerInherit) ||
                    !ace.AceFlags.HasFlag(AceFlags.ObjectInherit) ||
                    !expectedMasks.Remove(
                        ace.SecurityIdentifier.Value,
                        out var expectedMask) ||
                    ace.AccessMask != expectedMask)
                {
                    throw new InvalidDataException(
                        $"受保护目录 {directory} 的 ACL 不符合最小权限模板。");
                }
            }
            if (expectedMasks.Count != 0)
            {
                throw new InvalidDataException(
                    $"受保护目录 {directory} 缺少必须的 ACL。");
            }
        }
        finally
        {
            _ = LocalFree(descriptor);
        }
    }

    // Same stable-handle approval/deployment owner as the original release binaries.
    internal static ArchiveReleaseLease OpenArchiveRelease(string directory,
        IReadOnlyDictionary<string, string>? expected = null) => new(directory, expected ?? ArchiveReleaseManifest.Files);

    internal sealed class ArchiveReleaseLease : IDisposable
    {
        internal readonly List<TrustedBinary> Sources = [];
        internal ArchiveReleaseLease(string directory, IReadOnlyDictionary<string, string> expected)
        {
            try {
                if (expected.Count != ArchiveReleaseManifest.Files.Count ||
                    ArchiveReleaseManifest.Files.Keys.Any(name => !expected.ContainsKey(name)) ||
                    expected.Values.Any(hash => hash is not { Length: 64 } || !hash.All(char.IsAsciiHexDigit)))
                    throw new InvalidDataException("Incomplete archive release approval.");
                foreach (var (name, hash) in expected) Sources.Add(TrustedBinary.Open(Path.Combine(directory, name), hash));
            } catch { Dispose(); throw; }
        }
        public void Dispose() { foreach (var source in Sources) source.Dispose(); Sources.Clear(); }
    }

    private static bool ArchiveReleaseMatches(string directory)
    {
        try { using var lease = OpenArchiveRelease(directory); return true; }
        catch { return false; }
    }

    private static bool RegisteredArchiveReleaseMatches(RegistryKey? key, string directory, string platformHostHash)
    {
        var json = key?.GetValue(ArchiveHashesValueName) as string;
        // A different previously approved host may predate archive deployment. This build's
        // hash binds its new archive approval, so this exception never authorizes a partial current release.
        if (json is null) {
            lock (ProcessImageGate) return _processImage is not null &&
                !string.Equals(platformHostHash, _processImage.Sha256, StringComparison.OrdinalIgnoreCase);
        }
        if (json.Length > 4096) return false;
        try {
            var hashes = System.Text.Json.JsonSerializer.Deserialize<Dictionary<string, string>>(json);
            if (hashes is null) return false;
            using var lease = OpenArchiveRelease(directory, hashes); return true;
        } catch { return false; }
    }

    internal sealed class TrustedBinary : IDisposable
    {
        private readonly FileStream _stream;

        private TrustedBinary(
            FileStream stream,
            string fullPath,
            string sha256)
        {
            _stream = stream;
            FullPath = fullPath;
            Sha256 = sha256;
        }

        public string FullPath { get; }
        public string Sha256 { get; }

        public static TrustedBinary Open(
            string path,
            string? expectedSha256 = null)
        {
            var fullPath = Path.GetFullPath(path);
            var stream = new FileStream(
                fullPath,
                FileMode.Open,
                FileAccess.Read,
                FileShare.Read,
                64 * 1024,
                FileOptions.SequentialScan);
            try
            {
                ValidateStablePathHandle(
                    stream.SafeFileHandle,
                    fullPath,
                    expectDirectory: false);
                var hash = HashStream(stream);
                if (expectedSha256 is not null &&
                    (expectedSha256.Length != 64 ||
                     !expectedSha256.All(char.IsAsciiHexDigit) ||
                     !string.Equals(
                         hash,
                         expectedSha256,
                         StringComparison.OrdinalIgnoreCase)))
                {
                    throw new InvalidDataException(
                        $"{Path.GetFileName(fullPath)} 不属于已授权安装器的同一构建。");
                }
                return new TrustedBinary(stream, fullPath, hash);
            }
            catch
            {
                stream.Dispose();
                throw;
            }
        }

        public void CopyTo(Stream destination)
        {
            _stream.Position = 0;
            _stream.CopyTo(destination, 64 * 1024);
        }

        public void Dispose() => _stream.Dispose();

        private static string HashStream(Stream stream)
        {
            stream.Position = 0;
            var hash = Convert.ToHexString(SHA256.HashData(stream));
            stream.Position = 0;
            return hash;
        }
    }

    internal sealed class ProtectedDirectoryLease : IDisposable
    {
        private readonly List<(string Path, SafeFileHandle Handle)> _directories = [];

        public void Add(string directory)
        {
            var fullPath = Path.GetFullPath(directory);
            var handle = CreateFile(
                fullPath,
                0,
                FileShareRead | FileShareWrite,
                IntPtr.Zero,
                OpenExisting,
                FileFlagBackupSemantics | FileFlagOpenReparsePoint,
                IntPtr.Zero);
            if (handle.IsInvalid)
            {
                handle.Dispose();
                throw new Win32Exception(
                    Marshal.GetLastWin32Error(),
                    $"无法锁定受保护目录 {fullPath}。");
            }
            try
            {
                ValidateStablePathHandle(
                    handle,
                    fullPath,
                    expectDirectory: true);
                _directories.Add((fullPath, handle));
            }
            catch
            {
                handle.Dispose();
                throw;
            }
        }

        public void Validate()
        {
            foreach (var directory in _directories)
            {
                ValidateStablePathHandle(
                    directory.Handle,
                    directory.Path,
                    expectDirectory: true);
                VerifyProtectedDirectoryAcl(directory.Path);
            }
        }

        public void Dispose()
        {
            foreach (var directory in _directories)
            {
                directory.Handle.Dispose();
            }
            _directories.Clear();
        }
    }

    private static void ValidateStablePathHandle(
        SafeFileHandle handle,
        string expectedPath,
        bool expectDirectory)
    {
        if (!GetFileInformationByHandleEx(
                handle,
                FileAttributeTagInfoClass,
                out var attributes,
                (uint)Marshal.SizeOf<FileAttributeTagInfo>()))
        {
            throw new Win32Exception(
                Marshal.GetLastWin32Error(),
                $"无法验证受保护路径 {expectedPath}。");
        }
        var isDirectory =
            (attributes.FileAttributes & FileAttributeDirectory) != 0;
        if ((attributes.FileAttributes & FileAttributeReparsePoint) != 0 ||
            isDirectory != expectDirectory)
        {
            throw new InvalidDataException(
                $"受保护路径 {expectedPath} 不能是重解析点，且类型必须固定。");
        }

        var buffer = new char[32_768];
        var length = GetFinalPathNameByHandle(
            handle,
            buffer,
            (uint)buffer.Length,
            0);
        if (length == 0 || length >= buffer.Length)
        {
            throw new Win32Exception(
                Marshal.GetLastWin32Error(),
                $"无法解析受保护路径 {expectedPath} 的最终位置。");
        }
        var actualPath = NormalizeFinalPath(
            new string(buffer, 0, checked((int)length)));
        if (!string.Equals(
                actualPath,
                NormalizeFinalPath(expectedPath),
                StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException(
                $"受保护路径 {expectedPath} 被重定向到 {actualPath}。");
        }
    }

    private static string NormalizeFinalPath(string path)
    {
        var value = path;
        if (value.StartsWith(@"\\?\UNC\", StringComparison.OrdinalIgnoreCase))
        {
            value = @"\\" + value[8..];
        }
        else if (value.StartsWith(@"\\?\", StringComparison.OrdinalIgnoreCase))
        {
            value = value[4..];
        }
        return Path.GetFullPath(value).TrimEnd(
            Path.DirectorySeparatorChar,
            Path.AltDirectorySeparatorChar);
    }

    private static void RunIcacls(IReadOnlyList<string> arguments)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "icacls.exe",
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        foreach (var argument in arguments)
        {
            startInfo.ArgumentList.Add(argument);
        }
        using var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException(
                "Windows did not start icacls.exe.");
        if (!process.WaitForExit(OperationTimeoutMs))
        {
            process.Kill(entireProcessTree: true);
            throw new TimeoutException("icacls.exe timed out.");
        }
        if (process.ExitCode != 0)
        {
            var detail = process.StandardError.ReadToEnd().Trim();
            if (string.IsNullOrWhiteSpace(detail))
            {
                detail = process.StandardOutput.ReadToEnd().Trim();
            }
            throw new InvalidOperationException(
                $"icacls.exe failed with exit code {process.ExitCode}: {detail}");
        }
    }

    private static bool ServiceExists()
    {
        try
        {
            using var key = Registry.LocalMachine.OpenSubKey(
                $@"SYSTEM\CurrentControlSet\Services\{ServiceName}");
            return key is not null;
        }
        catch (Exception error) when (
            error is UnauthorizedAccessException or
            System.Security.SecurityException or
            IOException)
        {
            // Unknown service state is fail-closed: never turn a registry read
            // failure into a second administrator prompt.
            return true;
        }
    }

    private static async Task<ServiceInstallGate> AcquireServiceInstallGateAsync(
        CancellationToken cancellationToken)
    {
        return await Task.Run(() =>
        {
            Semaphore? semaphore = null;
            try
            {
                semaphore = new Semaphore(1, 1, ServiceInstallMutexName);
                var deadline = Environment.TickCount64 + OperationTimeoutMs;
                while (Environment.TickCount64 < deadline)
                {
                    cancellationToken.ThrowIfCancellationRequested();
                    if (semaphore.WaitOne(100))
                    {
                        var gate = new ServiceInstallGate(semaphore);
                        semaphore = null;
                        return gate;
                    }
                }
                throw new TimeoutException("等待索引服务安装锁超时。");
            }
            finally
            {
                semaphore?.Dispose();
            }
        }, cancellationToken).ConfigureAwait(false);
    }

    private static void WaitForServiceStopped()
    {
        WaitForServiceState(ServiceStopped, "等待旧索引服务停止超时。");
    }

    private static void WaitForServiceRunning()
    {
        WaitForServiceState(ServiceRunning, "等待索引服务启动超时。");
    }

    private static void WaitForServiceState(uint expectedState, string timeoutMessage)
    {
        var deadline = Environment.TickCount64 + OperationTimeoutMs;
        while (Environment.TickCount64 < deadline)
        {
            var state = QueryServiceState();
            if (state == expectedState)
            {
                return;
            }
            if (state is null && expectedState == ServiceStopped)
            {
                return;
            }
            Thread.Sleep(100);
        }
        throw new TimeoutException(timeoutMessage);
    }

    private static uint? QueryServiceState()
    {
        var manager = OpenSCManagerW(null, null, ScManagerConnect);
        if (manager == IntPtr.Zero)
        {
            throw new Win32Exception(Marshal.GetLastWin32Error());
        }
        try
        {
            var service = OpenServiceW(manager, ServiceName, ServiceQueryStatus);
            if (service == IntPtr.Zero)
            {
                var error = Marshal.GetLastWin32Error();
                if (error == ErrorServiceDoesNotExist)
                {
                    return null;
                }
                throw new Win32Exception(error);
            }
            try
            {
                if (!QueryServiceStatus(service, out var status))
                {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                return status.CurrentState;
            }
            finally
            {
                CloseServiceHandle(service);
            }
        }
        finally
        {
            CloseServiceHandle(manager);
        }
    }

    private const uint ScManagerConnect = 0x0001;
    private const uint ServiceQueryStatus = 0x0004;
    private const uint ServiceStopped = 0x00000001;
    private const uint ServiceRunning = 0x00000004;
    private const int ErrorServiceDoesNotExist = 1060;
    private const uint FileShareRead = 0x00000001;
    private const uint FileShareWrite = 0x00000002;
    private const uint OpenExisting = 3;
    private const uint FileAttributeDirectory = 0x00000010;
    private const uint FileAttributeReparsePoint = 0x00000400;
    private const uint FileFlagOpenReparsePoint = 0x00200000;
    private const uint FileFlagBackupSemantics = 0x02000000;
    private const int FileAttributeTagInfoClass = 9;
    private const int SeFileObject = 1;
    private const uint OwnerSecurityInformation = 0x00000001;
    private const uint DaclSecurityInformation = 0x00000004;

    [StructLayout(LayoutKind.Sequential)]
    private struct ServiceStatus
    {
        public uint ServiceType;
        public uint CurrentState;
        public uint ControlsAccepted;
        public uint Win32ExitCode;
        public uint ServiceSpecificExitCode;
        public uint CheckPoint;
        public uint WaitHint;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct FileAttributeTagInfo
    {
        public uint FileAttributes;
        public uint ReparseTag;
    }

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr OpenSCManagerW(string? machineName, string? databaseName, uint desiredAccess);

    [DllImport(
        "kernel32.dll",
        EntryPoint = "CreateFileW",
        CharSet = CharSet.Unicode,
        SetLastError = true)]
    private static extern SafeFileHandle CreateFile(
        string fileName,
        uint desiredAccess,
        uint shareMode,
        IntPtr securityAttributes,
        uint creationDisposition,
        uint flagsAndAttributes,
        IntPtr templateFile);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetFileInformationByHandleEx(
        SafeFileHandle file,
        int fileInformationClass,
        out FileAttributeTagInfo fileInformation,
        uint bufferSize);

    [DllImport(
        "kernel32.dll",
        EntryPoint = "GetFinalPathNameByHandleW",
        CharSet = CharSet.Unicode,
        SetLastError = true)]
    private static extern uint GetFinalPathNameByHandle(
        SafeFileHandle file,
        [Out] char[] filePath,
        uint filePathLength,
        uint flags);

    [DllImport(
        "advapi32.dll",
        EntryPoint = "GetNamedSecurityInfoW",
        CharSet = CharSet.Unicode)]
    private static extern uint GetNamedSecurityInfo(
        string objectName,
        int objectType,
        uint securityInfo,
        out IntPtr owner,
        out IntPtr group,
        out IntPtr dacl,
        out IntPtr sacl,
        out IntPtr securityDescriptor);

    [DllImport("advapi32.dll")]
    private static extern uint GetSecurityDescriptorLength(
        IntPtr securityDescriptor);

    [DllImport("kernel32.dll")]
    private static extern IntPtr LocalFree(IntPtr memory);

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr OpenServiceW(IntPtr manager, string serviceName, uint desiredAccess);

    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool QueryServiceStatus(IntPtr service, out ServiceStatus status);

    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseServiceHandle(IntPtr handle);

    internal static bool RegisteredImagePathMatches(string imagePath, string expectedIndexerPath)
    {
        var value = imagePath.Trim();
        string executablePath;
        string arguments;
        if (value.StartsWith('"'))
        {
            var end = value.IndexOf('"', 1);
            if (end <= 1)
            {
                return false;
            }
            executablePath = value[1..end];
            arguments = value[(end + 1)..].Trim();
        }
        else
        {
            var exe = value.IndexOf(".exe", StringComparison.OrdinalIgnoreCase);
            if (exe < 0)
            {
                return false;
            }
            executablePath = value[..(exe + 4)];
            arguments = value[(exe + 4)..].Trim();
        }
        return string.Equals(
                Path.GetFullPath(executablePath),
                Path.GetFullPath(expectedIndexerPath),
                StringComparison.OrdinalIgnoreCase) &&
            string.Equals(arguments, "--service", StringComparison.OrdinalIgnoreCase);
    }

    private static bool TryStartService(IPlatformLog log, Guid correlationId)
    {
        var started = Stopwatch.GetTimestamp();
        try
        {
            RunSc(["start", ServiceName], [0, 1056]);
            WaitForServiceRunning();
            return true;
        }
        catch (Exception error)
        {
            try
            {
                WriteDeploymentLog(log, correlationId, "start_failed", started,
                    new WindowsCommandResult(false, NativeErrorCode:
                        error is Win32Exception native ? native.NativeErrorCode : error.HResult));
            }
            catch { /* Diagnostics cannot change the service start result. */ }
            return false;
        }
    }

    private static string RunSc(
        IReadOnlyList<string> arguments,
        IReadOnlyCollection<int> acceptedExitCodes)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "sc.exe",
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        foreach (var argument in arguments)
        {
            startInfo.ArgumentList.Add(argument);
        }
        using var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException("Windows did not start sc.exe.");
        if (!process.WaitForExit(OperationTimeoutMs))
        {
            process.Kill(entireProcessTree: true);
            throw new TimeoutException($"sc.exe {arguments[0]} timed out.");
        }
        if (!acceptedExitCodes.Contains(process.ExitCode))
        {
            var detail = process.StandardError.ReadToEnd().Trim();
            if (string.IsNullOrWhiteSpace(detail))
            {
                detail = process.StandardOutput.ReadToEnd().Trim();
            }
            throw new InvalidOperationException(
                $"sc.exe {arguments[0]} failed with exit code {process.ExitCode}: {detail}");
        }
        return process.StandardOutput.ReadToEnd().Trim();
    }
}
