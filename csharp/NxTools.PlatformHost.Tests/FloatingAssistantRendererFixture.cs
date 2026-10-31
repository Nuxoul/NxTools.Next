using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace NxTools.PlatformHost.Tests;

internal static class FloatingAssistantRendererFixture
{
    internal static Task<int> Run()
    {
        var completion = new TaskCompletionSource<int>(TaskCreationOptions.RunContinuationsAsynchronously);
        var thread = new Thread(() =>
        {
            try
            {
                using var window = new PassiveRenderer();
                using var context = new ApplicationContext();
                var reader = new Thread(() =>
                {
                    while (Console.ReadLine() is { } line)
                    {
                        if (line == "move") Native.PostMessageW(window.Handle, 0x8001, IntPtr.Zero, IntPtr.Zero);
                        else if (line == "quit") break;
                    }
                    Native.PostMessageW(window.Handle, 0x0010, IntPtr.Zero, IntPtr.Zero);
                }) { IsBackground = true };
                window.Closed = context.ExitThread;
                // NativeWindow avoids Form replaying a managed non-topmost policy on show.
                window.Show(100, 100, 80, 80);
                Console.WriteLine(window.Handle.ToInt64());
                Console.Out.Flush();
                reader.Start();
                Application.Run(context);
                completion.SetResult(0);
            }
            catch (Exception error) { completion.SetException(error); }
        }) { IsBackground = true };
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        return completion.Task;
    }

    internal static Process Start()
    {
        var start = new ProcessStartInfo(Environment.ProcessPath!)
        { UseShellExecute = false, CreateNoWindow = true, RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true };
        start.ArgumentList.Add("--floating-renderer-fixture");
        return Process.Start(start) ?? throw new InvalidOperationException("fixture child did not start");
    }

    internal sealed class PassiveRenderer : NativeWindow, IDisposable
    {
        private readonly IntPtr _brush = Native.CreateSolidBrush(0x0065C72D);
        internal Action? Closed;
        internal PassiveRenderer()
        {
            CreateHandle(new CreateParams
            {
                Caption = "NxTools owned renderer fixture",
                Style = unchecked((int)0x80000000),
                ExStyle = 0x08000088,
                X = 100, Y = 100, Width = 80, Height = 80,
            });
        }
        internal void ShowPassive() => Show(100, 100, 80, 80);
        internal void Update() => Native.RedrawWindow(Handle, IntPtr.Zero, IntPtr.Zero, 0x0105);
        internal void Show(int x, int y, int width, int height)
        {
            if (!Native.SetWindowPos(Handle, new IntPtr(-1), x, y, width, height, 0x0050))
                throw new InvalidOperationException("Owned native renderer show failed.");
            Native.RedrawWindow(Handle, IntPtr.Zero, IntPtr.Zero, 0x0105);
        }
        protected override void WndProc(ref Message message)
        {
            if (message.Msg == 0x0021) { message.Result = new IntPtr(3); return; }
            if (message.Msg == 0x0014)
            {
                Native.GetClientRect(Handle, out var bounds);
                Native.FillRect(message.WParam, ref bounds, _brush);
                message.Result = new IntPtr(1);
                return;
            }
            if (message.Msg == 0x8001) { Show(200, 160, 96, 96); return; }
            if (message.Msg == 0x0010) { Closed?.Invoke(); return; }
            base.WndProc(ref message);
        }
        public void Dispose()
        {
            DestroyHandle();
            Native.DeleteObject(_brush);
        }
    }

    private static class Native
    {
        [StructLayout(LayoutKind.Sequential)] internal struct Rect { internal int Left, Top, Right, Bottom; }
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern bool PostMessageW(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern bool GetClientRect(IntPtr window, out Rect bounds);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern int FillRect(IntPtr dc, ref Rect bounds, IntPtr brush);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern bool RedrawWindow(IntPtr window, IntPtr update, IntPtr region, uint flags);
        [DllImport("gdi32.dll", ExactSpelling = true)] internal static extern IntPtr CreateSolidBrush(uint color);
        [DllImport("gdi32.dll", ExactSpelling = true)] internal static extern bool DeleteObject(IntPtr value);
    }
}
