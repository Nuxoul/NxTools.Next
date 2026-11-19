using System.Runtime.InteropServices;
using Microsoft.Win32;

namespace NxTools.PlatformHost.Windows.Archive;

/// <summary>
/// Real current-user Registry adapter for the P206 plan. It is reachable only through
/// <see cref="ArchiveIntegrationService"/> with apply explicitly enabled; isolated tests use a
/// fake store and never construct this type's mutating path. It never deletes a key tree:
/// restoration removes one exact value and then only owned keys proved created and now empty.
/// </summary>
internal sealed class WindowsArchiveIntegrationStore : IArchiveIntegrationStore
{
    private const string ClassesPrefix = @"Software\Classes\";

    public ArchiveRegistryValueSnapshot Capture(ArchiveRegistryValueTarget target)
    {
        Validate(target);
        var missing = OwnedAncestry(target)
            .Where(path => !KeyExists(path))
            .ToArray();
        using var key = Registry.CurrentUser.OpenSubKey(target.KeyPath, false);
        if (key is null || !key.GetValueNames().Contains(target.ValueName, StringComparer.OrdinalIgnoreCase))
            return new ArchiveRegistryValueSnapshot(false, null, missing);
        var data = key.GetValue(target.ValueName, null, RegistryValueOptions.DoNotExpandEnvironmentNames)
            ?? throw new IOException("Archive integration value disappeared during capture.");
        return new ArchiveRegistryValueSnapshot(true, FromRegistry(key.GetValueKind(target.ValueName), data), missing);
    }

    public void Write(ArchiveRegistryValueTarget target, ArchiveRegistryValue value)
    {
        Validate(target);
        using var key = Registry.CurrentUser.CreateSubKey(target.KeyPath, true)
            ?? throw new IOException("Unable to create the archive integration key.");
        key.SetValue(target.ValueName, value.Data, ToRegistry(value.Kind));
    }

    public void Restore(ArchiveRegistryValueTarget target, ArchiveRegistryValueSnapshot snapshot)
    {
        Validate(target);
        if (snapshot.Exists)
        {
            using var key = Registry.CurrentUser.CreateSubKey(target.KeyPath, true)
                ?? throw new IOException("Unable to restore the archive integration key.");
            key.SetValue(target.ValueName, snapshot.Value!.Data, ToRegistry(snapshot.Value.Kind));
            return;
        }
        using (var key = Registry.CurrentUser.OpenSubKey(target.KeyPath, true))
            key?.DeleteValue(target.ValueName, false);
        // Only keys absent before the write are candidates, deepest first, and only when empty.
        foreach (var path in snapshot.MissingOwnedKeyPaths.OrderByDescending(path => path.Length))
        {
            if (!IsOwnedPath(target, path)) continue;
            using (var key = Registry.CurrentUser.OpenSubKey(path, false))
            {
                if (key is null || key.ValueCount != 0 || key.SubKeyCount != 0) continue;
            }
            Registry.CurrentUser.DeleteSubKey(path, false);
        }
    }

    private const int ShcneAssocChanged = 0x08000000;
    private const uint ShcnfIdList = 0x0000;
    private const uint ShcnfFlush = 0x1000;

    [DllImport("shell32.dll")]
    private static extern void SHChangeNotify(int eventId, uint flags, IntPtr item1, IntPtr item2);

    /// <summary>Flushes the association notification before returning so current Shell clients can rebuild their view.</summary>
    internal static void NotifyAssociationsChanged() => NotifyAssociationsChanged(SHChangeNotify);

    internal static void NotifyAssociationsChanged(Action<int, uint, IntPtr, IntPtr> notify)
    {
        ArgumentNullException.ThrowIfNull(notify);
        notify(ShcneAssocChanged, ShcnfIdList | ShcnfFlush, IntPtr.Zero, IntPtr.Zero);
    }

    internal static bool IsPlainExtension(string extension) =>
        extension.Length is >= 2 and <= 32 && extension[0] == '.' &&
        extension.Skip(1).All(character => char.IsAsciiLetterOrDigit(character) || character is '.' or '_' or '-');

    internal static void Validate(ArchiveRegistryValueTarget target)
    {
        if (!target.KeyPath.StartsWith(ClassesPrefix, StringComparison.OrdinalIgnoreCase) ||
            !target.CleanupRootPath.StartsWith(ClassesPrefix, StringComparison.OrdinalIgnoreCase) ||
            target.KeyPath.Contains("UserChoice", StringComparison.OrdinalIgnoreCase) ||
            target.KeyPath.Contains(@"\\", StringComparison.Ordinal) ||
            !(target.KeyPath.Equals(target.CleanupRootPath, StringComparison.OrdinalIgnoreCase) ||
              target.KeyPath.StartsWith(target.CleanupRootPath + "\\", StringComparison.OrdinalIgnoreCase)))
            throw new InvalidOperationException("Archive integration target escaped current-user Classes ownership.");
    }

    private static bool IsOwnedPath(ArchiveRegistryValueTarget target, string path) =>
        OwnedAncestry(target).Contains(path, StringComparer.OrdinalIgnoreCase);

    private static IEnumerable<string> OwnedAncestry(ArchiveRegistryValueTarget target)
    {
        var current = target.CleanupRootPath;
        yield return current;
        if (target.KeyPath.Equals(current, StringComparison.OrdinalIgnoreCase)) yield break;
        foreach (var component in target.KeyPath[(current.Length + 1)..].Split('\\'))
        {
            current += "\\" + component;
            yield return current;
        }
    }

    private static bool KeyExists(string path)
    {
        using var key = Registry.CurrentUser.OpenSubKey(path, false);
        return key is not null;
    }

    private static RegistryValueKind ToRegistry(ArchiveRegistryValueKind kind) => kind switch
    {
        ArchiveRegistryValueKind.String => RegistryValueKind.String,
        ArchiveRegistryValueKind.ExpandString => RegistryValueKind.ExpandString,
        ArchiveRegistryValueKind.Dword => RegistryValueKind.DWord,
        ArchiveRegistryValueKind.Qword => RegistryValueKind.QWord,
        ArchiveRegistryValueKind.MultiString => RegistryValueKind.MultiString,
        _ => RegistryValueKind.Binary,
    };

    private static ArchiveRegistryValue FromRegistry(RegistryValueKind kind, object data) => kind switch
    {
        RegistryValueKind.String => new(ArchiveRegistryValueKind.String, (string)data),
        RegistryValueKind.ExpandString => new(ArchiveRegistryValueKind.ExpandString, (string)data),
        RegistryValueKind.DWord => new(ArchiveRegistryValueKind.Dword, (int)data),
        RegistryValueKind.QWord => new(ArchiveRegistryValueKind.Qword, (long)data),
        RegistryValueKind.MultiString => new(ArchiveRegistryValueKind.MultiString, (string[])data),
        RegistryValueKind.Binary => new(ArchiveRegistryValueKind.Binary, (byte[])data),
        _ => throw new InvalidDataException("Unsupported pre-existing archive integration value kind."),
    };
}
