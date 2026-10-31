using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text.Json;
using System.IO.Compression;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveVirtualDragTests
{
    internal static void Check(bool value,string detail){if(!value)throw new InvalidOperationException(detail);}
    internal static FORMATETC Format(short format,int index,TYMED medium)=>new(){cfFormat=format,dwAspect=DVASPECT.DVASPECT_CONTENT,lindex=index,tymed=medium};
    internal static Task PathAndDescriptorBoundaries()
    {
        Check(ArchiveJobWire.PathValid(@"C:\") && ArchiveJobWire.PathValid(@"d:\") && !ArchiveJobWire.PathValid("C:") && !ArchiveJobWire.PathValid("C:/"), "drive roots are valid destinations, not drive-relative paths");
        var command=Command(Synthetic(),@"C:\owned\source.zip",(new string('界',86),3,0));
        Check(ArchiveVirtualDragWire.Parse(Wire(command)).Items![0].Name==new string('界',86), "descriptor names use UTF16 units, not UTF8 bytes");
        var oversized=command with {Items=command.Items!.Select(item=>item with{Name=new string('界',256)}).ToArray()};
        try {ArchiveVirtualDragWire.Parse(Wire(oversized));throw new InvalidOperationException("oversized descriptor component admitted");}catch(InvalidDataException){}
        return Task.CompletedTask;
    }
    internal static Task DescriptorIndexedStreamsAndRetirement()
    {
        var completion=new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var thread=new Thread(()=>{try{
            var calls=new List<int>();
            using var data=new ArchiveVirtualFileDataObject([new("one.txt",3),new("目录/two.txt",3),new("目录",null,true)],index=>{calls.Add(index);return Memory(index==0?"one"u8.ToArray():"two"u8.ToArray());});
            var descriptor=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);
            Check(data.QueryGetData(ref descriptor)==0,"descriptor query must succeed");
            data.GetData(ref descriptor,out var group);
            try {var pointer=GlobalLock(group.unionmember);try{Check(Marshal.ReadInt32(pointer)==3,"FILEGROUPDESCRIPTOR count including explicit parent");Check(Marshal.PtrToStringUni(pointer+4+72)=="one.txt","SDK filename offset72");Check(Marshal.PtrToStringUni(pointer+4+592+72)=="目录\\two.txt","SDK descriptor size592 and directory separator");}finally{GlobalUnlock(group.unionmember);}}
            finally{ReleaseStgMedium(ref group);}
            Check(calls.Count==0,"descriptor must not materialize");
            var formats=data.EnumFormatEtc(DATADIR.DATADIR_GET);var values=new FORMATETC[2];var fetched=new int[1];Check(formats.Next(2,values,fetched)==0&&fetched[0]==2,"enum Next2");formats.Clone(out var clone);Check(clone.Next(2,values,fetched)==1&&fetched[0]==1,"enum Clone position/partial S_FALSE");Check(formats.Reset()==0&&formats.Skip(4)==1,"enum Reset/Skip");
            foreach(var bad in new[]{Format(15,-1,TYMED.TYMED_HGLOBAL),Format(ArchiveVirtualFileDataObject.ContentsFormat,-1,TYMED.TYMED_ISTREAM),Format(ArchiveVirtualFileDataObject.ContentsFormat,2,TYMED.TYMED_ISTREAM),Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_ISTREAM)}) {var requested=bad;Check(data.QueryGetData(ref requested)<0,"unsupported format/index/medium");}
            var contents=Format(ArchiveVirtualFileDataObject.ContentsFormat,1,TYMED.TYMED_ISTREAM);
            data.GetData(ref contents,out var first);data.GetData(ref contents,out var second);
            try {Check(calls.SequenceEqual(new[]{1,1}),"exact indexed requests");Check(Read(ref first,1)=="t","first cursor");Check(Read(ref second,3)=="two","independent repeat cursor");data.Dispose();Check(Read(ref first,2)=="wo","returned stream survives data-object retirement");Check(data.QueryGetData(ref contents)<0,"retired new requests rejected");}
            finally {ReleaseStgMedium(ref first);ReleaseStgMedium(ref second);}
            var invalid=Format(ArchiveVirtualFileDataObject.ContentsFormat,3,TYMED.TYMED_ISTREAM);Check(data.QueryGetData(ref invalid)<0,"invalid index");
            completion.SetResult();
        }catch(Exception failure){completion.SetException(failure);}});
        thread.SetApartmentState(ApartmentState.STA);thread.Start();return completion.Task.WaitAsync(TimeSpan.FromSeconds(15));
    }
    internal static IntPtr Memory(byte[] bytes){var stream=SHCreateMemStream(bytes,(uint)bytes.Length);Check(stream!=IntPtr.Zero,"native memory stream");return stream;}
    private static string Root()=>Directory.CreateDirectory(Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!,"archive-product-"+Guid.NewGuid())).FullName;
    private static ArchiveVirtualDragCommand Command(ArchiveBrowserContext browser,string path,params (string Name,ulong Size,ulong Ordinal)[] items)
    {
        var session=Guid.NewGuid();
        return new(1,Guid.NewGuid(),session,1,"start",42,items.Select(item=>new ArchiveVirtualDragItem(item.Name,item.Size,new(){Version=1,OperationId=Guid.NewGuid(),SessionId=session,Generation=1,Action="materialize",Path=path,Ordinal=item.Ordinal,Browser=browser,CopyMotw=false})).ToArray());
    }
    private static ArchiveBrowserContext Synthetic()=>new(Guid.NewGuid(),"Zip","",new("v1:fixture",Enumerable.Repeat(1u,32).ToArray(),1,1));
    private static ProtocolEnvelope Wire(ArchiveVirtualDragCommand c,string name="archive_virtual_drag")=>ProtocolEnvelope.Create("platform_command",new{command=name,data=c},c.OperationId);
    internal static async Task FormalWireAliasAndSourcePeerRejections()
    {
        var command=Command(Synthetic(),@"C:\owned\source.zip",("one.txt",3,0));
        var legacy=ArchiveVirtualDragWire.Parse(Wire(command,"archive_virtual_drag_spike"));
        Check(legacy.OperationId==command.OperationId&&legacy.Items![0].Entry.OperationId==command.Items![0].Entry.OperationId,"legacy spike parser compatibility and frozen entry identity");
        ArchiveVirtualDragCommand formal;
        try {formal=ArchiveVirtualDragWire.Parse(Wire(command));}
        catch(InvalidDataException failure){throw new InvalidOperationException("formal archive_virtual_drag must parse through the original virtual drag contract",failure);}
        Check(JsonSerializer.Serialize(formal)==JsonSerializer.Serialize(legacy),"formal and legacy aliases preserve the identical DTO/bounds/correlation");
        var cancel=command with{Action="cancel",SourceWindowHandle=0,Items=null};
        Check(ArchiveVirtualDragWire.Parse(Wire(cancel)).OperationId==command.OperationId,"formal exact cancel correlation");
        foreach(var name in new[]{"archive_virtual_drag","archive_virtual_drag_spike"}) {
            foreach(var invalid in new[]{command with{Generation=2},command with{Version=2},command with{SourceWindowHandle=0},command with{Items=[]},command with{Items=command.Items!.Concat(command.Items!).ToArray()},command with{Items=command.Items!.Select(item=>item with{Size=ulong.MaxValue}).ToArray()},cancel with{Items=command.Items}}) {
                try{ArchiveVirtualDragWire.Parse(Wire(invalid,name));throw new InvalidOperationException("invalid virtual selection admitted by "+name);}catch(InvalidDataException){}
            }
            try{ArchiveVirtualDragWire.Parse(Wire(command,name) with{CorrelationId=Guid.NewGuid()});throw new InvalidOperationException("mismatched correlation admitted");}catch(InvalidDataException){}
        }
        try{ArchiveVirtualDragWire.Parse(Wire(command,"archive_virtual_drag_invalid"));throw new InvalidOperationException("unknown virtual command admitted");}catch(InvalidDataException){}
        var capabilities=PlatformCapabilities.Create(false);
        Check(capabilities.Contains("archive_virtual_drag_v1")&&capabilities.Contains("archive_virtual_drag_spike_v1"),"formal capability and legacy capability both advertised");
        var root=Root();try {
            await using var entries=ArchiveEntrySession.ForOwnedTestPackage(root);
            var checkedSources=new List<(long Window,uint Peer)>();var prepared=false;
            await using var drag=new ArchiveVirtualDragSession(entries,validSource:(window,peer)=>{checkedSources.Add((window,peer));return window==42&&peer==7;},prepare:_=>{prepared=true;return null;},pressed:()=>false,loop:(_,_)=>throw new InvalidOperationException("rejected source must not enter OLE"));
            var wrongPeer=await drag.Dispatch(Wire(command),8);
            var wrongSource=await drag.Dispatch(Wire(command with{SourceWindowHandle=43}),7);
            Check(!wrongPeer.Success&&wrongPeer.Error=="invalid_source_window"&&!wrongSource.Success&&wrongSource.Error=="invalid_source_window","formal source HWND/connected peer rejection");
            var invalid=await drag.Dispatch(Wire(command with{Version=2}),7);
            Check(!invalid.Success&&invalid.Error=="invalid_virtual_drag"&&!prepared&&checkedSources.SequenceEqual(new[]{(42L,8u),(43L,7u)}),"invalid command rejected before source admission, STA or worker");
        }finally{Directory.Delete(root,true);}
    }
    internal static async Task StaGestureOrderingCancelAndReaderResponsiveness()
    {
        var root=Root();try {
            await using var entries=ArchiveEntrySession.ForOwnedTestPackage(root);
            var sequence=new List<string>();var entered=new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            var fixtureSource = Path.Combine(root,"source.zip");File.WriteAllBytes(fixtureSource,[1,2,3]);
            var command=Command(Synthetic(),fixtureSource,("one.txt",3,0));var loops=0;var caller=Environment.CurrentManagedThreadId;
            Check(!ArchiveVirtualDragWire.Parse(Wire(command)).Items!.Any(item=>item.Entry.PasswordInteraction),"formal drag has no hidden prompt authority");
            foreach(var invalid in new[]{command with{Generation=2},command with{Items=command.Items!.Concat(command.Items!).ToArray()},command with{Items=command.Items!.Select(item=>item with{Size=ulong.MaxValue}).ToArray()}}) {
                try{ArchiveVirtualDragWire.Parse(Wire(invalid));throw new InvalidOperationException("invalid formal selection admitted");}catch(InvalidDataException){}
            }
            await using var drag=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>{sequence.Add("capture");return null;},pressed:()=>{sequence.Add("button");return true;},loop:(data,source)=>{
                Check(Thread.CurrentThread.GetApartmentState()==ApartmentState.STA&&Environment.CurrentManagedThreadId!=caller,"dedicated STA not caller");
                sequence.Add("ole");Interlocked.Increment(ref loops);entered.SetResult();
                while(source.QueryContinueDrag(false,1)==0)Thread.Sleep(5);
                return(0x00040101,0);
            });
            var dispatcher=new PlatformCommandSequenceDispatcher();var done=drag.Dispatch(Wire(command),1);
            await dispatcher.DispatchAsync("archive_entry",()=>done); // Same synchronous admission and detached receipt as the pipe branch.
            await entered.Task.WaitAsync(TimeSpan.FromSeconds(5));
            var later=false;await dispatcher.DispatchAsync("later-command",()=>{later=true;return Task.CompletedTask;});Check(later&&!done!.IsCompleted,"serial reader remains responsive");
            var busy=await drag.Dispatch(Wire(command with{OperationId=Guid.NewGuid()}),1);
            var legacyBusy=await drag.Dispatch(Wire(command with{OperationId=Guid.NewGuid()},"archive_virtual_drag_spike"),1);
            Check(!busy.Success&&busy.Error=="virtual_drag_busy"&&!legacyBusy.Success&&legacyBusy.Error=="virtual_drag_busy","formal and legacy names share single native gesture admission");
            var staleCancel=await drag.Dispatch(Wire(command with{Action="cancel",SourceWindowHandle=0,Items=null,Generation=2}),1);
            Check(!staleCancel.Success&&staleCancel.Error=="stale_virtual_drag"&&!done.IsCompleted,"stale generation cannot cancel formal operation");
            var cancel=command with{Action="cancel",SourceWindowHandle=0,Items=null};Check((await drag.Dispatch(Wire(cancel),1)).Success,"exact cancellation admitted");
            Check((await done!.WaitAsync(TimeSpan.FromSeconds(5))).Data!.Contains("cancelled"),"cancel terminal");
            Check(sequence.SequenceEqual(new[]{"capture","button","ole"})&&loops==1,"capture/button/one-loop order");
            await dispatcher.DrainAsync();
            var earlyLoops=0;await using var early=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>false,loop:(_,_)=>{earlyLoops++;return(0,0);});
            var ended=await early.Dispatch(Wire(command with{OperationId=Guid.NewGuid()}),1);Check(earlyLoops==0&&ended.Data!.Contains("cancelled"),"mouse-up before OLE never starts loop");
        }finally{Directory.Delete(root,true);}
    }
    private static async Task<ArchiveBrowserContext> Browse(string root,string path)
    {
        await using var read=ArchiveReadSession.ForOwnedTestPackage(root);
        var request=ArchiveProductReadTests.Command(path:path,version:2,limits:new(){BrowserContext=true});var identity=ArchiveReadWire.Parse(request);
        Check(read.Dispatch(request).Success,"browse admission");
        while(true) {var frame=(await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(30))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if(frame.Batch is {} batch)Check(read.Dispatch(ArchiveProductReadTests.Command("ack",identity.OperationId,identity.SessionId,identity.Generation,sequence:batch.Sequence,version:2)).Success,"credit");
            if(frame.Terminal is {} terminal){Check(terminal.Status=="succeeded"&&terminal.CleanupComplete&&terminal.Browser is not null,"bound snapshot");return terminal.Browser!;}
        }
    }
    internal static async Task ActualWorkerDelayedIndexedContentsAndChangedSource()
    {
        var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");if(string.IsNullOrEmpty(worker)||!File.Exists(worker))throw new TestSkippedException("Explicit freshly built matching worker required");
        var root=Root();STGMEDIUM retained=default;try {
            File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));var path=Path.Combine(root,"source.zip");
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create))foreach(var pair in new[]{("one.txt","one"),("two.txt","two")}){using var writer=new StreamWriter(zip.CreateEntry(pair.Item1).Open());writer.Write(pair.Item2);}
            var browser=await Browse(root,path);var command=Command(browser,path,("one.txt",3,0),("two.txt",3,1));
            await using(var entries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                await using var drag=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,loop:(data,source)=>{
                    var descriptor=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);data.GetData(ref descriptor,out var group);ReleaseStgMedium(ref group);
                    Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"descriptor causes zero materialization");
                    var second=Format(ArchiveVirtualFileDataObject.ContentsFormat,1,TYMED.TYMED_ISTREAM);data.GetData(ref second,out retained);Check(Read(ref retained,1)=="t","indexed worker payload");
                    data.GetData(ref second,out var repeat);try{Check(Read(ref repeat,3)=="two","same indexed item repeated native cursor");}finally{ReleaseStgMedium(ref repeat);}
                    var first=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);data.GetData(ref first,out var one);try{Check(Read(ref one,3)=="one","multiple original ordinals");}finally{ReleaseStgMedium(ref one);}
                    return(0x00040100,1);
                });
                var done=await drag.Dispatch(Wire(command),1).WaitAsync(TimeSpan.FromSeconds(40));Check(done.Success&&done.Data!.Contains("dropped"),"worker-backed virtual result "+done.Error);
                Check(Read(ref retained,2)=="wo","target-held stream survives OLE/session retirement");ReleaseStgMedium(ref retained);retained=default;
            }
            File.AppendAllText(path,"source-changed");
            await using(var entries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                await using var stale=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,loop:(data,source)=>{
                    var request=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);data.GetData(ref request,out var medium);ReleaseStgMedium(ref medium);return(0x00040100,1);
                });
                var failed=await stale.Dispatch(Wire(command with{OperationId=Guid.NewGuid(),Items=command.Items!.Select(item=>item with{Entry=item.Entry with{OperationId=Guid.NewGuid()}}).ToArray()}),1).WaitAsync(TimeSpan.FromSeconds(40));
                Check(!failed.Success,"stale original source cannot deliver contents");
            }
        }finally{if(retained.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref retained);Directory.Delete(root,true);}
    }
    internal static async Task ActualSlowMaterializationCancelSourceCloseAndDisconnect()
    {
        var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");if(string.IsNullOrEmpty(worker)||!File.Exists(worker))throw new TestSkippedException("Explicit matching worker required");
        var root=Root();try {
            File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));var path=Path.Combine(root,"source.zip");
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create)){using var writer=new StreamWriter(zip.CreateEntry("one.txt").Open());writer.Write("one");}
            var browser=await Browse(root,path);
            foreach(var mode in new[]{"cancel","escape","source_closed","disconnect"}) {
                using var blocked=new ManualResetEventSlim();using var release=new ManualResetEventSlim();var alive=1;var escaped=0;
                await using var entries=ArchiveEntrySession.ForOwnedTestPackage(root);
                entries.ProbeWriteForFixture(_=>{blocked.Set();Check(release.Wait(TimeSpan.FromSeconds(15)),"test boundary released");});
                var command=Command(browser,path,("one.txt",3,0));
                await using var drag=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>Volatile.Read(ref alive)!=0,prepare:_=>null,pressed:()=>true,escape:()=>Volatile.Read(ref escaped)!=0,loop:(data,source)=>{
                    var request=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);data.GetData(ref request,out var medium);ReleaseStgMedium(ref medium);return(0x00040100,1);
                });
                var done=drag.Dispatch(Wire(command),1);
                try {
                    var reached=await Task.Run(()=>blocked.Wait(TimeSpan.FromSeconds(10)));
                    Check(reached,"actual worker reached slow boundary; terminal="+(done.IsCompleted?done.Result.Error:"pending"));
                    if(mode=="source_closed")Interlocked.Exchange(ref alive,0);
                    else if(mode=="escape")Interlocked.Exchange(ref escaped,1);
                    else if(mode=="disconnect")drag.Stop();
                    else Check((await drag.Dispatch(Wire(command with{Action="cancel",SourceWindowHandle=0,Items=null}),1)).Success,"cancel slow materialization");
                    await Task.Delay(120);Check(!done.IsCompleted,"terminal must wait for actual original worker cleanup");
                }finally{release.Set();}
                var terminal=await done.WaitAsync(TimeSpan.FromSeconds(15));Check(!terminal.Success||terminal.Data!.Contains("cancelled"),"no successful drop after retirement");
                Check(!Directory.GetFiles(root,".nxpart-*",SearchOption.AllDirectories).Any(),"cancelled original partial cleanup");
            }
        }finally{Directory.Delete(root,true);}
    }
    internal static Task NativeComAbiAndFinalCancelledOleBoundary()
    {
        var completion=new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var thread=new Thread(()=>{var initialized=false;try {
            Check(OleInitialize(IntPtr.Zero)>=0,"owned STA OleInitialize");initialized=true;
            var calls=0;using var data=new ArchiveVirtualFileDataObject([new("synthetic.txt",3)],_=>{calls++;return Memory("one"u8.ToArray());});
            var pointer=Marshal.GetComInterfaceForObject(data,typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
            try {
                var table=Marshal.ReadIntPtr(pointer);var get=Marshal.GetDelegateForFunctionPointer<GetDataAbi>(Marshal.ReadIntPtr(table,3*IntPtr.Size));
                var descriptor=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);
                var result=get(pointer,ref descriptor,out var medium);Check(result==0&&medium.tymed==TYMED.TYMED_HGLOBAL,"native IDataObject slot3/GetData ABI");ReleaseStgMedium(ref medium);
            }finally{Marshal.Release(pointer);}
            using var stop=new CancellationTokenSource();stop.Cancel();
            var terminal=ArchiveVirtualDragSession.NativeLoop(data,new(stop.Token,()=>true));
            Check(terminal.Item1==0x00040101&&terminal.Item2==0&&calls==0,"final native boundary rejects pre-cancelled OLE/no contents/no external drop");
            completion.SetResult();
        }catch(Exception failure){completion.SetException(failure);}finally{if(initialized)OleUninitialize();}});
        thread.IsBackground=true;thread.SetApartmentState(ApartmentState.STA);thread.Start();return completion.Task.WaitAsync(TimeSpan.FromSeconds(10));
    }
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]private delegate int GetDataAbi(IntPtr self,ref FORMATETC format,out STGMEDIUM medium);
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern int OleInitialize(IntPtr reserved);
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern void OleUninitialize();
    internal static string Read(ref STGMEDIUM medium,int count){var stream=(IStream)Marshal.GetObjectForIUnknown(medium.unionmember);try{var buffer=new byte[count];var actual=Marshal.AllocHGlobal(4);try{stream.Read(buffer,count,actual);return System.Text.Encoding.UTF8.GetString(buffer,0,Marshal.ReadInt32(actual));}finally{Marshal.FreeHGlobal(actual);}}finally{Marshal.ReleaseComObject(stream);}}
    [DllImport("shlwapi.dll",ExactSpelling=true)] private static extern IntPtr SHCreateMemStream(byte[] bytes,uint length);
    [DllImport("ole32.dll",ExactSpelling=true)] internal static extern void ReleaseStgMedium(ref STGMEDIUM medium);
    [DllImport("kernel32.dll",ExactSpelling=true)] private static extern IntPtr GlobalLock(IntPtr memory);
    [DllImport("kernel32.dll",ExactSpelling=true)] private static extern bool GlobalUnlock(IntPtr memory);
}
