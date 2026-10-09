using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

// WinUser.h: SPI_GETCLIENTAREAANIMATION (0x1042), SystemParametersInfoW.
// pvParam points to a BOOL (four bytes); FALSE disables client-area animations.
internal readonly record struct MotionPreferenceSnapshot(bool ReducedMotion, int NativeErrorCode);

internal interface ISystemMotionReader
{
    MotionPreferenceSnapshot Read();
}

internal sealed class WindowsSystemMotionReader : ISystemMotionReader
{
    private const uint SpiGetClientAreaAnimation = 0x1042;

    [DllImport("user32.dll", EntryPoint = "SystemParametersInfoW", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SystemParametersInfo(
        uint action, uint parameter, out int value, uint update);

    public MotionPreferenceSnapshot Read()
    {
        if (!SystemParametersInfo(SpiGetClientAreaAnimation, 0, out var enabled, 0))
        {
            var nativeCode = Marshal.GetLastWin32Error();
            // A failed BOOL can leave LastError at zero. Reserve a fixed code for that case.
            return new MotionPreferenceSnapshot(true, nativeCode == 0 ? -1 : nativeCode);
        }
        return new MotionPreferenceSnapshot(enabled == 0, 0);
    }
}

/// <summary>Thread-safe snapshot; synthetic readers can test transitions without changing Windows settings.</summary>
internal sealed class SystemMotionPreference(ISystemMotionReader reader)
{
    private readonly object _gate = new();
    private MotionPreferenceSnapshot _snapshot = new(true, -1);
    private bool _hasSample;

    public MotionPreferenceSnapshot Snapshot
    {
        get { lock (_gate) { return _snapshot; } }
    }

    public bool Refresh(out MotionPreferenceSnapshot snapshot)
    {
        MotionPreferenceSnapshot sampled;
        try
        {
            sampled = reader.Read();
        }
        catch (Exception)
        {
            // API access exceptions have no Win32 last-error; fixed nonzero code.
            sampled = new MotionPreferenceSnapshot(true, -2);
        }
        if (sampled.NativeErrorCode != 0)
        {
            sampled = sampled with { ReducedMotion = true };
        }
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
