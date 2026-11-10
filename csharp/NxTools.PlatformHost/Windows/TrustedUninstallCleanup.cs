using System.ComponentModel;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Security.Principal;
using System.ServiceProcess;
using System.Text.Json;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Generated;

namespace NxTools.PlatformHost.Windows;

// Privileged uninstall is admitted only by the current signed package and the complete fixed protected owners.
internal static class TrustedUninstallCleanup
{
    internal sealed record IndexerRegistration(
        string? ImagePath, string? Account, int StartMode, string SecurityDescriptor, string? DesktopPath,
        string? IndexerHash, string? DesktopHash, string? PlatformHostHash, string? BrowserHostHash,
        string? ArchiveHashesJson);

    internal sealed record UpdaterRegistration(
        string? ImagePath, string? Account, int StartMode, string SecurityDescriptor,
        string? KeyHash, string? BootstrapHash);

    internal sealed record Operations(
        Func<bool> IsAdministrator,
        Func<IndexerRegistration?> ReadIndexer,
        Func<UpdaterRegistration?> ReadUpdater,
        Func<IDisposable> EnterDeploymentGate,
        Action<string> StopService,
        Action<IReadOnlyCollection<string>> StopProtectedProcesses,
        Action<string> DeleteService,
        Action<string> DeleteTree);

    internal static int Run(IPlatformLog log, Guid correlation, string installRoot)
    {
        var started = Stopwatch.GetTimestamp();
        try
        {
            var package = Path.GetFullPath(installRoot);
            var caller = Environment.ProcessPath ?? throw new InvalidOperationException("PlatformHost image unavailable.");
            if (!SamePath(AppContext.BaseDirectory, package) ||
                !SamePath(Path.GetDirectoryName(Path.GetFullPath(caller)), package))
                throw new InvalidDataException("Privileged uninstall caller is not the installed package.");
            var operations = new Operations(
                () => new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator),
                ReadIndexerRegistration, ReadUpdaterRegistration, TrustedDeploymentGate.Acquire, StopService,
                paths => IndexerServiceInstaller.StopProtectedProcesses(paths, Environment.ProcessId, log, correlation),
                DeleteService,
                path =>
                {
                    TrustedUpdateTransaction.RejectReparseTree(path);
                    Directory.Delete(path, true);
                });
            return Run(log, correlation, package, caller, IndexerServiceInstaller.ProtectedProductDirectory(),
                Convert.FromBase64String(UpdateTrust.PublicKeySpkiBase64), operations);
        }
        catch (Exception failure) when (failure is not OutOfMemoryException)
        {
            Trace(log, correlation, "failed", started, "entry",
                failure is Win32Exception native ? native.NativeErrorCode : failure.HResult);
            return 1;
        }
    }

    internal static int Run(IPlatformLog log, Guid correlation, string package, string caller,
        string productRoot, byte[] publicKey, Operations operations)
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
            if (!SamePath(package, Path.GetDirectoryName(Path.GetFullPath(caller))!))
                throw new InvalidDataException("Uninstall caller is not the signed installed package.");

            var serviceRoot = Path.Combine(Path.GetFullPath(productRoot), "Service");
            var updaterRoot = Path.Combine(Path.GetFullPath(productRoot), "Updater");
            stage = "initial_ownership";
            var indexer = operations.ReadIndexer();
            var updater = operations.ReadUpdater();
            var serviceRootExists = RootExists(serviceRoot, "indexer");
            var updaterRootExists = RootExists(updaterRoot, "updater");
            if (indexer is null && updater is null && !serviceRootExists && !updaterRootExists)
            {
                Trace(log, correlation, "completed", started);
                return 0;
            }
            ValidateState(release, publicKey, serviceRoot, updaterRoot, indexer, updater,
                serviceRootExists, updaterRootExists, log);

            stage = "deployment_gate";
            using (operations.EnterDeploymentGate())
            {
                stage = "revalidation";
                var currentIndexer = operations.ReadIndexer();
                var currentUpdater = operations.ReadUpdater();
                var currentServiceRootExists = RootExists(serviceRoot, "indexer");
                var currentUpdaterRootExists = RootExists(updaterRoot, "updater");
                if (currentIndexer != indexer || currentUpdater != updater ||
                    currentServiceRootExists != serviceRootExists || currentUpdaterRootExists != updaterRootExists)
                    throw new InvalidDataException("Protected service registration or root changed before uninstall.");
                ValidateState(release, publicKey, serviceRoot, updaterRoot, currentIndexer, currentUpdater,
                    currentServiceRootExists, currentUpdaterRootExists, log);

                stage = "service_stop";
                if (currentIndexer is not null) operations.StopService(IndexerServiceInstaller.ServiceName);
                if (currentUpdater is not null) operations.StopService(TrustedUpdateBroker.ServiceName);
                if (currentServiceRootExists)
                {
                    var executables = release.Files
                        .Where(file => file.Path.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
                        .Select(file => Path.Combine(serviceRoot, file.Path.Replace('/', Path.DirectorySeparatorChar)))
                        .ToArray();
                    operations.StopProtectedProcesses(executables);
                }

                stage = "post_stop_revalidation";
                if (operations.ReadIndexer() != currentIndexer || operations.ReadUpdater() != currentUpdater ||
                    RootExists(serviceRoot, "indexer") != currentServiceRootExists ||
                    RootExists(updaterRoot, "updater") != currentUpdaterRootExists)
                    throw new InvalidDataException("Protected service registration or root changed during uninstall.");
                ValidateState(release, publicKey, serviceRoot, updaterRoot, currentIndexer, currentUpdater,
                    currentServiceRootExists, currentUpdaterRootExists, log);

                stage = "service_delete";
                if (currentIndexer is not null) operations.DeleteService(IndexerServiceInstaller.ServiceName);
                if (currentUpdater is not null) operations.DeleteService(TrustedUpdateBroker.ServiceName);
                if (operations.ReadIndexer() is not null || operations.ReadUpdater() is not null)
                    throw new InvalidDataException("Protected service registration deletion was not confirmed.");

                stage = "deployment_delete";
                if (currentServiceRootExists) operations.DeleteTree(serviceRoot);
                if (currentUpdaterRootExists) operations.DeleteTree(updaterRoot);
            }
            Trace(log, correlation, "completed", started);
            return 0;
        }
        catch (Exception failure) when (failure is not OutOfMemoryException)
        {
            Trace(log, correlation, "failed", started, stage,
                failure is Win32Exception native ? native.NativeErrorCode : failure.HResult);
            return 1;
        }
    }

    private static bool RootExists(string root, string owner)
    {
        try
        {
            var attributes = File.GetAttributes(root);
            if ((attributes & FileAttributes.Directory) == 0)
                throw new InvalidDataException(owner + " root is not a directory.");
            return true;
        }
        catch (FileNotFoundException) { return false; }
        catch (DirectoryNotFoundException) { return false; }
    }

    private static void ValidateState(VerifiedRelease release, byte[] publicKey, string serviceRoot,
        string updaterRoot, IndexerRegistration? indexer, UpdaterRegistration? updater,
        bool serviceRootExists, bool updaterRootExists, IPlatformLog log)
    {
        if (indexer is not null)
        {
            if (!serviceRootExists) throw new InvalidDataException("Indexer registration has no protected release root.");
            ValidateIndexer(release, serviceRoot, indexer);
        }
        else if (serviceRootExists)
        {
            ValidateExactTree(serviceRoot, release.Files);
        }

        if (updater is not null) ValidateUpdaterRegistration(release, publicKey, updaterRoot, updater);
        if (updaterRootExists)
            ValidateUpdaterRoot(release, publicKey, updaterRoot, log);
        else if (updater is null)
            return;
        // An exact updater registration with the whole fixed root absent is a supported
        // uninstall state; its signed registration identity remains the deletion authority.
    }

    private static void ValidateIndexer(VerifiedRelease release, string root, IndexerRegistration registration)
    {
        var expected = release.Files.ToDictionary(file => file.Path, StringComparer.OrdinalIgnoreCase);
        var archive = release.Files.Where(file => ArchiveReleaseManifest.Files.ContainsKey(file.Path))
            .ToDictionary(file => file.Path, file => file.Sha256, StringComparer.Ordinal);
        Dictionary<string, string>? registeredArchive = null;
        if (registration.ArchiveHashesJson is { Length: <= 4096 } json)
            registeredArchive = JsonSerializer.Deserialize<Dictionary<string, string>>(json);
        if (!IndexerServiceInstaller.RegisteredImagePathMatches(registration.ImagePath ?? "",
                Path.Combine(root, "NxTools.Indexer.exe")) ||
            !string.Equals(registration.Account, "LocalSystem", StringComparison.OrdinalIgnoreCase) ||
            registration.StartMode is not (2 or 3) ||
            !registration.SecurityDescriptor.Contains("(A;;CCLCSWRPWPLORC;;;AU)", StringComparison.Ordinal) ||
            !SamePath(registration.DesktopPath, Path.Combine(root, "NxTools.exe")) ||
            !SameHash(registration.IndexerHash, expected["NxTools.Indexer.exe"].Sha256) ||
            !SameHash(registration.DesktopHash, expected["NxTools.exe"].Sha256) ||
            !SameHash(registration.PlatformHostHash, expected["NxTools.PlatformHost.exe"].Sha256) ||
            !SameHash(registration.BrowserHostHash, expected["NxTools.BrowserHost.exe"].Sha256) ||
            registeredArchive is null || registeredArchive.Count != archive.Count ||
            archive.Any(item => !registeredArchive.TryGetValue(item.Key, out var hash) || !SameHash(hash, item.Value)))
            throw new InvalidDataException("Indexer registration is not the signed protected release.");
        ValidateExactTree(root, release.Files);
    }

    private static void ValidateUpdaterRegistration(VerifiedRelease release, byte[] publicKey, string root,
        UpdaterRegistration registration)
    {
        var bootstrap = Path.Combine(root, "Bootstrap", "NxTools.PlatformHost.exe");
        var expectedBootstrap = release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe");
        if (!string.Equals(registration.ImagePath, $"\"{bootstrap}\" --trusted-update-service",
                StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(registration.Account, "LocalSystem", StringComparison.OrdinalIgnoreCase) ||
            registration.StartMode is not (2 or 3) ||
            !registration.SecurityDescriptor.Contains("(A;;CCLCSWRPLORC;;;AU)", StringComparison.Ordinal) ||
            !SameHash(registration.KeyHash, Convert.ToHexString(SHA256.HashData(publicKey))) ||
            !SameHash(registration.BootstrapHash, expectedBootstrap.Sha256))
            throw new InvalidDataException("Updater registration is not the signed protected release.");
    }

    private static void ValidateUpdaterRoot(VerifiedRelease release, byte[] publicKey, string root,
        IPlatformLog log)
    {
        var bootstrap = Path.Combine(root, "Bootstrap", "NxTools.PlatformHost.exe");
        var expectedBootstrap = release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe");
        TrustedUpdateTransaction.RejectReparseTree(root);
        using (var binary = IndexerServiceInstaller.TrustedBinary.Open(bootstrap, expectedBootstrap.Sha256)) { }
        if (!new TrustedUpdateEnrollment(root).IsComplete())
            throw new InvalidDataException("Updater enrollment is incomplete.");
        var releases = Path.Combine(root, "Releases");
        var transaction = new TrustedUpdateTransaction(releases, publicKey, new NoDeployment(), log);
        var state = transaction.ReadState();
        if (state.DeploymentPending || state.Sequence != release.Manifest.ReleaseSequence ||
            state.ReleaseId != release.Manifest.ReleaseId || state.ManifestSha256 != release.ManifestSha256)
            throw new InvalidDataException("Updater release state is not the installed signed release.");
        if (File.Exists(Path.Combine(releases, "transaction.json")) ||
            Directory.Exists(Path.Combine(releases, "pending")) || Directory.Exists(Path.Combine(releases, "backup")) ||
            File.Exists(Path.Combine(root, "bootstrap-maintenance.json")) ||
            File.Exists(Path.Combine(root, "Bootstrap", "NxTools.PlatformHost.maintenance-old")) ||
            File.Exists(Path.Combine(root, "Bootstrap", "NxTools.PlatformHost.maintenance-new")))
            throw new InvalidOperationException("Protected update recovery must finish before uninstall.");
        using (var active = TrustedReleaseManifest.VerifyRecoverySnapshot(Path.Combine(releases, "active"), publicKey,
                   state.Sequence, state.ReleaseId, state.ManifestSha256)) { }
        var maintenanceState = Path.Combine(root, "bootstrap-maintenance-state.json");
        if (File.Exists(maintenanceState))
        {
            using var document = JsonDocument.Parse(File.ReadAllBytes(maintenanceState));
            var value = document.RootElement;
            if (value.GetProperty("ProtocolVersion").GetInt32() != 1 ||
                value.GetProperty("Sequence").GetInt64() != release.Manifest.ReleaseSequence ||
                value.GetProperty("ReleaseId").GetGuid() != release.Manifest.ReleaseId ||
                value.GetProperty("ManifestSha256").GetString() != release.ManifestSha256 ||
                !SameHash(value.GetProperty("BootstrapSha256").GetString(), expectedBootstrap.Sha256))
                throw new InvalidDataException("Updater maintenance state is not the installed signed release.");
        }
        ValidateUpdaterEntries(root, release);
    }

    private static void ValidateUpdaterEntries(string root, VerifiedRelease release)
    {
        var allowedRoot = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        { "Bootstrap", "Releases", "enrollment.json", "bootstrap-maintenance-state.json", "bootstrap-maintenance.lock" };
        var actualRoot = Directory.EnumerateFileSystemEntries(root).Select(path => Path.GetFileName(path)).ToArray();
        if (actualRoot.Any(name => name is null || !allowedRoot.Contains(name)))
            throw new InvalidDataException("Updater root contains unowned state.");
        var bootstrapEntries = Directory.EnumerateFileSystemEntries(Path.Combine(root, "Bootstrap"))
            .Select(path => Path.GetFileName(path)).ToArray();
        if (bootstrapEntries.Length != 1 || !string.Equals(bootstrapEntries[0], "NxTools.PlatformHost.exe", StringComparison.OrdinalIgnoreCase))
            throw new InvalidDataException("Updater bootstrap root contains unowned state.");
        var releases = Path.Combine(root, "Releases");
        var releaseEntries = Directory.EnumerateFileSystemEntries(releases).Select(path => Path.GetFileName(path)).ToArray();
        if (releaseEntries.Any(name => name is null ||
                !name.Equals("active", StringComparison.OrdinalIgnoreCase) &&
                !name.Equals("state.json", StringComparison.OrdinalIgnoreCase)) ||
            !releaseEntries.Contains("active", StringComparer.OrdinalIgnoreCase) ||
            !releaseEntries.Contains("state.json", StringComparer.OrdinalIgnoreCase))
            throw new InvalidDataException("Updater release root contains unowned state.");
        var activeExpected = release.Files.Select(file => file.Path)
            .Append(TrustedReleaseManifest.ManifestName).Append(TrustedReleaseManifest.SignatureName)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        var activeRoot = Path.Combine(releases, "active");
        var activeActual = Directory.EnumerateFiles(activeRoot, "*", SearchOption.AllDirectories)
            .Select(path => Path.GetRelativePath(activeRoot, path).Replace(Path.DirectorySeparatorChar, '/'))
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        if (!activeActual.SetEquals(activeExpected))
            throw new InvalidDataException("Updater active snapshot contains unowned files.");
    }

    private static void ValidateExactTree(string root, IReadOnlyList<ReleaseFile> files)
    {
        TrustedUpdateTransaction.RejectReparseTree(root);
        var expected = files.ToDictionary(file => file.Path, StringComparer.OrdinalIgnoreCase);
        var actual = Directory.EnumerateFiles(root, "*", SearchOption.AllDirectories)
            .Select(path => Path.GetRelativePath(root, path).Replace(Path.DirectorySeparatorChar, '/'))
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        if (!actual.SetEquals(expected.Keys)) throw new InvalidDataException("Protected service root contains unowned files.");
        foreach (var file in files)
        {
            var path = Path.Combine(root, file.Path.Replace('/', Path.DirectorySeparatorChar));
            if (new FileInfo(path).Length != file.Size) throw new InvalidDataException("Protected service file size changed.");
            using var binary = IndexerServiceInstaller.TrustedBinary.Open(path, file.Sha256);
        }
    }

    private static bool SamePath(string? actual, string expected) => actual is not null &&
        string.Equals(Path.GetFullPath(actual).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
            Path.GetFullPath(expected).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar), StringComparison.OrdinalIgnoreCase);
    private static bool SameHash(string? actual, string expected) => actual is { Length: 64 } &&
        string.Equals(actual, expected, StringComparison.OrdinalIgnoreCase);

    private sealed class NoDeployment : TrustedUpdateTransaction.IDeployment
    {
        public Task DeployAsync(string verifiedPackage, Guid correlationId, CancellationToken cancellationToken) =>
            throw new InvalidOperationException("Uninstall never performs update recovery.");
    }

    private static IndexerRegistration? ReadIndexerRegistration()
    {
        using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{IndexerServiceInstaller.ServiceName}");
        if (key is null) return null;
        return new(
            key.GetValue("ImagePath") as string, key.GetValue("ObjectName") as string,
            Convert.ToInt32(key.GetValue("Start") ?? -1), RunSc(["sdshow", IndexerServiceInstaller.ServiceName], [0]),
            key.GetValue("NxToolsDesktopPath") as string, key.GetValue("NxToolsBinarySha256") as string,
            key.GetValue("NxToolsDesktopSha256") as string, key.GetValue("NxToolsPlatformHostSha256") as string,
            key.GetValue("NxToolsBrowserHostSha256") as string, key.GetValue("NxToolsArchiveReleaseSha256") as string);
    }

    private static UpdaterRegistration? ReadUpdaterRegistration()
    {
        using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{TrustedUpdateBroker.ServiceName}");
        if (key is null) return null;
        return new(
            key.GetValue("ImagePath") as string, key.GetValue("ObjectName") as string,
            Convert.ToInt32(key.GetValue("Start") ?? -1), RunSc(["sdshow", TrustedUpdateBroker.ServiceName], [0]),
            key.GetValue("NxToolsUpdateKeySha256") as string, key.GetValue("NxToolsBootstrapSha256") as string);
    }

    private static void StopService(string name)
    {
        if (name is not (IndexerServiceInstaller.ServiceName or TrustedUpdateBroker.ServiceName))
            throw new InvalidOperationException("Unknown protected service.");
        using var service = new ServiceController(name);
        service.Refresh();
        if (service.Status == ServiceControllerStatus.Stopped) return;
        service.Stop();
        service.WaitForStatus(ServiceControllerStatus.Stopped, TimeSpan.FromMinutes(4) + TimeSpan.FromSeconds(15));
    }

    private static void DeleteService(string name)
    {
        if (name is not (IndexerServiceInstaller.ServiceName or TrustedUpdateBroker.ServiceName))
            throw new InvalidOperationException("Unknown protected service.");
        RunSc(["delete", name], [0]);
        var deadline = Environment.TickCount64 + 30_000;
        while (Environment.TickCount64 < deadline)
        {
            using var key = Registry.LocalMachine.OpenSubKey($@"SYSTEM\CurrentControlSet\Services\{name}");
            if (key is null) return;
            Thread.Sleep(100);
        }
        throw new System.TimeoutException("Protected service deletion was not confirmed.");
    }

    private static string RunSc(IReadOnlyList<string> arguments, IReadOnlyCollection<int> accepted)
    {
        var start = new ProcessStartInfo("sc.exe")
        {
            UseShellExecute = false, CreateNoWindow = true,
            RedirectStandardOutput = true, RedirectStandardError = true
        };
        foreach (var argument in arguments) start.ArgumentList.Add(argument);
        using var process = Process.Start(start) ?? throw new InvalidOperationException("Windows did not start sc.exe.");
        if (!process.WaitForExit(30_000))
        {
            process.Kill(entireProcessTree: true);
            throw new System.TimeoutException("Protected service command timed out.");
        }
        if (!accepted.Contains(process.ExitCode))
            throw new Win32Exception(process.ExitCode, "Protected service command failed.");
        return process.StandardOutput.ReadToEnd().Trim();
    }

    private static void Trace(IPlatformLog log, Guid correlation, string state, long started,
        string? stage = null, int? native = null)
    {
        try
        {
            log.Write(state == "failed" ? "error" : "info", "installer.protected_uninstall." + state,
                correlation, new Dictionary<string, object?>
                {
                    ["stage"] = stage, ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                    ["native_error_code"] = native
                });
        }
        catch { /* Diagnostics never change privileged uninstall outcome. */ }
    }
}
