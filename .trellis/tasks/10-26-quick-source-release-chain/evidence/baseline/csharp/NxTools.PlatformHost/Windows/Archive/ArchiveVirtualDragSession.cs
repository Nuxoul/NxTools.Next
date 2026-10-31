using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows.Archive;

internal delegate (int HResult,int Effect) ArchiveOleLoop(System.Runtime.InteropServices.ComTypes.IDataObject data,ArchiveVirtualDragSession.DropSource source);
internal sealed class ArchiveVirtualDragSession : IAsyncDisposable
{
    private readonly object gate=new();
    private readonly ArchiveEntrySession entries;
    private readonly IPlatformLog? log;
    private readonly Func<long,uint,bool> validSource;
    private readonly Func<long,IDisposable?> prepare;
    private readonly Func<bool> pressed;
    private readonly Func<bool> escape;
    private readonly ArchiveOleLoop loop;
    private Run? active;
    private bool stopped;
    private readonly HashSet<Guid> retired=[];
    private sealed class Run(ArchiveVirtualDragCommand command)
    {
        internal readonly ArchiveVirtualDragCommand Command=command;
        internal readonly CancellationTokenSource Stop=new(TimeSpan.FromSeconds(120));
        internal readonly TaskCompletionSource<WindowsCommandResult> Done=new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal readonly Stopwatch Clock=Stopwatch.StartNew();
        internal CancellationTokenRegistration AuthorizationStop;
    }
    internal ArchiveVirtualDragSession(ArchiveEntrySession entries,IPlatformLog? log=null,
        Func<long,uint,bool>? validSource=null,Func<long,IDisposable?>? prepare=null,Func<bool>? pressed=null,ArchiveOleLoop? loop=null,Func<bool>? escape=null)
    {
        this.entries=entries;this.log=log;this.validSource=validSource??ValidSource;this.prepare=prepare??Prepare;
        this.pressed=pressed??(()=> (GetAsyncKeyState(1)&0x8000)!=0);this.loop=loop??NativeLoop;
        this.escape=escape??(()=> (GetAsyncKeyState(0x1B)&0x8000)!=0);
    }
    internal Task<WindowsCommandResult> Dispatch(ProtocolEnvelope envelope,uint peer)
    {
        ArchiveVirtualDragCommand command;
        try{command=ArchiveVirtualDragWire.Parse(envelope);}catch{return Task.FromResult(new WindowsCommandResult(false,"invalid_virtual_drag"));}
        lock(gate) {
            if(command.Action=="cancel") {
                if(active is not {} prior||prior.Command.OperationId!=command.OperationId||prior.Command.SessionId!=command.SessionId||prior.Command.Generation!=command.Generation)return Task.FromResult(new WindowsCommandResult(false,"stale_virtual_drag"));
                prior.Stop.Cancel();return Task.FromResult(new WindowsCommandResult(true));
            }
            if(stopped||active is not null||retired.Contains(command.OperationId)||retired.Count>=4096)return Task.FromResult(new WindowsCommandResult(false,"virtual_drag_busy"));
            if(!validSource(command.SourceWindowHandle,peer))return Task.FromResult(new WindowsCommandResult(false,"invalid_source_window"));
            if(!entries.TryCaptureVirtualAuthorization(command.AuthorizationId,command.Items!,out var authorizationStop))return Task.FromResult(new WindowsCommandResult(false,"authorization_invalid"));
            var run=new Run(command);active=run;Diagnostic("accepted",run);
            run.AuthorizationStop=authorizationStop.Register(()=>{try{run.Stop.Cancel();}catch(ObjectDisposedException){}});
            var thread=new Thread(()=>Execute(run,peer)){IsBackground=true,Name="NxTools.ArchiveVirtualDragSpike"};thread.SetApartmentState(ApartmentState.STA);
            try{thread.Start();}catch{active=null;run.AuthorizationStop.Unregister();run.Stop.Dispose();run.Done.SetResult(new(false,"virtual_sta_start_failed"));}
            return run.Done.Task;
        }
    }
    private void Execute(Run run,uint peer)
    {
        WindowsCommandResult result;var initialized=false;var cleaned=false;
        ArchiveVirtualContent? content=null;
        var requested=new HashSet<int>();var ready=new HashSet<int>();
        try {
            content=new ArchiveVirtualContent(entries,run.Command.Items!,authorizationId:run.Command.AuthorizationId);
            var hr=OleInitialize(IntPtr.Zero);if(hr<0)throw new COMException("ole_initialize",hr);initialized=true;
            Diagnostic("started",run);
            using var input=prepare(run.Command.SourceWindowHandle);
            if(!validSource(run.Command.SourceWindowHandle,peer))throw new OperationCanceledException();
            if(!pressed()||run.Stop.IsCancellationRequested)throw new OperationCanceledException();
            using var monitor=new System.Threading.Timer(_=>{try{if(escape()||!validSource(run.Command.SourceWindowHandle,peer))run.Stop.Cancel();}catch(ObjectDisposedException){}},null,50,50);
            using var data=new ArchiveVirtualFileDataObject(run.Command.Items!.Select(item=>new ArchiveVirtualFile(item.Name,(long?)item.Size,item.Directory)).ToArray(),index=>{
                run.Stop.Token.ThrowIfCancellationRequested();if(requested.Add(index))Diagnostic("content_requested",run,index);
                try{var stream=content.Open(index,run.Stop.Token);if(ready.Add(index))Diagnostic("content_ready",run,index);return stream;}
                catch(OperationCanceledException){throw new COMException("virtual_cancelled",unchecked((int)0x800704C7));}
                catch(ArchiveOutputException failure){throw new COMException(failure.Stage,unchecked((int)0x8003001E));}
            });
            var source=new DropSource(run.Stop.Token,()=>validSource(run.Command.SourceWindowHandle,peer));
            var terminal=loop(data,source); // Exactly one native loop; returned media have independent native lifetime.
            result=terminal.HResult==0x00040101?new(true,Data:"{\"status\":\"cancelled\",\"drop_effect\":0}"):
                terminal.HResult==0x00040100&&terminal.Effect==1?new(true,Data:"{\"status\":\"dropped\",\"drop_effect\":1}"):
                terminal.HResult>=0?new(false,"virtual_target_not_accepted",Data:"{\"status\":\"not_dropped\",\"drop_effect\":0}"):
                new(false,"virtual_ole_failed",NativeErrorCode:terminal.HResult);
        }
        catch(OperationCanceledException){result=new(true,Data:"{\"status\":\"cancelled\",\"drop_effect\":0}");}
        catch(COMException failure){result=new(false,"virtual_ole_failed",NativeErrorCode:failure.HResult);}
        catch(Exception){result=new(false,"virtual_drag_failed");}
        finally {
            content?.Cancel();
            try{if(content is not null)content.DisposeAsync().AsTask().GetAwaiter().GetResult();cleaned=true;}catch{Diagnostic("cleanup_failed",run);}
            if(initialized)OleUninitialize();
        }
        if(!cleaned)result=new(false,"virtual_cleanup_incomplete",Data:"{\"cleanup_complete\":false}");
        lock(gate){if(cleaned&&ReferenceEquals(active,run))active=null;retired.Add(run.Command.OperationId);}
        Diagnostic("terminal",run,native:result.NativeErrorCode,success:result.Success,cleanup:cleaned);run.AuthorizationStop.Unregister();if(cleaned)run.Stop.Dispose();run.Done.TrySetResult(result);
    }
    internal void Stop(){lock(gate){stopped=true;active?.Stop.Cancel();}}
    public async ValueTask DisposeAsync(){Task? task;lock(gate){stopped=true;active?.Stop.Cancel();task=active?.Done.Task;}if(task is not null)await task;}
    private void Diagnostic(string stage,Run run,int? index=null,int? native=null,bool? success=null,bool? cleanup=null)
    {try{log?.Write("info","archive.virtual_drag."+stage,run.Command.OperationId,new Dictionary<string,object?>{["index"]=index,["count"]=run.Command.Items?.Length,["success"]=success,["cleanup_complete"]=cleanup,["native_error_code"]=native,["elapsed_ms"]=run.Clock.ElapsedMilliseconds});}catch{}}
    private static bool ValidSource(long hwnd,uint peer){var window=new IntPtr(hwnd);return hwnd>0&&IsWindow(window)&&GetWindowThreadProcessId(window,out var process)!=0&&process==peer;}
    private static IDisposable? Prepare(long hwnd)
    {
        var window=new IntPtr(hwnd);
        if(!NativeFileDragService.TryAttachSourceInputQueue(window,null,out var lease,out _) )throw new COMException("input_queue_failed");
        try{if(!NativeFileDragService.TryReleaseSourcePointerCapture(window,out _))throw new COMException("capture_release_failed");return lease;}
        catch{lease?.Dispose();throw;}
    }
    internal static (int,int) NativeLoop(System.Runtime.InteropServices.ComTypes.IDataObject data,DropSource source){
        var keys=(GetAsyncKeyState(1)&0x8000)!=0?1:0;
        var admission=source.QueryContinueDrag(false,keys);if(admission!=0)return(admission,0);
        var hr=DoDragDrop(data,source,1,out var effect);return(hr,effect);
    }
    [ComVisible(true),ClassInterface(ClassInterfaceType.None)]
    internal sealed class DropSource(CancellationToken stop,Func<bool> alive):IDropSource
    {
        public int QueryContinueDrag(bool escape,int keys)=>escape||stop.IsCancellationRequested||!alive()?0x00040101:(keys&1)==0?0x00040100:0;
        public int GiveFeedback(int effect)=>0x00040102;
    }
    [ComImport,Guid("00000121-0000-0000-C000-000000000046"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IDropSource{[PreserveSig]int QueryContinueDrag([MarshalAs(UnmanagedType.Bool)]bool escape,int keys);[PreserveSig]int GiveFeedback(int effect);}
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern int OleInitialize(IntPtr reserved);
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern void OleUninitialize();
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern int DoDragDrop(System.Runtime.InteropServices.ComTypes.IDataObject data,IDropSource source,int effects,out int effect);
    [DllImport("user32.dll",ExactSpelling=true)]private static extern bool IsWindow(IntPtr hwnd);
    [DllImport("user32.dll",ExactSpelling=true)]private static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint process);
    [DllImport("user32.dll",ExactSpelling=true)]private static extern short GetAsyncKeyState(int key);
}
