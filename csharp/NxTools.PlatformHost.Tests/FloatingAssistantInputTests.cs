using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class FloatingAssistantInputTests
{
    internal static Task TransparentHitAndPassThrough() => OnSta(() =>
    {
        using var renderer = new FloatingAssistantRendererFixture.PassiveRenderer();
        renderer.ShowPassive();
        var foreground = Native.GetForegroundWindow();
        var bounds = Native.GetBounds(renderer.Handle);
        var center = new Native.Point(bounds.Left + bounds.Width / 2, bounds.Top + bounds.Height / 2);
        // Prior self-owned HWND fixtures can still be finishing native window teardown;
        // require this fixture's own topmost pixel before claiming input transparency.
        var ready = System.Diagnostics.Stopwatch.StartNew();
        while (ready.Elapsed < TimeSpan.FromSeconds(2) &&
               (Native.WindowFromPoint(center) != renderer.Handle || Native.PixelAt(center) != 0x0065C72D))
        { renderer.ShowPassive(); renderer.Update(); Application.DoEvents(); Thread.Sleep(10); }
        Assert(Native.WindowFromPoint(center) == renderer.Handle && Native.PixelAt(center) == 0x0065C72D,
            $"fixture renderer must own its client pixel before input layer; hit={Native.WindowFromPoint(center).ToInt64():X}, renderer={renderer.Handle.ToInt64():X}, pixel={Native.PixelAt(center):X}, bounds={bounds}");
        var before = Native.PixelAt(center);
        using (var surface = new FloatingAssistantInputSurface(renderer.Handle, (uint)Environment.ProcessId))
        {
            Assert(!Native.IsWindowVisible(surface.Handle), "input layer must start hidden");
            surface.SetInteraction(true, false);
            Application.DoEvents();
            Assert(Native.WindowFromPoint(center) == surface.Handle, "visible input layer must own native hit testing");
            Assert(Native.PixelAt(center) == before, "input layer must not repaint or dim the GPUI renderer");
            Assert(Native.GetForegroundWindow() == foreground, "input layer reveal must not activate");
            Assert(Native.SendMessageW(surface.Handle, 0x21, IntPtr.Zero, IntPtr.Zero).ToInt64() == 3,
                "WM_MOUSEACTIVATE must return MA_NOACTIVATE");
            surface.SetInteraction(true, true);
            Application.DoEvents();
            Assert(Native.WindowFromPoint(center) == renderer.Handle, "click-through must return the real underlying HWND");
            surface.SetInteraction(true, false);
            Application.DoEvents();
            Assert(Native.WindowFromPoint(center) == surface.Handle, "click-through must be reversible");
            surface.SetInteraction(false, false);
            Assert(!Native.IsWindowVisible(surface.Handle), "hidden receiver cannot leave an input catcher");
        }
        Assert(Native.WindowFromPoint(center) == renderer.Handle, "disposal must remove input layer");
    });

    internal static Task ForeignRendererOleLifecycle() => OnSta(() =>
    {
        using var child = FloatingAssistantRendererFixture.Start();
        var renderer = new IntPtr(long.Parse(child.StandardOutput.ReadLineAsync().WaitAsync(TimeSpan.FromSeconds(5)).GetAwaiter().GetResult()!));
        var foreground = Native.GetForegroundWindow();
        var received = new List<FloatingAssistantDropData>();
        var target = new FloatingAssistantDropTarget(() => true, value => { received.Add(value); return true; });
        try
        {
            using var surface = new FloatingAssistantInputSurface(renderer, (uint)child.Id);
            // Match coordinator Apply: native owner policy is applied after receiver creation.
            Assert(Native.SetWindowPos(renderer, new IntPtr(-1), 0, 0, 0, 0, 0x0013) &&
                (Native.GetWindowLongPtrW(renderer, -20).ToInt64() & 8) != 0,
                "foreign fixture renderer native topmost policy must be observed");
            surface.SetInteraction(true, false);
            Application.DoEvents();
            Assert(Native.GetWindow(surface.Handle, 4) == renderer, "receiver owned by the foreign renderer, never reparented");
            Assert(Native.PixelAt(new(140, 140)) == 0x0065C72D && Native.WindowFromPoint(new(140, 140)) == surface.Handle,
                "foreign renderer remains visible beneath actual input target");
            Assert(Native.WindowFromPoint(new(101, 101)) == renderer, "ball corners do not form an invisible input rectangle");
            var duplicate = Native.RegisterDragDrop(surface.Handle, target);
            if (duplicate == 0) _ = Native.RevokeDragDrop(surface.Handle);
            Assert(duplicate == unchecked((int)0x80040101), "input HWND already owns exactly one actual OLE registration");
            Assert(Native.GetForegroundWindow() == foreground, "foreign receiver never activates");
            child.StandardInput.WriteLine("move"); child.StandardInput.Flush();
            var deadline = System.Diagnostics.Stopwatch.StartNew();
            while (deadline.Elapsed < TimeSpan.FromSeconds(3) && Native.WindowFromPoint(new(240, 200)) != renderer)
            { Application.DoEvents(); Thread.Sleep(10); }
            surface.SetInteraction(true, false);
            Application.DoEvents();
            Assert(Native.WindowFromPoint(new(240, 200)) == surface.Handle && Native.WindowFromPoint(new(140, 140)) != surface.Handle,
                "receiver follows confirmed new renderer bounds without stale catcher");
            surface.SetInteraction(true, true);
            Assert(Native.WindowFromPoint(new(240, 200)) == renderer, "foreign pass-through reaches underlying renderer");
        }
        finally
        {
            child.StandardInput.WriteLine("quit"); child.StandardInput.Flush();
            if (!child.WaitForExit(5000)) { child.Kill(entireProcessTree: true); child.WaitForExit(); }
        }
    });

    internal static Task GesturesStaySingleAndLockStillClicks() => OnSta(() =>
    {
        using var renderer = new FloatingAssistantRendererFixture.PassiveRenderer(); renderer.ShowPassive();
        var events = new List<FloatingAssistantInputEvent>();
        using var surface = new FloatingAssistantInputSurface(renderer.Handle, (uint)Environment.ProcessId,
            input: value => events.Add(value));
        surface.SetInteraction(true, false);
        void Send(uint message, int x, int y) => Native.SendMessageW(surface.Handle, message,
            message == 0x0201 || message == 0x0200 ? new IntPtr(1) : IntPtr.Zero, new IntPtr((y << 16) | (x & 65535)));
        Send(0x0201, 30, 30); Send(0x0202, 30, 30);
        Assert(events.Count(value => value.Kind == FloatingAssistantInputKind.LeftClick) == 1, "one native left click");
        Send(0x0204, 30, 30); Send(0x0205, 30, 30);
        Assert(events.Count(value => value.Kind == FloatingAssistantInputKind.RightClick) == 1, "one native right click");
        Send(0x0201, 30, 30); Send(0x0200, 48, 42); Send(0x0202, 48, 42);
        Assert(events.Count(value => value.Kind == FloatingAssistantInputKind.LeftClick) == 1 &&
            events.Count(value => value.Kind == FloatingAssistantInputKind.MoveCompleted) == 1, "drag moves once and never clicks");
        surface.PositionLocked = true;
        Send(0x0201, 30, 30); Send(0x0202, 30, 30);
        Assert(events.Count(value => value.Kind == FloatingAssistantInputKind.LeftClick) == 2, "position lock retains click");
        Send(0x0201, 30, 30); Send(0x0200, 48, 42); Send(0x0202, 48, 42);
        Assert(events.Count(value => value.Kind == FloatingAssistantInputKind.MoveCompleted) == 1 &&
            events.Count(value => value.Kind == FloatingAssistantInputKind.LeftClick) == 2, "locked drag neither moves nor misclicks");
        Send(0x0201, 30, 30); Send(0x0215, 0, 0); Send(0x0202, 30, 30);
        Assert(events.Count(value => value.Kind == FloatingAssistantInputKind.LeftClick) == 2, "capture loss cancels click");
        surface.PositionLocked = false;
        Send(0x0201, 30, 30); Send(0x0200, 42, 40); Send(0x0215, 0, 0); Send(0x0202, 42, 40);
        Assert(events.Count(value => value.Kind == FloatingAssistantInputKind.MoveCompleted) == 2 &&
            events.Count(value => value.Kind == FloatingAssistantInputKind.LeftClick) == 2,
            "capture loss commits a moved renderer anchor once, never dispatches a click");
    });

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }

    internal static Task OnSta(Action operation)
    {
        var completion = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var thread = new Thread(() =>
        {
            try { operation(); completion.SetResult(); }
            catch (Exception error) { completion.SetException(error); }
        }) { IsBackground = true, Name = "NxTools.FloatingInputFixture" };
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        return completion.Task.WaitAsync(TimeSpan.FromSeconds(15));
    }


    internal static class Native
    {
        [StructLayout(LayoutKind.Sequential)]
        internal readonly record struct Point(int X, int Y);
        internal static System.Drawing.Rectangle GetBounds(IntPtr window)
        {
            if (!GetWindowRect(window, out var bounds)) throw new Win32Exception(Marshal.GetLastWin32Error());
            return new(bounds.Left, bounds.Top, bounds.Right - bounds.Left, bounds.Bottom - bounds.Top);
        }
        [StructLayout(LayoutKind.Sequential)] private struct NativeRect
        { internal int Left, Top, Right, Bottom; }
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetWindowRect(IntPtr window, out NativeRect bounds);
        internal static uint PixelAt(Point point)
        {
            var dc = GetDC(IntPtr.Zero);
            if (dc == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            try { return GetPixel(dc, point.X, point.Y); }
            finally { _ = ReleaseDC(IntPtr.Zero, dc); }
        }
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr GetWindow(IntPtr window, uint kind);
        [DllImport("ole32.dll", ExactSpelling = true)] internal static extern int RegisterDragDrop(IntPtr window, IFloatingAssistantDropTarget target);
        [DllImport("ole32.dll", ExactSpelling = true)] internal static extern int RevokeDragDrop(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr WindowFromPoint(Point point);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool IsWindowVisible(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr SendMessageW(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr GetWindowLongPtrW(IntPtr window, int index);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
        [DllImport("user32.dll", ExactSpelling = true)] private static extern IntPtr GetDC(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true)] private static extern int ReleaseDC(IntPtr window, IntPtr dc);
        [DllImport("gdi32.dll", ExactSpelling = true)] private static extern uint GetPixel(IntPtr dc, int x, int y);
    }
}
