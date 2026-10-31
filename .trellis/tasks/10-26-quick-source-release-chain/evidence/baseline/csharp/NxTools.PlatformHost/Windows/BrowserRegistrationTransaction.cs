using Microsoft.Win32;

namespace NxTools.PlatformHost.Windows;

internal readonly record struct BrowserRegistrationChange(string Target, string? Value);
internal interface IBrowserRegistrationStore
{
    object? Capture(string target);
    void Write(string target, string? value);
    void Restore(string target, object? snapshot);
}

internal static class BrowserRegistrationTransaction
{
    internal static void Apply(IBrowserRegistrationStore store, IReadOnlyList<BrowserRegistrationChange> changes)
    {
        // Capture every prior state before the first mutation, including value kinds
        // and child keys. Add to rollback BEFORE Write: a write may mutate then fail.
        var prior = changes.Select(change => store.Capture(change.Target)).ToArray();
        var attempted = -1;
        try
        {
            for (var index = 0; index < changes.Count; index++)
            {
                attempted = index;
                store.Write(changes[index].Target, changes[index].Value);
            }
        }
        catch (Exception failure)
        {
            var rollbackFailures = new List<Exception>();
            for (var index = attempted; index >= 0; index--)
            {
                try { store.Restore(changes[index].Target, prior[index]); }
                catch (Exception rollbackFailure) { rollbackFailures.Add(rollbackFailure); }
            }
            if (rollbackFailures.Count > 0)
                throw new AggregateException("Browser registration failed and compensation was incomplete.",
                    new[] { failure }.Concat(rollbackFailures));
            throw;
        }
    }
}

internal sealed class WindowsBrowserRegistrationStore : IBrowserRegistrationStore
{
    private sealed record KeySnapshot(Dictionary<string, (object Value, RegistryValueKind Kind)> Values,
        Dictionary<string, KeySnapshot> Children);
    public object? Capture(string target)
    {
        if (target.StartsWith("manifest:", StringComparison.Ordinal))
        {
            var path = target[9..];
            return File.Exists(path) ? File.ReadAllBytes(path) : null;
        }
        using var key = Registry.CurrentUser.OpenSubKey(target[4..]);
        return key is null ? null : CaptureKey(key);
    }
    private static KeySnapshot CaptureKey(RegistryKey key)
    {
        var values = key.GetValueNames().ToDictionary(name => name,
            name => (key.GetValue(name, null, RegistryValueOptions.DoNotExpandEnvironmentNames)!, key.GetValueKind(name)));
        var children = new Dictionary<string, KeySnapshot>();
        foreach (var name in key.GetSubKeyNames())
        {
            using var child = key.OpenSubKey(name) ?? throw new IOException("Native messaging registry changed during capture.");
            children[name] = CaptureKey(child);
        }
        return new(values, children);
    }
    public void Write(string target, string? value)
    {
        if (target.StartsWith("manifest:", StringComparison.Ordinal))
        {
            var path = target[9..];
            if (value is null) File.Delete(path);
            else WriteFileAtomically(path, System.Text.Encoding.UTF8.GetBytes(value));
            return;
        }
        var keyPath = target[4..];
        if (value is null) Registry.CurrentUser.DeleteSubKeyTree(keyPath, false);
        else
        {
            using var key = Registry.CurrentUser.CreateSubKey(keyPath, true)
                ?? throw new IOException("Unable to create native messaging registry key.");
            key.SetValue(null, value, RegistryValueKind.String);
        }
    }
    public void Restore(string target, object? snapshot)
    {
        if (target.StartsWith("manifest:", StringComparison.Ordinal))
        {
            if (snapshot is byte[] bytes) WriteFileAtomically(target[9..], bytes);
            else File.Delete(target[9..]);
            return;
        }
        var keyPath = target[4..];
        Registry.CurrentUser.DeleteSubKeyTree(keyPath, false);
        if (snapshot is KeySnapshot prior)
        {
            using var key = Registry.CurrentUser.CreateSubKey(keyPath, true)
                ?? throw new IOException("Unable to restore native messaging registry key.");
            RestoreKey(key, prior);
        }
    }
    private static void RestoreKey(RegistryKey key, KeySnapshot snapshot)
    {
        foreach (var pair in snapshot.Values) key.SetValue(pair.Key, pair.Value.Value, pair.Value.Kind);
        foreach (var pair in snapshot.Children)
        {
            using var child = key.CreateSubKey(pair.Key, true)
                ?? throw new IOException("Unable to restore native messaging child key.");
            RestoreKey(child, pair.Value);
        }
    }
    private static void WriteFileAtomically(string path, byte[] bytes)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        var temporary = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
        try { File.WriteAllBytes(temporary, bytes); File.Move(temporary, path, true); }
        finally { if (File.Exists(temporary)) File.Delete(temporary); }
    }
}
