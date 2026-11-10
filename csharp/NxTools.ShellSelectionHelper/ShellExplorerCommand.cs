using System.Runtime.InteropServices;

namespace NxTools.ShellSelectionHelper;

// Windows SDK ShObjIdl_core.idl IExplorerCommand supplies the selection during both
// title query and invocation. The original local-server identity/lifetime owns both.
internal sealed partial class ArchiveSelectionCommand : IExplorerCommand
{
    internal bool IsQuickCreate => _verb is ShellVerb.CompressZip or ShellVerb.CompressSevenZ;

    public int GetTitle(IShellItemArray? selection, out IntPtr title)
    {
        title = IntPtr.Zero;
        var selected = ShellSelectionReader.Read(selection);
        if (!selected.Succeeded) return selected.NativeError < 0 ? selected.NativeError : EFail;
        string label;
        if (IsQuickCreate)
        {
            var hr = selection!.GetItemAt(0, out var item);
            if (hr < 0 || item is null) return hr < 0 ? hr : EFail;
            uint attributes;
            try { hr = item.GetAttributes(0x20000000, out attributes); }
            finally { if (Marshal.IsComObject(item)) Marshal.ReleaseComObject(item); }
            if (hr < 0) return hr;
            var name = Path.GetFileName(selected.Paths[0].TrimEnd('\\', '/'));
            var extension = name.LastIndexOf('.');
            if ((attributes & 0x20000000) == 0 && extension > 0) name = name[..extension];
            if (name.Length == 0) return EFail;
            label = $"压缩为“{name}.{(_verb == ShellVerb.CompressZip ? "zip" : "7z")}”";
        }
        else
        {
            label = _verb switch
            {
                ShellVerb.Open => "使用 NxTools 打开",
                ShellVerb.ExtractSmart => "智能解压",
                ShellVerb.ExtractHere => "解压到此处",
                ShellVerb.ExtractName => "解压到同名文件夹",
                ShellVerb.ExtractDialog => "解压到…",
                ShellVerb.Test => "测试压缩包",
                _ => "新建压缩包…",
            };
        }
        title = Marshal.StringToCoTaskMemUni(label);
        return SOk;
    }

    public int GetIcon(IShellItemArray? selection, out IntPtr icon) { icon = IntPtr.Zero; return 1; }
    public int GetToolTip(IShellItemArray? selection, out IntPtr tooltip) { tooltip = IntPtr.Zero; return 1; }
    public int GetCanonicalName(out Guid name) { name = HelperIdentity.VerbClasses[_verb]; return SOk; }
    public int GetState(IShellItemArray? selection, bool slow, out uint state)
    {
        state = 2; // ECS_HIDDEN until a bounded selection is supplied.
        if (selection is null) return SOk;
        var hr = selection.GetCount(out var count);
        if (hr < 0) return hr;
        if (count is > 0 and <= ShellSelectionReader.MaxPaths)
            state = Volatile.Read(ref _executed) == 0 ? 0u : 1u;
        return SOk;
    }
    public int Invoke(IShellItemArray? selection, IntPtr bindContext)
    {
        var hr = SetSelection(selection);
        return hr < 0 ? hr : Execute();
    }
    public int GetFlags(out uint flags) { flags = 0; return SOk; }
    public int EnumSubCommands(out IntPtr commands) { commands = IntPtr.Zero; return unchecked((int)0x80004001); }
}
