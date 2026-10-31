using System.Buffers.Binary;
using System.Diagnostics;
using System.Text.Json;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using RecoveryDescriptor = NxTools.PlatformHost.Protocol.ArchiveRecoveryInspection;

namespace NxTools.PlatformHost.Windows.Archive;

// Record discovery only: no writable handles, output qualification, payload scan or replay.
internal static class ArchiveRecoveryInspection
{
    internal static Task DispatchAsync(PlatformCommandSequenceDispatcher dispatcher,ArchiveRecoveryInspectCommand command,
        IPlatformLog? log,CancellationToken cancellation,Func<WindowsCommandResult,CancellationToken,Task> publish) =>
        dispatcher.DispatchAsync("archive_entry",async token=> {
            var result=Run(command,log,token);
            if(!token.IsCancellationRequested) await publish(result,token);
        },cancellation);

    internal static WindowsCommandResult Run(ArchiveRecoveryInspectCommand command,IPlatformLog? log,CancellationToken cancellation,
        Func<CancellationToken,WindowsCommandResult>? inspectFixture=null)
    {
        var clock=Stopwatch.StartNew();Diagnostic(log,command,"accepted","accepted",clock.ElapsedMilliseconds,null);
        if(!command.Valid) { Diagnostic(log,command,"finished","invalid_request",clock.ElapsedMilliseconds,null);return new(false,"invalid_request"); }
        if(cancellation.IsCancellationRequested) { Diagnostic(log,command,"finished","cancelled",clock.ElapsedMilliseconds,null);return new(false,"cancelled"); }
        if(BoundedStaOperationRunner.TryRun("NxTools.ArchiveRecoveryInspect",TimeSpan.FromSeconds(5),2,
            token=>inspectFixture?.Invoke(token)??Observe(command,token),out WindowsCommandResult? result,out var runnerReason,cancellation)) {
            var terminal=result??new WindowsCommandResult(false,"recovery_unavailable");
            Diagnostic(log,command,"finished",terminal.Success?"found":terminal.Error??"recovery_unavailable",clock.ElapsedMilliseconds,terminal.NativeErrorCode);
            return terminal;
        }
        var code=cancellation.IsCancellationRequested?"cancelled":runnerReason?.Contains("timed out",StringComparison.Ordinal)==true?"recovery_timeout":
            runnerReason?.Contains("still finishing",StringComparison.Ordinal)==true?"recovery_busy":"recovery_unavailable";
        Diagnostic(log,command,"finished",code,clock.ElapsedMilliseconds,null);return new(false,code);
    }
    internal static WindowsCommandResult Observe(ArchiveRecoveryInspectCommand command,CancellationToken cancellation=default)
    {
        if(!command.Valid)return new(false,"invalid_request");
        string stage="source";
        try {
            cancellation.ThrowIfCancellationRequested();
            using var source=ArchiveSourceLease.OpenForRecoveryInspection(command.Source);
            var sourceIdentity=source.InspectionSourceIdentity;
            stage="marker";
            using var marker=ArchiveReclamationNative.OpenMarker(source.InspectionSource,false);
            var markerLength=RandomAccess.GetLength(marker);
            if(markerLength is <1 or >1024)throw new RecordFailure("recovery_corrupt");
            var binding=Parse(ArchiveReclamationNative.ReadExact(marker,0,(int)markerLength));
            RequireFields(binding,"schema","recovery_id","companion_identity","source_identity");
            if(binding.GetProperty("schema").GetInt32()!=1 || binding.GetProperty("source_identity").GetString()!=sourceIdentity)
                throw new RecordFailure("recovery_mismatch");
            var recovery=binding.GetProperty("recovery_id").GetGuid();
            if(recovery==Guid.Empty)throw new RecordFailure("recovery_corrupt");
            stage="companion";
            using var journal=OpenCompanion(source.InspectionParent,recovery);
            var companion=ArchiveReclamationNative.Identity(journal);
            if(binding.GetProperty("companion_identity").GetString()!=companion)throw new RecordFailure("recovery_mismatch");
            stage="journal";
            var inspected=ReadRecords(journal,recovery,sourceIdentity,source.InspectionParentIdentity,companion,cancellation);
            string? target=null;
            if(command.Destination is { } candidate) {
                stage="destination";
                try { using var root=OpenDirectory(candidate);if(ArchiveReclamationNative.Identity(root.Handle)==inspected.RootIdentity)target=candidate; }
                catch(ArchiveOutputException e) when(e.NativeCode is 2 or 3 or 267 || e.Stage is "wrong_kind" or "reparse_component") { }
            } else if(source.InspectionParentIdentity==inspected.RootIdentity) {
                target=Path.GetDirectoryName(command.Source);
            }
            cancellation.ThrowIfCancellationRequested();
            if(source.InspectionSourceIdentity!=sourceIdentity || source.InspectionParentIdentity!=inspected.ParentIdentity ||
               ArchiveReclamationNative.Identity(journal)!=companion || RandomAccess.GetLength(journal)!=inspected.Length)
                throw new RecordFailure("recovery_mismatch");
            var descriptor=new RecoveryDescriptor(1,command.OperationId,command.SessionId,command.Generation,
                recovery,sourceIdentity,target,inspected.CopyMotw,inspected.Completed);
            if(!descriptor.ValidFor(command))throw new RecordFailure("recovery_corrupt");
            return new(true,Data:JsonSerializer.Serialize(descriptor,JsonDefaults.Options));
        }
        catch(OperationCanceledException) { return new(false,"cancelled"); }
        catch(RecordFailure error) { return new(false,error.Code); }
        catch(ArchiveOutputException error) {
            var native=error.NativeCode;
            var code=stage switch {
                "marker" when native is 2 or 3 or 123 or 1 or 50 => "recovery_not_found",
                "companion" when native is 2 or 3 => "recovery_missing_companion",
                "source" when native is 2 or 3 => "recovery_source_unavailable",
                _ when native is 32 or 33 => "recovery_busy",
                _ when native==5 => "recovery_denied",
                _ when error.Stage is "invalid_source" or "invalid_component" or "reparse_component" or "wrong_kind" => "recovery_path_invalid",
                _ => "recovery_corrupt" };
            return new(false,code,NativeErrorCode:native);
        }
        catch(UnauthorizedAccessException error) { return new(false,"recovery_denied",NativeErrorCode:error.HResult&0xffff); }
        catch(IOException error) { var native=error.HResult&0xffff;return new(false,native is 32 or 33?"recovery_busy":native==5?"recovery_denied":"recovery_path_invalid",NativeErrorCode:native); }
        catch(JsonException) { return new(false,"recovery_corrupt"); }
        catch(KeyNotFoundException) { return new(false,"recovery_corrupt"); }
        catch(InvalidOperationException) { return new(false,"recovery_corrupt"); }
        catch(FormatException) { return new(false,"recovery_corrupt"); }
        catch(OverflowException) { return new(false,"recovery_corrupt"); }
    }
    internal static SafeFileHandle OpenCompanion(SafeFileHandle parent,Guid id) =>
        OutputNative.Open(parent,".nxtools-hz5-"+id.ToString("N")+".journal",directory:false,readOnly:true);
    internal sealed class HeldDirectory : IDisposable
    {
        private readonly List<SafeFileHandle> handles=[];
        internal SafeFileHandle Handle=>handles[^1];
        internal void Add(SafeFileHandle handle)=>handles.Add(handle);
        public void Dispose() { for(var i=handles.Count-1;i>=0;i--)handles[i].Dispose();handles.Clear(); }
    }
    internal static HeldDirectory OpenDirectory(string path)
    {
        if(!ArchiveJobWire.PathValid(path)||!ArchiveRecoveryPath.Valid(path))throw new ArchiveOutputException("invalid_component");
        var held=new HeldDirectory();
        try {
            held.Add(OutputNative.Open(null,"\\??\\"+char.ToUpperInvariant(path[0])+":\\",directory:true,readOnly:true));
            foreach(var part in path[3..].Split('\\')) held.Add(OutputNative.Open(held.Handle,part,directory:true,readOnly:true));
            return held;
        } catch { held.Dispose();throw; }
    }
    private sealed class RecordFailure(string code):Exception { internal string Code=>code; }
    private static void Reject(string code="recovery_corrupt")=>throw new RecordFailure(code);
    private static JsonElement Parse(ReadOnlyMemory<byte> bytes)
    {
        using var doc=JsonDocument.Parse(bytes,new JsonDocumentOptions{MaxDepth=32});
        void Unique(JsonElement e) {
            if(e.ValueKind==JsonValueKind.Object) { var names=new HashSet<string>();foreach(var p in e.EnumerateObject()) {if(!names.Add(p.Name))Reject();Unique(p.Value);} }
            else if(e.ValueKind==JsonValueKind.Array)foreach(var item in e.EnumerateArray())Unique(item);
        }
        Unique(doc.RootElement);return doc.RootElement.Clone();
    }
    private static void RequireFields(JsonElement element,params string[] fields) {
        if(element.ValueKind!=JsonValueKind.Object || element.EnumerateObject().Count()!=fields.Length || fields.Any(f=>!element.TryGetProperty(f,out _)))Reject();
    }
    private static void RequirePresent(JsonElement element,params string[] fields) {
        if(element.ValueKind!=JsonValueKind.Object || fields.Any(f=>!element.TryGetProperty(f,out _)))Reject();
    }
    private sealed record RecordFacts(string RootIdentity,string ParentIdentity,bool CopyMotw,bool Completed,long Length);
    private static RecordFacts ReadRecords(SafeFileHandle journal,Guid recovery,string source,string parent,string companion,CancellationToken stop)
    {
        var length=RandomAccess.GetLength(journal);
        if(length is <8 or >32<<20)Reject();
        long offset=0;ulong sequence=0;string? root=null;bool copy=false,completed=false,sealedPlan=false,torn=false;
        var plans=new List<JsonElement>();var frontier=0;var phase=0;ulong quantum=0;
        var sourceLength=Convert.ToUInt64(source[(source.LastIndexOf(':')+1)..],16);
        string? partialIdentity=null;ulong pendingBefore=0,intervalBefore=0;
        while(offset<length) {
            stop.ThrowIfCancellationRequested();
            if(length-offset<8) { torn=true;break; }
            var size=BinaryPrimitives.ReadInt32LittleEndian(ArchiveReclamationNative.ReadExact(journal,offset,4));
            if(size is <1 or >65536)Reject();
            if(size+8>length-offset) { torn=true;break; }
            var frame=ArchiveReclamationNative.ReadExact(journal,offset+4,size+4);
            if(BinaryPrimitives.ReadInt32LittleEndian(frame.AsSpan(size))!=size)Reject();
            var envelope=Parse(frame.AsMemory(0,size));RequireFields(envelope,"schema","sequence","kind","value");
            if(envelope.GetProperty("schema").GetInt32()!=1 || envelope.GetProperty("sequence").GetUInt64()!=++sequence)Reject();
            var kind=envelope.GetProperty("kind").GetString();var value=envelope.GetProperty("value");
            if(value.ValueKind!=JsonValueKind.Object || completed)Reject();
            if(sequence==1) {
                RequireFields(value,"mode","recovery_id","logical_id","source_identity","root_identity","parent_identity","companion_identity","original_modified","original_attributes","copy_motw","source_zone_hash","cluster_bytes");
                if(kind!="header" || value.GetProperty("mode").GetString()!="stored_zip_whole_file_sparse" ||
                    value.GetProperty("recovery_id").GetGuid()!=recovery || value.GetProperty("logical_id").GetGuid()!=recovery ||
                    value.GetProperty("source_identity").GetString()!=source || value.GetProperty("parent_identity").GetString()!=parent ||
                    value.GetProperty("companion_identity").GetString()!=companion)Reject("recovery_mismatch");
                root=value.GetProperty("root_identity").GetString();if(root is null || root.Length>256 || !root.StartsWith("v1:",StringComparison.Ordinal))Reject();
                copy=value.GetProperty("copy_motw").GetBoolean();
                quantum=value.GetProperty("cluster_bytes").GetUInt64();if(quantum<65536)Reject();
                _=value.GetProperty("original_modified").GetInt64();_=value.GetProperty("original_attributes").GetUInt32();
                _=value.GetProperty("source_zone_hash");
            } else if(kind=="plan_entry" && !sealedPlan) {
                RequireFields(value,"index","ordinal","path","header_start","payload_start","payload_end","record_end","release_start","release_end","size","crc32","metadata");
                var path=value.GetProperty("path").GetString()??throw new RecordFailure("recovery_corrupt");
                if(plans.Count>=4096 || value.GetProperty("index").GetInt32()!=plans.Count ||
                   path.Length>32767 || path.Split('/').Length>64)Reject();
                foreach(var component in path.Split('/')) {
                    try { ArchiveOutputSession.Validate(component); }
                    catch(ArchiveOutputException) { Reject(); }
                }
                _=ArchiveOutputMetadata.Parse(value.GetProperty("metadata"));
                var hs=value.GetProperty("header_start").GetUInt64();var ps=value.GetProperty("payload_start").GetUInt64();
                var pe=value.GetProperty("payload_end").GetUInt64();var re=value.GetProperty("record_end").GetUInt64();
                var rs=value.GetProperty("release_start").GetUInt64();var end=value.GetProperty("release_end").GetUInt64();
                var payloadSize=value.GetProperty("size").GetUInt64();_=value.GetProperty("crc32").GetUInt32();
                _=value.GetProperty("ordinal").GetUInt64();
                if(hs>ps || ps>pe || pe>re || re>sourceLength || payloadSize!=pe-ps || rs>end || end>sourceLength ||
                   rs%quantum!=0 || end%quantum!=0 || rs!=end && (rs<ps || end>pe) ||
                   plans.Any(p=>p.GetProperty("path").GetString()!.Equals(path,StringComparison.OrdinalIgnoreCase) ||
                       hs<p.GetProperty("record_end").GetUInt64() && p.GetProperty("header_start").GetUInt64()<re ||
                       p.GetProperty("ordinal").GetUInt64()==value.GetProperty("ordinal").GetUInt64()))Reject();
                plans.Add(value);
            } else if(kind=="sealed" && !sealedPlan) {
                RequireFields(value,"count","structure");
                if(plans.Count==0 || value.GetProperty("count").GetInt32()!=plans.Count ||
                   !plans.Any(p=>p.GetProperty("release_end").GetUInt64()>p.GetProperty("release_start").GetUInt64()))Reject();
                var structure=Convert.FromBase64String(value.GetProperty("structure").GetString()!);
                if(structure.Length is <1 or >49152)Reject();sealedPlan=true;
            } else if(!sealedPlan)Reject();
            else if(kind=="completed_consumed") { RequireFields(value,"count");if(frontier!=plans.Count || phase!=0 || value.GetProperty("count").GetInt32()!=plans.Count)Reject();completed=true; }
            else {
                var index=value.GetProperty("index").GetInt32();if(index!=frontier || index>=plans.Count)Reject();
                switch(kind) {
                    case "partial" when phase==0:
                        RequirePresent(value,"index","path","partial_name","parent_identity","identity");
                        var partialName=value.GetProperty("partial_name").GetString();
                        partialIdentity=value.GetProperty("identity").GetString();
                        if(value.GetProperty("path").GetString()!=plans[index].GetProperty("path").GetString() ||
                           partialName is null || !partialName.StartsWith(".nxpart-",StringComparison.Ordinal) ||
                           partialIdentity is null || !partialIdentity.StartsWith("v1:",StringComparison.Ordinal) ||
                           value.GetProperty("parent_identity").GetString() is not {Length:>3})Reject();
                        phase=1;break;
                    case "ready" when phase==1:
                        RequirePresent(value,"index","identity","size","modified","fingerprint","attributes","zone_identifier_hash");
                        if(value.GetProperty("identity").GetString()!=partialIdentity ||
                           value.GetProperty("size").GetUInt64()!=plans[index].GetProperty("size").GetUInt64() ||
                           value.GetProperty("fingerprint").GetString() is null)Reject();
                        _=value.GetProperty("modified").GetInt64();_=value.GetProperty("attributes").GetUInt32();
                        var zone=value.GetProperty("zone_identifier_hash");if(zone.ValueKind!=JsonValueKind.Null)_=zone.GetString()??throw new RecordFailure("recovery_corrupt");
                        phase=2;break;
                    case "pending" when phase==2:
                        RequirePresent(value,"index","interval_allocated_before","source_allocated_before","free_before");
                        intervalBefore=value.GetProperty("interval_allocated_before").GetUInt64();
                        pendingBefore=value.GetProperty("source_allocated_before").GetUInt64();
                        _=value.GetProperty("free_before").GetUInt64();
                        if(intervalBefore>plans[index].GetProperty("release_end").GetUInt64()-plans[index].GetProperty("release_start").GetUInt64())Reject();
                        phase=3;break;
                    case "applied" when phase==3:
                        RequirePresent(value,"index","released_bytes","interval_allocated_after","source_allocated_after","free_after");
                        var after=value.GetProperty("source_allocated_after").GetUInt64();
                        _=value.GetProperty("free_after").GetUInt64();
                        if(value.GetProperty("interval_allocated_after").GetUInt64()!=0 || after>pendingBefore ||
                           value.GetProperty("released_bytes").GetUInt64()!=pendingBefore-after || pendingBefore-after>intervalBefore)Reject();
                        frontier++;phase=0;break;
                    default:Reject();break;
                }
            }
            offset+=size+8;
        }
        if(!sealedPlan || root is null)Reject();
        return new(root!,parent,copy,completed&&!torn,length);
    }
    private static void Diagnostic(IPlatformLog? log,ArchiveRecoveryInspectCommand command,string eventName,string outcome,long elapsed,int? native)
    {
        try {log?.Write("Information","archive.recovery.inspect."+eventName,command.OperationId,new Dictionary<string,object?> {
            ["outcome"]=outcome,["elapsed_ms"]=elapsed,["native_error_code"]=native });}
        catch { /* Observational sink failure does not alter the result. */ }
    }
}
