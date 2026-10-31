using System.Security.Cryptography;

namespace NxTools.PlatformHost.Windows.Archive;

// Selection is fixed at construction, never from command data/environment/PATH/current directory.
internal sealed class ArchiveReadHelper
{
    internal const string DeploymentName = "NxTools.ArchiveWorker.exe";
    internal string Executable { get; }
    internal string Argument { get; }
    private readonly byte[]? fixtureHash;
    private string? releaseHash;
    private ArchiveReadHelper(string executable, string argument = "--archive-worker", byte[]? fixtureHash = null)
    { Executable = executable; Argument = argument; this.fixtureHash = fixtureHash; }
    internal static ArchiveReadHelper Production()
    {
        var directory = Path.GetFullPath(AppContext.BaseDirectory).TrimEnd('\\') + "\\";
#if DEBUG
        const string debugSuffix = "csharp\\NxTools.PlatformHost\\bin\\Debug\\net8.0-windows\\win-x64\\";
        // Only the exact source-layout Debug apphost gets the Cargo-name mapping.
        if (directory.EndsWith(debugSuffix, StringComparison.OrdinalIgnoreCase) &&
            string.Equals(Path.GetFullPath(Environment.ProcessPath ?? ""), Path.Combine(directory, "NxTools.PlatformHost.exe"), StringComparison.OrdinalIgnoreCase))
            return new(Path.Combine(directory[..^debugSuffix.Length], "target", "debug", "nxtools-archive-worker.exe"));
#endif
        return new(Path.Combine(directory, DeploymentName)) { releaseHash = Generated.ArchiveReleaseManifest.Files[DeploymentName] };
    }
    internal static ArchiveReadHelper OwnedPackage(string root, bool fixture = false)
    {
        GuardOwnedRoot(root);
        var executable = Path.Combine(Path.GetFullPath(root), DeploymentName);
        byte[]? fingerprint = null;
        if (fixture) {
            using var expected = File.OpenRead(Environment.ProcessPath!);
            fingerprint = SHA256.HashData(expected);
        }
        var helper = new ArchiveReadHelper(executable, fixture ? "--archive-product-worker-fixture" : "--archive-worker", fingerprint);
        if (fixture) { using var verified = helper.Acquire(); }
        return helper;
    }
    internal static void GuardOwnedRoot(string root)
    {
        if (!string.Equals(Path.GetFileName(Environment.ProcessPath), "NxTools.PlatformHost.Tests.exe", StringComparison.OrdinalIgnoreCase) ||
            !Path.IsPathFullyQualified(root) || !Directory.Exists(root)) throw new ArgumentException("Invalid owned package.");
        var full = Path.GetFullPath(root).TrimEnd('\\');
        var temporary = Path.GetFullPath(Path.GetTempPath()).TrimEnd('\\') + "\\";
        var scratch = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".pi-desktop", "scratch") + "\\";
        // The Rust pipe tracer owns a GUID root with a literal package child; no arbitrary parent search.
        var owner = Path.GetFileName(full) == "package" ? Path.GetDirectoryName(full)! : full;
        var leaf = Path.GetFileName(owner);
        var sessionScratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        if (sessionScratch is not null && string.Equals(Path.GetDirectoryName(owner), Path.GetFullPath(sessionScratch).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase) &&
            leaf.StartsWith("hz5-", StringComparison.Ordinal) && Guid.TryParseExact(leaf[4..], "D", out var recoveryFixture) && recoveryFixture != Guid.Empty) return;
        if ((!full.StartsWith(temporary, StringComparison.OrdinalIgnoreCase) && !full.StartsWith(scratch, StringComparison.OrdinalIgnoreCase)) ||
            !leaf.StartsWith("archive-product-", StringComparison.Ordinal) || leaf.Length < 36 || !Guid.TryParse(leaf[^36..], out _))
            throw new ArgumentException("Package must be an owned archive-product GUID root under temporary storage.");
    }
    internal IDisposable Acquire()
    {
        var lease = Acquire(Executable);
        try {
            // Revalidate under the same no-reparse/write-denying lease transferred to the Run.
            if (releaseHash is not null) {
                try { IndexerServiceInstaller.VerifiedPackagedBinaryHashMatches(Executable, releaseHash); }
                catch (Exception failure) when (failure is IOException or InvalidDataException or UnauthorizedAccessException)
                    { throw new ArchiveOutputException("helper_untrusted", failure.HResult); }
            }
            if (fixtureHash is not null) {
                using var actual = File.OpenRead(Executable);
                if (!CryptographicOperations.FixedTimeEquals(fixtureHash, SHA256.HashData(actual))) throw new ArchiveOutputException("helper_untrusted");
            }
            return lease;
        } catch { lease.Dispose(); throw; }
    }
    private static ArchiveSourceLease Acquire(string executable)
    {
        try { return ArchiveSourceLease.Open(executable); }
        catch (ArchiveOutputException failure)
        {
            throw new ArchiveOutputException(failure.NativeCode is 2 or 3 ? "helper_missing" : "helper_untrusted", failure.NativeCode);
        }
    }
}
