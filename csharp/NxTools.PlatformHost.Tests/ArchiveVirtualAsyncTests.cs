using System.Diagnostics;
using System.IO.Compression;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveVirtualDragTests;

namespace NxTools.PlatformHost.Tests;

// Owned consumer uses real COM QueryInterface/vtable and standard inter-apartment marshalling.
// The only delayed variable is the original native output writer checkpoint, not a fake extractor.
internal static class ArchiveVirtualAsyncTests
{
    private static readonly Guid AsyncId=new("3D8B0590-F691-11d2-8EA9-006097DF5BD4");
    private static readonly Guid DataId=new("0000010e-0000-0000-C000-000000000046");
    internal static async Task ActualSlowConsumerReturnsUiBeforeContent()
    {
        var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if(string.IsNullOrEmpty(worker)||!File.Exists(worker))throw new TestSkippedException("Explicit matching worker required");
        var root=Directory.CreateDirectory(Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!,"archive-product-async-"+Guid.NewGuid())).FullName;
        using var blocked=new ManualResetEventSlim();using var release=new ManualResetEventSlim();
        var uiReturned=new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var receiverDone=new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        long uiMs=-1;var negotiated=false;STGMEDIUM retained=default;
        try {
            File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));var path=Path.Combine(root,"slow.zip");
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create)){using var output=zip.CreateEntry("large.bin",CompressionLevel.Fastest).Open();var bytes=new byte[65536];for(var i=0;i<64;i++)output.Write(bytes);}
            ArchiveBrowserContext browser;
            await using(var read=ArchiveReadSession.ForOwnedTestPackage(root)) {
                var request=ArchiveProductReadTests.Command(path:path,version:2,limits:new(){BrowserContext=true});var id=ArchiveReadWire.Parse(request);Check(read.Dispatch(request).Success,"browse admitted");
                while(true){var e=(await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(30))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                    if(e.Batch is {} b)Check(read.Dispatch(ArchiveProductReadTests.Command("ack",id.OperationId,id.SessionId,id.Generation,sequence:b.Sequence,version:2)).Success,"credit");
                    if(e.Terminal is {} t){Check(t.Status=="succeeded"&&t.Browser is not null,"source bound");browser=t.Browser!;break;}}
            }
            var session=Guid.NewGuid();var command=new ArchiveVirtualDragCommand(1,Guid.NewGuid(),session,1,"start",42,[new("large.bin",4194304,new(){Version=1,OperationId=Guid.NewGuid(),SessionId=session,Generation=1,Action="materialize",Path=path,Ordinal=0,Browser=browser,CopyMotw=false})]);
            await using var entries=ArchiveEntrySession.ForOwnedTestPackage(root);
            entries.ProbeWriteForFixture(_=>{blocked.Set();Check(release.Wait(TimeSpan.FromSeconds(10)),"bounded slow writer release");});
            await using var drag=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,escape:()=>false,loop:(data,_)=>{
                var clock=Stopwatch.StartNew();var pointer=Marshal.GetComInterfaceForObject(data,typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
                try {
                    var iid=AsyncId;var hr=Marshal.QueryInterface(pointer,ref iid,out var capability);
                    if(hr==0){try{
                        Check(Call<GetBool>(capability,4)(capability,out var enabled)==0&&enabled!=0,"source explicitly enables async mode");
                        Check(Call<Start>(capability,5)(capability,IntPtr.Zero)==0,"StartOperation before Drop returns");negotiated=true;
                        var dataIid=DataId;Check(CoMarshalInterThreadInterfaceInStream(ref dataIid,pointer,out var marshalled)==0,"standard IDataObject marshal");
                        var receiver=new Thread(()=>{var initialized=CoInitializeEx(IntPtr.Zero,0)>=0;IntPtr remote=IntPtr.Zero,asyncRemote=IntPtr.Zero;try{
                            var id=DataId;Check(CoGetInterfaceAndReleaseStream(marshalled,ref id,out remote)==0,"background unmarshal");
                            var aid=AsyncId;Check(Marshal.QueryInterface(remote,ref aid,out asyncRemote)==0,"background async QI");
                            var format=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);
                            Check(Call<GetData>(remote,3)(remote,ref format,out retained)==0,"actual native GetData after UI return");
                            Check(Call<End>(asyncRemote,7)(asyncRemote,0,IntPtr.Zero,1)==0,"receiver EndOperation");receiverDone.TrySetResult();
                        }catch(Exception failure){receiverDone.TrySetException(failure);}finally{if(asyncRemote!=IntPtr.Zero)Marshal.Release(asyncRemote);if(remote!=IntPtr.Zero)Marshal.Release(remote);if(initialized)CoUninitialize();}}){IsBackground=true};
                        receiver.SetApartmentState(ApartmentState.MTA);receiver.Start();
                    }finally{Marshal.Release(capability);}}
                    else {
                        // Baseline target fallback: its Drop/UI thread must request synchronously.
                        var format=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);
                        Check(Call<GetData>(pointer,3)(pointer,ref format,out retained)==0,"baseline real GetData");receiverDone.TrySetResult();
                    }
                }finally{Marshal.Release(pointer);uiMs=clock.ElapsedMilliseconds;uiReturned.TrySetResult();}
                return(0x00040100,1);
            });
            var done=drag.Dispatch(ProtocolEnvelope.Create("platform_command",new{command="archive_virtual_drag",data=command},command.OperationId),1);
            try {
                Check(await Task.Run(()=>blocked.Wait(TimeSpan.FromSeconds(10))),"original actual writer reached");
                await Task.Delay(600);
                Console.WriteLine($"MEASUREMENT drag-native slow_gate_ms=600 target_ui_returned={uiReturned.Task.IsCompleted} async_negotiated={negotiated}");
                var responsive=uiReturned.Task.IsCompleted;
                release.Set();await receiverDone.Task.WaitAsync(TimeSpan.FromSeconds(20));var terminal=await done.WaitAsync(TimeSpan.FromSeconds(20));
                Console.WriteLine($"MEASUREMENT drag-native target_ui_ms={uiMs} terminal_success={terminal.Success}");
                Check(responsive,"slow actual content blocks native consumer Drop/UI phase (600ms checkpoint); async negotiation/lifetime required");
                Check(terminal.Success&&negotiated,"target-reported completion succeeds only after EndOperation and actual cleanup");
                Check(Read(ref retained,1)=="\0","already returned native stream survives EndOperation/session retirement");
            }finally{release.Set();}
        }finally{if(retained.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref retained);Directory.Delete(root,true);}
    }
    private static T Call<T>(IntPtr pointer,int slot) where T:Delegate=>Marshal.GetDelegateForFunctionPointer<T>(Marshal.ReadIntPtr(Marshal.ReadIntPtr(pointer),slot*IntPtr.Size));
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]private delegate int GetData(IntPtr self,ref FORMATETC format,out STGMEDIUM medium);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]private delegate int GetBool(IntPtr self,out int value);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]private delegate int Start(IntPtr self,IntPtr reserved);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]private delegate int End(IntPtr self,int result,IntPtr reserved,uint effect);
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern int CoMarshalInterThreadInterfaceInStream(ref Guid iid,IntPtr value,out IntPtr stream);
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern int CoGetInterfaceAndReleaseStream(IntPtr stream,ref Guid iid,out IntPtr value);
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern int CoInitializeEx(IntPtr reserved,uint flags);
    [DllImport("ole32.dll",ExactSpelling=true)]private static extern void CoUninitialize();
}
