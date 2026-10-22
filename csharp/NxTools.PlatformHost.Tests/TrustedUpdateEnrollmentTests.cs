using System.Security.Cryptography;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class TrustedUpdateEnrollmentTests
{
    internal static Task PartialEnrollmentRetainsTrustAndRetries()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(), "enrollment-" + Guid.NewGuid().ToString("N"));
        try
        {
            var identity = new TrustedUpdateEnrollment.Identity(new string('A', 64), new string('B', 64), 1, Guid.NewGuid());
            var enrollment = new TrustedUpdateEnrollment(root);
            var initialized = 0;
            var configured = 0;
            var validated = 0;
            void Prepare() => Directory.CreateDirectory(root);
            void Initialize() => initialized++;
            void Configure() => configured++;
            void Validate() => validated++;
            try
            {
                enrollment.Run(identity, Prepare, Initialize, () => throw new IOException("synthetic SCM failure"), Validate);
                throw new InvalidOperationException("Expected enrollment failure.");
            }
            catch (IOException) { }
            Require(!enrollment.IsComplete() && initialized == 1, "partial enrollment cannot be treated as completed broker");
            try
            {
                enrollment.Run(identity with { PublicKeySha256 = new string('C', 64) }, Prepare, Initialize, Configure, Validate);
                throw new InvalidOperationException("Expected trust change rejection.");
            }
            catch (CryptographicException) { }
            Require(configured == 0 && initialized == 1, "recovery cannot silently enroll another key");
            new TrustedUpdateEnrollment(root).Run(identity, Prepare, Initialize, Configure, Validate);
            Require(enrollment.IsComplete() && initialized == 2 && configured == 1 && validated == 1,
                "same signed enrollment resumes after process restart");
            enrollment.Run(identity, Prepare, Initialize, Configure, Validate);
            Require(initialized == 2 && configured == 1 && validated == 2, "completed enrollment is validated, not recreated");
            return Task.CompletedTask;
        }
        finally { if (Directory.Exists(root)) Directory.Delete(root, true); }
    }
    private static void Require(bool value, string message)
    {
        if (!value) throw new InvalidOperationException(message);
    }
}
