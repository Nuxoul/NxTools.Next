using System.Collections.Concurrent;
using System.IO.Compression;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveVirtualDragTests;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveVirtualAsyncLifecycleTests
{
    private static readonly Guid AsyncId=new("3D8B0590-F691-11d2-8EA9-006097DF5BD4");
    private static ProtocolEnvelope Wire(ArchiveVirtualDragCommand command)=>ProtocolEnvelope.Create("platform_command",new{command="archive_virtual_drag",data=command},command.OperationId);
    private static T Slot<T>(IntPtr pointer,int slot) where T:Delegate=>Marshal.GetDelegateForFunctionPointer<T>(Marshal.ReadIntPtr(Marshal.ReadIntPtr(pointer),slot*IntPtr.Size));
    private static IntPtr Capability(System.Runtime.InteropServices.ComTypes.IDataObject data)
    {
        var pointer=Marshal.GetComInterfaceForObject(data,typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
        try{var iid=AsyncId;Check(Marshal.QueryInterface(pointer,ref iid,out var result)==0,"real native QI for unchanged IAsyncOperation IID");return result;}finally{Marshal.Release(pointer);}
    }
    internal static Task NativeAsyncAbiTransitionsAndReferences()
    {
        using var stop=new CancellationTokenSource();var opened=0;
        using var data=new ArchiveVirtualFileDataObject([new("one.txt",3)],_=>{opened++;return Memory("one"u8.ToArray());},stop.Token);
        var pointer=Capability(data);
        try {
            Check(Marshal.AddRef(pointer)>1&&Marshal.Release(pointer)>0,"balanced target AddRef/Release while source owns object");
            var mode=Slot<GetBool>(pointer,4);var active=Slot<GetBool>(pointer,6);var set=Slot<SetBool>(pointer,3);var start=Slot<Start>(pointer,5);var end=Slot<End>(pointer,7);
            Check(mode(pointer,out var enabled)==0&&enabled==0,"SDK default FALSE32");
            Check(active(pointer,out var running)==0&&running==0,"not started");
            Check(start(pointer,IntPtr.Zero)<0&&end(pointer,0,IntPtr.Zero,1)<0,"no phantom operation before negotiation/start");
            Check(set(pointer,1)==0&&mode(pointer,out enabled)==0&&enabled==1,"Set/Get slot3/4 BOOL32");
            Check(start(pointer,new IntPtr(1))<0&&start(pointer,IntPtr.Zero)==0&&start(pointer,IntPtr.Zero)<0,"reservedNULL and one start");
            Check(active(pointer,out running)==0&&running==1&&set(pointer,0)<0,"InOperation slot6 and immutable negotiated operation");
            var format=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);data.GetData(ref format,out var held);
            try {
                Check(end(pointer,0,new IntPtr(1),1)<0&&active(pointer,out running)==0&&running==1,"invalid reserved cannot finish");
                Check(end(pointer,unchecked((int)0x80004004),IntPtr.Zero,0)==0&&end(pointer,0,IntPtr.Zero,1)<0,"End HRESULT/effect once, receiver abort");
                Check(active(pointer,out running)==0&&running==0&&data.AsyncCompletion.Result.Result==unchecked((int)0x80004004),"native failure preserved");
                Check(data.QueryGetData(ref format)<0&&Read(ref held,3)=="one","End rejects new requests, never closes transferred stream");
            }finally{ReleaseStgMedium(ref held);}
            Check(opened==1,"no late re-decode");
        }finally{Marshal.Release(pointer);}
        using var cancelled=new ArchiveVirtualFileDataObject([new("one.txt",3)],_=>Memory("one"u8.ToArray()),stop.Token);
        cancelled.SetAsyncMode(true);stop.Cancel();Check(cancelled.StartOperation(IntPtr.Zero)<0,"cancellation forbids new async negotiation");
        using var late=new ArchiveVirtualFileDataObject([new("one.txt",3)],_=>Memory("one"u8.ToArray()));late.SetAsyncMode(true);late.LoopEnded();Check(late.StartOperation(IntPtr.Zero)<0,"Start must precede native loop return");
        return Task.CompletedTask;
    }
    private static string Root()
    {
        var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if(string.IsNullOrEmpty(worker)||!File.Exists(worker))throw new TestSkippedException("Explicit matching worker required");
        var root=Directory.CreateDirectory(Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!,"archive-product-async-lifecycle-"+Guid.NewGuid())).FullName;
        File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));return root;
    }
    private static async Task<ArchiveBrowserContext> Browse(string root,string path)
    {
        await using var read=ArchiveReadSession.ForOwnedTestPackage(root);var request=ArchiveProductReadTests.Command(path:path,version:2,limits:new(){BrowserContext=true});var id=ArchiveReadWire.Parse(request);Check(read.Dispatch(request).Success,"browse");
        while(true){var e=(await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(30))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if(e.Batch is {} b)Check(read.Dispatch(ArchiveProductReadTests.Command("ack",id.OperationId,id.SessionId,id.Generation,sequence:b.Sequence,version:2)).Success,"credit");
            if(e.Terminal is {} t){Check(t.Status=="succeeded"&&t.CleanupComplete&&t.Browser is not null,"original cleanup/source bound");return t.Browser!;}}
    }
    private static ArchiveVirtualDragCommand Command(ArchiveBrowserContext browser,string path,bool tree=false)
    {
        var session=Guid.NewGuid();ArchiveEntryCommand Entry(ulong? ordinal)=>new(){Version=1,OperationId=Guid.NewGuid(),SessionId=session,Generation=1,Action="materialize",Path=path,Ordinal=ordinal,Browser=browser,CopyMotw=false};
        return new(1,Guid.NewGuid(),session,1,"start",42,tree?[new("root",null,Entry(null),true),new("root/one.txt",3,Entry(1)),new("root/two.txt",null,Entry(2))]:[new("one.txt",3,Entry(0))]);
    }
    private static void Zip(string path,bool tree=false)
    {
        using var zip=ZipFile.Open(path,ZipArchiveMode.Create);if(tree)zip.CreateEntry("root/");
        using(var writer=new StreamWriter(zip.CreateEntry(tree?"root/one.txt":"one.txt").Open()))writer.Write("one");
        if(tree){using var writer=new StreamWriter(zip.CreateEntry("root/two.txt").Open());writer.Write("two");}
    }
    internal static async Task ActualReceiverLeaseCancelAbortDisconnect()
    {
        var root=Root();try {
            var path=Path.Combine(root,"source.zip");Zip(path);var browser=await Browse(root,path);
            foreach(var mode in new[]{"cancel","receiver_abort","source_closed","disconnect"}) {
                await using var entries=ArchiveEntrySession.ForOwnedTestPackage(root);
                var command=Command(browser,path);var arrived=new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);var alive=1;var capability=IntPtr.Zero;
                System.Runtime.InteropServices.ComTypes.IDataObject? original=null;STGMEDIUM held=default;
                var events=new ConcurrentQueue<ArchiveVirtualDragProgress>();var detached=new TrackedInput();
                await using var drag=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>Volatile.Read(ref alive)!=0,prepare:_=>detached,pressed:()=>true,escape:()=>false,loop:(data,_)=>{
                    original=data;capability=Capability(data);Check(Slot<Start>(capability,5)(capability,IntPtr.Zero)==0,"start receiver lease");
                    var f=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);data.GetData(ref f,out held);arrived.TrySetResult();return(0x00040100,1);
                });
                drag.Progress=e=>events.Enqueue(e.Payload.GetProperty("data").Deserialize<ArchiveVirtualDragProgress>(JsonDefaults.Options)!);
                var done=drag.Dispatch(Wire(command),1);
                try {
                    await arrived.Task.WaitAsync(TimeSpan.FromSeconds(20));await Task.Delay(150);
                    Check(detached.Disposed&&!done.IsCompleted&&Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Length==1,"OLE return detaches input but retains original content lease until End");
                    var dispatcher=new PlatformCommandSequenceDispatcher();await dispatcher.DispatchAsync("archive_entry",()=>done);var responsive=false;
                    await dispatcher.DispatchAsync("later-command",()=>{responsive=true;return Task.CompletedTask;});Check(responsive&&!done.IsCompleted,"actual later serial dispatch while receiver lease active");
                    Check(!(await drag.Dispatch(Wire(command with{OperationId=Guid.NewGuid()}),1)).Success,"one slot, no replacement after loop returns");
                    if(mode=="cancel")Check((await drag.Dispatch(Wire(command with{Action="cancel",Items=null,SourceWindowHandle=0}),1)).Success,"exact cancel");
                    else if(mode=="source_closed")Volatile.Write(ref alive,0);
                    else if(mode=="disconnect"){drag.Stop();entries.Stop();}
                    await Task.Delay(150);
                    Check(!done.IsCompleted&&Read(ref held,1)=="o","cancel/disconnect cannot fake End or break returned handle");
                    var request=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);
                    if(mode!="receiver_abort")Check(original!.QueryGetData(ref request)<0,"new requests rejected after cancellation");
                    Check(Slot<End>(capability,7)(capability,unchecked((int)0x80004004),IntPtr.Zero,0)==0,"receiver abort End ends held ownership");
                    var terminal=await done.WaitAsync(TimeSpan.FromSeconds(20));Check(!terminal.Success,"receiver failure cannot be fake-success");
                    Check(Read(ref held,2)=="ne"&&original!.QueryGetData(ref request)<0,"native stream after End/retirement; late gets reject");
                    Check(entries.ActiveWorkerCountForFixture==0,"actual worker slot remains occupied after receiver terminal: "+mode);
                    using(var reopened=File.Open(path,FileMode.Open,FileAccess.ReadWrite,FileShare.None))Check(reopened.CanWrite,"original native source actually released");
                    Check(events.Last().Phase==(mode=="receiver_abort"?"failed":"cancelled")&&events.Last().CleanupComplete==true&&
                        (mode!="disconnect"||terminal.Error=="virtual_cancelled"),
                        "truthful failure/cancel/disconnected release status: "+mode+"/"+events.Last().Phase+"/"+events.Last().CleanupComplete+"/"+terminal.Error);
                    Check(events.Last().NativeErrorCode==unchecked((int)0x80004004),"target HRESULT at original boundary");
                    await dispatcher.DrainAsync();
                }finally{if(capability!=IntPtr.Zero)Marshal.Release(capability);if(held.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref held);}
            }
        }finally{Directory.Delete(root,true);}
    }
    internal static async Task ActualMultiDirectoryUnknownProgressAndEndOrdering()
    {
        await ActualMemberByteProgress();
        var root=Root();try {
            var path=Path.Combine(root,"tree.zip");Zip(path,true);var browser=await Browse(root,path);
            foreach(var earlyEnd in new[]{false,true}) {
                var log=new CaptureLog();
                await using var entries=ArchiveEntrySession.ForOwnedTestPackage(root,log);var command=Command(browser,path,true);
                entries.ProbeWriteForFixture(_=>Thread.Sleep(125));
                var events=new ConcurrentQueue<ArchiveVirtualDragProgress>();
                using var inLoop=new ManualResetEventSlim();using var exitLoop=new ManualResetEventSlim();var pointer=IntPtr.Zero;STGMEDIUM held=default;
                await using var drag=new ArchiveVirtualDragSession(entries,log,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,escape:()=>false,loop:(data,_)=>{
                    pointer=Capability(data);Check(Slot<Start>(pointer,5)(pointer,IntPtr.Zero)==0,"start");
                    var descriptor=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);data.GetData(ref descriptor,out var group);ReleaseStgMedium(ref group);
                    Check(events.IsEmpty&&!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"no eager work/progress");
                    var directory=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);Check(data.QueryGetData(ref directory)<0,"directory remains descriptor-only");
                    var one=Format(ArchiveVirtualFileDataObject.ContentsFormat,1,TYMED.TYMED_ISTREAM);data.GetData(ref one,out held);Thread.Sleep(120);
                    data.GetData(ref one,out var repeat);ReleaseStgMedium(ref repeat);
                    var two=Format(ArchiveVirtualFileDataObject.ContentsFormat,2,TYMED.TYMED_ISTREAM);data.GetData(ref two,out var second);ReleaseStgMedium(ref second);Thread.Sleep(120);
                    Check(Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Length==2,"repeat does not decode twice; directories never materialize");
                    if(earlyEnd)Check(Slot<End>(pointer,7)(pointer,0,IntPtr.Zero,1)==0,"End before native loop return");
                    inLoop.Set();Check(exitLoop.Wait(TimeSpan.FromSeconds(10)),"bounded loop gate");return(0x00040100,1);
                });
                drag.Progress=e=>{Check(e.CorrelationId==command.OperationId&&e.Payload.GetProperty("event").GetString()=="archive_virtual_drag_progress","original correlation/envelope");events.Enqueue(e.Payload.GetProperty("data").Deserialize<ArchiveVirtualDragProgress>(JsonDefaults.Options)!);};
                var done=drag.Dispatch(Wire(command),1);
                try {
                    Check(await Task.Run(()=>inLoop.Wait(TimeSpan.FromSeconds(20))),"all original materializations ready");
                    Check(!done.IsCompleted&&events.All(e=>e.Phase!="finished"),"cache ready != target complete; End alone != loop end");
                    Check(events.All(e=>e.FilesTotal==2&&e.BytesTotal is null&&e.SessionId==command.SessionId&&e.Generation==1),"file-only total, any unknown stays unknown");
                    Check(events.Any(e=>e.Phase=="materializing"&&e.CurrentIndex==1&&e.CurrentMember=="root/one.txt")&&events.Any(e=>e.Phase=="target_copying"),"demand/current frozen descriptor and copy phase");
                    var ordered=events.ToArray();
                    Check(ordered.Select(e=>e.Sequence).All(value=>value is >0)&&
                        ordered.Zip(ordered.Skip(1)).All(pair=>pair.First.Sequence<pair.Second.Sequence),"native snapshots have original publication order");
                    Check(ordered.Any(e=>e.CurrentIndex==2&&e.CurrentRequestId==command.Items![2].Entry.OperationId&&
                        e.CurrentBytesDone==0&&e.CurrentBytesTotal is null&&e.BytesDone==3&&e.BytesProcessed==3),
                        "member switch resets only current processed bytes; unknown total remains unknown");
                    Check(ordered.All(e=>e.CurrentIndex is not {} index||e.CurrentRequestId==command.Items![index].Entry.OperationId),
                        "each published descriptor is bound to its frozen request");
                    exitLoop.Set();await Task.Delay(120);
                    if(!earlyEnd){Check(!done.IsCompleted,"loop end alone != EndOperation");Check(Slot<End>(pointer,7)(pointer,0,IntPtr.Zero,1)==0,"receiver reports success");}
                    var result=await done.WaitAsync(TimeSpan.FromSeconds(20));Check(result.Success,"receiver+loop+cleanup success");
                    var final=events.Last();Check(final.Phase=="finished"&&final.Code=="receiver_reported_complete"&&final.CleanupComplete==true&&final.FilesDone==2&&final.BytesDone==6,"actual sealed6 bytes/2 files not source/archive size");
                    Check(final.BytesProcessed==6&&final.CurrentBytesDone==3&&final.Sequence is >0,"terminal sealed/processed bytes never double-count");
                    Check(new[]{"descriptor_delivered","async_start","async_end","ole_return","terminal"}.All(stage=>
                        log.Lines.Any(line=>line.Contains("archive.virtual_drag."+stage,StringComparison.Ordinal))),
                        "bounded descriptor/receiver/OLE lifecycle observations share original drag correlation");
                    Check(log.Lines.Any(line=>line.Contains("archive.entry.drag.cleanup",StringComparison.Ordinal)&&
                        line.Contains("seal_count",StringComparison.Ordinal)&&line.Contains("cursor_ms",StringComparison.Ordinal)),
                        "aggregate Seal and verified native cursor time belongs to original cleanup");
                    Check(events.Count<=12&&log.Lines.All(line=>!line.Contains("root/one.txt",StringComparison.Ordinal)&&!line.Contains(path,StringComparison.Ordinal)),"bounded original callback and payload-free diagnostics");
                    Check(Read(ref held,3)=="one","stream handle survives early/late End + retirement");
                }finally{exitLoop.Set();if(pointer!=IntPtr.Zero)Marshal.Release(pointer);if(held.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref held);}
            }
        }finally{Directory.Delete(root,true);}
    }
    private static async Task ActualMemberByteProgress()
    {
        var root=Root();
        try {
            var path=Path.Combine(root,"progress.zip");
            const int length=2*1024*1024;
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create)) {
                using var output=zip.CreateEntry("large.bin",CompressionLevel.NoCompression).Open();
                output.Write(new byte[length]);
            }
            var browser=await Browse(root,path);
            await using var entries=ArchiveEntrySession.ForOwnedTestPackage(root);
            entries.ProbeWriteForFixture(_=>Thread.Sleep(25));
            var command=Command(browser,path);
            command=command with { Items=[command.Items![0] with { Name="large.bin",Size=length }] };
            var snapshots=new ConcurrentQueue<JsonElement>();
            var capability=IntPtr.Zero;
            using var allReady=new ManualResetEventSlim();
            await using var drag=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>null,
                pressed:()=>true,escape:()=>false,loop:(data,_)=>{
                    capability=Capability(data);
                    Check(Slot<Start>(capability,5)(capability,IntPtr.Zero)==0,"progress receiver start");
                    var format=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);
                    data.GetData(ref format,out var medium);
                    try { Check(Read(ref medium,1)=="\0","real verified stream payload"); }
                    finally { ReleaseStgMedium(ref medium); }
                    allReady.Set();return(0x00040100,1);
                });
            drag.Progress=e=>snapshots.Enqueue(e.Payload.GetProperty("data").Clone());
            var completed=drag.Dispatch(Wire(command),1);
            try {
                Check(await Task.Run(()=>SpinWait.SpinUntil(()=>snapshots.Any(e=>e.GetProperty("phase").GetString()=="materializing"&&
                    e.TryGetProperty("current_bytes_done",out var bytes)&&bytes.ValueKind==JsonValueKind.Number&&bytes.GetUInt64()>0),TimeSpan.FromSeconds(20))),
                    "original worker must publish a nonzero current snapshot before Seal");
                var stale=JsonSerializer.SerializeToElement(new { op="progress",progress=new {
                    drag_demand=new { request_id=Guid.NewGuid(), index=0, ordinal=0UL }, current_bytes_done=(ulong)length+1,
                    current_bytes_total=(ulong)length } },JsonDefaults.Options);
                var ingress=typeof(ArchiveEntrySession).GetMethod("OnVirtualRunner",System.Reflection.BindingFlags.Instance|System.Reflection.BindingFlags.NonPublic)!;
                Check((bool)ingress.Invoke(entries,[new ArchiveWorkerEvent("frame",command.OperationId,Frame:stale)])!,
                    "original runner callback consumes stale progress without granting output or cancelling worker");
                Check(await Task.Run(()=>allReady.Wait(TimeSpan.FromSeconds(20))),"bounded original worker materialization");
                await Task.Delay(120);
                Check(!completed.IsCompleted,"verified cache is not receiver completion");
                Check(snapshots.Any(e=>e.GetProperty("phase").GetString()=="materializing"&&
                    e.TryGetProperty("current_bytes_done",out var bytes)&&bytes.ValueKind==JsonValueKind.Number&&
                    bytes.GetUInt64()>0&&bytes.GetUInt64()<length),"actual worker current bytes must arrive before Seal");
                var observed=snapshots.Select(e=>e.Deserialize<ArchiveVirtualDragProgress>(JsonDefaults.Options)!).ToArray();
                Check(observed.Any(e=>e.Phase=="materializing"&&e.CurrentIndex==0&&e.CurrentRequestId==command.Items![0].Entry.OperationId&&
                    e.BytesDone==0&&e.CurrentBytesDone is >0 and <length&&e.BytesProcessed==e.CurrentBytesDone&&e.CurrentBytesTotal==length),
                    "real worker snapshot preserves current request and processed-versus-sealed units");
                Check(observed.All(e=>e.BytesProcessed is null||e.BytesProcessed<=length)&&
                    observed.Zip(observed.Skip(1)).All(pair=>pair.First.Sequence<pair.Second.Sequence),
                    "wrong-demand injection cannot publish oversize bytes or regress sequence");
                Check(snapshots.All(e=>e.GetProperty("phase").GetString()!="finished"),"no early success");
            } finally {
                if(capability!=IntPtr.Zero) {
                    Slot<End>(capability,7)(capability,0,IntPtr.Zero,1);
                    Marshal.Release(capability);
                }
                await completed.WaitAsync(TimeSpan.FromSeconds(20));
            }
        } finally { Directory.Delete(root,true); }
    }
    private sealed class TrackedInput:IDisposable {internal bool Disposed;public void Dispose()=>Disposed=true;}
    private sealed class CaptureLog:IPlatformLog {internal readonly ConcurrentQueue<string> Lines=new();public void Write(string level,string eventName,Guid? correlationId=null,IReadOnlyDictionary<string,object?>? fields=null)=>Lines.Enqueue(JsonSerializer.Serialize(new{eventName,correlationId,fields}));}
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]private delegate int SetBool(IntPtr self,int value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]private delegate int GetBool(IntPtr self,out int value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]private delegate int Start(IntPtr self,IntPtr reserved);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]private delegate int End(IntPtr self,int result,IntPtr reserved,uint effect);
}
