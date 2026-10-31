using System.Buffers.Binary;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
using NxTools.PlatformHost.Windows;
using RecoveryDescriptor = NxTools.PlatformHost.Protocol.ArchiveRecoveryInspection;
using ArchiveRecoveryInspection = NxTools.PlatformHost.Windows.Archive.ArchiveRecoveryInspection;

namespace NxTools.PlatformHost.Tests;

// Synthetic GUID-owned records only; no source mutation during inspection or recovery execution.
internal static class ArchiveRecoveryInspectionTests
{
    private static void Check(bool ok, string reason) { if (!ok) throw new InvalidOperationException(reason); }
    private sealed class Fixture : IDisposable
    {
        internal readonly DirectoryInfo Root = Directory.CreateDirectory(Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("Fixture requires PI_SCRATCH_DIR"), "hz5-inspect-" + Guid.NewGuid().ToString("N")));
        internal string Source => Path.Combine(Root.FullName, "sample.zip");
        internal string Target => Path.Combine(Root.FullName, "original");
        internal string Other => Path.Combine(Root.FullName, "other");
        internal readonly Guid Recovery = Guid.NewGuid();
        internal string Journal => Path.Combine(Root.FullName, ".nxtools-hz5-" + Recovery.ToString("N") + ".journal");
        internal ArchiveRecoveryInspectCommand Command => new(1, Guid.NewGuid(), Guid.NewGuid(), 9, Source, Target);
        internal Fixture() { File.WriteAllBytes(Source, new byte[65536]); Directory.CreateDirectory(Target); Directory.CreateDirectory(Other); }
        internal void Records(bool completed = false, bool torn = false, bool rootIsParent = false, string planPath = "x")
        {
            // Repeated fixture setup deletes only its own previously written marker and companion.
            File.Delete(Source + ArchiveReclamationNative.MarkerName);
            File.WriteAllBytes(Journal, []);
            string sourceId, parentId, rootId, journalId;
            using (var source = ArchiveSourceLease.Open(Source)) {
                sourceId = source.InspectionSourceIdentity;
                parentId = source.InspectionParentIdentity;
                using var root = ArchiveRecoveryInspection.OpenDirectory(Target);
                using var journal = ArchiveRecoveryInspection.OpenCompanion(source.InspectionParent, Recovery);
                rootId = rootIsParent ? parentId : ArchiveReclamationNative.Identity(root.Handle);
                journalId = ArchiveReclamationNative.Identity(journal);
            }
            var header = new { mode="stored_zip_whole_file_sparse", recovery_id=Recovery, logical_id=Recovery,
                source_identity=sourceId, root_identity=rootId, parent_identity=parentId,
                companion_identity=journalId, original_modified=0L, original_attributes=0u, copy_motw=true,
                source_zone_hash=(string?)null, cluster_bytes=65536UL };
            var frames = new List<byte[]> { Frame(1,"header",header),
                Frame(2,"plan_entry",new { index=0, ordinal=0UL,path=planPath,header_start=0UL,payload_start=0UL,payload_end=65536UL,record_end=65536UL,release_start=0UL,release_end=65536UL,size=65536UL,crc32=0u,metadata=new { modified=(object?)null,attributes=(object?)null } }),
                Frame(3,"sealed",new { count=1,structure=Convert.ToBase64String([1]) }) };
            if (completed) {
                frames.Add(Frame(4,"partial",new {index=0,path="x",partial_name=".nxpart-test",parent_identity=parentId,identity="v1:part"}));
                frames.Add(Frame(5,"ready",new {index=0,identity="v1:part",size=65536UL,modified=0L,fingerprint="AB",attributes=0u,zone_identifier_hash=(string?)null}));
                frames.Add(Frame(6,"pending",new {index=0,interval_allocated_before=65536UL,source_allocated_before=65536UL,free_before=0UL}));
                frames.Add(Frame(7,"applied",new {index=0,interval_allocated_after=0UL,source_allocated_after=0UL,released_bytes=65536UL,free_after=0UL}));
                frames.Add(Frame(8,"completed_consumed",new { count=1 }));
            }
            using (var output = new FileStream(Journal,FileMode.Open,FileAccess.Write,FileShare.ReadWrite)) {
                foreach (var frame in frames) output.Write(frame);
                if(torn) output.Write([1,2,3]);
            }
            File.WriteAllBytes(Source + ArchiveReclamationNative.MarkerName,
                JsonSerializer.SerializeToUtf8Bytes(new { schema=1,recovery_id=Recovery,companion_identity=journalId,source_identity=sourceId }));
        }
        internal static byte[] Frame(ulong sequence,string kind,object value)
        {
            var payload=JsonSerializer.SerializeToUtf8Bytes(new {schema=1,sequence,kind,value});
            var frame=new byte[payload.Length+8]; BinaryPrimitives.WriteInt32LittleEndian(frame,payload.Length);
            payload.CopyTo(frame,4); BinaryPrimitives.WriteInt32LittleEndian(frame.AsSpan(payload.Length+4),payload.Length);return frame;
        }
        public void Dispose() => Root.Delete(true);
    }
    private static RecoveryDescriptor Data(WindowsCommandResult result)
    {
        Check(result.Success,"inspection failed: "+result.Error);
        return JsonSerializer.Deserialize<RecoveryDescriptor>(result.Data!,JsonDefaults.Options)!;
    }
    public static Task ReadonlyDescriptorAndTarget()
    {
        using var f=new Fixture();
        Check(ArchiveRecoveryInspection.Observe(f.Command).Error=="recovery_not_found","ordinary source not_found");
        f.Records();
        var sourceBefore=File.ReadAllBytes(f.Source);var markerBefore=File.ReadAllBytes(f.Source+ArchiveReclamationNative.MarkerName);
        var journalBefore=File.ReadAllBytes(f.Journal);var mtime=File.GetLastWriteTimeUtc(f.Source);
        using var held=ArchiveSourceLease.OpenForRecoveryInspection(f.Source);
        var sourceId=held.InspectionSourceIdentity;
        using var journal=ArchiveRecoveryInspection.OpenCompanion(held.InspectionParent,f.Recovery);
        var journalId=ArchiveReclamationNative.Identity(journal);
        var journalMtime=File.GetLastWriteTimeUtc(f.Journal);
        var descriptor=Data(ArchiveRecoveryInspection.Observe(f.Command));
        Check(descriptor.RecoveryId==f.Recovery && descriptor.SourceIdentity==sourceId && descriptor.Destination==f.Target && descriptor.CopyMotw && !descriptor.Completed,"exact uncompleted descriptor");
        Check(Data(ArchiveRecoveryInspection.Observe(f.Command with {Destination=f.Other})).Destination is null,"wrong original target unresolved");
        Check(Data(ArchiveRecoveryInspection.Observe(f.Command with {Destination=null})).Destination is null,"parent not original root");
        Check(File.ReadAllBytes(f.Source).SequenceEqual(sourceBefore)&&File.ReadAllBytes(f.Source+ArchiveReclamationNative.MarkerName).SequenceEqual(markerBefore)&&File.ReadAllBytes(f.Journal).SequenceEqual(journalBefore)&&File.GetLastWriteTimeUtc(f.Source)==mtime&&File.GetLastWriteTimeUtc(f.Journal)==journalMtime&&held.InspectionSourceIdentity==sourceId&&ArchiveReclamationNative.Identity(journal)==journalId,"inspection preserves source/marker/journal bytes, mtime and native identities");
        Directory.Delete(f.Other);
        Check(Data(ArchiveRecoveryInspection.Observe(f.Command with {Destination=f.Other})).Destination is null,"missing original target unresolved");
        File.WriteAllBytes(f.Other,[1]);
        Check(Data(ArchiveRecoveryInspection.Observe(f.Command with {Destination=f.Other})).Destination is null,"wrong-kind original target unresolved");
        try { using var ordinary=ArchiveSourceLease.Open(f.Source); throw new InvalidOperationException("ordinary lease admitted marker"); }
        catch(ArchiveOutputException e) { Check(e.Stage=="reclamation_recovery_required","ordinary lease still refuses marker"); }
        return Task.CompletedTask;
    }
    public static Task CompletedTornAndInvalidRecords()
    {
        using var f=new Fixture();f.Records(completed:true);
        Check(Data(ArchiveRecoveryInspection.Observe(f.Command)).Completed,"fully framed completion");
        using(var stream=new FileStream(f.Journal,FileMode.Append,FileAccess.Write))stream.Write([1,2,3]);
        Check(!Data(ArchiveRecoveryInspection.Observe(f.Command)).Completed,"torn final append never certifies completion");
        f.Records(torn:true);
        Check(!Data(ArchiveRecoveryInspection.Observe(f.Command)).Completed,"torn uncompleted descriptor survives");
        f.Records();using(var append=new FileStream(f.Journal,FileMode.Append,FileAccess.Write))append.Write(Fixture.Frame(4,"unrecognized",new { }));
        Check(ArchiveRecoveryInspection.Observe(f.Command).Error=="recovery_corrupt","unknown complete kind refuses");
        f.Records();File.WriteAllBytes(f.Journal,[1,0,0,0,0,0,0,0]);
        Check(ArchiveRecoveryInspection.Observe(f.Command).Error=="recovery_corrupt","complete corrupt frame refuses");
        f.Records();using(var append=new FileStream(f.Journal,FileMode.Append,FileAccess.Write))append.Write(Fixture.Frame(5,"completed_consumed",new { }));
        Check(ArchiveRecoveryInspection.Observe(f.Command).Error=="recovery_corrupt","noncontiguous sequence refuses");
        f.Records();using(var append=new FileStream(f.Journal,FileMode.Append,FileAccess.Write)) {var size=new byte[4];BinaryPrimitives.WriteInt32LittleEndian(size,65537);append.Write(size);append.Write(new byte[65541]);}
        Check(ArchiveRecoveryInspection.Observe(f.Command).Error=="recovery_corrupt","oversized framed record refuses");
        foreach(var invalidPath in new[] { "bad:name", @"a\b" }) {
            f.Records(planPath:invalidPath);
            Check(ArchiveRecoveryInspection.Observe(f.Command).Error=="recovery_corrupt","executor-invalid plan component refuses during readonly discovery");
        }
        return Task.CompletedTask;
    }
    public static Task ReplacedAuthorityAndStrictWire()
    {
        using var f=new Fixture();f.Records();var command=f.Command;
        Check(command.Valid && Data(ArchiveRecoveryInspection.Observe(command)).ValidFor(command),"strict matching response");
        Check(!(command with {Generation=0}).Valid && !(command with {OperationId=Guid.Empty}).Valid &&
            !(command with {Destination=@"C:\bad\..\target"}).Valid,"invalid request rejected");
        var wire=ProtocolEnvelope.Create("platform_command",new {command="archive_recovery_inspect",data=command});
        Check(ArchiveRecoveryInspectCommand.Parse(wire)==command,"strict command wire roundtrip");
        var descriptor=Data(ArchiveRecoveryInspection.Observe(command));
        Check(!(descriptor with {Generation=0}).ValidFor(command) && !(descriptor with {Destination=f.Other}).ValidFor(command),"stale or unobserved target refused");
        var missingBool=JsonSerializer.SerializeToElement(descriptor,JsonDefaults.Options).EnumerateObject().Where(p=>p.Name!="copy_motw").ToDictionary(p=>p.Name,p=>p.Value);
        try { JsonSerializer.Deserialize<RecoveryDescriptor>(JsonSerializer.Serialize(missingBool),ArchiveJobWire.Strict);throw new InvalidOperationException("missing typed bool admitted"); }
        catch(JsonException) { }
        var invalidBool=JsonSerializer.SerializeToElement(descriptor,JsonDefaults.Options).EnumerateObject().ToDictionary(p=>p.Name,p=>(object)p.Value);
        invalidBool["completed"]="false";
        try { JsonSerializer.Deserialize<RecoveryDescriptor>(JsonSerializer.Serialize(invalidBool),ArchiveJobWire.Strict);throw new InvalidOperationException("string bool admitted"); }
        catch(JsonException) { }
        f.Records(rootIsParent:true);
        Check(Data(ArchiveRecoveryInspection.Observe(command with {Destination=null})).Destination==f.Root.FullName,"original source parent returned only on matching held root identity");
        Check(Data(ArchiveRecoveryInspection.Observe(command with {Destination=f.Other})).Destination is null,"explicit wrong candidate never replaced by matching parent");
        File.WriteAllBytes(f.Source+ArchiveReclamationNative.MarkerName,JsonSerializer.SerializeToUtf8Bytes(new {schema=1,recovery_id=f.Recovery,companion_identity="v1:wrong",source_identity="v1:wrong:0"}));
        Check(ArchiveRecoveryInspection.Observe(command).Error=="recovery_mismatch","replaced marker source authority refuses");
        f.Records(rootIsParent:true);
        File.Move(f.Journal,f.Journal+".old");
        Check(ArchiveRecoveryInspection.Observe(command).Error=="recovery_missing_companion","missing companion distinct");
        File.Move(f.Journal+".old",f.Journal);File.Delete(f.Journal);File.WriteAllBytes(f.Journal,[1,2,3]);
        Check(ArchiveRecoveryInspection.Observe(command).Error=="recovery_mismatch","replaced native authority refused");
        return Task.CompletedTask;
    }
    private sealed class RecordingLog : NxTools.PlatformHost.Diagnostics.IPlatformLog
    {
        internal readonly List<(string Name,Guid? Correlation,IReadOnlyDictionary<string,object?> Fields)> Events=[];
        public void Write(string level,string eventName,Guid? correlationId=null,IReadOnlyDictionary<string,object?>? fields=null)
        {
            lock(Events)Events.Add((eventName,correlationId,new Dictionary<string,object?>(fields!)));
            throw new IOException("synthetic sink failure");
        }
    }
    public static async Task BoundedDetachedAndPrivateDiagnostics()
    {
        using var f=new Fixture();f.Records();var command=f.Command;
        using var entered=new CountdownEvent(2);using var release=new ManualResetEventSlim();using var stop=new CancellationTokenSource();
        var log=new RecordingLog();int calls=0;
        WindowsCommandResult Block(CancellationToken token) {
            Interlocked.Increment(ref calls);entered.Signal();
            Check(release.Wait(TimeSpan.FromSeconds(10)),"blocked observer released");
            return ArchiveRecoveryInspection.Observe(command,token);
        }
        Task<WindowsCommandResult>? first=null,second=null;
        try {
            first=Task.Run(()=>ArchiveRecoveryInspection.Run(command,log,stop.Token,Block));
            second=Task.Run(()=>ArchiveRecoveryInspection.Run(command,log,CancellationToken.None,Block));
            Check(entered.Wait(TimeSpan.FromSeconds(3)),"two bounded native slots entered");
            var busy=ArchiveRecoveryInspection.Run(command,log,CancellationToken.None,_=>{Interlocked.Increment(ref calls);return new(true);});
            Check(!busy.Success && busy.Error=="recovery_busy" && calls==2,"third request rejected without entering native work");
            stop.Cancel();Check((await first.WaitAsync(TimeSpan.FromSeconds(2))).Error=="cancelled","logical cancel does not release native slot");
            Check(ArchiveRecoveryInspection.Run(command,log,CancellationToken.None).Error=="recovery_busy","cancelled native slot held until return");
            Check(!File.Exists(Path.Combine(f.Target,"x")),"bounded inspection creates no output");
        } finally { release.Set();stop.Cancel();if(first is not null)await first.WaitAsync(TimeSpan.FromSeconds(5));if(second is not null)await second.WaitAsync(TimeSpan.FromSeconds(5)); }
        Check(ArchiveRecoveryInspection.Run(command,log,CancellationToken.None).Success,"native slot returned after actual cleanup");
        Check(log.Events.Count(e=>e.Name=="archive.recovery.inspect.accepted")==5 && log.Events.Count(e=>e.Name=="archive.recovery.inspect.finished")==5,"one correlated terminal per admission");
        foreach(var entry in log.Events)Check(entry.Correlation==command.OperationId && entry.Fields.Keys.All(k=>k is "outcome" or "elapsed_ms" or "native_error_code"),"private fixed diagnostic fields");
        Check(!JsonSerializer.Serialize(log.Events).Contains(f.Root.FullName,StringComparison.OrdinalIgnoreCase),"diagnostics omit all paths");
    }
}
