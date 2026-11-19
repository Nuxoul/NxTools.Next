using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class ElevatedInputTests
{
    public static Task ForegroundChangeNeverInjects()
    {
        var operations = new PasteOperations { LoseForegroundOnRelease = true };
        var result = new WindowsPasteCoordinator(operations).Execute(
            new TextClipboardPastePayload("fixture", 10, 11, true), Guid.NewGuid());
        Assert(!result.Success && operations.Shortcuts == 0,
            "foreground changed after activation: never inject Ctrl+V into the new foreground");
        return Task.CompletedTask;
    }

    public static Task ClipboardWriteAndInputOwnership()
    {
        foreach (var success in new[] { true, false })
        {
            var operations = new PasteOperations();
            var elevated = new PasteInput { Result = new(success, success ? null : "fixture denied") };
            var id = Guid.NewGuid();
            var result = new WindowsPasteCoordinator(operations, elevatedInput: elevated).Execute(
                new TextClipboardPastePayload("private fixture content", 10, 11, true), id);
            Assert(result.Success == success && operations.Writes == 1 && operations.Shortcuts == 0 &&
                operations.Activations == 0 && elevated.Calls == 1 && elevated.Input == new ElevatedPasteInput(10, 11, id),
                "clipboard stays local once; only input target and correlation delegate; confirmed failure never replays");
        }
        var local = new PasteOperations();
        var unavailable = new PasteInput();
        Assert(new WindowsPasteCoordinator(local, elevatedInput: unavailable).Execute(
            new TextClipboardPastePayload("fixture", 10, 11, true), Guid.NewGuid()).Success &&
            local.Writes == 1 && local.Shortcuts == 1, "unavailable-before-dispatch preserves ordinary paste");
        foreach (var copyOnly in new[] { true, false })
        {
            local = new PasteOperations { WriteSuccess = copyOnly };
            var input = new PasteInput { Result = new(true) };
            var result = new WindowsPasteCoordinator(local, elevatedInput: input).Execute(
                new TextClipboardPastePayload("fixture", 10, 11, !copyOnly), Guid.NewGuid());
            Assert(result.Success == copyOnly && input.Calls == 0 && local.Writes == 1 && local.Shortcuts == 0,
                "copy-only and failed clipboard writes must never dispatch input");
        }
        local = new PasteOperations();
        Assert(new WindowsPasteCoordinator(local).ExecuteInput(new(10, 11, Guid.NewGuid())).Success &&
            local.Writes == 0 && local.Shortcuts == 1, "helper input stage must never write clipboard");
        return Task.CompletedTask;
    }

    public static Task GestureSourcesNeverDuplicate()
    {
        var router = new QuickLaunchInputRouter();
        Assert(router.Accept(false, false, true, 100) && !router.Accept(true, false, true, 100),
            "ordinary foreground belongs to ordinary hook");
        Assert(!router.Accept(false, true, true, 300) && router.Accept(true, true, true, 300),
            "elevated foreground belongs to confirmed helper");
        Assert(!router.Accept(false, false, true, 300),
            "helper already summoned: ordinary queued message cannot toggle after popup changed foreground");
        Assert(!router.Accept(true, true, false, 500) && router.Accept(false, true, false, 500),
            "helper unavailable or obsolete: retain ordinary fallback");
        Assert(router.Accept(false, false, true, 501), "distinct gestures are not time-debounced");
        return Task.CompletedTask;
    }

    private sealed class PasteInput : IElevatedPasteInput
    {
        public int Calls;
        public ElevatedPasteInput? Input;
        public WindowsCommandResult? Result;
        public WindowsCommandResult? TryPasteInput(ElevatedPasteInput input)
        { Calls++; Input = input; return Result; }
    }

    internal sealed class PasteOperations : IWindowsPasteOperations
    {
        public int Writes, Shortcuts, Activations;
        public bool LoseForegroundOnRelease, Foreground = true, WriteSuccess = true;
        public bool IsWindow(long windowHandle) => windowHandle is 10 or 11;
        public uint GetClipboardSequenceNumber() => (uint)Writes;
        public ClipboardWriteResult WriteClipboard(ClipboardPastePayload payload)
        {
            Writes++;
            return new(WriteSuccess, WriteSuccess ? null : "fixture write failed", []);
        }
        public PasteWindowSnapshot CaptureWindowState(long targetWindowHandle) => new(10, 10, 10, 1, 2, 2, 3, 3);
        public bool TryActivateWindow(long targetWindowHandle, long focusWindowHandle) { Activations++; return true; }
        public bool IsForegroundWindowForTarget(long targetWindowHandle) => Foreground;
        public ModifierReleaseResult ReleaseActivationModifiers()
        {
            if (LoseForegroundOnRelease) Foreground = false;
            return new(true, [], [], 1, []);
        }
        public PasteShortcutResult SendPasteShortcut() { Shortcuts++; return new(4, 40, []); }
        public PostPasteResult PostPaste(long targetWindowHandle) => new(false, 5, "fixture denied");
        public void Delay(int milliseconds) { }
    }

    internal static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
