namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    internal static IWindowsPasteOperations CreatePasteInputOperations() => new PasteInputOperations();

    private sealed class PasteInputOperations : IWindowsPasteOperations
    {
        public PasteInputOperations() => WindowsKeyboardHookPlatform.EnsureThreadMessageQueue();

        public bool IsWindow(long windowHandle) => NativeMethods.IsWindow(new IntPtr(windowHandle));

        public uint GetClipboardSequenceNumber() =>
            throw new NotSupportedException("Elevated input does not read the clipboard sequence.");

        public ClipboardWriteResult WriteClipboard(ClipboardPastePayload payload) =>
            throw new NotSupportedException("Elevated input must never write the clipboard.");

        public PasteWindowSnapshot CaptureWindowState(long targetWindowHandle) =>
            NativeMethods.CaptureWindowState(new IntPtr(targetWindowHandle));

        public bool TryActivateWindow(long targetWindowHandle, long focusWindowHandle) =>
            NativeMethods.ActivateWindow(new IntPtr(targetWindowHandle), new IntPtr(focusWindowHandle));

        public bool IsForegroundWindowForTarget(long targetWindowHandle) =>
            NativeMethods.IsForegroundWindowForTarget(new IntPtr(targetWindowHandle));

        public ModifierReleaseResult ReleaseActivationModifiers() =>
            NativeMethods.ReleaseActivationModifiers();

        public PasteShortcutResult SendPasteShortcut() => NativeMethods.SendPasteShortcut();

        public PostPasteResult PostPaste(long targetWindowHandle) =>
            NativeMethods.PostPaste(new IntPtr(targetWindowHandle));

        public void Delay(int milliseconds) => Thread.Sleep(milliseconds);
    }
}
