using System.Runtime.InteropServices;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Reviewed effects: owned scratch files, injected HWND snapshots and self-owned in-memory COM only. No live Shell.
internal static class ExplorerRevealFeedbackTests
{
    public static Task ConfirmedTargetSurvivesDeniedForeground()
    {
        WithTargets((directory, file) =>
        {
            foreach (var target in new[] { directory, file })
            {
                var stages = new List<string>();
                var fake = new Platform { Confirmed = true };
                var result = ExplorerShellActivation.LaunchAndActivate(target, fake, stages.Add);
                Assert(result.Success, "observed exact target must succeed even when Windows denies foreground");
                Assert(result.Data == "target_confirmed_foreground_not_confirmed", "target and foreground outcomes remain distinct");
                Assert(fake.Activations == 1 && fake.Waits == 0 && fake.Fallbacks == 0, "confirmed target cannot trigger recovery launches or focus polling");
                Assert(stages.Contains("target_confirmed") && stages.Contains("foreground_not_confirmed"), "separate payload-free stages");
            }
        });
        WithTargets((directory,_)=>{
            var views=new Views{Values=[new(new IntPtr(10),new IntPtr(20),true,directory,[])]};
            Assert(ExplorerShellActivation.ObserveTarget(directory,true,views,true)==new IntPtr(10),"directory entry confirms its active view without a selected child");
            var entered=new Platform{Views=views,Foreground=new IntPtr(10)};
            Assert(ExplorerShellActivation.EnterAndActivate(directory,entered).Success && entered.NativeCalls==0 && entered.Activations==1,"existing actual directory view reused without duplicate launch");
        });
        return Task.CompletedTask;
    }

    public static Task FolderAndAcceptanceNeverConfirmFile()
    {
        WithTargets((_, file) =>
        {
            foreach (var nativeResult in new[] { 0, unchecked((int)0x80070005) })
            {
                var fake = new Platform { Foreground = new IntPtr(10), NativeResult = nativeResult };
                var result = ExplorerShellActivation.LaunchAndActivate(file, fake);
                Assert(!result.Success && result.Data == "target_not_confirmed", "existing folder/foreground/API acceptance cannot confirm unselected file");
                Assert(fake.Waits == 7 && fake.Observations == 8 && fake.Fallbacks == 1 && fake.Activations == 0, "single bounded observation loop and fallback; no activation of wrong target");
            }
        });
        return Task.CompletedTask;
    }

    public static Task DelayedSelectionAndNativeFailureRemainRetryable()
    {
        WithTargets((_, file) =>
        {
            var fake = new Platform { PublishAfter = 4, Foreground = new IntPtr(11) };
            var result = ExplorerShellActivation.LaunchAndActivate(file, fake);
            Assert(result.Success && result.Data == "foreground_confirmed" && fake.Waits == 3, "delayed selection then matching child-root foreground");
            fake = new Platform();
            Assert(!ExplorerShellActivation.LaunchAndActivate(file, fake).Success, "unobserved target fails");
            fake.Confirmed = true;
            Assert(ExplorerShellActivation.LaunchAndActivate(file, fake).Success, "failure permits explicit retry");
            fake = new Platform { Confirmed = true, ThrowActivation = true };
            result = ExplorerShellActivation.LaunchAndActivate(file, fake, _ => throw new IOException("private diagnostic payload"));
            Assert(result.Success && result.Error is null && result.NativeErrorCode == unchecked((int)0x80131509),
                "activation exception retains native code but cannot undo target success; diagnostics cannot alter result");
            fake = new Platform { ThrowObservation = true };
            var failed = ExplorerShellActivation.LaunchAndActivate(file, fake);
            Assert(!failed.Success && failed.NativeErrorCode == unchecked((int)0x80004005) &&
                failed.Error is not null && !failed.Error.Contains("private"), "observation failure retains native HRESULT without payload");
            Assert(!ExplorerShellActivation.LaunchAndActivate(file + ".missing", new Platform()).Success, "missing file fails without parent fallback");
        });
        return Task.CompletedTask;
    }

    public static Task DestroyedOrRemovedTargetCannotSucceed()
    {
        WithTargets((_, file) =>
        {
            var fake = new Platform { Confirmed = true, DestroyAfterActivation = true };
            Assert(!ExplorerShellActivation.LaunchAndActivate(file, fake).Success, "destroyed observed window cannot authorize completion");
            fake = new Platform { Confirmed = true, DuringObservation = () => File.Delete(file) };
            Assert(!ExplorerShellActivation.LaunchAndActivate(file, fake).Success, "removed target cannot authorize completion");
        });
        return Task.CompletedTask;
    }

    public static Task ActiveViewSelectionRejectsTabRootAmbiguity()
    {
        WithTargets((directory, file) =>
        {
            var snapshots = new Views();
            var root = new IntPtr(10);
            snapshots.Values =
            [
                new(root, new IntPtr(21), false, directory, [file]), // inactive matching tab
                new(root, new IntPtr(22), true, directory, [file + ".wrong"]),
                new(new IntPtr(30), new IntPtr(31), true, directory + "-other", [file]),
            ];
            var fake = new Platform { Views = snapshots, Foreground = root };
            Assert(!ExplorerShellActivation.LaunchAndActivate(file, fake).Success,
                "hidden matching tab, same-root wrong selection and wrong folder cannot confirm target");
            Assert(ExplorerShellActivation.ObserveTarget(directory, true, snapshots) == IntPtr.Zero,
                "Reveal directory must not accept entering the directory itself");
            var parent = Path.GetDirectoryName(directory)!;
            snapshots.Values = [new(root, new IntPtr(22), true, parent, [directory])];
            Assert(ExplorerShellActivation.LaunchAndActivate(directory, new Platform { Views = snapshots }).Success,
                "directory Reveal confirms the exact directory selected in its parent");
            snapshots.Values = [new(root, new IntPtr(22), true, parent, [directory + "-wrong"])];
            Assert(!ExplorerShellActivation.LaunchAndActivate(directory, new Platform { Views = snapshots }).Success,
                "wrong selected directory remains unconfirmed");
            var driveRoot = Path.GetPathRoot(directory)!;
            snapshots.Values = [new(root, new IntPtr(22), true, driveRoot, [])];
            Assert(ExplorerShellActivation.ObserveTarget(driveRoot, true, snapshots) == root,
                "a drive root has no parent and confirms its own active view");
            snapshots.Values = [new(root, new IntPtr(22), true, directory, [file.ToUpperInvariant()])];
            fake = new Platform { Views = snapshots };
            Assert(ExplorerShellActivation.LaunchAndActivate(file, fake).Success,
                "exact active-view selection uses case-insensitive full filesystem identity");
            Assert(fake.Observations == 1 && fake.Activations == 1, "production matcher feeds real entry without second worker");
            Assert(ExplorerShellActivation.ObserveTarget(@"\\?\" + file, false, snapshots) == root,
                "an extended DOS target and the Shell's normal path identify the same selected file");
            snapshots.Values = [new(root, new IntPtr(22), true, @"\\?\" + directory, [@"\\?\" + file])];
            Assert(ExplorerShellActivation.ObserveTarget(file, false, snapshots) == root,
                "equivalent filesystem prefixes may occur on either side of target observation");
            snapshots.Values = [new(root, new IntPtr(22), true, directory, [file + ".wrong"])];
            Assert(ExplorerShellActivation.ObserveTarget(@"\\?\" + file, false, snapshots) == IntPtr.Zero,
                "prefix normalization must not accept another filename");
            const string uncDirectory = @"\\fixture-server\fixture-share\下载";
            const string uncFile = @"\\fixture-server\fixture-share\下载\fixture.bin";
            snapshots.Values = [new(root, new IntPtr(22), true, uncDirectory, [uncFile])];
            Assert(ExplorerShellActivation.ObserveTarget(@"\\?\UNC\fixture-server\fixture-share\下载\fixture.bin", false, snapshots) == root,
                "extended UNC and Shell UNC path aliases preserve exact share and selection");
            Assert(ExplorerShellActivation.ObserveTarget(@"\\?\UNC\other-server\fixture-share\下载\fixture.bin", false, snapshots) == IntPtr.Zero,
                "different UNC server identities must never collapse");
        });
        return Task.CompletedTask;
    }

    public static Task NativeComContractsHaveExactVtableSlots()
    {
        var browser = typeof(ExplorerShellActivation.IExplorerShellBrowser);
        var view = typeof(ExplorerShellActivation.IExplorerShellView);
        Assert(browser.GUID == new Guid("000214E2-0000-0000-C000-000000000046") &&
            view.GUID == new Guid("000214E3-0000-0000-C000-000000000046"), "native Shell browser/view IIDs");
        foreach (var contract in new[] { browser, view, typeof(ExplorerShellActivation.IExplorerServiceProvider), typeof(ExplorerShellActivation.IExplorerFolder2) })
        {
            var kind = System.Reflection.CustomAttributeExtensions.GetCustomAttribute<System.Runtime.InteropServices.InterfaceTypeAttribute>(contract)!;
            Assert(kind.Value == System.Runtime.InteropServices.ComInterfaceType.InterfaceIsIUnknown && contract.IsImport, "real IUnknown COM contract");
            Assert(contract.GetMethods().All(method => (method.MethodImplementationFlags & System.Reflection.MethodImplAttributes.PreserveSig) != 0), "HRESULTs remain explicit");
        }
        var browserMethods = browser.GetMethods().OrderBy(method => method.MetadataToken).ToArray();
        var viewMethods = view.GetMethods().OrderBy(method => method.MetadataToken).ToArray();
        Assert(browserMethods[0].Name == "GetWindow" && browserMethods[1].Name == "ContextSensitiveHelp" &&
            browserMethods[12].Name == "QueryActiveShellView", "IShellBrowser includes IOleWindow and all preceding slots");
        Assert(viewMethods[0].Name == "GetWindow" && viewMethods[1].Name == "ContextSensitiveHelp" &&
            viewMethods[12].Name == "GetItemObject", "IShellView background dispatch is slot15 including IUnknown");
        Assert(browserMethods[12].GetParameters()[0].ParameterType == view.MakeByRefType(), "typed active shell view output");
        Assert(viewMethods[12].GetParameters()[1].ParameterType == typeof(Guid).MakeByRefType() &&
            viewMethods[12].GetParameters()[2].IsOut, "GetItemObject REFIID and COM out interface");
        var folder2 = typeof(ExplorerShellActivation.IExplorerFolder2);
        var folderMethods = folder2.GetMethods().OrderBy(method => method.MetadataToken).ToArray();
        Assert(folder2.GUID == new Guid("f0d2d8ef-3890-11d2-bf8b-00c04fb93661") &&
            folderMethods[0].Name == "GetTypeInfoCount" && folderMethods[13].Name == "GetDetailsOf" &&
            folderMethods[14].Name == "GetSelf" && folderMethods[14].GetParameters()[0].IsOut,
            "Folder2.get_Self preserves IDispatch and every base Folder native slot");
        FolderSelfUsesTheFolder2NativeContract();
        ExplorerObservationTests.BrowserChildTraversesNativeActiveViewChain().GetAwaiter().GetResult();
        return Task.CompletedTask;
    }

    private static void FolderSelfUsesTheFolder2NativeContract()
    {
        Exception? failure = null;
        var thread = new Thread(() =>
        {
            try
            {
                using var apartment = ExplorerShellApartment.Enter();
                Marshal.ThrowExceptionForHR(apartment.Result);
                foreach (var nativeResult in new[] { 0, unchecked((int)0x80070005) })
                {
                    using var fixture = new FolderComFixture(nativeResult);
                    object? folder = null;
                    object? self = null;
                    try
                    {
                        folder = Marshal.GetObjectForIUnknown(fixture.Folder);
                        if (nativeResult == 0)
                        {
                            self = ExplorerShellActivation.ReadFolderSelf(folder);
                            var identity = Marshal.GetIUnknownForObject(self);
                            try { Assert(identity == fixture.Item, "get_Self must return the owned FolderItem COM identity"); }
                            finally { Marshal.Release(identity); }
                        }
                        else
                        {
                            var failed = false;
                            try { self = ExplorerShellActivation.ReadFolderSelf(folder); }
                            catch (Exception error) { failed = error.HResult == nativeResult; }
                            Assert(failed && self is null, "native Folder2 failure must retain HRESULT and cannot manufacture self evidence");
                        }
                        Assert(fixture.Folder2Queries > 0 && fixture.SelfCalls == 1 && fixture.WrongSlotCalls == 0,
                            "Folder.Self must query IID_Folder2 and invoke get_Self at native slot17; base Folder IDispatch has no Self");
                    }
                    finally
                    {
                        if (self is not null && Marshal.IsComObject(self)) Marshal.FinalReleaseComObject(self);
                        if (folder is not null && Marshal.IsComObject(folder)) Marshal.FinalReleaseComObject(folder);
                    }
                    Assert(fixture.References == 1, "all reader/RCW references are released on the owning STA");
                }
            }
            catch (Exception error) { failure = error; }
        });
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        Assert(thread.Join(TimeSpan.FromSeconds(10)), "owned COM contract fixture must finish within its bound");
        if (failure is not null) throw new InvalidOperationException("owned native Folder reader regression: " + failure.Message, failure);
    }

    // Raw owned COM tables avoid a managed CCW being unwrapped back into the
    // original C# object. No CoCreateInstance, Shell enumeration or registry lookup.
    // SDK ShlDisp.h: Folder has10 members after IDispatch; Folder2 adds get_Self.
    private sealed class FolderComFixture : IDisposable
    {
        [UnmanagedFunctionPointer(CallingConvention.StdCall)]
        private delegate int Query(IntPtr instance, ref Guid iid, out IntPtr value);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)]
        private delegate uint RefCount(IntPtr instance);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)]
        private delegate int TypeCount(IntPtr instance, out uint count);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)]
        private delegate int TypeInfo(IntPtr instance, uint index, uint locale, out IntPtr info);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)]
        private delegate int Names(IntPtr instance, ref Guid iid, IntPtr names, uint count, uint locale, IntPtr ids);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)]
        private delegate int Invoke(IntPtr instance, int id, ref Guid iid, uint locale, ushort flags,
            IntPtr parameters, IntPtr result, IntPtr exception, IntPtr argumentError);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)]
        private delegate int Self(IntPtr instance, out IntPtr value);

        private readonly List<Delegate> _callbacks = [];
        private readonly List<IntPtr> _allocations = [];
        private readonly IntPtr _folder2;
        internal IntPtr Folder { get; }
        internal IntPtr Item { get; }
        internal int References { get; private set; } = 1;
        internal int Folder2Queries { get; private set; }
        internal int SelfCalls { get; private set; }
        internal int WrongSlotCalls { get; private set; }

        internal FolderComFixture(int nativeResult)
        {
            Query query = QueryInterface;
            RefCount add = _ => (uint)++References;
            RefCount release = _ => (uint)--References;
            TypeCount count = (IntPtr _, out uint value) => { value = 0; return 0; };
            TypeInfo info = (IntPtr _, uint index, uint locale, out IntPtr value) => { value = IntPtr.Zero; return unchecked((int)0x80004001); };
            Names names = (IntPtr _, ref Guid iid, IntPtr values, uint length, uint locale, IntPtr ids) => unchecked((int)0x80020006);
            Invoke invoke = (IntPtr _, int id, ref Guid iid, uint locale, ushort flags, IntPtr parameters,
                IntPtr result, IntPtr exception, IntPtr argumentError) => unchecked((int)0x80020003);
            Self preceding = (IntPtr _, out IntPtr value) => { WrongSlotCalls++; value = IntPtr.Zero; return unchecked((int)0x80004001); };
            Self self = (IntPtr _, out IntPtr value) =>
            {
                SelfCalls++;
                value = IntPtr.Zero;
                if (nativeResult < 0) return nativeResult;
                References++;
                value = Item;
                return nativeResult;
            };
            Delegate[] dispatch = [query, add, release, count, info, names, invoke];
            Folder = Allocate(dispatch);
            Item = Allocate(dispatch);
            _folder2 = Allocate([.. dispatch, .. Enumerable.Repeat<Delegate>(preceding, 10), self]);
        }

        private int QueryInterface(IntPtr instance, ref Guid iid, out IntPtr value)
        {
            value = IntPtr.Zero;
            if (iid == new Guid("f0d2d8ef-3890-11d2-bf8b-00c04fb93661") && instance != Item)
            { Folder2Queries++; value = _folder2; }
            else if (iid == new Guid("00000000-0000-0000-C000-000000000046") ||
                     iid == new Guid("00020400-0000-0000-C000-000000000046"))
                value = instance == Item ? Item : Folder;
            if (value == IntPtr.Zero) return unchecked((int)0x80004002);
            References++;
            return 0;
        }

        private IntPtr Allocate(Delegate[] methods)
        {
            var table = Marshal.AllocHGlobal(methods.Length * IntPtr.Size);
            var instance = Marshal.AllocHGlobal(IntPtr.Size);
            _allocations.Add(table); _allocations.Add(instance);
            for (var index = 0; index < methods.Length; index++)
            {
                _callbacks.Add(methods[index]);
                Marshal.WriteIntPtr(table, index * IntPtr.Size, Marshal.GetFunctionPointerForDelegate(methods[index]));
            }
            Marshal.WriteIntPtr(instance, table);
            return instance;
        }

        public void Dispose()
        {
            foreach (var allocation in _allocations) Marshal.FreeHGlobal(allocation);
            GC.KeepAlive(_callbacks);
        }
    }

    private sealed class Views : IExplorerTargetObservationPlatform
    {
        public ExplorerTargetView[] Values = [];
        public IEnumerable<ExplorerTargetView> ReadActiveViews() => Values;
    }

    private sealed class Platform : IExplorerShellPlatform
    {
        public bool SupportsTargetObservation => true;
        public bool Confirmed;
        public int PublishAfter = int.MaxValue;
        public int Observations;
        public int Waits;
        public int Fallbacks;
        public int Activations;
        public int NativeResult;
        public bool ThrowActivation;
        public bool ThrowObservation;
        public bool DestroyAfterActivation;
        public Action? DuringObservation;
        public IntPtr Foreground = new(99);
        public IExplorerTargetObservationPlatform? Views;
        public int NativeCalls;
        public IntPtr FindEnteredDirectory(string directory)=>Views is null?IntPtr.Zero:ExplorerShellActivation.ObserveTarget(directory,true,Views,true);
        public IntPtr FindConfirmedTarget(string target, bool isDirectory)
        {
            Observations++;
            if (ThrowObservation) throw new System.Runtime.InteropServices.COMException("private payload", unchecked((int)0x80004005));
            DuringObservation?.Invoke();
            if (Views is not null) return ExplorerShellActivation.ObserveTarget(target, isDirectory, Views);
            return Confirmed || Observations >= PublishAfter ? new IntPtr(10) : IntPtr.Zero;
        }
        public int OpenNative(string target, bool isDirectory) {NativeCalls++;return NativeResult;}
        public IntPtr FindExplorerWindow(string directory) => new(10); // same folder is intentionally NOT selection evidence
        public void LaunchExplorer(string arguments) => Fallbacks++;
        public bool IsWindow(IntPtr window) => window != IntPtr.Zero && !(DestroyAfterActivation && Activations > 0 && window == new IntPtr(10));
        public bool IsIconic(IntPtr window) => false;
        public void RestoreWindow(IntPtr window) { }
        public void TryActivateWindow(IntPtr window) { Activations++; if (ThrowActivation) throw new InvalidOperationException("private payload"); }
        public IntPtr GetForegroundWindow() => Foreground;
        public IntPtr GetRootWindow(IntPtr window) => window == new IntPtr(11) ? new IntPtr(10) : window;
        public void WaitForNextAttempt() => Waits++;
        public int LastErrorCode => 5;
    }

    private static void WithTargets(Action<string, string> test)
    {
        var root = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath();
        var directory = Path.Combine(root, $"nxtools-reveal-feedback-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        var file = Path.Combine(directory, "fixture.bin");
        File.WriteAllBytes(file, []);
        try { test(directory, file); }
        finally { Directory.Delete(directory, true); }
    }
    private static void Assert(bool condition, string message)
    { if (!condition) throw new InvalidOperationException(message); }
}
