namespace NxTools.PlatformHost.Windows;

internal interface IClipboardHotKeyRegistrar
{
    int LastError { get; }
    bool Register(uint modifiers, uint virtualKey);
    bool Unregister();
}

internal sealed class ClipboardHotKeyRegistration
{
    private readonly IClipboardHotKeyRegistrar _registrar;
    private ClipboardHotKeyBinding _current;

    public ClipboardHotKeyRegistration(
        IClipboardHotKeyRegistrar registrar,
        ClipboardHotKeyBinding initial)
    {
        _registrar = registrar;
        _current = initial;
    }

    public bool IsAvailable { get; private set; }
    public bool IsRecording { get; private set; }
    public ClipboardHotKeyBinding Current => _current;

    public WindowsCommandResult Initialize() => RegisterCurrent();

    public WindowsCommandResult BeginRecording()
    {
        if (IsRecording)
        {
            return new WindowsCommandResult(true);
        }
        if (IsAvailable && !_registrar.Unregister())
        {
            return new WindowsCommandResult(
                false,
                $"Could not suspend {_current.Display}: Win32={_registrar.LastError}.");
        }
        IsAvailable = false;
        IsRecording = true;
        return new WindowsCommandResult(true);
    }

    public WindowsCommandResult CancelRecording()
    {
        if (!IsRecording)
        {
            return new WindowsCommandResult(true);
        }
        IsRecording = false;
        return RegisterCurrent();
    }

    public WindowsCommandResult Apply(ClipboardHotKeyBinding requested)
    {
        var previous = _current;
        var previousWasAvailable = IsAvailable;
        if (IsAvailable && !_registrar.Unregister())
        {
            return new WindowsCommandResult(
                false,
                $"Could not replace {_current.Display}: Win32={_registrar.LastError}.");
        }

        IsAvailable = false;
        IsRecording = false;
        if (_registrar.Register(requested.Modifiers, requested.VirtualKey))
        {
            _current = requested;
            IsAvailable = true;
            return new WindowsCommandResult(true);
        }

        var requestedError = _registrar.LastError;
        _current = previous;
        if (!previousWasAvailable)
        {
            return new WindowsCommandResult(
                false,
                $"Could not register {requested.Display}: Win32={requestedError}.");
        }
        var restored = _registrar.Register(previous.Modifiers, previous.VirtualKey);
        IsAvailable = restored;
        return new WindowsCommandResult(
            false,
            restored
                ? $"Could not register {requested.Display}: Win32={requestedError}."
                : $"Could not register {requested.Display} or restore {previous.Display}: Win32={requestedError}/{_registrar.LastError}.");
    }

    public void Stop()
    {
        if (IsAvailable)
        {
            _ = _registrar.Unregister();
        }
        IsAvailable = false;
        IsRecording = false;
    }

    private WindowsCommandResult RegisterCurrent()
    {
        if (_registrar.Register(_current.Modifiers, _current.VirtualKey))
        {
            IsAvailable = true;
            return new WindowsCommandResult(true);
        }
        IsAvailable = false;
        return new WindowsCommandResult(
            false,
            $"Could not register {_current.Display}: Win32={_registrar.LastError}.");
    }
}
