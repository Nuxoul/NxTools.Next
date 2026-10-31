using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
namespace NxTools.PlatformHost.Tests;
// Injected native presentation/owner/existence only. No real dialog/user files.
internal static class ExtensionSettingPathTests
{
    internal static Task WireOwnerCancellationAndSelection()
    {
        var command=new PickExtensionSettingPathCommand(123,42,"file",@"C:\Synthetic\old.txt");
        var wire=ProtocolEnvelope.Create("platform_command",new {Command="pick_extension_setting_path",Data=command});
        Assert(PlatformCommandParser.ParsePickExtensionSettingPath(wire)==command,"typed wire");
        foreach(var data in new object[] {new {SourceWindowHandle="123",SourceProcessId=42,Kind="file",CurrentPath=""},new {SourceWindowHandle=123,SourceProcessId=42,Kind="bad",CurrentPath=""},new {SourceWindowHandle=123,SourceProcessId=42,Kind="file",CurrentPath="",Extra=true}}) {
            try {PlatformCommandParser.ParsePickExtensionSettingPath(ProtocolEnvelope.Create("platform_command",new {Command="pick_extension_setting_path",Data=data}));throw new Exception("bad wire admitted");}catch(InvalidDataException){}
        }
        var calls=0;
        var selected=ExtensionSettingPathPicker.PickCore(command,CancellationToken.None,(window,process)=>window==123&&process==42,_=>true,_=>true,(_,_)=>{calls++;return @"C:\Synthetic\new.txt";});
        Assert(selected.Success&&selected.Data==@"C:\Synthetic\new.txt"&&calls==1,"one selected existing file");
        var invalid=ExtensionSettingPathPicker.PickCore(command,CancellationToken.None,(_,_)=>false,_=>throw new Exception("no probe"),_=>throw new Exception("no probe"),(_,_)=>throw new Exception("no dialog"));
        Assert(!invalid.Success&&invalid.NativeErrorCode==1400,"wrong owner rejected before work");
        using var cancel=new CancellationTokenSource();
        var late=ExtensionSettingPathPicker.PickCore(command,cancel.Token,(_,_)=>true,_=>true,_=>true,(_,_)=>{cancel.Cancel();return @"C:\Synthetic\late.txt";});
        Assert(!late.Success&&late.NativeErrorCode==995&&late.Data is null,"late cancel rejects selection");
        var folder=ExtensionSettingPathPicker.PickCore(command with {Kind="folder"},CancellationToken.None,(_,_)=>true,_=>false,_=>true,(_,_)=>@"C:\Synthetic");
        Assert(folder.Success&&folder.Data==@"C:\Synthetic","folder selection");
        var dismissed=ExtensionSettingPathPicker.PickCore(command,CancellationToken.None,(_,_)=>true,_=>true,_=>true,(_,_)=>null);
        Assert(dismissed.Success&&dismissed.Data=="","dismissal is quiet");
        return Task.CompletedTask;
    }
    private static void Assert(bool condition,string message){if(!condition)throw new InvalidOperationException(message);}
}
