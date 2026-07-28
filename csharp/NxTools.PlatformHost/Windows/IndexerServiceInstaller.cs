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

internal static class IndexerServiceInstaller
{
    internal const string ServiceName = "NxToolsIndexerV1";
    private const int OperationTimeoutMs = 30_000;
    private const int ProtectedProcessExitTimeoutMs = 5_000;
    private const string BinaryHashValueName = "NxToolsBinarySha256";
    private const string DesktopHashValueName = "NxToolsDesktopSha256";
    private const string PlatformHostHashValueName = "NxToolsPlatformHostSha256";
    private const string DesktopPathValueName = "NxToolsDesktopPath";
    private const string ServiceSddl =
        "D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)" +
        "(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)" +
        "(A;;CCLCSWLOCRRC;;;IU)(A;;CCLCSWLOCRRC;;;SU)" +
        "(A;;CCLCSWRPWPLORC;;;AU)";
    private static readonly object ProcessImageGate = new();
    private static TrustedBinary? _processImage;

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
        IPlatformLog log,
        CancellationToken cancellationToken)
    {
        try
        {
            var hostPath = Environment.ProcessPath
                ?? throw new InvalidOperationException("Cannot locate NxTools.PlatformHost.exe.");
            var indexerPath = ValidatePackagedIndexerPath(
                hostPath,
                Path.Combine(AppContext.BaseDirectory, "NxTools.Indexer.exe"));

            if (IsInstalledAt(indexerPath) && TryStartService(log))
            {
                return new WindowsCommandResult(true, Data: "running");
            }

            var startInfo = new ProcessStartInfo
            {
                FileName = hostPath,
                Verb = "runas",
                UseShellExecute = true,
                WindowStyle = ProcessWindowStyle.Hidden
            };
            startInfo.ArgumentList.Add("--install-indexer-service");
            startInfo.ArgumentList.Add(indexerPath);
            using var installer = Process.Start(startInfo)
                ?? throw new InvalidOperationException("Windows did not start the elevated service installer.");
            using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            timeout.CancelAfter(OperationTimeoutMs);
            await installer.WaitForExitAsync(timeout.Token);
            if (installer.ExitCode != 0)
            {
                return new WindowsCommandResult(
                    false,
                    $"索引服务安装失败（退出码 {installer.ExitCode}）。");
            }
            return IsInstalledAt(indexerPath)
                ? new WindowsCommandResult(true, Data: "installed")
                : new WindowsCommandResult(false, "安装程序结束，但服务注册路径未通过验证。");
        }
        catch (Win32Exception error) when (error.NativeErrorCode == 1223)
        {
            return new WindowsCommandResult(false, "已取消索引服务安装授权。");
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return new WindowsCommandResult(false, "索引服务安装等待超时。");
        }
        catch (Exception error) when (error is IOException or InvalidDataException or InvalidOperationException or Win32Exception)
        {
            log.Write("error", "indexer_service.ensure_failed", fields: new Dictionary<string, object?>
            {
                ["exception_type"] = error.GetType().FullName,
                ["message"] = error.Message
            });
            return new WindowsCommandResult(false, error.Message);
        }
    }

    public static int InstallElevated(string indexerPath, IPlatformLog log)
    {
        try
        {
            var hostPath = Environment.ProcessPath
                ?? throw new InvalidOperationException("Cannot locate NxTools.PlatformHost.exe.");
            indexerPath = ValidatePackagedIndexerPath(hostPath, indexerPath);
            var desktopPath = ValidatePackagedSibling(hostPath, "NxTools.exe");
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

            var serviceExists = ServiceExists();
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
            StopProtectedProcesses(
                [protectedDesktopPath, protectedPlatformHostPath],
                Environment.ProcessId,
                log);
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
            RunSc([
                RegistrationVerb(serviceExists), ServiceName,
                "binPath=", $"\"{protectedIndexerPath}\" --service",
                "start=", "auto",
                "obj=", "LocalSystem",
                "DisplayName=", "NxTools File Indexer"
            ], [0]);
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
                key.SetValue(
                    DesktopPathValueName,
                    protectedDesktopPath,
                    RegistryValueKind.String);
            }
            protectedDirectories.Validate();
            RunSc(["start", ServiceName], [0, 1056]);
            WaitForServiceRunning();
            log.Write("info", "indexer_service.installed", fields: new Dictionary<string, object?>
            {
                ["service_name"] = ServiceName
            });
            return 0;
        }
        catch (Exception error)
        {
            log.Write("error", "indexer_service.install_failed", fields: new Dictionary<string, object?>
            {
                ["exception_type"] = error.GetType().FullName,
                ["message"] = error.Message
            });
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
            registration.StartMode == 2 &&
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

    internal static bool BinaryHashMatches(string registeredHash, string indexerPath) =>
        string.Equals(
            registeredHash,
            ComputeBinaryHash(indexerPath),
            StringComparison.OrdinalIgnoreCase);

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
        IPlatformLog log)
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
                        log.Write(
                            "warning",
                            "indexer_service.protected_process_inspection_failed",
                            fields: new Dictionary<string, object?>
                            {
                                ["process_id"] = process.Id,
                                ["process_name"] = processName,
                                ["message"] = error.Message
                            });
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
                        log.Write(
                            "info",
                            "indexer_service.protected_process_stopped",
                            fields: new Dictionary<string, object?>
                            {
                                ["process_id"] = process.Id,
                                ["path"] = processPath
                            });
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

    private sealed class TrustedBinary : IDisposable
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

    private sealed class ProtectedDirectoryLease : IDisposable
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
        using var key = Registry.LocalMachine.OpenSubKey(
            $@"SYSTEM\CurrentControlSet\Services\{ServiceName}");
        return key is not null;
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

    private static bool TryStartService(IPlatformLog log)
    {
        try
        {
            RunSc(["start", ServiceName], [0, 1056]);
            WaitForServiceRunning();
            return true;
        }
        catch (Exception error)
        {
            log.Write("warning", "indexer_service.start_failed", fields: new Dictionary<string, object?>
            {
                ["message"] = error.Message
            });
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
