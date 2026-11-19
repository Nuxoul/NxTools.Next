using System.Runtime.InteropServices;
using System.Threading;

namespace NxTools.PlatformHost.Windows;

// Slots and interface identities are from the installed Windows SDK UIAutomationClient.h
// (10.0.26100.0). All interface references belong to the initiating MTA; a focus
// callback may hand an owned reference to that same MTA, but never to an STA.
internal static class NativeUiaCom
{
    internal const int ElementNotAvailable = unchecked((int)0x80040201);
    internal const int NotSupported = unchecked((int)0x80040204);
    internal const int WrongThread = unchecked((int)0x8001010E);
    internal const int Failure = unchecked((int)0x80004005);
    internal const int NoInterface = unchecked((int)0x80004002);
    internal static readonly Guid AutomationId = new("30cbe57d-d9d0-452a-ab13-7ac5ac4825ee");
    internal static readonly Guid Automation2Id = new("34723aff-0c9d-49d0-9896-7ab52df8cd8a");
    internal static readonly Guid ValuePatternId = new("a94cd8b1-0844-4cd6-9d2d-640537ab39e9");

    internal static T Method<T>(nint pointer, int slot) where T : Delegate =>
        Marshal.GetDelegateForFunctionPointer<T>(
            Marshal.ReadIntPtr(Marshal.ReadIntPtr(pointer), slot * IntPtr.Size));

    internal static void Check(int hr) => Marshal.ThrowExceptionForHR(hr);

    internal static nint Required(int hr, nint pointer)
    {
        if (hr < 0)
        {
            if (pointer != 0) Marshal.Release(pointer);
            Check(hr);
        }
        if (pointer == 0) Check(ElementNotAvailable);
        return pointer;
    }

    internal static nint? Optional(int hr, nint pointer)
    {
        if (hr == ElementNotAvailable)
        {
            if (pointer != 0) Marshal.Release(pointer);
            return null;
        }
        if (hr < 0)
        {
            if (pointer != 0) Marshal.Release(pointer);
            Check(hr);
        }
        return pointer == 0 ? null : pointer;
    }

    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int NoArgOut(nint self, out nint result);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int OnePtrOut(nint self, nint arg, out nint result);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int TwoPtrOut(nint self, nint first, nint second, out int result);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int HandleCacheOut(nint self, nint hwnd, nint cache, out nint result);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int ScopeConditionOut(nint self, int scope, nint condition, out nint result);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int IntOut(nint self, out int result);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int HandleOut(nint self, out nint result);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int IntIn(nint self, int value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int PtrIn(nint self, nint value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int TwoPtrIn(nint self, nint first, nint second);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int PatternAs(nint self, int pattern, ref Guid iid, out nint result);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    internal delegate int PropertyCondition(nint self, int property, NativeUiaVariant value, out nint result);

    [DllImport("ole32.dll", ExactSpelling = true)]
    internal static extern int CoInitializeEx(nint reserved, uint model);
    [DllImport("ole32.dll", ExactSpelling = true)]
    internal static extern void CoUninitialize();
    [DllImport("ole32.dll", ExactSpelling = true)]
    internal static extern int CoGetApartmentType(out int type, out int qualifier);
    [DllImport("ole32.dll", ExactSpelling = true)]
    internal static extern int CoCreateInstance(ref Guid clsid, nint outer, uint context, ref Guid iid, out nint instance);
    [DllImport("oleaut32.dll", ExactSpelling = true)]
    internal static extern int VariantClear(ref NativeUiaVariant variant);
    [DllImport("oleaut32.dll", ExactSpelling = true)]
    internal static extern void SysFreeString(nint value);
}

// 24-byte x64 VARIANT: vt at offset 0; the BSTR union arm at offset 8.
[StructLayout(LayoutKind.Explicit, Size = 24)]
internal struct NativeUiaVariant
{
    [FieldOffset(0)] internal ushort Type;
    [FieldOffset(8)] internal nint Bstr;
}

internal sealed class NativeUiAutomation : IDisposable
{
    private static readonly Guid ClassId = new("e22ad333-b25f-460c-83d0-0581107395c9");
    private nint _client;
    private bool _initialized;
    private NativeFocusSubscription? _subscription;

    internal NativeUiAutomation()
    {
        NativeUiaCom.Check(NativeUiaCom.CoInitializeEx(0, 0)); // COINIT_MULTITHREADED; S_FALSE also owns one uninitialize.
        _initialized = true;
        try
        {
            NativeUiaCom.Check(NativeUiaCom.CoGetApartmentType(out var apartment, out _));
            if (apartment != 1) NativeUiaCom.Check(NativeUiaCom.WrongThread); // APTTYPE_MTA
            var clsid = ClassId;
            var iid = NativeUiaCom.AutomationId;
            _client = NativeUiaCom.Required(
                NativeUiaCom.CoCreateInstance(ref clsid, 0, 1, ref iid, out var client), client);
        }
        catch
        {
            Dispose();
            throw;
        }
    }

    private nint Client => _client != 0 ? _client : throw new ObjectDisposedException(nameof(NativeUiAutomation));

    internal void SetTimeouts(int milliseconds)
    {
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(milliseconds);
        var iid = NativeUiaCom.Automation2Id;
        var automation2 = NativeUiaCom.Required(Marshal.QueryInterface(Client, ref iid, out var queried), queried);
        try
        {
            // IUIAutomation has 58 slots (0..57); IUIAutomation2 appends AutoSetFocus
            // get/put, ConnectionTimeout get/put, TransactionTimeout get/put.
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.IntIn>(automation2, 61)(automation2, milliseconds));
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.IntIn>(automation2, 63)(automation2, milliseconds));
        }
        finally
        {
            Marshal.Release(automation2);
        }
    }

    internal NativeUiaElement ElementFromHandle(long hwnd)
    {
        var client = Client;
        using var cache = CreateIdentityCache();
        var hr = NativeUiaCom.Method<NativeUiaCom.HandleCacheOut>(client, 10)(
            client, new nint(hwnd), cache.Handle, out var element);
        return new NativeUiaElement(NativeUiaCom.Required(hr, element));
    }

    internal NativeUiaElement? FocusedElement()
    {
        var client = Client;
        using var cache = CreateIdentityCache();
        var hr = NativeUiaCom.Method<NativeUiaCom.OnePtrOut>(client, 12)(client, cache.Handle, out var element);
        var owned = NativeUiaCom.Optional(hr, element);
        return owned is nint pointer ? new NativeUiaElement(pointer) : null;
    }

    internal NativeUiaTreeWalker RawViewWalker()
    {
        var client = Client;
        var hr = NativeUiaCom.Method<NativeUiaCom.NoArgOut>(client, 16)(client, out var walker);
        return new NativeUiaTreeWalker(NativeUiaCom.Required(hr, walker));
    }

    internal NativeUiaCondition AutomationIdCondition(string id)
    {
        ArgumentNullException.ThrowIfNull(id);
        var bstr = Marshal.StringToBSTR(id);
        var variant = new NativeUiaVariant { Type = 8, Bstr = bstr }; // VT_BSTR
        try
        {
            var client = Client;
            var hr = NativeUiaCom.Method<NativeUiaCom.PropertyCondition>(client, 23)(
                client, 30011, variant, out var condition);
            return new NativeUiaCondition(NativeUiaCom.Required(hr, condition));
        }
        finally
        {
            NativeUiaCom.Check(NativeUiaCom.VariantClear(ref variant));
        }
    }

    internal bool Compare(NativeUiaElement left, NativeUiaElement right)
    {
        var client = Client;
        NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.TwoPtrOut>(client, 3)(
            client, left.Handle, right.Handle, out var same));
        return same != 0; // Win32 BOOL, not a C# bool.
    }

    internal IDisposable SubscribeFocus(
        Func<int, long, bool> acceptCachedIdentity,
        Action<NativeUiaElement> onFocus,
        Action<int> onNativeFailure)
    {
        ArgumentNullException.ThrowIfNull(acceptCachedIdentity);
        ArgumentNullException.ThrowIfNull(onFocus);
        ArgumentNullException.ThrowIfNull(onNativeFailure);
        var client = Client;
        if (_subscription is not null) throw new InvalidOperationException("Focus handler already registered.");
        var cache = CreateIdentityCache();
        NativeFocusHandler? handler = null;
        try
        {
            handler = new NativeFocusHandler(acceptCachedIdentity, onFocus, onNativeFailure);
            var hr = NativeUiaCom.Method<NativeUiaCom.TwoPtrIn>(client, 39)(client, cache.Handle, handler.Handle);
            NativeUiaCom.Check(hr);
            var subscription = new NativeFocusSubscription(this, client, cache, handler, onNativeFailure);
            _subscription = subscription;
            return subscription;
        }
        catch
        {
            handler?.Stop();
            if (handler is not null)
            {
                try
                {
                    var remove = NativeUiaCom.Method<NativeUiaCom.PtrIn>(client, 40)(client, handler.Handle);
                    if (remove < 0) NativeFocusSubscription.Report(onNativeFailure, remove);
                }
                catch (Exception cleanupError) { NativeFocusSubscription.Report(onNativeFailure, cleanupError.HResult); }
            }
            handler?.Drain();
            handler?.ReleaseLocal();
            cache.Dispose();
            throw;
        }
    }

    private NativeUiaCacheRequest CreateIdentityCache()
    {
        var client = Client;
        var hr = NativeUiaCom.Method<NativeUiaCom.NoArgOut>(client, 20)(client, out var cache);
        var request = new NativeUiaCacheRequest(NativeUiaCom.Required(hr, cache));
        try
        {
            var pointer = request.Handle;
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.IntIn>(pointer, 3)(pointer, 30002)); // ProcessId
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.IntIn>(pointer, 3)(pointer, 30020)); // NativeWindowHandle
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.IntIn>(pointer, 7)(pointer, 1)); // put_TreeScope: TreeScope_Element
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.IntIn>(pointer, 11)(pointer, 1)); // put_AutomationElementMode: Full
            return request;
        }
        catch
        {
            request.Dispose();
            throw;
        }
    }

    internal void Forget(NativeFocusSubscription subscription)
    {
        if (ReferenceEquals(_subscription, subscription)) _subscription = null;
    }

    public void Dispose()
    {
        try
        {
            _subscription?.Dispose();
        }
        finally
        {
            _subscription = null;
            var client = Interlocked.Exchange(ref _client, 0);
            if (client != 0) Marshal.Release(client);
            if (_initialized)
            {
                _initialized = false;
                NativeUiaCom.CoUninitialize();
            }
        }
    }
}

internal abstract class NativeUiaReference : IDisposable
{
    private nint _pointer;

    protected NativeUiaReference(nint ownedPointer) => _pointer = ownedPointer;
    internal nint Handle => _pointer != 0 ? _pointer : throw new ObjectDisposedException(GetType().Name);

    public void Dispose()
    {
        var pointer = Interlocked.Exchange(ref _pointer, 0);
        if (pointer != 0) Marshal.Release(pointer);
    }
}

internal sealed class NativeUiaCacheRequest(nint ownedPointer) : NativeUiaReference(ownedPointer) { }
internal sealed class NativeUiaCondition(nint ownedPointer) : NativeUiaReference(ownedPointer) { }

internal sealed class NativeUiaElement(nint ownedPointer) : NativeUiaReference(ownedPointer)
{
    internal int ProcessId
    {
        get
        {
            var pointer = Handle;
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.IntOut>(pointer, 20)(pointer, out var id));
            return id;
        }
    }

    internal long NativeWindowHandle
    {
        get
        {
            var pointer = Handle;
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.HandleOut>(pointer, 36)(pointer, out var hwnd));
            return hwnd.ToInt64();
        }
    }

    internal NativeUiaElement Clone()
    {
        var pointer = Handle;
        Marshal.AddRef(pointer);
        return new NativeUiaElement(pointer);
    }

    internal NativeUiaElement? FindFirst(NativeUiaCondition condition)
    {
        var pointer = Handle;
        var hr = NativeUiaCom.Method<NativeUiaCom.ScopeConditionOut>(pointer, 5)(
            pointer, 7, condition.Handle, out var found); // TreeScope_Subtree
        var owned = NativeUiaCom.Optional(hr, found);
        return owned is nint element ? new NativeUiaElement(element) : null;
    }

    internal NativeUiaValuePattern? ValuePattern()
    {
        var pointer = Handle;
        var iid = NativeUiaCom.ValuePatternId;
        var hr = NativeUiaCom.Method<NativeUiaCom.PatternAs>(pointer, 14)(
            pointer, 10002, ref iid, out var pattern); // UIA_ValuePatternId
        if (hr is NativeUiaCom.NotSupported or NativeUiaCom.NoInterface)
        {
            if (pattern != 0) Marshal.Release(pattern);
            return null;
        }
        if (hr < 0)
        {
            if (pattern != 0) Marshal.Release(pattern);
            NativeUiaCom.Check(hr);
        }
        return pattern == 0 ? null : new NativeUiaValuePattern(pattern);
    }
}

internal sealed class NativeUiaTreeWalker(nint ownedPointer) : NativeUiaReference(ownedPointer)
{
    internal NativeUiaElement? Parent(NativeUiaElement element)
    {
        var pointer = Handle;
        var hr = NativeUiaCom.Method<NativeUiaCom.OnePtrOut>(pointer, 3)(pointer, element.Handle, out var parent);
        var owned = NativeUiaCom.Optional(hr, parent);
        return owned is nint node ? new NativeUiaElement(node) : null;
    }
}

internal sealed class NativeUiaValuePattern(nint ownedPointer) : NativeUiaReference(ownedPointer)
{
    internal string Value
    {
        get
        {
            var pointer = Handle;
            var hr = NativeUiaCom.Method<NativeUiaCom.NoArgOut>(pointer, 4)(pointer, out var bstr);
            try
            {
                NativeUiaCom.Check(hr);
                return bstr == 0 ? string.Empty : Marshal.PtrToStringBSTR(bstr);
            }
            finally
            {
                if (bstr != 0) NativeUiaCom.SysFreeString(bstr);
            }
        }
    }

    internal void SetValue(string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        var bstr = Marshal.StringToBSTR(value);
        try
        {
            var pointer = Handle;
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.PtrIn>(pointer, 3)(pointer, bstr));
        }
        finally
        {
            NativeUiaCom.SysFreeString(bstr);
        }
    }
}

internal sealed class NativeFocusSubscription : IDisposable
{
    private NativeUiAutomation? _owner;
    private readonly nint _client;
    private readonly NativeUiaCacheRequest _cache;
    private readonly NativeFocusHandler _handler;
    private readonly Action<int> _failure;
    private int _removalError;

    internal NativeFocusSubscription(NativeUiAutomation owner, nint client, NativeUiaCacheRequest cache,
        NativeFocusHandler handler, Action<int> failure)
    {
        _owner = owner;
        _client = client;
        _cache = cache;
        _handler = handler;
        _failure = failure;
    }

    internal static void Report(Action<int> failure, int hr)
    {
        try { failure(hr); }
        catch { /* An observer cannot escape a native callback or suppress COM cleanup. */ }
    }

    public void Dispose()
    {
        var owner = Interlocked.Exchange(ref _owner, null);
        if (owner is null)
        {
            NativeUiaCom.Check(_removalError);
            return;
        }
        _handler.Stop();
        try
        {
            _removalError = NativeUiaCom.Method<NativeUiaCom.PtrIn>(_client, 40)(_client, _handler.Handle);
            if (_removalError < 0) Report(_failure, _removalError);
        }
        catch (Exception ex)
        {
            _removalError = ex.HResult;
            Report(_failure, _removalError);
        }
        finally
        {
            _handler.Drain();
            _handler.ReleaseLocal();
            _cache.Dispose();
            if (_removalError >= 0) owner.Forget(this);
        }
        NativeUiaCom.Check(_removalError);
    }
}

// Native IUnknown/IUIAutomationFocusChangedEventHandler. Its GCHandle and vtable stay
// allocated until COM's actual last Release, including references retained by UIA.
internal sealed class NativeFocusHandler
{
    private static readonly Guid UnknownId = new("00000000-0000-0000-c000-000000000046");
    private static readonly Guid FocusId = new("c270f6b5-5c69-4290-9745-7a7f97169468");
    private static readonly Guid AgileId = new("94ea2b94-e9cc-49e0-c0ff-ee64ca8f5b90");
    private static readonly QueryInterfaceDelegate Query = QueryInterface;
    private static readonly RefDelegate Add = AddRef;
    private static readonly RefDelegate Release = ReleaseRef;
    private static readonly FocusDelegate Focus = HandleFocus;
    private static readonly nint Vtable = CreateVtable();

    private readonly Func<int, long, bool> _accept;
    private readonly Action<NativeUiaElement> _onFocus;
    private readonly Action<int> _failure;
    private readonly object _drainGate = new();
    private int _references = 1;
    private int _entered;
    private int _stopped;
    private nint _instance;
    private GCHandle _root;

    internal NativeFocusHandler(Func<int, long, bool> accept, Action<NativeUiaElement> onFocus, Action<int> failure)
    {
        _accept = accept;
        _onFocus = onFocus;
        _failure = failure;
        _root = GCHandle.Alloc(this);
        try
        {
            _instance = Marshal.AllocHGlobal(2 * IntPtr.Size);
            Marshal.WriteIntPtr(_instance, Vtable);
            Marshal.WriteIntPtr(_instance, IntPtr.Size, GCHandle.ToIntPtr(_root));
        }
        catch
        {
            if (_instance != 0) Marshal.FreeHGlobal(_instance);
            _root.Free();
            throw;
        }
    }

    internal nint Handle => _instance;
    internal void Stop() => Volatile.Write(ref _stopped, 1);
    internal void ReleaseLocal() => ReleaseRef(_instance);

    internal void Drain()
    {
        lock (_drainGate)
        {
            while (Volatile.Read(ref _entered) != 0) Monitor.Wait(_drainGate);
        }
    }

    private static nint CreateVtable()
    {
        var table = Marshal.AllocHGlobal(4 * IntPtr.Size);
        Marshal.WriteIntPtr(table, 0, Marshal.GetFunctionPointerForDelegate(Query));
        Marshal.WriteIntPtr(table, IntPtr.Size, Marshal.GetFunctionPointerForDelegate(Add));
        Marshal.WriteIntPtr(table, 2 * IntPtr.Size, Marshal.GetFunctionPointerForDelegate(Release));
        Marshal.WriteIntPtr(table, 3 * IntPtr.Size, Marshal.GetFunctionPointerForDelegate(Focus));
        return table; // One process-lifetime table; each COM instance owns only its own allocation.
    }

    private static NativeFocusHandler From(nint instance) =>
        (NativeFocusHandler)GCHandle.FromIntPtr(Marshal.ReadIntPtr(instance, IntPtr.Size)).Target!;

    private static int QueryInterface(nint instance, ref Guid iid, out nint result)
    {
        result = 0;
        try
        {
            if (iid != UnknownId && iid != FocusId && iid != AgileId)
                return unchecked((int)0x80004002); // E_NOINTERFACE
            AddRef(instance);
            result = instance;
            return 0;
        }
        catch { return NativeUiaCom.Failure; }
    }

    private static uint AddRef(nint instance)
    {
        try { return (uint)Interlocked.Increment(ref From(instance)._references); }
        catch { return 0; }
    }

    private static uint ReleaseRef(nint instance)
    {
        try
        {
            var handler = From(instance);
            var remaining = Interlocked.Decrement(ref handler._references);
            if (remaining == 0)
            {
                Marshal.FreeHGlobal(instance);
                handler._root.Free();
            }
            return (uint)remaining;
        }
        catch { return 0; }
    }

    private static int HandleFocus(nint instance, nint sender)
    {
        NativeFocusHandler? handler = null;
        try
        {
            handler = From(instance);
            Interlocked.Increment(ref handler._entered);
            if (Volatile.Read(ref handler._stopped) != 0 || sender == 0) return 0;
            if (NativeUiaCom.CoGetApartmentType(out var apartment, out _) < 0 || apartment != 1)
                NativeUiaCom.Check(NativeUiaCom.WrongThread);

            // The event cache includes both properties, even for HWND-less WPF children.
            // Reject before AddRef, before the live property getters, and before traversal.
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.IntOut>(sender, 52)(sender, out var pid));
            NativeUiaCom.Check(NativeUiaCom.Method<NativeUiaCom.HandleOut>(sender, 68)(sender, out var hwnd));
            if (!handler._accept(pid, hwnd.ToInt64())) return 0;
            Marshal.AddRef(sender);
            using var owned = new NativeUiaElement(sender);
            handler._onFocus(owned);
        }
        catch (Exception ex)
        {
            if (handler is not null) NativeFocusSubscription.Report(handler._failure, ex.HResult);
        }
        finally
        {
            if (handler is not null && Interlocked.Decrement(ref handler._entered) == 0)
            {
                lock (handler._drainGate) Monitor.PulseAll(handler._drainGate);
            }
        }
        return 0; // No managed exception crosses the reverse COM boundary.
    }

    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate int QueryInterfaceDelegate(nint self, ref Guid iid, out nint result);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate uint RefDelegate(nint self);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate int FocusDelegate(nint self, nint sender);
}
