namespace NxTools.PlatformHost.Windows;

/// <summary>
/// Recognizes two distinct presses of one modifier key. The key must be released between presses;
/// key-repeat and any intervening non-target key reset the sequence.
/// </summary>
internal sealed class ModifierDoubleTapDetector(
    int targetVirtualKey,
    uint minimumIntervalMilliseconds,
    uint maximumIntervalMilliseconds)
{
    private uint _lastDownTime;
    private bool _targetWasReleased = true;
    private bool _hasFirstTap;
    private bool _triggerOnRelease;

    public bool OnKeyDown(int virtualKey, uint timestamp)
    {
        if (virtualKey != targetVirtualKey)
        {
            Reset();
            return false;
        }
        if (!_targetWasReleased)
        {
            return false;
        }

        _targetWasReleased = false;
        if (_hasFirstTap)
        {
            var elapsed = unchecked(timestamp - _lastDownTime);
            if (elapsed >= minimumIntervalMilliseconds && elapsed <= maximumIntervalMilliseconds)
            {
                // Do not summon a new foreground window while the modifier is physically down.
                // Arm the gesture now and publish it only after the matching key-up.
                _lastDownTime = 0;
                _hasFirstTap = false;
                _triggerOnRelease = true;
                _targetWasReleased = false;
                return false;
            }
        }

        _lastDownTime = timestamp;
        _hasFirstTap = true;
        return false;
    }

    public bool OnKeyUp(int virtualKey)
    {
        if (virtualKey != targetVirtualKey)
        {
            return false;
        }

        _targetWasReleased = true;
        var triggered = _triggerOnRelease;
        _triggerOnRelease = false;
        return triggered;
    }

    public void Reset()
    {
        _lastDownTime = 0;
        _hasFirstTap = false;
        _triggerOnRelease = false;
        _targetWasReleased = true;
    }
}
