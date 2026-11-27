using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

// SystemBackdrop: DWMWA_SYSTEMBACKDROP_TYPE (build 22621). RoundedCorners: DWM attributes 33/34 (build 22000).
internal readonly record struct MaterialCapabilitySnapshot(
    bool Supported, bool TransparencyEnabled, bool HighContrast,
    bool EnergySaver, bool RemoteSession, int NativeErrorCode,
    bool SystemBackdrop, bool RoundedCorners);

internal interface ISystemMaterialReader
{
    MaterialCapabilitySnapshot Read();
}

internal sealed class WindowsSystemMaterialReader : ISystemMaterialReader
{
    public MaterialCapabilitySnapshot Read()
    {
        var error = WindowMaterialNative.ReadVersion(out var major, out var build);
        var compositionResult = DwmIsCompositionEnabled(out var composition);
        if (error == 0 && compositionResult != 0) error = compositionResult;
        var supported = major >= 10 && build >= 17763 && compositionResult == 0 && composition;

        uint size = sizeof(uint);
        var registryResult = RegGetValueW(new IntPtr(unchecked((int)0x80000001)),
            @"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "EnableTransparency",
            0x10, out _, out var transparency, ref size); // RRF_RT_REG_DWORD, read only.
        // Windows defaults to transparency enabled when the key/value does not exist.
        var transparencyEnabled = registryResult is 2 or 3 || registryResult == 0 && transparency != 0;
        if (error == 0 && registryResult is not (0 or 2 or 3)) error = registryResult;

        var contrast = new HighContrast { Size = (uint)Marshal.SizeOf<HighContrast>() };
        var contrastOk = SystemParametersInfoW(0x0042, contrast.Size, ref contrast, 0); // SPI_GETHIGHCONTRAST
        if (!contrastOk && error == 0) error = WindowMaterialNative.LastError();
        var powerOk = GetSystemPowerStatus(out var power);
        if (!powerOk && error == 0) error = WindowMaterialNative.LastError();
        var remote = GetSystemMetrics(0x1000) != 0; // SM_REMOTESESSION
        return new(supported, error == 0 && transparencyEnabled,
            !contrastOk || (contrast.Flags & 1) != 0, !powerOk || (power.SystemStatusFlag & 1) != 0, remote, error,
            WindowMaterialNative.HasSystemBackdrop(major, build), WindowMaterialNative.HasRoundedCorners(major, build));
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct HighContrast { internal uint Size, Flags; internal nint DefaultScheme; }
    [StructLayout(LayoutKind.Sequential)]
    private struct PowerStatus
    {
        internal byte AcLineStatus, BatteryFlag, BatteryLifePercent, SystemStatusFlag;
        internal uint BatteryLifeTime, BatteryFullLifeTime;
    }
    [DllImport("dwmapi.dll", ExactSpelling = true)]
    private static extern int DwmIsCompositionEnabled([MarshalAs(UnmanagedType.Bool)] out bool enabled);
    [DllImport("advapi32.dll", ExactSpelling = true, CharSet = CharSet.Unicode)]
    private static extern int RegGetValueW(nint key, string subKey, string value, uint flags,
        out uint type, out uint data, ref uint size);
    [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SystemParametersInfoW(uint action, uint parameter, ref HighContrast value, uint update);
    [DllImport("kernel32.dll", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool GetSystemPowerStatus(out PowerStatus status);
    [DllImport("user32.dll", ExactSpelling = true)]
    private static extern int GetSystemMetrics(int index);
}

internal sealed class SystemMaterialPreference(ISystemMaterialReader reader)
{
    private readonly object _gate = new();
    private MaterialCapabilitySnapshot _snapshot = new(false, false, true, true, false, -1, false, false);
    private bool _hasSample;

    public MaterialCapabilitySnapshot Snapshot
    {
        get { lock (_gate) return _snapshot; }
    }

    public bool Refresh(out MaterialCapabilitySnapshot snapshot)
    {
        lock (_gate)
        {
            MaterialCapabilitySnapshot sampled;
            try { sampled = reader.Read(); }
            catch (Exception error)
            {
                sampled = new(false, false, true, true, false, error.HResult == 0 ? -1 : error.HResult, false, false);
            }
            if (sampled.NativeErrorCode != 0) sampled = sampled with { TransparencyEnabled = false };
            var changed = !_hasSample || _snapshot != sampled;
            _hasSample = true;
            snapshot = _snapshot = sampled;
            return changed;
        }
    }
}
