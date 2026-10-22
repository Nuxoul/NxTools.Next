using System.Runtime.InteropServices;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Owned HWNDs and unmanaged COM tables only; no Shell activation/enumeration.
public static class ExplorerObservationTests
{
    public static Task BrowserChildTraversesNativeActiveViewChain()
    {
        Exception? failure = null;
        var worker = new Thread(() =>
        {
            try
            {
                using var apartment = ExplorerShellApartment.Enter();
                Marshal.ThrowExceptionForHR(apartment.Result);
                using var fixture = new ChainFixture();
                foreach (var childBrowser in new[] { false, true })
                {
                    fixture.Reset(childBrowser);
                    var failures = new List<int>();
                    var stages = new List<string>();
                    var reader = new ExplorerShellActivation.NativeTargetObservationPlatform(false, failures.Add,
                        () => new OwnedCollection(fixture), stages.Add);
                    Assert(!reader.ReadActiveViews().Any(), "failed background object cannot manufacture target evidence");
                    Assert(fixture.HwndCalls == 1 && fixture.ServiceCalls == 1 && fixture.ActiveViewCalls == 1 &&
                        fixture.WindowCalls == 1 && fixture.BackgroundCalls == 1,
                        $"native chain child={childBrowser} counts={fixture.HwndCalls}/{fixture.ServiceCalls}/{fixture.ActiveViewCalls}/{fixture.WindowCalls}/{fixture.BackgroundCalls} failures={failures.Count}");
                    Assert(failures.SequenceEqual(new[] { ChainFixture.AccessDenied }), "background HRESULT must survive the actual chain");
                    Assert(stages.Contains("observation_background_failed") &&
                        stages.Contains("observation_browser_child_hwnd") == childBrowser,
                        "reader reports payload-free child and failing COM seam stages");
                    Assert(fixture.References == 1 && fixture.WrongCalls == 0, "native COM slots and references remain exact");
                }
                foreach (var rejection in new[] { "hidden", "other_root" })
                {
                    fixture.Reset(true);
                    fixture.Rejection = rejection;
                    var failures = new List<int>();
                    var reader = new ExplorerShellActivation.NativeTargetObservationPlatform(false, failures.Add,
                        () => new OwnedCollection(fixture));
                    Assert(!reader.ReadActiveViews().Any() && fixture.BackgroundCalls == 0 && failures.Count == 0,
                        "hidden active view or view belonging to another root must be rejected before reading background");
                    Assert(fixture.References == 1, "rejected view references are balanced");
                    reader = new ExplorerShellActivation.NativeTargetObservationPlatform(false, failures.Add,
                        () => new OwnedCollection(fixture), _ => throw new InvalidOperationException("owned diagnostic failure"));
                    Assert(!reader.ReadActiveViews().Any() && fixture.References == 1,
                        "diagnostic callback exceptions cannot alter reader behavior or reference cleanup");
                }
            }
            catch (Exception error) { failure = error; }
        });
        worker.SetApartmentState(ApartmentState.STA);
        worker.Start();
        Assert(worker.Join(TimeSpan.FromSeconds(10)), "owned native observation fixture must finish within bound");
        if (failure is not null) throw new InvalidOperationException("owned Explorer observation chain regression: " + failure.Message, failure);
        return Task.CompletedTask;
    }

    // Dynamic collection injection changes only ShellWindows creation. Every candidate
    // is an RCW over unmanaged tables, not a managed implementation of the COM interfaces.
    public sealed class OwnedCollection
    {
        private readonly ChainFixture _fixture;
        internal OwnedCollection(ChainFixture fixture) => _fixture = fixture;
        public int Count => 1;
        public object Item(int index) => Marshal.GetObjectForIUnknown(_fixture.Candidate);
    }

    internal sealed class ChainFixture : IDisposable
    {
        internal const int AccessDenied = unchecked((int)0x80070005);
        private const int NotImplemented = unchecked((int)0x80004001);
        private static readonly Guid UnknownId = new("00000000-0000-0000-C000-000000000046");
        private static readonly Guid DispatchId = new("00020400-0000-0000-C000-000000000046");
        private static readonly Guid BrowserId = new("D30C1661-CDAF-11D0-8A3E-00C04FC9E26E");
        private static readonly Guid ProviderId = typeof(ExplorerShellActivation.IExplorerServiceProvider).GUID;
        private static readonly Guid ShellBrowserId = typeof(ExplorerShellActivation.IExplorerShellBrowser).GUID;
        private static readonly Guid ShellViewId = typeof(ExplorerShellActivation.IExplorerShellView).GUID;
        [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int Query(IntPtr instance, ref Guid iid, out IntPtr value);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate uint RefCount(IntPtr instance);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int TypeCount(IntPtr instance, out uint count);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int TypeInfo(IntPtr instance, uint index, uint locale, out IntPtr info);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int Names(IntPtr instance, ref Guid iid, IntPtr names, uint count, uint locale, IntPtr ids);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int Invoke(IntPtr instance, int id, ref Guid iid, uint locale, ushort flags, IntPtr parameters, IntPtr result, IntPtr exception, IntPtr argumentError);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int Service(IntPtr instance, ref Guid service, ref Guid iid, out IntPtr value);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int Output(IntPtr instance, out IntPtr value);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int Background(IntPtr instance, uint flags, ref Guid iid, out IntPtr value);
        private readonly List<Delegate> _callbacks = [];
        private readonly List<IntPtr> _allocations = [];
        private readonly WndProc _windowProcedure = DefWindowProc;
        private readonly List<string> _classes = [];
        private readonly List<IntPtr> _windows = [];
        private readonly IntPtr _root;
        private readonly IntPtr _tab;
        private readonly IntPtr _view;
        private readonly IntPtr _hiddenView;
        private readonly IntPtr _otherView;
        private readonly IntPtr _provider;
        private readonly IntPtr _browser;
        private readonly IntPtr _shellView;
        private bool _childBrowser;
        internal string? Rejection;
        internal IntPtr Candidate { get; }
        internal int References = 1;
        internal int HwndCalls, ServiceCalls, ActiveViewCalls, WindowCalls, BackgroundCalls, WrongCalls;

        internal ChainFixture()
        {
            Register("CabinetWClass"); Register("ShellTabWindowClass");
            _root = Window("CabinetWClass", IntPtr.Zero, true);
            _tab = Window("ShellTabWindowClass", _root, true);
            _view = Window("STATIC", _tab, true);
            _hiddenView = Window("STATIC", _tab, false);
            var otherRoot = Window("STATIC", IntPtr.Zero, true);
            _otherView = Window("STATIC", otherRoot, true);
            Query query = QueryInterface;
            RefCount add = _ => (uint)++References;
            RefCount release = _ => (uint)--References;
            TypeCount count = (IntPtr _, out uint value) => { value = 0; return 0; };
            TypeInfo info = (IntPtr _, uint index, uint locale, out IntPtr value) => { value = IntPtr.Zero; return NotImplemented; };
            Names names = (IntPtr _, ref Guid iid, IntPtr values, uint length, uint locale, IntPtr ids) => unchecked((int)0x80020006);
            Invoke invoke = (IntPtr _, int id, ref Guid iid, uint locale, ushort flags, IntPtr parameters, IntPtr result, IntPtr exception, IntPtr argumentError) =>
            {
                if (id != -515 || (flags & 2) == 0) { WrongCalls++; return unchecked((int)0x80020003); }
                HwndCalls++;
                Marshal.WriteInt16(result, 3); // VT_I4, IWebBrowser2.HWND is LONG
                Marshal.WriteInt32(result, 8, (_childBrowser ? _tab : _root).ToInt32());
                return 0;
            };
            Output unused = (IntPtr _, out IntPtr value) => { WrongCalls++; value = IntPtr.Zero; return NotImplemented; };
            Service service = (IntPtr _, ref Guid sid, ref Guid iid, out IntPtr value) =>
            {
                ServiceCalls++; value = IntPtr.Zero;
                if (sid != new Guid("4C96BE40-915C-11CF-99D3-00AA004AE837") || iid != ShellBrowserId) { WrongCalls++; return NotImplemented; }
                References++; value = _browser; return 0;
            };
            Output active = (IntPtr _, out IntPtr value) => { ActiveViewCalls++; References++; value = _shellView; return 0; };
            Output window = (IntPtr _, out IntPtr value) =>
            {
                WindowCalls++; value = Rejection == "hidden" ? _hiddenView : Rejection == "other_root" ? _otherView : _view; return 0;
            };
            Background background = (IntPtr _, uint flags, ref Guid iid, out IntPtr value) =>
            {
                BackgroundCalls++; value = IntPtr.Zero;
                if (flags != 0 || iid != DispatchId) { WrongCalls++; return NotImplemented; }
                return AccessDenied; // deliberate boundary failure, never target evidence
            };
            Candidate = Allocate([query, add, release, count, info, names, invoke]);
            _provider = Allocate([query, add, release, service]);
            _browser = Allocate([query, add, release, .. Enumerable.Repeat<Delegate>(unused, 12), active]);
            _shellView = Allocate([query, add, release, window, .. Enumerable.Repeat<Delegate>(unused, 11), background]);
        }

        internal void Reset(bool childBrowser)
        { _childBrowser = childBrowser; Rejection = null; HwndCalls = ServiceCalls = ActiveViewCalls = WindowCalls = BackgroundCalls = WrongCalls = 0; }
        private int QueryInterface(IntPtr instance, ref Guid iid, out IntPtr value)
        {
            value = IntPtr.Zero;
            if (instance == Candidate || instance == _provider)
            {
                if (iid == UnknownId || iid == DispatchId || iid == BrowserId) value = Candidate;
                else if (iid == ProviderId) value = _provider;
            }
            else if (instance == _browser && (iid == UnknownId || iid == ShellBrowserId)) value = _browser;
            else if (instance == _shellView && (iid == UnknownId || iid == ShellViewId)) value = _shellView;
            if (value == IntPtr.Zero) return unchecked((int)0x80004002);
            References++; return 0;
        }
        private IntPtr Allocate(Delegate[] methods)
        {
            var table = Marshal.AllocHGlobal(methods.Length * IntPtr.Size);
            var instance = Marshal.AllocHGlobal(IntPtr.Size);
            _allocations.Add(table); _allocations.Add(instance);
            for (var index = 0; index < methods.Length; index++)
            { _callbacks.Add(methods[index]); Marshal.WriteIntPtr(table, index * IntPtr.Size, Marshal.GetFunctionPointerForDelegate(methods[index])); }
            Marshal.WriteIntPtr(instance, table); return instance;
        }
        private void Register(string name)
        {
            var windowClass = new WindowClass { Size = (uint)Marshal.SizeOf<WindowClass>(), Procedure = _windowProcedure, ClassName = name };
            Assert(RegisterClassEx(ref windowClass) != 0, "owned Explorer-class registration");
            _classes.Add(name);
        }
        private IntPtr Window(string name, IntPtr parent, bool visible)
        {
            // NOACTIVATE/TOOLWINDOW: no user window mutation or foreground changes.
            var style = (parent == IntPtr.Zero ? 0x80000000u : 0x40000000u) | (visible ? 0x10000000u : 0);
            var window = CreateWindowEx(0x08000080, name, "", style, -30000, -30000, 32, 32, parent, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            Assert(window != IntPtr.Zero, "owned native window creation"); _windows.Add(window); return window;
        }
        public void Dispose()
        {
            foreach (var window in Enumerable.Reverse(_windows)) DestroyWindow(window);
            foreach (var name in _classes) UnregisterClass(name, IntPtr.Zero);
            foreach (var allocation in _allocations) Marshal.FreeHGlobal(allocation);
            GC.KeepAlive(_callbacks); GC.KeepAlive(_windowProcedure);
        }
        private delegate IntPtr WndProc(IntPtr hwnd, uint message, IntPtr wparam, IntPtr lparam);
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        private struct WindowClass
        {
            public uint Size, Style;
            public WndProc Procedure;
            public int ClassExtra, WindowExtra;
            public IntPtr Instance, Icon, Cursor, Background;
            public string? MenuName, ClassName;
            public IntPtr SmallIcon;
        }
        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern ushort RegisterClassEx(ref WindowClass value);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern bool UnregisterClass(string name, IntPtr instance);
        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern IntPtr CreateWindowEx(uint exStyle, string className, string title, uint style, int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
        [DllImport("user32.dll")] private static extern bool DestroyWindow(IntPtr window);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr DefWindowProc(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    }
    private static void Assert(bool condition, string message)
    { if (!condition) throw new InvalidOperationException(message); }
}
