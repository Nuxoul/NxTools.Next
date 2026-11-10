using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
namespace NxTools.PlatformHost.Windows;

internal static class ArchiveJobPicker {
    private sealed record Owner(IntPtr Handle):IWin32Window;
    internal static ArchiveOwnedPickerResult Pick(ArchivePickerCommand c,uint peer,CancellationToken token,
        ArchiveJobFolderPicker? folders=null,IPlatformLog? log=null) {
        var started=Stopwatch.GetTimestamp();
        void Diagnostic(string stage,string? category=null,int count=0,int? native=null) {
            try {
                log?.Write("info","archive.picker."+stage,c.OperationId,new Dictionary<string,object?> {
                    ["purpose"]=c.Purpose,["generation"]=c.Generation,["category"]=category,
                    ["count"]=count,["native_error_code"]=native,
                    ["elapsed_ms"]=Stopwatch.GetElapsedTime(started).TotalMilliseconds
                });
            }catch { /* Diagnostics cannot change the picker result. */ }
        }
        ArchiveOwnedPickerResult Result(string[]? paths=null,bool cancelled=false,string? code=null,int? native=null) {
            Diagnostic("terminal",cancelled?"cancelled":code??"selected",paths?.Length??0,native);
            return new(1,c.OperationId,c.SessionId,c.Generation,paths??[],cancelled,code,native);
        }
        Diagnostic("accepted");
        if(token.IsCancellationRequested)return Result(cancelled:true);
        var window=new IntPtr(c.OwnerWindowHandle);
        bool OwnerAlive()=>IsWindow(window) && GetWindowThreadProcessId(window,out var process)!=0 && process==peer;
        if(!OwnerAlive())return Result(code:"invalid_owner",native:1400);
        var ok=BoundedStaOperationRunner.TryRun("NxTools archive owned picker",Timeout.InfiniteTimeSpan,
            cancel=>StaDialogRunner.RunInline(()=> {
                if(cancel.IsCancellationRequested)return new WindowsCommandResult(true,Data:"cancelled");
                if(!OwnerAlive())return new WindowsCommandResult(false,"invalid_owner",NativeErrorCode:1400);
                using var suspended = PickerOwner.SuspendTopmost(peer);
                if(!OwnerAlive())return new WindowsCommandResult(false,"invalid_owner",NativeErrorCode:1400);
                Diagnostic("start");
                string[] selected;
                var owner=new Owner(window);
                var initialDirectory=c.CurrentDirectory is { } candidate && Directory.Exists(candidate)?candidate:"";
                if(c.Purpose is "folders" or "destination") {
                    try { selected=(folders??new ArchiveJobFolderPicker()).Show(window,initialDirectory,c.Purpose=="folders"); }
                    catch(ArchivePickerLimitException) { return new WindowsCommandResult(false,"picker_limit"); }
                }else if(c.Purpose=="save") {
                    using var dialog=new SaveFileDialog{Title="保存压缩包",InitialDirectory=initialDirectory,FileName=c.SuggestedName??"",RestoreDirectory=true,CheckPathExists=true,OverwritePrompt=true,AutoUpgradeEnabled=true};
                    selected=dialog.ShowDialog(owner)==DialogResult.OK?[dialog.FileName]:[];
                }else {
                    using var dialog=new OpenFileDialog{Title="选择源文件",InitialDirectory=initialDirectory,Multiselect=true,CheckFileExists=true,CheckPathExists=true,RestoreDirectory=true,AutoUpgradeEnabled=true};
                    selected=dialog.ShowDialog(owner)==DialogResult.OK?dialog.FileNames:[];
                }
                if(cancel.IsCancellationRequested || selected.Length==0)return new WindowsCommandResult(true,Data:"cancelled");
                if(!OwnerAlive())return new WindowsCommandResult(false,"invalid_owner",NativeErrorCode:1400);
                if(!ArchiveOwnedPickerResult.SelectionValid(selected))return new WindowsCommandResult(false,"picker_limit");
                return new WindowsCommandResult(true,Data:JsonSerializer.Serialize(selected));
            },cancel),out WindowsCommandResult? result,out _,token);
        if(token.IsCancellationRequested || result?.Data=="cancelled" || result?.NativeErrorCode==995)return Result(cancelled:true);
        if(!ok || result?.Success!=true || result.Data is null) {
            var code=result?.Error is "picker_limit" or "invalid_owner"?result.Error:"picker_failed";
            return Result(code:code,native:result?.NativeErrorCode);
        }
        if(!OwnerAlive())return Result(code:"invalid_owner",native:1400);
        return Result(JsonSerializer.Deserialize<string[]>(result.Data));
    }
    [DllImport("user32.dll")][return:MarshalAs(UnmanagedType.Bool)]private static extern bool IsWindow(IntPtr hwnd);
    [DllImport("user32.dll")]private static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint process);
}
