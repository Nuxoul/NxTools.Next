using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;
namespace NxTools.PlatformHost.Windows;

internal static class ArchiveJobPicker {
    private sealed record Owner(IntPtr Handle):IWin32Window;
    internal static ArchiveOwnedPickerResult Pick(ArchivePickerCommand c,uint peer,CancellationToken token) {
        ArchiveOwnedPickerResult Result(string[]? paths=null,bool cancelled=false,string? code=null,int? native=null)=>new(1,c.OperationId,c.SessionId,c.Generation,paths??[],cancelled,code,native);
        if(token.IsCancellationRequested)return Result(cancelled:true);
        var window=new IntPtr(c.OwnerWindowHandle);
        if(!IsWindow(window) || GetWindowThreadProcessId(window,out var process)==0 || process!=peer)return Result(code:"invalid_owner",native:1400);
        var ok=BoundedStaOperationRunner.TryRun("NxTools archive owned picker",TimeSpan.FromMinutes(2),
            cancel=>StaDialogRunner.RunInline(()=> {
                if(cancel.IsCancellationRequested)return new WindowsCommandResult(true,Data:"cancelled");
                string[] selected;
                var owner=new Owner(window);
                if(c.Purpose is "folders" or "destination") {
                    using var dialog=new FolderBrowserDialog{Description="选择文件夹",UseDescriptionForTitle=true,InitialDirectory=c.CurrentDirectory??"",ShowNewFolderButton=true};
                    selected=dialog.ShowDialog(owner)==DialogResult.OK?[dialog.SelectedPath]:[];
                }else if(c.Purpose=="save") {
                    using var dialog=new SaveFileDialog{Title="保存压缩包",InitialDirectory=c.CurrentDirectory??"",FileName=c.SuggestedName??"",RestoreDirectory=true,CheckPathExists=true,OverwritePrompt=true,AutoUpgradeEnabled=true};
                    selected=dialog.ShowDialog(owner)==DialogResult.OK?[dialog.FileName]:[];
                }else {
                    using var dialog=new OpenFileDialog{Title="选择源文件",InitialDirectory=c.CurrentDirectory??"",Multiselect=true,CheckFileExists=true,CheckPathExists=true,RestoreDirectory=true,AutoUpgradeEnabled=true};
                    selected=dialog.ShowDialog(owner)==DialogResult.OK?dialog.FileNames:[];
                }
                if(cancel.IsCancellationRequested || selected.Length==0)return new WindowsCommandResult(true,Data:"cancelled");
                if(selected.Length>32 || selected.Any(p=>!ArchiveJobWire.PathValid(p)) || selected.Sum(p=>Encoding.UTF8.GetByteCount(p))>131072)return new WindowsCommandResult(false,"picker_limit");
                return new WindowsCommandResult(true,Data:JsonSerializer.Serialize(selected));
            },cancel),out WindowsCommandResult? result,out _,token);
        if(token.IsCancellationRequested || result?.Data=="cancelled" || result?.Error=="cancelled")return Result(cancelled:true);
        if(!ok || result?.Success!=true || result.Data is null)return Result(code:"picker_failed",native:result?.NativeErrorCode);
        return Result(JsonSerializer.Deserialize<string[]>(result.Data));
    }
    [DllImport("user32.dll")][return:MarshalAs(UnmanagedType.Bool)]private static extern bool IsWindow(IntPtr hwnd);
    [DllImport("user32.dll")]private static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint process);
}
