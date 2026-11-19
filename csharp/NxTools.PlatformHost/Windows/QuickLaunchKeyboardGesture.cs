namespace NxTools.PlatformHost.Windows;

internal sealed record QuickLaunchKeyboardConfiguration(
    bool Enabled, bool Recording, QuickLaunchTriggerKind Trigger, long ResetGeneration);

internal sealed class QuickLaunchKeyboardGesture
{
    // These detectors are touched only by the current hook thread, never the publisher.
    private readonly ModifierDoubleTapDetector _left = new(0xA2, 60, 800);
    private readonly ModifierDoubleTapDetector _right = new(0xA3, 60, 800);
    private QuickLaunchKeyboardConfiguration _configuration = new(true, false, QuickLaunchTriggerKind.DoubleControl, 0);
    private long _appliedResetGeneration = -1;
    public uint Timestamp { get; private set; }
    public long Generation => _appliedResetGeneration;

    public void Publish(QuickLaunchKeyboardConfiguration configuration) => Volatile.Write(ref _configuration, configuration);

    private bool PrepareCallback()
    {
        var configuration = Volatile.Read(ref _configuration);
        if (_appliedResetGeneration != configuration.ResetGeneration)
        {
            _left.Reset();
            _right.Reset();
            _appliedResetGeneration = configuration.ResetGeneration;
        }
        return configuration.Enabled && !configuration.Recording && configuration.Trigger == QuickLaunchTriggerKind.DoubleControl;
    }

    public bool OnKeyDown(int key, uint timestamp)
    {
        Timestamp = timestamp;
        if (!PrepareCallback()) return false;
        if (key == 0xA2)
        {
            _right.Reset();
            return _left.OnKeyDown(key, timestamp);
        }
        if (key == 0xA3)
        {
            _left.Reset();
            return _right.OnKeyDown(key, timestamp);
        }
        _left.Reset();
        _right.Reset();
        return false;
    }

    public bool OnKeyUp(int key) => PrepareCallback() && (_left.OnKeyUp(key) | _right.OnKeyUp(key));
}
