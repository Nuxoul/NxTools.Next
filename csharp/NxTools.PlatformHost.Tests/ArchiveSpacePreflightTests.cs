using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;
namespace NxTools.PlatformHost.Tests;

// GUID-owned tiny ordinary files/directories only. No product/worker, ACL, Shell or service changes.
internal static class ArchiveSpacePreflightTests
{
    private static void Check(bool value,string why) { if(!value) throw new InvalidOperationException(why); }
    private sealed class Fixture : IDisposable
    {
        internal readonly DirectoryInfo Root=Directory.CreateTempSubdirectory("hz1-space-"+Guid.NewGuid()+"-");
        internal string Source => Path.Combine(Root.FullName,"private-source.bin");
        internal string Destination => Path.Combine(Root.FullName,"output");
        internal Fixture() { File.WriteAllBytes(Source,[1,2,3,4]); }
        internal ArchiveSpaceCommand Command => new(1,Guid.NewGuid(),Guid.NewGuid(),7,Source,Destination,null,null);
        public void Dispose() => Root.Delete(true);
    }
    private static ArchiveSpaceObservation Data(WindowsCommandResult result)
    {
        Check(result.Success,"space observation failed: "+result.Error+" native="+result.NativeErrorCode);
        return JsonSerializer.Deserialize<ArchiveSpaceObservation>(result.Data!,JsonDefaults.Options)!;
    }
    private static void Failure(WindowsCommandResult result,string code,int? native=null)
    {
        Check(!result.Success && result.Error==code && result.NativeErrorCode==native && result.Data is null,
            "expected "+code+"/"+native+", observed "+result.Error+"/"+result.NativeErrorCode);
    }
    public static Task HeldObservationDoesNotCreateOrMutate()
    {
        using var fixture=new Fixture();
        var destination=Path.Combine(fixture.Root.FullName,"missing","child");
        var before=File.GetLastWriteTimeUtc(fixture.Source);
        var command=fixture.Command with {Destination=destination};
        var extended=command with{Version=2};
        var extendedResult=ArchiveSpacePreflight.Observe(extended);
        Check(extendedResult.Success,"HZ3 v2 must reuse original held observation");
        using(var extendedWire=JsonDocument.Parse(extendedResult.Data!)){
            var set=extendedWire.RootElement.GetProperty("source_set");
            Check(set.GetProperty("identity").GetString()!.StartsWith("v1:",StringComparison.Ordinal)&&set.GetProperty("primary").GetString()==fixture.Source&&set.GetProperty("members").GetArrayLength()==1,"HZ3 source-set facts from actual native leases");
        }
        var data=Data(ArchiveSpacePreflight.Observe(command));
        using(var wire=JsonDocument.Parse(JsonSerializer.Serialize(data,JsonDefaults.Options))) {
            Check(wire.RootElement.TryGetProperty("missing_destination_directories",out var missing) && missing.GetUInt32()==2,
                "held observation must carry the exact two missing destination components");
        }
        Check(data.Version==1 && data.OperationId==command.OperationId && data.SessionId==command.SessionId && data.Generation==7,"request identity roundtrip");
        Check(data.AllocationBytes>0 && data.TargetIdentity.StartsWith("v1:") && data.VolumeIdentity.StartsWith("v1:"),"real allocation and native identity");
        Check(data.DestinationMissing && data.SourceMatches,"missing nearest verified ancestor");
        Check(data.MissingDestinationDirectories==2 && data.ValidFor(command),"two missing target components bound to the actual request");
        var single=Data(ArchiveSpacePreflight.Observe(command with {Destination=Path.Combine(fixture.Root.FullName,"one")}));
        Check(single.MissingDestinationDirectories==1 && single.DestinationMissing,"one missing target component");
        Check(!Directory.Exists(Path.Combine(fixture.Root.FullName,"missing")),"advisory preflight must not create target");
        Check(File.ReadAllBytes(fixture.Source).SequenceEqual(new byte[]{1,2,3,4}) && File.GetLastWriteTimeUtc(fixture.Source)==before,"source bytes and last-write unchanged");
        Check(Directory.GetFileSystemEntries(fixture.Root.FullName).Length==1,"no scratch/capability probe written");
        return Task.CompletedTask;
    }
    public static Task NativeAllocationAndIdentity()
    {
        using var fixture=new Fixture(); Directory.CreateDirectory(fixture.Destination);
        using var held=OutputNative.Open(null,@"\??\"+fixture.Destination,directory:true);
        Check(FullId(held,18,out var nativeId,(uint)Marshal.SizeOf<NativeId>()),"independent native full file ID");
        var path=new StringBuilder(32768);
        Check(FinalPath(held,path,(uint)path.Capacity,1)>0,"independent volume GUID name");
        var stable=path.ToString();var end=stable.IndexOf('}');var volumeRoot=stable[..(end+1)]+"\\";
        Check(DiskAllocation(volumeRoot,out var sectors,out var bytes,out _,out _),"independent volume allocation");
        Check(DiskCapacity(volumeRoot,out _,out var total,out _),"independent volume capacity");
        var data=Data(ArchiveSpacePreflight.Observe(fixture.Command));
        Check(!data.DestinationMissing && data.AllocationBytes==(ulong)sectors*bytes && data.AvailableBytes<=total,"actual native allocation, not assumed 4KiB");
        Check(data.MissingDestinationDirectories==0 && data.ValidFor(fixture.Command with {OperationId=data.OperationId,SessionId=data.SessionId}),"existing target carries exact zero missing components");
        Check(data.TargetIdentity.StartsWith($"v1:{nativeId.Volume:X}:{nativeId.High:X16}{nativeId.Low:X16}:",StringComparison.Ordinal),"full 128-bit target ID and 64-bit serial");
        Check(data.VolumeIdentity==$"v1:{nativeId.Volume:X}:{stable[11..end]}","actual volume GUID plus serial, not drive-letter equality");
        return Task.CompletedTask;
    }
    public static Task MissingSuffixBindsExpectedTarget()
    {
        using var fixture=new Fixture();
        var first=fixture.Command with {Destination=Path.Combine(fixture.Root.FullName,"missing-a","child")};
        var second=first with {Destination=Path.Combine(fixture.Root.FullName,"missing-b","child")};
        var a=Data(ArchiveSpacePreflight.Observe(first));var b=Data(ArchiveSpacePreflight.Observe(second));
        Check(a.TargetIdentity!=b.TargetIdentity,"different missing suffixes must not share expected-target identity");
        Failure(ArchiveSpacePreflight.Observe(second with {ExpectedTarget=a.TargetIdentity}),"target_changed");
        Directory.CreateDirectory(first.Destination);
        Failure(ArchiveSpacePreflight.Observe(first with {ExpectedTarget=a.TargetIdentity}),"target_changed");
        var appeared=Data(ArchiveSpacePreflight.Observe(first));
        Check(!appeared.DestinationMissing && appeared.MissingDestinationDirectories==0,"new target observation reflects appearance rather than stale missing count");
        return Task.CompletedTask;
    }
    public static Task ExpectedSourceAndTargetChanges()
    {
        using var fixture=new Fixture();Directory.CreateDirectory(fixture.Destination);
        var command=fixture.Command;
        string sourceIdentity;using(var sources=ArchiveVolumeSources.Open(fixture.Source,4ul<<20,100ul<<30))sourceIdentity=sources.Identity();
        var original=Data(ArchiveSpacePreflight.Observe(command));
        command=command with {ExpectedSource=sourceIdentity,ExpectedTarget=original.TargetIdentity};
        Check(Data(ArchiveSpacePreflight.Observe(command)).SourceMatches,"original family identity accepted");
        File.WriteAllBytes(fixture.Source,[5,6,7,8,9]);
        Failure(ArchiveSpacePreflight.Observe(command),"source_changed");
        command=command with {ExpectedSource=null};
        Directory.Move(fixture.Destination,fixture.Destination+"-old");Directory.CreateDirectory(fixture.Destination);
        Failure(ArchiveSpacePreflight.Observe(command),"target_changed");
        Check(Directory.GetFileSystemEntries(fixture.Destination).Length==0,"changed target not touched");
        return Task.CompletedTask;
    }
    public static Task CapacityOverrideIsObservationOnly()
    {
        using var fixture=new Fixture();var command=fixture.Command;
        var ordinary=Data(ArchiveSpacePreflight.Observe(command));var calls=0;
        foreach(var available in new[]{0ul,ulong.MaxValue}) {
            var data=Data(ArchiveSpacePreflight.Observe(command,capacityFixture:actual=>{calls++;Check(actual>0,"native observation precedes byte-only seam");return available;}));
            Check(data.AvailableBytes==available && data.AllocationBytes==ordinary.AllocationBytes && data.TargetIdentity==ordinary.TargetIdentity && data.VolumeIdentity==ordinary.VolumeIdentity,"capacity override cannot replace native identity/allocation");
        }
        Check(calls==2 && !Directory.Exists(fixture.Destination) && File.ReadAllBytes(fixture.Source).SequenceEqual(new byte[]{1,2,3,4}),"zero/max advisory observation has no execution/output/source effects");
        return Task.CompletedTask;
    }
    public static Task NativeOpenFailuresAreTyped()
    {
        using var fixture=new Fixture();var command=fixture.Command;
        Failure(ArchiveSpacePreflight.Observe(command with {Source=Path.Combine(fixture.Root.FullName,"absent.bin")}),"source_unavailable",2);
        foreach(var invalid in new[]{@"\\server\share\source.bin",@"\\?\C:\device",fixture.Root.FullName+"\\..\\escape.bin",fixture.Source+":stream"})
            Check(!ArchiveSpacePreflight.Observe(command with {Source=invalid}).Success,"UNC/device/traversal/ADS source remains rejected");
        foreach(var invalid in new[]{@"\\server\share\output",@"\\?\C:\device",fixture.Root.FullName+"\\..\\escape",fixture.Destination+":stream"})
            Check(!ArchiveSpacePreflight.Observe(command with {Destination=invalid}).Success,"UNC/device/traversal/ADS destination remains rejected");
        using(var busy=new FileStream(fixture.Source,FileMode.Open,FileAccess.ReadWrite,FileShare.None))
            Failure(ArchiveSpacePreflight.Observe(command),"source_busy",32);
        File.WriteAllText(fixture.Destination,"not a directory");
        Failure(ArchiveSpacePreflight.Observe(command),"target_wrong_kind",267);
        // NtCreateFile(FILE_NON_DIRECTORY_FILE) reports real ERROR_ACCESS_DENIED for a directory.
        // Exercises native code 5 without altering any ACL; this is not an ACL-denied-user acceptance test.
        Failure(ArchiveSpacePreflight.Observe(command with {Source=fixture.Root.FullName}),"source_denied",5);
        Check(File.ReadAllText(fixture.Destination)=="not a directory","failed native opens preserve target bytes");
        File.Delete(fixture.Destination);Directory.CreateDirectory(fixture.Destination);
        // Deny only write sharing through an owned read handle, not an ACL change. The original
        // writable-directory admission must preserve its typed denial and real sharing code.
        using(var readOnlySharing=ReadDirectory(fixture.Destination,0x100081,1,IntPtr.Zero,3,0x02000000,IntPtr.Zero)) {
            Check(!readOnlySharing.IsInvalid,"owned read-sharing directory handle");
            Failure(ArchiveSpacePreflight.Observe(command),"target_denied",32);
        }
        return Task.CompletedTask;
    }
    public static Task NativeReparseIsRejected()
    {
        using var fixture=new Fixture();Directory.CreateDirectory(fixture.Destination);
        var link=Path.Combine(fixture.Root.FullName,"linked-directory");
        Directory.CreateSymbolicLink(link,fixture.Destination);
        try {
            Failure(ArchiveSpacePreflight.Observe(fixture.Command with {Destination=Path.Combine(link,"child")}),"reparse_component");
            Check(!Directory.Exists(Path.Combine(fixture.Destination,"child")),"reparse target not followed/created");
        } finally { Directory.Delete(link); }
        var sourceLink=Path.Combine(fixture.Root.FullName,"linked-source.bin");File.CreateSymbolicLink(sourceLink,fixture.Source);
        try { Failure(ArchiveSpacePreflight.Observe(fixture.Command with {Source=sourceLink}),"reparse_component"); }
        finally { File.Delete(sourceLink); }
        return Task.CompletedTask;
    }
    public static Task CancellationCannotPublishObservation()
    {
        using var fixture=new Fixture();var command=fixture.Command;using var stop=new CancellationTokenSource();stop.Cancel();
        var calls=0;
        Failure(ArchiveSpacePreflight.Observe(command,cancellation:stop.Token,capacityFixture:actual=>{calls++;return actual;}),"cancelled");
        Check(calls==0,"pre-cancelled work performs no observation");
        using var during=new CancellationTokenSource();
        Failure(ArchiveSpacePreflight.Observe(command,cancellation:during.Token,capacityFixture:actual=>{calls++;during.Cancel();return actual;}),"cancelled");
        Check(calls==1 && !Directory.Exists(fixture.Destination),"cancel at held native observation has no output");
        return Task.CompletedTask;
    }
    public static Task StrictWireAndPrivateDiagnostics()
    {
        using var fixture=new Fixture();var command=fixture.Command;
        ProtocolEnvelope Wire(object data)=>ProtocolEnvelope.Create("platform_command",new{command="archive_space",data});
        Check(ArchiveSpaceCommand.Parse(Wire(command))==command,"strict original DTO roundtrip");
        foreach(var invalid in new[]{command with {Version=3},command with {Generation=0},command with {OperationId=Guid.Empty},command with {SessionId=Guid.Empty},command with {Source=@"\\server\share\a"},command with {Destination=@"\\?\C:\device"},command with {ExpectedSource="v1:\tsecret"},command with {ExpectedTarget="opaque"}}) {
            try { ArchiveSpaceCommand.Parse(Wire(invalid));throw new InvalidOperationException("invalid wire accepted"); }
            catch(InvalidDataException) { }
        }
        var extra=JsonSerializer.SerializeToElement(command,JsonDefaults.Options);
        var fields=extra.EnumerateObject().ToDictionary(p=>p.Name,p=>(object)p.Value);fields["new_budget"]=ulong.MaxValue;
        try { ArchiveSpaceCommand.Parse(Wire(fields));throw new InvalidOperationException("unknown wire field accepted"); }
        catch(JsonException) { }
        var observed=Data(ArchiveSpacePreflight.Observe(command));
        Check(observed.ValidFor(command),"current observation request binding");
        Check(!observed.ValidFor(command with {OperationId=Guid.NewGuid()}),"stale observation rejected");
        foreach(var count in new uint[]{0,65})Check(!(observed with {MissingDestinationDirectories=count}).ValidFor(command),"missing count bound and flag consistency");
        var serialized=JsonSerializer.SerializeToElement(observed,JsonDefaults.Options);
        foreach(var invalidCount in new[]{"-1","4294967296","1.5","\"2\""}) {
            var values=serialized.EnumerateObject().ToDictionary(p=>p.Name,p=>p.Value);
            values["missing_destination_directories"]=JsonDocument.Parse(invalidCount).RootElement.Clone();
            try { JsonSerializer.Deserialize<ArchiveSpaceObservation>(JsonSerializer.Serialize(values),ArchiveJobWire.Strict);throw new InvalidOperationException("invalid missing count accepted"); }
            catch(JsonException) { }
        }
        var legacyValues=serialized.EnumerateObject().Where(p=>p.Name!="missing_destination_directories").ToDictionary(p=>p.Name,p=>p.Value);
        var legacy=JsonSerializer.Deserialize<ArchiveSpaceObservation>(JsonSerializer.Serialize(legacyValues),ArchiveJobWire.Strict)!;
        Check(legacy.MissingDestinationDirectories is null && legacy.ValidFor(command),"legacy omission remains unknown under v1");
        var log=new RecordingLog {Throw=true};
        Check(ArchiveSpacePreflight.Observe(command,log).Success,"throwing diagnostic sink cannot change success");
        Failure(ArchiveSpacePreflight.Observe(command with {Source=Path.Combine(fixture.Root.FullName,"private-password-absent.bin")},log),"source_unavailable",2);
        Failure(ArchiveSpacePreflight.Observe(command with {Generation=0},log),"invalid_request");
        Check(log.Events.Count(e=>e.Name=="archive.space.started")==3 && log.Events.Count(e=>e.Name is "archive.space.finished" or "archive.space.failed")==3,"one observed terminal including validation rejection");
        foreach(var entry in log.Events) {
            Check(entry.Correlation==command.OperationId && entry.Fields.ContainsKey("elapsed_ms") && entry.Fields.ContainsKey("session_id") && entry.Fields.ContainsKey("generation"),"diagnostic binding/elapsed fields");
            Check(entry.Fields.Keys.All(k=>k is "stage" or "elapsed_ms" or "native_error_code" or "session_id" or "generation"),"fixed privacy-safe diagnostic schema");
        }
        var diagnostic=JsonSerializer.Serialize(log.Events);
        Check(!diagnostic.Contains(fixture.Root.FullName,StringComparison.OrdinalIgnoreCase) && !diagnostic.Contains("private-password") && !command.ToString().Contains(fixture.Source),"no paths, source bytes or password-like names logged");
        var runLog=new RecordingLog {Throw=true};using var stopped=new CancellationTokenSource();stopped.Cancel();
        Failure(ArchiveSpacePreflight.Run(command,runLog,stopped.Token),"cancelled");
        Check(runLog.Events.Count==2 && runLog.Events[0].Name=="archive.space.accepted" && runLog.Events[1].Name=="archive.space.wait_ended","pre-cancelled bounded admission has truthful lifecycle even with throwing sink");
        Check(ArchiveSpacePreflight.Run(command,runLog,CancellationToken.None).Success,"bounded run preserves success with throwing sink");
        Check(!Directory.Exists(fixture.Destination),"wire/diagnostics never create output");
        return Task.CompletedTask;
    }
    public static async Task OriginalDispatcherRetainsBlockedSlots()
    {
        using var fixture=new Fixture();Directory.CreateDirectory(fixture.Destination);
        using var release=new ManualResetEventSlim();using var entered=new CountdownEvent(2);using var left=new CountdownEvent(2);
        using var stop=new CancellationTokenSource();var dispatcher=new PlatformCommandSequenceDispatcher();
        var published=new TaskCompletionSource<WindowsCommandResult>(TaskCreationOptions.RunContinuationsAsynchronously);
        var calls=0;var replacementCalls=0;var wrongApartment=0;
        ulong Block(ulong actual) {
            Interlocked.Increment(ref calls);
            if(Thread.CurrentThread.GetApartmentState()!=ApartmentState.STA)Interlocked.Increment(ref wrongApartment);
            entered.Signal();
            try { Check(release.Wait(TimeSpan.FromSeconds(20)),"bounded fixture release deadline");return actual; }
            finally { left.Signal(); }
        }
        Task<WindowsCommandResult>? second=null;
        try {
            await ArchiveSpacePreflight.DispatchAsync(dispatcher,fixture.Command,null,CancellationToken.None,
                (result,_)=>{published.TrySetResult(result);return Task.CompletedTask;},Block).WaitAsync(TimeSpan.FromSeconds(2));
            second=Task.Run(()=>ArchiveSpacePreflight.Run(fixture.Command,null,stop.Token,Block));
            Check(entered.Wait(TimeSpan.FromSeconds(3)),"two actual native held observations entered");
            Check(!published.Task.IsCompleted,"dispatch returns before blocking native observation");
            var later=false;
            await dispatcher.DispatchAsync("set_window_visibility",()=>{later=true;return Task.CompletedTask;}).WaitAsync(TimeSpan.FromSeconds(1));
            Check(later,"later serial command remains responsive while original observer blocks");
            var writeDenied=false;try { using var write=File.Open(fixture.Source,FileMode.Open,FileAccess.Write,FileShare.ReadWrite|FileShare.Delete); }
            catch(IOException) { writeDenied=true; }
            var renameDenied=false;try { Directory.Move(fixture.Destination,fixture.Destination+"-moved"); }
            catch(IOException) { renameDenied=true; }
            Check(writeDenied && renameDenied,"original source and nearest-target leases stay held while blocked");
            stop.Cancel();Failure(await second.WaitAsync(TimeSpan.FromSeconds(2)),"cancelled");
            Failure(ArchiveSpacePreflight.Run(fixture.Command,null,CancellationToken.None,actual=>{Interlocked.Increment(ref replacementCalls);return actual;}),"space_busy_or_timeout");
            Failure(await published.Task.WaitAsync(TimeSpan.FromSeconds(7)),"space_busy_or_timeout");
            Failure(ArchiveSpacePreflight.Run(fixture.Command,null,CancellationToken.None,actual=>{Interlocked.Increment(ref replacementCalls);return actual;}),"space_busy_or_timeout");
            Check(calls==2 && replacementCalls==0 && wrongApartment==0,"max2 original STA slots retained across cancellation and five-second timeout, no replacements");
        } finally {
            release.Set();stop.Cancel();
            if(second is not null)await second.WaitAsync(TimeSpan.FromSeconds(5));
            await dispatcher.DrainAsync(TimeSpan.FromSeconds(5));
            Check(left.Wait(TimeSpan.FromSeconds(5)),"blocked native calls actually released");
            Check(SpinWait.SpinUntil(()=>ArchiveSpacePreflight.Run(fixture.Command,null,CancellationToken.None).Success,TimeSpan.FromSeconds(5)),"slots return only after real native cleanup");
        }
        Check(!File.Exists(Path.Combine(fixture.Destination,"anything")) && File.ReadAllBytes(fixture.Source).SequenceEqual(new byte[]{1,2,3,4}),"blocked/cancelled observations never publish output or mutate source");
    }
    public static async Task DetachedCancellationHasNoLateReceipt()
    {
        using var fixture=new Fixture();using var stop=new CancellationTokenSource();
        using var release=new ManualResetEventSlim();using var entered=new ManualResetEventSlim();using var left=new ManualResetEventSlim();
        var dispatcher=new PlatformCommandSequenceDispatcher();var published=0;
        try {
            await ArchiveSpacePreflight.DispatchAsync(dispatcher,fixture.Command,null,stop.Token,
                (_,_)=>{Interlocked.Increment(ref published);return Task.CompletedTask;},actual=> {
                    entered.Set();try { Check(release.Wait(TimeSpan.FromSeconds(10)),"cancelled dispatch release deadline");return actual; }
                    finally { left.Set(); }
                }).WaitAsync(TimeSpan.FromSeconds(2));
            Check(entered.Wait(TimeSpan.FromSeconds(2)),"cancelled dispatch reached held native observation");
            stop.Cancel();await dispatcher.DrainAsync(TimeSpan.FromSeconds(2));
            Check(published==0,"cancelled pipe generation has no receipt while native call remains held");
        } finally {
            stop.Cancel();release.Set();await dispatcher.DrainAsync(TimeSpan.FromSeconds(5));
            Check(left.Wait(TimeSpan.FromSeconds(5)),"cancelled dispatch released actual observation");
        }
        Check(SpinWait.SpinUntil(()=>ArchiveSpacePreflight.Run(fixture.Command,null,CancellationToken.None).Success,TimeSpan.FromSeconds(5)),"cancelled dispatch actual slot cleanup");
        Check(published==0 && !Directory.Exists(fixture.Destination),"no late receipt/output after cancelled native work completes");
    }
    private sealed class RecordingLog : IPlatformLog
    {
        internal sealed record Entry(string Name,Guid? Correlation,IReadOnlyDictionary<string,object?> Fields);
        internal readonly List<Entry> Events=[];
        internal bool Throw;
        public void Write(string level,string eventName,Guid? correlationId=null,IReadOnlyDictionary<string,object?>? fields=null)
        {
            lock(Events)Events.Add(new(eventName,correlationId,new Dictionary<string,object?>(fields!)));
            if(Throw)throw new IOException("private diagnostic sink failure");
        }
    }
    [DllImport("kernel32.dll",EntryPoint="CreateFileW",SetLastError=true,CharSet=CharSet.Unicode)]
    private static extern SafeFileHandle ReadDirectory(string path,uint access,uint share,IntPtr attributes,uint disposition,uint flags,IntPtr template);
    [StructLayout(LayoutKind.Sequential)] private struct NativeId { internal ulong Volume,Low,High; }
    [DllImport("kernel32.dll",EntryPoint="GetFileInformationByHandleEx",SetLastError=true)]
    [return:MarshalAs(UnmanagedType.Bool)] private static extern bool FullId(SafeFileHandle file,int kind,out NativeId id,uint size);
    [DllImport("kernel32.dll",EntryPoint="GetFinalPathNameByHandleW",SetLastError=true,CharSet=CharSet.Unicode)]
    private static extern uint FinalPath(SafeFileHandle file,StringBuilder name,uint size,uint flags);
    [DllImport("kernel32.dll",EntryPoint="GetDiskFreeSpaceW",SetLastError=true,CharSet=CharSet.Unicode)]
    [return:MarshalAs(UnmanagedType.Bool)] private static extern bool DiskAllocation(string root,out uint sectors,out uint bytes,out uint free,out uint total);
    [DllImport("kernel32.dll",EntryPoint="GetDiskFreeSpaceExW",SetLastError=true,CharSet=CharSet.Unicode)]
    [return:MarshalAs(UnmanagedType.Bool)] private static extern bool DiskCapacity(string root,out ulong available,out ulong total,out ulong free);
}
