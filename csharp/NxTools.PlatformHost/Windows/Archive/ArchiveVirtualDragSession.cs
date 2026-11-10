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
    // Destination location owner; tests supply an owned temp root and watch roots.
    internal Func<Guid,string,ArchiveDropLocator> CreateLocator { get; init; }=(operation,wrapper)=>ArchiveDropLocator.Create(operation,wrapper);
    internal Func<IReadOnlyList<string>> LocateProbeFolders { get; init; }=()=>ArchiveDropLocator.ProbeFolders(ArchiveDropLocator.ExplorerFolders());
    // A receiver's own merge/overwrite confirmation can hold the copy; an unaccepted drop gets a short window.
    internal TimeSpan LocateWindow { get; init; }=TimeSpan.FromSeconds(60);
    internal TimeSpan UnconfirmedLocateWindow { get; init; }=TimeSpan.FromSeconds(3);
    internal Action<ProtocolEnvelope>? Progress { get; set; }
    private Run? active;
    private bool stopped;
    private readonly HashSet<Guid> retired=[];
    private sealed class Run(ArchiveVirtualDragCommand command)
    {
        internal readonly ArchiveVirtualDragCommand Command=command;
        internal readonly ArchiveVirtualPlan Plan=command.Plan??throw new InvalidDataException();
        internal readonly ulong FilesTotal=(ulong)command.Items!.Count(item=>!item.Directory);
        internal readonly ulong? BytesTotal=KnownTotal(command.Items!);
        private static ulong? KnownTotal(ArchiveVirtualDragItem[] items)
        {
            ulong total=0;
            foreach(var file in items.Where(item=>!item.Directory)) {
                if(file.Size is not {} size||ulong.MaxValue-total<size)return null;
                total+=size;
            }
            return total;
        }
        internal readonly CancellationTokenSource Stop=new(TimeSpan.FromSeconds(120));
        internal readonly TaskCompletionSource<WindowsCommandResult> Done=new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal readonly Stopwatch Clock=Stopwatch.StartNew();
        internal ArchiveSourceActivity.Reservation? SourceActivity;
        internal CancellationTokenRegistration AuthorizationStop;
        internal readonly object ProgressGate=new();
        internal ulong BytesDone,FilesDone,NextSequence;
        internal int? CurrentIndex;
        internal Guid? CurrentRequestId;
        internal ulong? CurrentBytesDone,CurrentBytesTotal;
        internal bool CurrentSealed;
        internal bool Demanded;
        internal string Phase="";
        internal ulong LastBytes,LastFiles;
        internal int? LastIndex;
        internal Guid? LastRequestId;
        internal ulong? LastCurrentBytes,LastCurrentTotal;
        internal bool LastSealed;
        internal long LastPublished=-100;
        internal ProtocolEnvelope? Latest;
        internal System.Threading.Timer? ProgressTimer;
        internal bool ProgressEnded;
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
            if(command.Action=="locate") {
                // No archive read happens during the gesture; the ordinary extraction job owns the source.
                var located=new Run(command);active=located;Diagnostic("accepted",located);
                var worker=new Thread(()=>Locate(located,peer)){IsBackground=true,Name="NxTools.ArchiveDropLocate"};worker.SetApartmentState(ApartmentState.STA);
                try{worker.Start();}catch{active=null;located.Stop.Dispose();located.Done.SetResult(new(false,"virtual_sta_start_failed"));}
                return located.Done.Task;
            }
            if(!entries.TryCaptureVirtualAuthorization(command.AuthorizationId,command.Items!,out var authorizationStop))return Task.FromResult(new WindowsCommandResult(false,"authorization_invalid"));
            ArchiveSourceActivity.Reservation sourceActivity;
            try { sourceActivity = ArchiveSourceLease.ReserveActivity(command.Items![0].Entry.Path!, false, command.OperationId); }
            catch (ArchiveOutputException failure) { return Task.FromResult(new WindowsCommandResult(false, failure.Stage, NativeErrorCode: failure.NativeCode)); }
            var run=new Run(command) { SourceActivity=sourceActivity };active=run;Diagnostic("accepted",run);
            run.AuthorizationStop=authorizationStop.Register(()=>{try{run.Stop.Cancel();}catch(ObjectDisposedException){}});
            var thread=new Thread(()=>Execute(run,peer)){IsBackground=true,Name="NxTools.ArchiveVirtualDragSpike"};thread.SetApartmentState(ApartmentState.STA);
            try{thread.Start();}catch{active=null;run.SourceActivity?.Dispose();run.AuthorizationStop.Unregister();run.Stop.Dispose();run.Done.SetResult(new(false,"virtual_sta_start_failed"));}
            return run.Done.Task;
        }
    }
    private void Execute(Run run,uint peer)
    {
        WindowsCommandResult result=new(false,"virtual_drag_failed");var initialized=false;var cleaned=false;
        ArchiveVirtualContent? content=null;ArchiveVirtualFileDataObject? data=null;IntPtr reference=IntPtr.Zero;
        var requested=new HashSet<int>();var ready=new HashSet<int>();
        var outcome="no_target_completion";var phase="failed";int? targetError=null;
        CancellationTokenRegistration contentStop=default;
        try {
            content=new ArchiveVirtualContent(entries,run.Plan,authorizationId:run.Command.AuthorizationId,operation:run.Command.OperationId,
                progress:(index,sealedData,bytes)=>{
                    lock(run.ProgressGate) {
                        var request=run.Command.Items![index].Entry.OperationId;
                        if(!sealedData) {
                            run.Demanded=true;run.CurrentIndex=index;run.CurrentRequestId=request;
                            run.CurrentBytesDone=0;run.CurrentBytesTotal=run.Command.Items[index].Size;run.CurrentSealed=false;
                        } else if(run.CurrentIndex==index&&run.CurrentRequestId==request&&!run.CurrentSealed) {
                            run.BytesDone+=bytes;run.FilesDone++;run.CurrentBytesDone=bytes;run.CurrentBytesTotal=bytes;run.CurrentSealed=true;
                        } else return;
                        Publish(run,sealedData?"content_ready":"materializing");
                    }
                },
                memberProgress:snapshot=>{
                    lock(run.ProgressGate) {
                        if(run.ProgressEnded||run.CurrentSealed||run.CurrentIndex!=snapshot.Index||
                           run.CurrentRequestId!=snapshot.RequestId||run.Command.Items![snapshot.Index].Entry.Ordinal!=snapshot.Ordinal||
                           snapshot.BytesDone<run.CurrentBytesDone||snapshot.BytesTotal is {} total&&snapshot.BytesDone>total)return;
                        run.CurrentBytesDone=snapshot.BytesDone;run.CurrentBytesTotal=snapshot.BytesTotal;
                        Publish(run,"materializing");
                    }
                });
            contentStop=run.Stop.Token.Register(content.Cancel);
            var hr=OleInitialize(IntPtr.Zero);if(hr<0)throw new COMException("ole_initialize",hr);initialized=true;
            Diagnostic("started",run);
            using var monitor=new System.Threading.Timer(_=>{try{if(escape()||!validSource(run.Command.SourceWindowHandle,peer))run.Stop.Cancel();}catch(ObjectDisposedException){}},null,50,50);
            data=new ArchiveVirtualFileDataObject(run.Plan,index=>{
                run.Stop.Token.ThrowIfCancellationRequested();lock(requested){if(requested.Add(index))Diagnostic("content_requested",run,index);}
                try{var stream=content.Open(index,run.Stop.Token);lock(ready){if(ready.Add(index)){Diagnostic("content_ready",run,index);lock(run.ProgressGate)if(run.CurrentIndex==index&&run.CurrentSealed)Publish(run,"target_copying");}}return stream;}
                catch(OperationCanceledException){throw new COMException("virtual_cancelled",unchecked((int)0x800704C7));}
                catch(ArchiveOutputException failure){throw new COMException(failure.Stage,unchecked((int)0x8003001E));}
            },run.Stop.Token,stage=>Diagnostic(stage,run));
            // An explicit source reference is held across loop return, EndOperation and cleanup.
            reference=Marshal.GetComInterfaceForObject(data,typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
            data.SetAsyncMode(true);
            (int HResult,int Effect) terminal;
            try {
                using var input=prepare(run.Command.SourceWindowHandle);
                if(!validSource(run.Command.SourceWindowHandle,peer)||!pressed()||run.Stop.IsCancellationRequested)throw new OperationCanceledException();
                var source=new DropSource(run.Stop.Token,()=>validSource(run.Command.SourceWindowHandle,peer));
                terminal=loop(data,source); // Exactly one native loop. Never restart after cancellation.
            }finally{data.LoopEnded();Diagnostic("ole_return",run);} // Input queues detach before receiver work.
            if(data.AsyncStarted) {
                Diagnostic("receiver_pending",run);
                // Cancellation rejects new requests/cancels original worker, but is NOT EndOperation.
                // An uncooperative receiver retains this one slot/STA; never spawn a replacement.
                PumpUntil(data.AsyncCompletion);
                var target=data.AsyncCompletion.GetAwaiter().GetResult();targetError=target.Result<0?target.Result:null;
                outcome=target.Result>=0&&target.Effects==1?"receiver_reported_complete":"receiver_aborted";
                result=target.Result>=0&&target.Effects==1&&terminal.HResult==0x00040100&&terminal.Effect==1&&!run.Stop.IsCancellationRequested
                    ?new(true,Data:"{\"status\":\"dropped\",\"drop_effect\":1}")
                    :new(false,run.Stop.IsCancellationRequested?"virtual_cancelled":"virtual_receiver_failed",NativeErrorCode:targetError);
            } else {
                outcome="ole_only";
                result=terminal.HResult==0x00040101?new(true,Data:"{\"status\":\"cancelled\",\"drop_effect\":0}"):
                    terminal.HResult==0x00040100&&terminal.Effect==1?new(true,Data:"{\"status\":\"dropped\",\"drop_effect\":1}"):
                    terminal.HResult>=0?new(false,"virtual_target_not_accepted",Data:"{\"status\":\"not_dropped\",\"drop_effect\":0}"):
                    new(false,"virtual_ole_failed",NativeErrorCode:terminal.HResult);
            }
            phase=run.Stop.IsCancellationRequested||terminal.HResult==0x00040101?"cancelled":result.Success?"finished":"failed";
        }
        catch(OperationCanceledException){phase="cancelled";result=new(true,Data:"{\"status\":\"cancelled\",\"drop_effect\":0}");}
        catch(COMException failure){phase=run.Stop.IsCancellationRequested?"cancelled":"failed";result=new(false,phase=="cancelled"?"virtual_cancelled":"virtual_ole_failed",NativeErrorCode:failure.HResult);}
        catch(Exception){result=new(false,"virtual_drag_failed");}
        finally {
            // Even an exceptional loop must not strand a receiver's valid StartOperation lease.
            if(data is not null){data.LoopEnded();if(data.AsyncStarted&&!data.AsyncCompletion.IsCompleted){run.Stop.Cancel();PumpUntil(data.AsyncCompletion);}data.Dispose();}
            if(run.Stop.IsCancellationRequested||!result.Success)content?.Cancel();
            try{if(content is not null){var cleanup=content.DisposeAsync().AsTask();if(initialized)PumpUntil(cleanup);cleanup.GetAwaiter().GetResult();}cleaned=true;}catch{Diagnostic("cleanup_failed",run);}
            contentStop.Unregister();
            if(reference!=IntPtr.Zero)Marshal.Release(reference);
            if(initialized)OleUninitialize();
        }
        if(!cleaned){phase="failed";result=new(false,"virtual_cleanup_incomplete",Data:"{\"cleanup_complete\":false}");}
        if(cleaned&&phase=="finished"&&content?.TerminalFailure is {} materializationFailure){phase="failed";result=new(false,materializationFailure);}
        if (cleaned) run.SourceActivity?.Dispose();
        lock(gate){if(cleaned&&ReferenceEquals(active,run))active=null;retired.Add(run.Command.OperationId);}
        Publish(run,phase,cleaned,phase=="finished"?outcome:result.Error??outcome,result.NativeErrorCode??targetError);
        Diagnostic("terminal",run,native:result.NativeErrorCode,success:result.Success,cleanup:cleaned);run.AuthorizationStop.Unregister();if(cleaned)run.Stop.Dispose();run.Done.TrySetResult(result);
    }
    // Destination drag: the receiver copies one real wrapper folder holding a unique marker; the folder
    // where that marker lands is reported to Desktop, which runs an ordinary extraction into it.
    // Same held-gesture admission, capture release and single OLE loop as the virtual drag.
    private void Locate(Run run,uint peer)
    {
        WindowsCommandResult result=new(false,"drop_locate_failed");var initialized=false;ArchiveDropLocator? locator=null;
        try {
            locator=CreateLocator(run.Command.OperationId,run.Command.Items![0].Name);
            var hr=OleInitialize(IntPtr.Zero);if(hr<0)throw new COMException("ole_initialize",hr);initialized=true;
            Diagnostic("locate_started",run);
            using var monitor=new System.Threading.Timer(_=>{try{if(escape()||!validSource(run.Command.SourceWindowHandle,peer))run.Stop.Cancel();}catch(ObjectDisposedException){}},null,50,50);
            var data=NativeFileDragService.CreateFileDropDataObject([locator.Folder]);
            (int HResult,int Effect) terminal;
            try {
                using var input=prepare(run.Command.SourceWindowHandle);
                if(!validSource(run.Command.SourceWindowHandle,peer)||!pressed()||run.Stop.IsCancellationRequested)throw new OperationCanceledException();
                terminal=loop(data,new DropSource(run.Stop.Token,()=>validSource(run.Command.SourceWindowHandle,peer)));
            }finally{Diagnostic("ole_return",run);}
            if(terminal.HResult==0x00040101)throw new OperationCanceledException();
            if(terminal.HResult!=0x00040100)
                result=terminal.HResult>=0?new(false,"virtual_target_not_accepted",Data:"{\"status\":\"not_dropped\",\"drop_effect\":0}"):new(false,"virtual_ole_failed",NativeErrorCode:terminal.HResult);
            else {
                var window=terminal.Effect==0?UnconfirmedLocateWindow:LocateWindow;
                var probes=LocateProbeFolders;
                var wait=Task.Run(()=>locator.Wait(window,run.Stop.Token,probes));
                PumpUntil(wait);
                var destination=wait.GetAwaiter().GetResult();
                if(destination is null)
                    result=run.Stop.IsCancellationRequested?new(true,Data:"{\"status\":\"cancelled\",\"drop_effect\":0}"):new(false,"drop_location_unknown");
                else if(!ArchiveDropLocator.RemoveMarker(destination,locator.MarkerName,run.Stop.Token))
                    result=new(false,"drop_marker_retained");
                else {
                    Diagnostic("located",run);
                    result=new(true,Data:System.Text.Json.JsonSerializer.Serialize(new{status="located",drop_effect=terminal.Effect,destination}));
                }
            }
        }
        catch(OperationCanceledException){result=new(true,Data:"{\"status\":\"cancelled\",\"drop_effect\":0}");}
        catch(COMException failure){result=new(false,"virtual_ole_failed",NativeErrorCode:failure.HResult);}
        catch(Exception){result=new(false,"drop_locate_failed");}
        finally{locator?.Dispose();if(initialized)OleUninitialize();}
        lock(gate){if(ReferenceEquals(active,run))active=null;retired.Add(run.Command.OperationId);}
        Diagnostic("terminal",run,native:result.NativeErrorCode,success:result.Success,cleanup:true);
        run.Stop.Dispose();run.Done.TrySetResult(result);
    }
    // Pump standard COM calls and window messages on the ORIGINAL OLE apartment after Drop.
    // This is not another OLE loop; timeout only lets us observe task completion, never retires it.
    private static void PumpUntil(Task task)
    {
        using var completed=new ManualResetEvent(false);
        var signal=task.ContinueWith(_=>completed.Set(),CancellationToken.None,TaskContinuationOptions.ExecuteSynchronously,TaskScheduler.Default);
        var handles=new[]{completed.SafeWaitHandle.DangerousGetHandle()};
        while(!signal.IsCompleted){var hr=CoWaitForMultipleHandles(0x18,100,1,handles,out _);if(hr<0&&hr!=unchecked((int)0x80010115))Thread.Sleep(10);}
        signal.GetAwaiter().GetResult();
    }
    private void Publish(Run run,string phase,bool? cleanup=null,string? code=null,int? native=null)
    {
        lock(run.ProgressGate){
            if(run.ProgressEnded||!run.Demanded&&cleanup is null)return; // No progress before actual demand or after terminal.
            if(cleanup is null&&run.Phase==phase&&run.LastBytes==run.BytesDone&&run.LastFiles==run.FilesDone&&
                run.LastIndex==run.CurrentIndex&&run.LastRequestId==run.CurrentRequestId&&
                run.LastCurrentBytes==run.CurrentBytesDone&&run.LastCurrentTotal==run.CurrentBytesTotal&&run.LastSealed==run.CurrentSealed)return;
            run.LastBytes=run.BytesDone;run.LastFiles=run.FilesDone;run.LastIndex=run.CurrentIndex;
            run.LastRequestId=run.CurrentRequestId;run.LastCurrentBytes=run.CurrentBytesDone;
            run.LastCurrentTotal=run.CurrentBytesTotal;run.LastSealed=run.CurrentSealed;
            run.Phase=phase;
            var member=run.CurrentIndex is {} index?run.Command.Items![index].Name:null;
            var processed=run.BytesDone+(run.CurrentSealed?0:run.CurrentBytesDone??0);
            var envelope=new ArchiveVirtualDragProgress(1,run.Command.OperationId,run.Command.SessionId,run.Command.Generation,
                phase,run.BytesDone,run.BytesTotal,run.FilesDone,run.FilesTotal,run.CurrentIndex,member,cleanup,code,native,
                run.CurrentRequestId,run.CurrentBytesDone,run.CurrentBytesTotal,run.Demanded?processed:null,++run.NextSequence).Envelope();
            if(cleanup is not null){
                run.ProgressEnded=true;run.ProgressTimer?.Dispose();run.ProgressTimer=null;
                if(run.Latest is {} latest){run.Latest=null;Emit(latest);}Emit(envelope);return;
            }
            var elapsed=run.Clock.ElapsedMilliseconds-run.LastPublished;
            if(elapsed>=100){run.Latest=null;run.LastPublished=run.Clock.ElapsedMilliseconds;Emit(envelope);return;}
            run.Latest=envelope;
            run.ProgressTimer??=new System.Threading.Timer(_=>Flush(run),null,Timeout.Infinite,Timeout.Infinite);
            run.ProgressTimer.Change((int)(100-elapsed),Timeout.Infinite);
        }
    }
    private void Flush(Run run)
    {
        lock(run.ProgressGate){if(run.ProgressEnded||run.Latest is not {} latest)return;run.Latest=null;run.LastPublished=run.Clock.ElapsedMilliseconds;Emit(latest);}
    }
    private void Emit(ProtocolEnvelope envelope){try{Progress?.Invoke(envelope);}catch{}}
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
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern int CoWaitForMultipleHandles(uint flags,uint timeout,uint count,IntPtr[] handles,out uint index);
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern int OleInitialize(IntPtr reserved);
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern void OleUninitialize();
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern int DoDragDrop(System.Runtime.InteropServices.ComTypes.IDataObject data,IDropSource source,int effects,out int effect);
    [DllImport("user32.dll",ExactSpelling=true)]private static extern bool IsWindow(IntPtr hwnd);
    [DllImport("user32.dll",ExactSpelling=true)]private static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint process);
    [DllImport("user32.dll",ExactSpelling=true)]private static extern short GetAsyncKeyState(int key);
}
