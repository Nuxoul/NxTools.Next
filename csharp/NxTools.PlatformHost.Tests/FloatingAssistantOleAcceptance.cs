using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using NxTools.PlatformHost.Windows;
using ComData = System.Runtime.InteropServices.ComTypes.IDataObject;

namespace NxTools.PlatformHost.Tests;

// Separately authorized: SetCursorPos temporarily changes the actual user pointer.
// All windows/data are owned fixtures. No button/key injection, clipboard, network or user files.
internal static class FloatingAssistantOleAcceptance
{
    internal static Task ActualOleRoutesAcrossOwnedProcesses() => FloatingAssistantInputTests.OnSta(() =>
    {
        if (!Native.GetCursorPos(out var original)) throw new InvalidOperationException("cursor snapshot failed");
        using var child = FloatingAssistantRendererFixture.Start();
        try
        {
            var line = child.StandardOutput.ReadLineAsync().WaitAsync(TimeSpan.FromSeconds(5)).GetAwaiter().GetResult();
            var renderer = new IntPtr(long.Parse(line!));
            var received = new List<FloatingAssistantDropData>();
            using var surface = new FloatingAssistantInputSurface(renderer, (uint)child.Id,
                value => { received.Add(value); return true; });
            surface.SetInteraction(true, false);
            if (!Native.SetCursorPos(140, 140)) throw new InvalidOperationException("owned cursor positioning failed");
            var foreground = FloatingAssistantInputTests.Native.GetForegroundWindow();
            using var timer = new System.Windows.Forms.Timer { Interval = 20 };
            timer.Tick += (_, _) => { };
            timer.Start();
            var data = new DataObject();
            data.SetText("owned real OLE 文本", TextDataFormat.UnicodeText);
            var result = Native.DoDragDrop((ComData)data, new TimedDropSource(surface.Handle), 1, out var effect);
            Assert(result == 0x00040100 && effect == 1 && received.Count == 1 && received[0].Text == "owned real OLE 文本",
                "actual Windows DoDragDrop routes exact text into registered receiver");
            Assert(FloatingAssistantInputTests.Native.GetForegroundWindow() == foreground, "actual OLE never activates receiver");
            using var bitmap = new System.Drawing.Bitmap(2, 2);
            var image = new DataObject(); image.SetImage(bitmap);
            result = Native.DoDragDrop((ComData)image, new TimedDropSource(surface.Handle), 1, out effect);
            Assert(result == 0x00040100 && effect == 1 && received.Count == 2 && received[1].ImageBytes is { Length: > 0 },
                "actual Windows DoDragDrop routes image data without clipboard");
            var files = new DataObject();
            files.SetFileDropList(new System.Collections.Specialized.StringCollection { @"C:\synthetic\image.gif" });
            result = Native.DoDragDrop((ComData)files, new TimedDropSource(surface.Handle), 1, out effect);
            Assert(result == 0x00040100 && effect == 1 && received.Count == 3 && received[2].Files.Single() == @"C:\synthetic\image.gif",
                "actual OLE retains local image path without file IO");
            surface.SetInteraction(false, false);
            Assert(FloatingAssistantInputTests.Native.WindowFromPoint(new(140, 140)) == renderer,
                "native hide removes receiver from OLE/pointer discovery");
        }
        finally
        {
            _ = Native.SetCursorPos(original.X, original.Y);
            if (!child.HasExited)
            {
                child.StandardInput.WriteLine("quit"); child.StandardInput.Flush();
                if (!child.WaitForExit(5000)) { child.Kill(entireProcessTree: true); child.WaitForExit(); }
            }
        }
    });

    private static void Assert(bool condition, string message)
    { if (!condition) throw new InvalidOperationException(message); }

    [ComVisible(true), ClassInterface(ClassInterfaceType.None)]
    private sealed class TimedDropSource(IntPtr target) : IDropSource
    {
        private readonly Stopwatch _elapsed = Stopwatch.StartNew();
        public int QueryContinueDrag(bool escape, uint keys)
        {
            if (escape || _elapsed.Elapsed > TimeSpan.FromSeconds(3) || !Native.GetCursorPos(out var point) ||
                FloatingAssistantInputTests.Native.WindowFromPoint(new(point.X, point.Y)) != target)
                return 0x00040101;
            return _elapsed.Elapsed > TimeSpan.FromMilliseconds(200) ? 0x00040100 : 0;
        }
        public int GiveFeedback(uint effect) => 0x00040102;
    }
    [ComImport, Guid("00000121-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IDropSource
    {
        [PreserveSig] int QueryContinueDrag([MarshalAs(UnmanagedType.Bool)] bool escape, uint keys);
        [PreserveSig] int GiveFeedback(uint effect);
    }
    private static class Native
    {
        [StructLayout(LayoutKind.Sequential)] internal readonly record struct Point(int X, int Y);
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool GetCursorPos(out Point point);
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool SetCursorPos(int x, int y);
        [DllImport("ole32.dll", ExactSpelling = true)] internal static extern int DoDragDrop(ComData data, IDropSource source, uint effects, out uint effect);
    }
}
