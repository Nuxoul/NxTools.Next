using System.Runtime.InteropServices;
using System.Security.Principal;
using System.Text;
using Microsoft.Win32;
using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows.Archive;

/// <summary>
/// Current-user Explorer default program for an extension: <c>FileExts\.ext\UserChoice</c> with
/// its ProgId and the proof Explorer recomputes. Only explicit default claims and their Revert
/// reach this adapter; isolated tests substitute the interface.
/// </summary>
internal sealed class WindowsArchiveDefaultClaim : IArchiveDefaultClaim
{
    private const string FileExtsPrefix = @"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\";
    private const uint AssocStrProgId = 20;
    private const int ProofAttempts = 3;
    private static readonly Lazy<string> Experience = new(() =>
    {
        var shell32 = Path.Combine(Environment.SystemDirectory, "shell32.dll");
        return UserChoiceHash.FindExperience(File.ReadAllBytes(shell32)) ?? UserChoiceHash.DefaultExperience;
    });
    private readonly string _userSid = WindowsIdentity.GetCurrent().User?.Value
        ?? throw new InvalidOperationException("Current user SID is unavailable.");

    public string? Current(string extension)
    {
        using var key = Registry.CurrentUser.OpenSubKey(ChoicePath(extension), false);
        return key?.GetValue("ProgId") as string;
    }

    public void Set(string extension, string? progId)
    {
        var path = ChoicePath(extension);
        // The newer choice scheme is not owned by our legacy writer. Do not remove a later
        // Windows/user choice to make a legacy claim or restoration appear to succeed.
        using (var latest = Registry.CurrentUser.OpenSubKey(FileExtsPrefix + extension + @"\UserChoiceLatest", false))
            if (latest is not null) throw new InvalidOperationException("Explorer owns a newer default choice.");
        Registry.CurrentUser.DeleteSubKey(path, false);
        if (progId is null) return;

        using var key = Registry.CurrentUser.CreateSubKey(path, true)
            ?? throw new IOException("Unable to create the Explorer default choice.");
        key.SetValue("ProgId", progId, RegistryValueKind.String);
        // The proof covers the key's last-write minute; recompute if writing crossed a minute.
        for (var attempt = 0; attempt < ProofAttempts; attempt++)
        {
            var before = LastWriteUtc(key.Handle);
            key.SetValue("Hash", UserChoiceHash.Compute(extension, _userSid, progId, before, Experience.Value), RegistryValueKind.String);
            if (SameMinute(before, LastWriteUtc(key.Handle))) return;
        }
        throw new IOException("Explorer default choice timestamp did not settle.");
    }

    public bool Resolves(string extension, string progId)
    {
        var buffer = new StringBuilder(256);
        var length = (uint)buffer.Capacity;
        return AssocQueryString(0, AssocStrProgId, extension, null, buffer, ref length) == 0 &&
            string.Equals(buffer.ToString(), progId, StringComparison.OrdinalIgnoreCase);
    }

    private static string ChoicePath(string extension)
    {
        if (!IsPlainExtension(extension)) throw new ArgumentException("Invalid archive extension.", nameof(extension));
        return FileExtsPrefix + extension + @"\UserChoice";
    }

    internal static bool IsPlainExtension(string extension) =>
        extension.Length is >= 2 and <= 32 && extension[0] == '.' &&
        extension.Skip(1).All(character => char.IsAsciiLetterOrDigit(character) || character is '.' or '_' or '-');

    private static bool SameMinute(DateTime left, DateTime right) =>
        left.Ticks / TimeSpan.TicksPerMinute == right.Ticks / TimeSpan.TicksPerMinute;

    private static DateTime LastWriteUtc(SafeRegistryHandle handle)
    {
        var status = RegQueryInfoKey(handle, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero,
            IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, out var fileTime);
        if (status != 0) throw new System.ComponentModel.Win32Exception(status);
        return DateTime.FromFileTimeUtc(fileTime);
    }

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
    private static extern int RegQueryInfoKey(SafeRegistryHandle key, IntPtr className, IntPtr classLength,
        IntPtr reserved, IntPtr subKeys, IntPtr maxSubKeyLength, IntPtr maxClassLength, IntPtr values,
        IntPtr maxValueNameLength, IntPtr maxValueLength, IntPtr securityDescriptor, out long lastWriteTime);

    [DllImport("shlwapi.dll", CharSet = CharSet.Unicode)]
    private static extern int AssocQueryString(uint flags, uint str, string association, string? extra,
        StringBuilder output, ref uint length);
}
