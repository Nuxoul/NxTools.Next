using System.Runtime.InteropServices;

namespace NxTools.ShellSelectionHelper;

internal static partial class SelectionCommandIdentity
{
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int ArrayTextFn(IntPtr self, IntPtr selection, out IntPtr value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int GuidOutFn(IntPtr self, out Guid value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int StateFn(IntPtr self, IntPtr selection, int slow, out uint state);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int InvokeFn(IntPtr self, IntPtr selection, IntPtr bindContext);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int UIntOutFn(IntPtr self, out uint value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int PointerOutFn(IntPtr self, out IntPtr value);

    // Exact SDK IExplorerCommand slots3..10, sharing the original canonical IUnknown.
    private static Delegate[] ExplorerMethods() =>
    [
        new QueryInterfaceFn((IntPtr self, ref Guid iid, out IntPtr result) => QueryInterface(self - 2 * SlotSize, ref iid, out result)),
        new CountFn(self => AddRef(self - 2 * SlotSize)),
        new CountFn(self => Release(self - 2 * SlotSize)),
        new ArrayTextFn(ExplorerTitle),
        new ArrayTextFn((IntPtr self, IntPtr selection, out IntPtr value) => { value = IntPtr.Zero; return 1; }),
        new ArrayTextFn((IntPtr self, IntPtr selection, out IntPtr value) => { value = IntPtr.Zero; return 1; }),
        new GuidOutFn((IntPtr self, out Guid value) => OwnerOf(self - 2 * SlotSize).Command.GetCanonicalName(out value)),
        new StateFn(ExplorerState),
        new InvokeFn((self, selection, _) => Call(self - 2 * SlotSize, command =>
        {
            var hr = SetSelection(command, selection);
            return hr < 0 ? hr : command.Execute();
        })),
        new UIntOutFn((IntPtr self, out uint value) => { value = 0; return 0; }),
        new PointerOutFn((IntPtr self, out IntPtr value) => { value = IntPtr.Zero; return unchecked((int)0x80004001); }),
    ];

    private static int ExplorerTitle(IntPtr self, IntPtr array, out IntPtr value)
    {
        value = IntPtr.Zero;
        IShellItemArray? selection = null;
        try
        {
            selection = array == IntPtr.Zero ? null : (IShellItemArray)Marshal.GetObjectForIUnknown(array);
            return OwnerOf(self - 2 * SlotSize).Command.GetTitle(selection, out value);
        }
        catch (Exception error) { return error.HResult < 0 ? error.HResult : ArchiveSelectionCommand.EFail; }
        finally { if (selection is not null && Marshal.IsComObject(selection)) Marshal.ReleaseComObject(selection); }
    }

    private static int ExplorerState(IntPtr self, IntPtr array, int slow, out uint state)
    {
        state = 2;
        IShellItemArray? selection = null;
        try
        {
            selection = array == IntPtr.Zero ? null : (IShellItemArray)Marshal.GetObjectForIUnknown(array);
            return OwnerOf(self - 2 * SlotSize).Command.GetState(selection, slow != 0, out state);
        }
        catch (Exception error) { return error.HResult < 0 ? error.HResult : ArchiveSelectionCommand.EFail; }
        finally { if (selection is not null && Marshal.IsComObject(selection)) Marshal.ReleaseComObject(selection); }
    }
}
