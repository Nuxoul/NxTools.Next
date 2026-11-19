namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    internal sealed partial class EventWindow
    {
        private readonly QuickLaunchInputRouter _inputRouter = new();

        private void HandleQuickLaunchGesture(int helperProcessId, long generation, uint timestamp)
        {
            if (!_quickLaunchTriggerEnabled || _quickLaunchRecording ||
                _quickLaunchTrigger.Kind != QuickLaunchTriggerKind.DoubleControl ||
                generation != _quickLaunchGestureResetGeneration) return;
            var input = _elevatedDirectoryNavigation as IElevatedInputChannel;
            var foreground = _foregroundContextProvider.Capture();
            var elevated = foreground.ProcessId != 0 &&
                ElevatedDirectoryHelperPeerVerifier.IsProcessElevated(foreground.ProcessId);
            var helperOwns = input?.OwnsInput(helperProcessId, generation) == true;
            if (!_inputRouter.Accept(helperProcessId != 0, elevated, helperOwns, timestamp)) return;
            PublishQuickLaunchPanelRequest(QuickLaunchSummonTrigger.DoubleControl);
        }

        internal void HandleElevatedPointerPress(long rootWindow, int helperProcessId, uint timestamp)
        {
            var input = _elevatedDirectoryNavigation as IElevatedInputChannel;
            if (!NativeMethods.IsWindow(new IntPtr(rootWindow)) ||
                input?.AcceptPointer(helperProcessId, timestamp) != true) return;
            var processId = ElevatedInputClientPlatform.WindowProcessId(rootWindow);
            if (processId == 0 || !ElevatedDirectoryHelperPeerVerifier.IsSameSession(
                    processId, (uint)Environment.ProcessId)) return;
            var age = unchecked((int)((uint)Environment.TickCount64 - timestamp));
            PublishPointerPress(rootWindow, rootWindow, processId,
                DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - age);
        }

        private void PublishPointerPress(long window, long root, uint processId, long sampledAtUnixMs)
        {
            var eventSequence = NextPlatformEventSequence();
            _contextMenus?.ObservePointerPress(window, root);
            _events.TryWrite(new GlobalPointerPressedEvent(window, root, processId,
                sampledAtUnixMs, eventSequence));
        }
    }

    internal static bool AcceptInputSource(bool helperSource, bool elevatedForeground, bool helperOwns) =>
        helperSource ? elevatedForeground && helperOwns : !elevatedForeground || !helperOwns;
}
