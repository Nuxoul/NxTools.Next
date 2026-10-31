using NxTools.PlatformHost.Protocol;
using System.Runtime.InteropServices;
using System.Text;

namespace NxTools.PlatformHost.Windows;

internal interface IFileDialogWindowOperations
{
    bool TryInspect(long windowHandle, out string? currentDirectory);
    bool TryNavigate(long windowHandle, string targetDirectory, out string? error);
    bool TryOpenFile(long windowHandle, string filePath, out string? error);
}

internal static class FileDialogWindowAdapter
{
    internal static bool IsPotentialHost(ForegroundContext foreground) =>
        foreground.WindowHandle != 0 &&
        Win32FileDialogWindowOperations.IsDialogWindowClass(
            foreground.WindowHandle);

    public static FileDialogContext? TryCapture(ForegroundContext foreground) =>
        TryCapture(foreground, new Win32FileDialogWindowOperations());

    internal static FileDialogContext? TryCapture(
        ForegroundContext foreground,
        IFileDialogWindowOperations operations)
    {
        if (foreground.WindowHandle == 0 ||
            !operations.TryInspect(foreground.WindowHandle, out var currentDirectory))
        {
            return null;
        }
        return new FileDialogContext(foreground.WindowHandle, currentDirectory);
    }

    public static WindowsCommandResult Navigate(NavigateFileDialogCommand command) =>
        Navigate(command, new Win32FileDialogWindowOperations(), Directory.Exists);

    internal static WindowsCommandResult Navigate(
        NavigateFileDialogCommand command,
        IFileDialogWindowOperations operations,
        Func<string, bool> directoryExists)
    {
        if (command.WindowHandle == 0)
        {
            return new WindowsCommandResult(false, "The captured file dialog is unavailable.");
        }
        if (string.IsNullOrWhiteSpace(command.TargetDirectory) ||
            !directoryExists(command.TargetDirectory))
        {
            return new WindowsCommandResult(false, "The target directory no longer exists.");
        }
        return operations.TryNavigate(
            command.WindowHandle,
            command.TargetDirectory,
            out var error)
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(
                false,
                error ?? "The captured file dialog could not be navigated.");
    }

    public static WindowsCommandResult OpenFile(OpenFileInDialogCommand command) =>
        OpenFile(command, new Win32FileDialogWindowOperations(), File.Exists);

    internal static WindowsCommandResult OpenFile(
        OpenFileInDialogCommand command,
        IFileDialogWindowOperations operations,
        Func<string, bool> fileExists)
    {
        if (command.WindowHandle == 0)
        {
            return new WindowsCommandResult(false, "The captured file dialog is unavailable.");
        }
        if (string.IsNullOrWhiteSpace(command.FilePath) ||
            !fileExists(command.FilePath))
        {
            return new WindowsCommandResult(false, "The selected file no longer exists.");
        }
        return operations.TryOpenFile(
            command.WindowHandle,
            command.FilePath,
            out var error)
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(
                false,
                error ?? "The captured file dialog could not open the selected file.");
    }
}

internal sealed class Win32FileDialogWindowOperations : IFileDialogWindowOperations
{
    internal static int NativeInputSize => Marshal.SizeOf<Input>();
    internal static bool IsDialogWindowClass(long windowHandle) =>
        GetWindowClass(new IntPtr(windowHandle)).Equals(
            "#32770",
            StringComparison.Ordinal);

    private readonly Func<long, long, bool> _activateWindow;
    private readonly Func<long, string, bool>? _navigationCompleted;
    private readonly Func<long, IntPtr, IntPtr, IntPtr> _resolveNavigationEdit;

    public Win32FileDialogWindowOperations()
        : this(WindowsPlatformAdapter.TryActivateNativeWindow)
    {
    }

    internal Win32FileDialogWindowOperations(
        Func<long, long, bool> activateWindow,
        Func<long, string, bool>? navigationCompleted = null,
        Func<long, IntPtr, IntPtr, IntPtr>? resolveNavigationEdit = null)
    {
        _activateWindow = activateWindow;
        _navigationCompleted = navigationCompleted;
        _resolveNavigationEdit = resolveNavigationEdit ?? ResolveNavigationEdit;
    }

    public bool TryInspect(long windowHandle, out string? currentDirectory)
    {
        currentDirectory = null;
        if (!TryFindDialogControls(new IntPtr(windowHandle), out var controls))
        {
            return false;
        }
        currentDirectory = TryReadCurrentDirectory(controls.BreadcrumbParent) ??
            TryReadCommonDialogDirectory(new IntPtr(windowHandle));
        return true;
    }

    public bool TryNavigate(long windowHandle, string targetDirectory, out string? error)
    {
        error = null;
        if (!TryFindDialogControls(new IntPtr(windowHandle), out var controls))
        {
            error = "The captured window is no longer a supported Windows file dialog.";
            return false;
        }

        var normalized = Path.GetFullPath(targetDirectory);
        if (!Path.EndsInDirectorySeparator(normalized))
        {
            normalized += Path.DirectorySeparatorChar;
        }

        var navigationEdit = controls.EditHandle;
        if (controls.BreadcrumbParent != IntPtr.Zero)
        {
            if (!_activateWindow(windowHandle, 0))
            {
                error = "Windows did not restore the captured file dialog before opening its address bar.";
                return false;
            }
            navigationEdit = _resolveNavigationEdit(
                windowHandle,
                controls.EditHandle,
                controls.BreadcrumbParent);
            if (navigationEdit == IntPtr.Zero)
            {
                error = "Windows did not expose the captured file dialog address bar.";
                return false;
            }
        }

        var setText = SendMessageTimeout(
            navigationEdit,
            WmSetText,
            IntPtr.Zero,
            normalized,
            SmtoAbortIfHung,
            MessageTimeoutMs,
            out var setTextResult);
        if (setText == IntPtr.Zero || setTextResult == IntPtr.Zero)
        {
            error = "Windows did not accept the target directory in the file dialog.";
            return false;
        }

        var editParent = GetParent(navigationEdit);
        if (editParent != IntPtr.Zero)
        {
            var controlId = GetDlgCtrlID(navigationEdit);
            var notification = new IntPtr((EnChange << 16) | (controlId & 0xFFFF));
            var notified = SendMessageTimeout(
                editParent,
                WmCommand,
                notification,
                navigationEdit,
                SmtoAbortIfHung,
                MessageTimeoutMs,
                out _);
            if (notified == IntPtr.Zero)
            {
                error = "The file dialog stopped responding while its path was updated.";
                return false;
            }
        }

        if (!_activateWindow(windowHandle, navigationEdit.ToInt64()))
        {
            error = "Windows did not restore the captured file dialog and its path field.";
            return false;
        }
        if (!PostMessage(navigationEdit, WmKeyDown, new IntPtr(VkReturn), IntPtr.Zero) ||
            !PostMessage(navigationEdit, WmKeyUp, new IntPtr(VkReturn), IntPtr.Zero))
        {
            error = $"Windows did not queue directory navigation for the file dialog (error: {Marshal.GetLastWin32Error()}).";
            return false;
        }
        var completed = _navigationCompleted?.Invoke(windowHandle, normalized) ??
            WaitForDirectory(windowHandle, controls.BreadcrumbParent, normalized);
        if (!completed)
        {
            error = "The file dialog did not switch to the requested directory before the bounded verification timeout.";
            return false;
        }
        return true;
    }

    public bool TryOpenFile(long windowHandle, string filePath, out string? error)
    {
        error = null;
        if (!TryFindDialogControls(new IntPtr(windowHandle), out var controls))
        {
            error = "The captured window is no longer a supported Windows file dialog.";
            return false;
        }
        string normalized;
        try
        {
            normalized = Path.GetFullPath(filePath);
        }
        catch (Exception exception)
            when (exception is ArgumentException or NotSupportedException)
        {
            error = "The selected file path is invalid.";
            return false;
        }

        var setText = SendMessageTimeout(
            controls.EditHandle,
            WmSetText,
            IntPtr.Zero,
            normalized,
            SmtoAbortIfHung,
            MessageTimeoutMs,
            out var setTextResult);
        if (setText == IntPtr.Zero || setTextResult == IntPtr.Zero)
        {
            error = "Windows did not accept the selected file in the file dialog.";
            return false;
        }
        var editParent = GetParent(controls.EditHandle);
        if (editParent != IntPtr.Zero)
        {
            var controlId = GetDlgCtrlID(controls.EditHandle);
            var notification = new IntPtr((EnChange << 16) | (controlId & 0xFFFF));
            var notified = SendMessageTimeout(
                editParent,
                WmCommand,
                notification,
                controls.EditHandle,
                SmtoAbortIfHung,
                MessageTimeoutMs,
                out _);
            if (notified == IntPtr.Zero)
            {
                error = "The file dialog stopped responding while the selected file was entered.";
                return false;
            }
        }
        if (!_activateWindow(windowHandle, controls.EditHandle.ToInt64()))
        {
            error = "Windows did not restore the captured file dialog and its file name field.";
            return false;
        }
        if (!PostMessage(
                controls.EditHandle,
                WmKeyDown,
                new IntPtr(VkReturn),
                IntPtr.Zero) ||
            !PostMessage(
                controls.EditHandle,
                WmKeyUp,
                new IntPtr(VkReturn),
                IntPtr.Zero))
        {
            error =
                $"Windows did not submit the selected file to the dialog (error: {Marshal.GetLastWin32Error()}).";
            return false;
        }
        return true;
    }

    private static IntPtr ResolveNavigationEdit(
        long windowHandle,
        IntPtr fileNameEdit,
        IntPtr breadcrumbParent)
    {
        if (windowHandle == 0 || breadcrumbParent == IntPtr.Zero)
        {
            return fileNameEdit;
        }

        var inputs = new[]
        {
            KeyboardInput(VkControl, 0),
            KeyboardInput(VkL, 0),
            KeyboardInput(VkL, KeyEventKeyUp),
            KeyboardInput(VkControl, KeyEventKeyUp)
        };
        if (SendInput(
                checked((uint)inputs.Length),
                inputs,
                Marshal.SizeOf<Input>()) != inputs.Length)
        {
            return IntPtr.Zero;
        }

        var dialog = new IntPtr(windowHandle);
        var threadId = GetWindowThreadProcessId(dialog, out _);
        var deadline = Environment.TickCount64 + AddressBarFocusTimeoutMs;
        do
        {
            var info = new GuiThreadInfo
            {
                Size = (uint)Marshal.SizeOf<GuiThreadInfo>()
            };
            if (threadId != 0 &&
                GetGUIThreadInfo(threadId, ref info) &&
                info.FocusWindow != IntPtr.Zero &&
                info.FocusWindow != fileNameEdit &&
                IsChild(dialog, info.FocusWindow) &&
                GetWindowClass(info.FocusWindow).Equals(
                    "Edit",
                    StringComparison.OrdinalIgnoreCase) &&
                !HasSearchAncestor(info.FocusWindow, dialog))
            {
                return info.FocusWindow;
            }
            Thread.Sleep(AddressBarFocusPollMs);
        }
        while (Environment.TickCount64 < deadline && IsWindow(dialog));
        return IntPtr.Zero;
    }

    private static Input KeyboardInput(ushort virtualKey, uint flags) =>
        new()
        {
            Type = InputKeyboard,
            Data = new InputUnion
            {
                Keyboard = new KeyboardInputData
                {
                    VirtualKey = virtualKey,
                    Flags = flags
                }
            }
        };

    private static bool TryFindDialogControls(
        IntPtr windowHandle,
        out FileDialogControls controls)
    {
        controls = default;
        if (windowHandle == IntPtr.Zero || !IsWindow(windowHandle) ||
            !GetWindowClass(windowHandle).Equals("#32770", StringComparison.Ordinal))
        {
            return false;
        }

        var breadcrumbParent = IntPtr.Zero;
        var preferredEdit = IntPtr.Zero;
        var fallbackEdit = IntPtr.Zero;
        var hasFileNameMarker = false;
        var hasFileTypeComboBox = false;
        _ = EnumChildWindows(windowHandle, (child, _) =>
        {
            var className = GetWindowClass(child);
            var controlId = GetDlgCtrlID(child);
            if (className.Equals("Breadcrumb Parent", StringComparison.OrdinalIgnoreCase))
            {
                breadcrumbParent = child;
            }
            if (className.Equals("ComboBox", StringComparison.OrdinalIgnoreCase) ||
                className.Equals("ComboBoxEx32", StringComparison.OrdinalIgnoreCase))
            {
                hasFileTypeComboBox |= controlId == FileTypeComboId;
            }
            if (!className.Equals("Edit", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
            if (controlId is FileNameEditId or FileNameComboId ||
                HasAncestorControlId(child, windowHandle, FileNameEditId) ||
                HasAncestorControlId(child, windowHandle, FileNameComboId))
            {
                hasFileNameMarker = true;
                preferredEdit = child;
            }
            else if (fallbackEdit == IntPtr.Zero && !HasSearchAncestor(child, windowHandle))
            {
                fallbackEdit = child;
            }
            return true;
        }, IntPtr.Zero);

        var modern = breadcrumbParent != IntPtr.Zero;
        var classic = hasFileNameMarker && hasFileTypeComboBox;
        var edit = preferredEdit != IntPtr.Zero ? preferredEdit : fallbackEdit;
        if ((!modern && !classic) || edit == IntPtr.Zero)
        {
            return false;
        }
        controls = new FileDialogControls(edit, breadcrumbParent);
        return true;
    }

    private static bool WaitForDirectory(
        long windowHandle,
        IntPtr breadcrumbParent,
        string targetDirectory)
    {
        var deadline = Environment.TickCount64 + NavigationVerificationTimeoutMs;
        do
        {
            Thread.Sleep(NavigationVerificationPollMs);
            var currentDirectory = TryReadCurrentDirectory(breadcrumbParent) ??
                TryReadCommonDialogDirectory(new IntPtr(windowHandle));
            if (PathsEqual(currentDirectory, targetDirectory))
            {
                return true;
            }
        }
        while (Environment.TickCount64 < deadline && IsWindow(new IntPtr(windowHandle)));
        return false;
    }

    private static bool PathsEqual(string? left, string right) =>
        left is not null && string.Equals(
            left.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
            right.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
            StringComparison.OrdinalIgnoreCase);

    private static string? TryReadCommonDialogDirectory(IntPtr windowHandle)
    {
        const int maxCharacters = 1_024;
        var processHandle = IntPtr.Zero;
        var remoteBuffer = IntPtr.Zero;
        try
        {
            _ = GetWindowThreadProcessId(windowHandle, out var processId);
            if (processId == 0)
            {
                return null;
            }
            processHandle = OpenProcess(
                ProcessVmOperation | ProcessVmRead | ProcessVmWrite,
                false,
                processId);
            if (processHandle == IntPtr.Zero)
            {
                return null;
            }
            var byteCount = (nuint)(maxCharacters * sizeof(char));
            remoteBuffer = VirtualAllocEx(
                processHandle,
                IntPtr.Zero,
                byteCount,
                MemCommit | MemReserve,
                PageReadWrite);
            if (remoteBuffer == IntPtr.Zero)
            {
                return null;
            }
            var sent = SendMessageTimeout(
                windowHandle,
                CdmGetFolderPath,
                new IntPtr(maxCharacters),
                remoteBuffer,
                SmtoAbortIfHung,
                MessageTimeoutMs,
                out var copiedCharacters);
            if (sent == IntPtr.Zero || copiedCharacters.ToInt64() <= 0)
            {
                return null;
            }
            var localBuffer = new byte[(int)byteCount];
            if (!ReadProcessMemory(
                    processHandle,
                    remoteBuffer,
                    localBuffer,
                    byteCount,
                    out _))
            {
                return null;
            }
            var value = Encoding.Unicode.GetString(localBuffer).TrimEnd('\0');
            return NormalizeAbsolutePath(value);
        }
        finally
        {
            if (remoteBuffer != IntPtr.Zero)
            {
                _ = VirtualFreeEx(processHandle, remoteBuffer, 0, MemRelease);
            }
            if (processHandle != IntPtr.Zero)
            {
                _ = CloseHandle(processHandle);
            }
        }
    }

    private static bool HasAncestorControlId(IntPtr child, IntPtr root, int expected)
    {
        var current = GetParent(child);
        while (current != IntPtr.Zero && current != root)
        {
            if (GetDlgCtrlID(current) == expected)
            {
                return true;
            }
            current = GetParent(current);
        }
        return false;
    }

    private static bool HasSearchAncestor(IntPtr child, IntPtr root)
    {
        var current = GetParent(child);
        while (current != IntPtr.Zero && current != root)
        {
            if (GetWindowClass(current).Contains("Search", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
            current = GetParent(current);
        }
        return false;
    }

    private static string? TryReadCurrentDirectory(IntPtr breadcrumbParent)
    {
        if (breadcrumbParent == IntPtr.Zero)
        {
            return null;
        }
        string? currentDirectory = null;
        _ = EnumChildWindows(breadcrumbParent, (child, _) =>
        {
            if (!GetWindowClass(child).Equals("ToolbarWindow32", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
            var text = new StringBuilder(1024);
            var sent = SendMessageTimeout(
                child,
                WmGetText,
                new IntPtr(text.Capacity),
                text,
                SmtoAbortIfHung,
                25,
                out _);
            if (sent == IntPtr.Zero)
            {
                return false;
            }
            var candidate = NormalizeBreadcrumbText(text.ToString());
            if (candidate is null)
            {
                return false;
            }
            currentDirectory = candidate;
            return false;
        }, IntPtr.Zero);
        return currentDirectory;
    }

    internal static string? NormalizeBreadcrumbText(string value)
    {
        var text = value.Trim();
        var direct = NormalizeAbsolutePath(text);
        if (direct is not null)
        {
            return direct;
        }
        var separators = new[] { ':', '：' };
        var separator = text.IndexOfAny(separators);
        if (separator >= 0 && separator + 1 < text.Length)
        {
            var candidate = text[(separator + 1)..].Trim();
            return NormalizeAbsolutePath(candidate);
        }
        return null;
    }

    private static string? NormalizeAbsolutePath(string value)
    {
        try
        {
            return Path.IsPathFullyQualified(value) ? Path.GetFullPath(value) : null;
        }
        catch (Exception error) when (error is ArgumentException or NotSupportedException)
        {
            return null;
        }
    }

    private static string GetWindowClass(IntPtr windowHandle)
    {
        var className = new StringBuilder(256);
        var length = GetClassName(windowHandle, className, className.Capacity);
        return length > 0 ? className.ToString() : string.Empty;
    }

    private const int FileNameEditId = 1152;
    private const int FileNameComboId = 1148;
    private const int FileTypeComboId = 1136;
    private const uint WmGetText = 0x000D;
    private const uint WmSetText = 0x000C;
    private const uint WmCommand = 0x0111;
    private const uint WmKeyDown = 0x0100;
    private const uint WmKeyUp = 0x0101;
    private const uint WmUser = 0x0400;
    private const uint CdmGetFolderPath = WmUser + 102;
    private const int EnChange = 0x0300;
    private const int VkReturn = 0x0D;
    private const ushort VkControl = 0x11;
    private const ushort VkL = 0x4C;
    private const uint InputKeyboard = 1;
    private const uint KeyEventKeyUp = 0x0002;
    private const uint SmtoAbortIfHung = 0x0002;
    private const uint MessageTimeoutMs = 100;
    private const int NavigationVerificationTimeoutMs = 700;
    private const int NavigationVerificationPollMs = 35;
    private const int AddressBarFocusTimeoutMs = 300;
    private const int AddressBarFocusPollMs = 15;
    private const uint ProcessVmOperation = 0x0008;
    private const uint ProcessVmRead = 0x0010;
    private const uint ProcessVmWrite = 0x0020;
    private const uint MemCommit = 0x1000;
    private const uint MemReserve = 0x2000;
    private const uint MemRelease = 0x8000;
    private const uint PageReadWrite = 0x04;

    private readonly record struct FileDialogControls(
        IntPtr EditHandle,
        IntPtr BreadcrumbParent);

    [StructLayout(LayoutKind.Sequential)]
    private struct Input
    {
        public uint Type;
        public InputUnion Data;
    }

    [StructLayout(LayoutKind.Explicit)]
    private struct InputUnion
    {
        [FieldOffset(0)]
        public KeyboardInputData Keyboard;

        [FieldOffset(0)]
        public MouseInputData Mouse;

        [FieldOffset(0)]
        public HardwareInputData Hardware;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct KeyboardInputData
    {
        public ushort VirtualKey;
        public ushort ScanCode;
        public uint Flags;
        public uint Time;
        public UIntPtr ExtraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MouseInputData
    {
        public int X;
        public int Y;
        public uint MouseData;
        public uint Flags;
        public uint Time;
        public UIntPtr ExtraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct HardwareInputData
    {
        public uint Message;
        public ushort ParameterLow;
        public ushort ParameterHigh;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct GuiThreadInfo
    {
        public uint Size;
        public uint Flags;
        public IntPtr ActiveWindow;
        public IntPtr FocusWindow;
        public IntPtr CaptureWindow;
        public IntPtr MenuOwnerWindow;
        public IntPtr MoveSizeWindow;
        public IntPtr CaretWindow;
        public Rect CaretRect;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Rect
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    private delegate bool EnumChildCallback(IntPtr windowHandle, IntPtr parameter);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr windowHandle);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsChild(IntPtr parent, IntPtr windowHandle);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetGUIThreadInfo(uint threadId, ref GuiThreadInfo info);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(uint inputCount, Input[] inputs, int inputSize);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr windowHandle,
        StringBuilder className,
        int maxCount);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumChildWindows(
        IntPtr parent,
        EnumChildCallback callback,
        IntPtr parameter);

    [DllImport("user32.dll")]
    private static extern int GetDlgCtrlID(IntPtr windowHandle);

    [DllImport("user32.dll")]
    private static extern IntPtr GetParent(IntPtr windowHandle);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(
        IntPtr windowHandle,
        out int processId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr SendMessageTimeout(
        IntPtr windowHandle,
        uint message,
        IntPtr wParam,
        StringBuilder lParam,
        uint flags,
        uint timeout,
        out IntPtr result);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr SendMessageTimeout(
        IntPtr windowHandle,
        uint message,
        IntPtr wParam,
        string lParam,
        uint flags,
        uint timeout,
        out IntPtr result);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SendMessageTimeout(
        IntPtr windowHandle,
        uint message,
        IntPtr wParam,
        IntPtr lParam,
        uint flags,
        uint timeout,
        out IntPtr result);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool PostMessage(
        IntPtr windowHandle,
        uint message,
        IntPtr wParam,
        IntPtr lParam);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr OpenProcess(
        uint processAccess,
        [MarshalAs(UnmanagedType.Bool)] bool inheritHandle,
        int processId);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr VirtualAllocEx(
        IntPtr processHandle,
        IntPtr address,
        nuint size,
        uint allocationType,
        uint protection);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool VirtualFreeEx(
        IntPtr processHandle,
        IntPtr address,
        nuint size,
        uint freeType);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ReadProcessMemory(
        IntPtr processHandle,
        IntPtr baseAddress,
        byte[] buffer,
        nuint size,
        out nuint bytesRead);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(IntPtr handle);

}
