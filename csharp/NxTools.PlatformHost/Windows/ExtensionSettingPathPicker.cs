using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
namespace NxTools.PlatformHost.Windows;

internal static class ExtensionSettingPathPicker
{
    private sealed record Owner(IntPtr Handle) : IWin32Window;
    internal static WindowsCommandResult Pick(PickExtensionSettingPathCommand command, Guid correlation, IPlatformLog? log, CancellationToken cancellation)
    {
        var clock=Stopwatch.StartNew();
        ExtensionRuntimeDiagnostics.Write(log,"setting_path","accepted",correlation,clock);
        var completed=BoundedStaOperationRunner.TryRun("NxTools extension setting picker",Timeout.InfiniteTimeSpan, token=>
            StaDialogRunner.RunInline(()=> PickerOwner.Run(command.SourceWindowHandle, command.SourceProcessId, token, _=> {
                ExtensionRuntimeDiagnostics.Write(log,"setting_path","started",correlation,clock);
                return PickCore(command,token,ValidOwner,File.Exists,Directory.Exists,Show);
            }),token),out WindowsCommandResult? result,out _,cancellation);
        result=completed && result is not null ? result : new(false,cancellation.IsCancellationRequested?"setting_picker_cancelled":"setting_picker_busy",NativeErrorCode:cancellation.IsCancellationRequested?995:170);
        ExtensionRuntimeDiagnostics.Write(log,"setting_path",result.Success?"completed":result.NativeErrorCode==995?"cancelled":"failed",correlation,clock,result.Success?(string.IsNullOrEmpty(result.Data)?"dismissed":"selected"):"picker_failure",result.NativeErrorCode);
        return result;
    }
    internal static WindowsCommandResult PickCore(PickExtensionSettingPathCommand command,CancellationToken cancellation,
        Func<long,uint,bool> validOwner,Func<string,bool> fileExists,Func<string,bool> directoryExists,
        Func<PickExtensionSettingPathCommand,string,string?> show)
    {
        if(cancellation.IsCancellationRequested)return new(false,"setting_picker_cancelled",NativeErrorCode:995);
        if(command.Kind is not ("file" or "folder") || !validOwner(command.SourceWindowHandle,command.SourceProcessId))return new(false,"setting_picker_invalid_owner",NativeErrorCode:1400);
        try {
            var current=command.CurrentPath;
            var directory=directoryExists(current)?current:Path.GetDirectoryName(current)??"";
            var selected=show(command,directoryExists(directory)?directory:"");
            if(cancellation.IsCancellationRequested)return new(false,"setting_picker_cancelled",NativeErrorCode:995);
            if(!validOwner(command.SourceWindowHandle,command.SourceProcessId))return new(false,"setting_picker_invalid_owner",NativeErrorCode:1400);
            if(string.IsNullOrEmpty(selected))return new(true,Data:"");
            if(selected.Length>=32767 || selected.Contains('\0') || !Path.IsPathFullyQualified(selected) || !(command.Kind=="file"?fileExists(selected):directoryExists(selected)))return new(false,"setting_picker_invalid_selection",NativeErrorCode:2);
            return new(true,Data:Path.GetFullPath(selected));
        } catch(Win32Exception e){return new(false,"setting_picker_failed",NativeErrorCode:e.NativeErrorCode);}
        catch(Exception e) when(e is ArgumentException or IOException or UnauthorizedAccessException or InvalidOperationException or ExternalException){return new(false,"setting_picker_failed",NativeErrorCode:e.HResult);}
    }
    private static string? Show(PickExtensionSettingPathCommand command,string directory)
    {
        var owner=new Owner(new IntPtr(command.SourceWindowHandle));
        if(command.Kind=="folder") {
            using var dialog=new FolderBrowserDialog{Description="选择扩展设置文件夹",UseDescriptionForTitle=true,InitialDirectory=directory,ShowNewFolderButton=false};
            return dialog.ShowDialog(owner)==DialogResult.OK?dialog.SelectedPath:null;
        }
        using var file=new OpenFileDialog{Title="选择扩展设置文件",InitialDirectory=directory,CheckFileExists=true,CheckPathExists=true,Multiselect=false,RestoreDirectory=true,AutoUpgradeEnabled=true,Filter="所有文件 (*.*)|*.*"};
        return file.ShowDialog(owner)==DialogResult.OK?file.FileName:null;
    }
    private static bool ValidOwner(long window,uint process)=>window>0 && process>0 && IsWindow(new IntPtr(window)) && GetWindowThreadProcessId(new IntPtr(window),out var actual)!=0 && actual==process;
    [DllImport("user32.dll")][return:MarshalAs(UnmanagedType.Bool)]private static extern bool IsWindow(IntPtr window);
    [DllImport("user32.dll")]private static extern uint GetWindowThreadProcessId(IntPtr window,out uint process);
}
