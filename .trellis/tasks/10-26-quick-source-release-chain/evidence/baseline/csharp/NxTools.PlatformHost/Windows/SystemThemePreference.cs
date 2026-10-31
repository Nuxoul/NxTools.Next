using Microsoft.Win32;

namespace NxTools.PlatformHost.Windows;

internal readonly record struct ThemePreferenceSnapshot(bool Dark, int NativeErrorCode);
internal interface ISystemThemeReader { ThemePreferenceSnapshot Read(); }
internal sealed class WindowsSystemThemeReader : ISystemThemeReader
{
    public ThemePreferenceSnapshot Read()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
            return key?.GetValue("AppsUseLightTheme") is int value
                ? new ThemePreferenceSnapshot(value == 0, 0)
                : new ThemePreferenceSnapshot(true, 2);
        }
        catch (Exception failure) when (failure is IOException or UnauthorizedAccessException or System.Security.SecurityException)
        {
            return new ThemePreferenceSnapshot(true, failure.HResult);
        }
    }
}
internal sealed class SystemThemePreference(ISystemThemeReader reader)
{
    private readonly object _gate = new();
    private ThemePreferenceSnapshot _snapshot = new(true, -1);
    private bool _hasSample;
    public ThemePreferenceSnapshot Snapshot { get { lock (_gate) return _snapshot; } }
    public bool Refresh(out ThemePreferenceSnapshot snapshot)
    {
        ThemePreferenceSnapshot sampled;
        try { sampled = reader.Read(); }
        catch (Exception) { sampled = new(true, -2); }
        if (sampled.NativeErrorCode != 0) sampled = sampled with { Dark = true };
        lock (_gate)
        {
            var changed = !_hasSample || _snapshot != sampled;
            _hasSample = true;
            _snapshot = sampled;
            snapshot = sampled;
            return changed;
        }
    }
}
