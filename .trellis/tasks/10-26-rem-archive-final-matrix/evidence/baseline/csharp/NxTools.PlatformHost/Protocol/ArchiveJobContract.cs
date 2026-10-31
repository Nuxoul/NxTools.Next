using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Protocol;

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobCommand {
    public required uint Version {get;init;}
    public required Guid OperationId {get;init;}
    public required Guid SessionId {get;init;}
    public required ulong Generation {get;init;}
    public required string Action {get;init;}
    public ArchiveJobRequest? Request {get;init;}
    public ArchiveJobReply? Reply {get;init;}
}
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobLimits {
    public ulong MaxEntries {get;init;}=1000000;
    public ulong MaxMetadataBytes {get;init;}=256ul<<20;
    public ulong MaxWorkingBytes {get;init;}=512ul<<20;
    public ulong MaxEntryBytes {get;init;}=32ul<<30;
    public ulong MaxOutputBytes {get;init;}=100ul<<30;
    internal bool Valid => MaxEntries is >0 and <=1000000 && MaxMetadataBytes is >0 and <=268435456 && MaxWorkingBytes is >=67108864 and <=536870912 && MaxEntryBytes is >0 and <=34359738368 && MaxOutputBytes is >0 and <=1125899906842624;
}
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobRequest {
    public required string Kind {get;init;}
    public required string[] Paths {get;init;}
    public string? Destination {get;init;}
    public uint Codepage {get;init;}
    public ArchiveJobLimits Limits {get;init;}=new();
    public ArchiveJobExtract? Extract {get;init;}
    public ArchiveJobCreate? Create {get;init;}
    public ArchiveJobModify? Modify {get;init;}
    public ArchiveJobReadmission? Readmission {get;init;}
}
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobExtract {
    public required string Strategy {get;init;}
    public required bool SingleFileFolder {get;init;}
    public required string Conflict {get;init;}
    public required bool CopyMotw {get;init;}
    public ArchiveJobSelection? Selection {get;init;}
    internal bool Valid => Strategy is "here" or "name" or "smart" && Conflict is "ask" or "overwrite" or "skip" or "auto_rename" &&
        (Selection is null || Selection.Browser.Valid && Selection.Ordinals.Length is >0 and <=4096 && Selection.Ordinals.Distinct().Count()==Selection.Ordinals.Length);
}
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobSelection(ArchiveBrowserContext Browser, ulong[] Ordinals);
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobCreate {
    public required string Format {get;init;}
    public string? Level {get;init;}
    public string? Method {get;init;}
    [JsonConverter(typeof(ArchiveJobSecretConverter))]
    public byte[]? Password {get;init;}
    public bool? Solid {get;init;}
    public bool? EncryptHeader {get;init;}
    public required bool StripSingleRoot {get;init;}
    public required string[] ExcludePatterns {get;init;}
    public required bool TestAfterCreate {get;init;}
    public required string Name {get;init;}
    public ulong? VolumeBytes {get;init;}
    public override string ToString()=>"ArchiveJobCreate([PRIVATE])";
}
// Immutable browser revision is checked again by the owning runner, never trusted as a path grant.
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobModify(ArchiveBrowserContext Browser,string Action,ulong[] Ordinals,string? Value);
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobTicket(Guid OperationId,Guid PromptId,ulong Generation);
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobReply {
    public required ArchiveJobTicket Ticket {get;init;}
    public required string Kind {get;init;}
    [JsonConverter(typeof(ArchiveJobSecretConverter))]
    public byte[]? Password {get;init;}
    public string? Decision {get;init;}
    public required bool ApplyToAll {get;init;}
    public ulong? ApprovedLimit {get;init;}
    public override string ToString()=>"ArchiveJobReply([PRIVATE])";
}
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobOutputBudget(ulong EntryBytes,ulong OutputBytes);
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobOutputSource(string Native,uint[] Digest);
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobReadmission(Guid LogicalId,ArchiveJobOutputSource? ExpectedSource);
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobOutputShortfall(ArchiveJobOutputBudget Current,ArchiveJobOutputBudget Required,ArchiveJobOutputSource Source);
internal sealed record ArchiveJobProgress(string Phase,ulong BytesDone,ulong? BytesTotal,ulong FilesDone,ulong? FilesTotal);
// UI-only values; never pass these strings into native output or authorization APIs.
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveJobPromptDisplay(string? IncomingMember,string Target) {
    private static bool Text(string? value,int limit)=>!string.IsNullOrEmpty(value) && Encoding.UTF8.GetByteCount(value)<=limit && !value.Any(char.IsControl);
    internal bool Valid(string kind)=>Text(Target,65535) && (kind=="conflict"?Text(IncomingMember,32767):kind=="create_overwrite" && IncomingMember is null);
    public override string ToString()=>"ArchiveJobPromptDisplay([PRIVATE])";
}
internal sealed record ArchiveJobPrompt(ArchiveJobTicket Ticket,string Kind,string? ErrorCode=null,uint Attempt=0,ulong? ExistingBytes=null,ulong? IncomingBytes=null,ulong? ApprovedLimit=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] ArchiveJobPromptDisplay? Display=null);
internal sealed record ArchiveJobCommit(ulong Ordinal,string State,bool Replaced,ulong Bytes,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] string? NativeIdentity=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] bool? Published=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] string? Cleanup=null);
internal sealed record ArchiveJobTerminal(string Outcome,string? Code=null,ulong FilesCommitted=0,ulong BytesCommitted=0,
    ulong FilesVerified=0,ulong BytesVerified=0,ulong FilesSkipped=0,bool CleanupComplete=true,string? CommitState=null,
    ulong ElapsedMs=0,int? NativeErrorCode=null,IReadOnlyList<ArchiveJobCommit>? Journal=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] ArchiveJobOutputShortfall? OutputShortfall=null);
internal sealed record ArchiveJobEvent(uint Version,Guid OperationId,Guid SessionId,ulong Generation,string Kind,
    string? State=null,ArchiveJobProgress? Progress=null,ArchiveJobPrompt? Prompt=null,ArchiveJobTerminal? Terminal=null) {
    internal ProtocolEnvelope Envelope()=>ProtocolEnvelope.Create("platform_event",new{@event="archive_job",data=this},OperationId);
    internal static ArchiveJobEvent For(ArchiveJobCommand c,string kind,string? state=null,ArchiveJobProgress? progress=null,ArchiveJobPrompt? prompt=null,ArchiveJobTerminal? terminal=null)=>new(1,c.OperationId,c.SessionId,c.Generation,kind,state,progress,prompt,terminal);
}
internal static class ArchiveJobWire {
    internal static readonly JsonSerializerOptions Strict=new(JsonDefaults.Options){PropertyNameCaseInsensitive=false,NumberHandling=JsonNumberHandling.Strict};
    internal static bool PathValid(string? path)=>path is {Length:>=4 and <=32767} && Encoding.UTF8.GetByteCount(path)<=32767 && char.IsAsciiLetter(path[0]) && path[1..3]==":\\" && !path.Contains('/') && !path.Any(char.IsControl);
    internal static bool NameValid(string? name) {
        if(string.IsNullOrEmpty(name) || Encoding.UTF8.GetByteCount(name)>255 || name.EndsWith('.') || name.EndsWith(' ') || name.Any(c=>char.IsControl(c) || "\\/:*?\"<>|".Contains(c))) return false;
        var stem=name.Split('.')[0].ToUpperInvariant();
        return stem is not ("CON" or "PRN" or "AUX" or "NUL") && !(stem.Length==4 && (stem.StartsWith("COM")||stem.StartsWith("LPT")) && stem[3] is >= '1' and <= '9');
    }
    internal static bool PasswordValid(byte[]? bytes) {
        if(bytes is null) return true;
        if(bytes.Length is <1 or >4096 || bytes.Contains((byte)0))return false;
        try { new UTF8Encoding(false,true).GetCharCount(bytes);return true; }catch{return false;}
    }
    internal static bool ModifyTargetValid(string? value,bool directory) {
        if(value is null || Encoding.UTF8.GetByteCount(value)>32767 || directory && !value.EndsWith('/'))return false;
        var path=directory ? value[..^1] : value;
        return path.Length>0 && path.Split('/').All(NameValid);
    }
    private static bool ValidModify(ArchiveJobModify? edit,int paths) {
        if(edit?.Browser.Valid!=true || edit.Browser.Format!="Zip" || edit.Ordinals is null || edit.Ordinals.Length>4096 || edit.Ordinals.Distinct().Count()!=edit.Ordinals.Length)return false;
        return edit.Action switch {
            "add"=>edit.Ordinals.Length==0 && paths>1 && (edit.Value is null || ModifyTargetValid(edit.Value,true)),
            "delete"=>paths==1 && edit.Ordinals.Length>0 && edit.Value is null,
            "rename"=>paths==1 && edit.Ordinals.Length==1 && edit.Value is not null && ModifyTargetValid(edit.Value,edit.Value.EndsWith('/')),
            "new_directory"=>paths==1 && edit.Ordinals.Length==0 && ModifyTargetValid(edit.Value,true),
            "comment"=>paths==1 && edit.Ordinals.Length==0 && edit.Value is not null && Encoding.UTF8.GetByteCount(edit.Value)<=65535 && !edit.Value.Contains('\0'),
            _=>false};
    }
    internal static ArchiveJobCommand Parse(ProtocolEnvelope envelope) {
        var data=Data(envelope,"archive_job");
        var c=data.Deserialize<ArchiveJobCommand>(Strict)??throw new InvalidDataException();
        if(c.Version!=1)throw new ArchiveReadVersionException();
        if(c.OperationId==Guid.Empty || c.SessionId==Guid.Empty || c.Generation==0 || envelope.CorrelationId!=c.OperationId || envelope.Id==c.OperationId)throw new InvalidDataException();
        if(c.Action=="start") { if(c.Reply is not null || c.Request is not {} r || !Valid(r))throw new InvalidDataException(); }
        else if(c.Action=="reply") {
            if(c.Request is not null || c.Reply is not {} p || p.Ticket.OperationId!=c.OperationId || p.Ticket.PromptId==Guid.Empty || p.Ticket.Generation==0 || !PasswordValid(p.Password))throw new InvalidDataException();
            var valid=p.Kind switch {
                "password"=>p.Decision is null && p.ApprovedLimit is null && !p.ApplyToAll,
                "conflict"=>p.Password is null && p.ApprovedLimit is null && p.Decision is null or "overwrite" or "skip" or "auto_rename",
                "create_overwrite"=>p.Password is null && p.ApprovedLimit is null && !p.ApplyToAll && p.Decision is null or "overwrite",
                "destination"=>p.Password is null && p.ApprovedLimit is null && !p.ApplyToAll && p.Decision is null or "overwrite",
                "risk"=>p.Password is null && p.Decision is null && !p.ApplyToAll && (p.ApprovedLimit is null or >0 and <=34359738368),_=>false};
            if(!valid)throw new InvalidDataException();
        } else if(c.Action is not ("cancel" or "pause" or "resume") || c.Request is not null || c.Reply is not null)throw new InvalidDataException();
        return c;
    }
    internal static JsonElement Data(ProtocolEnvelope envelope,string command) {
        CheckDuplicates(envelope.Payload);
        if(JsonSerializer.SerializeToUtf8Bytes(envelope.Payload).Length>262144 || envelope.Payload.EnumerateObject().Any(p=>p.Name is not ("command" or "data")) || envelope.Payload.GetProperty("command").GetString()!=command)throw new InvalidDataException();
        return envelope.Payload.GetProperty("data");
    }
    private static void CheckDuplicates(JsonElement value) {
        if(value.ValueKind==JsonValueKind.Object) { var keys=new HashSet<string>(StringComparer.Ordinal);foreach(var p in value.EnumerateObject()){if(!keys.Add(p.Name))throw new InvalidDataException();CheckDuplicates(p.Value);} }
        else if(value.ValueKind==JsonValueKind.Array)foreach(var item in value.EnumerateArray())CheckDuplicates(item);
    }
    private static bool Valid(ArchiveJobRequest r) {
        if(r.Readmission is {} admission && (r.Kind!="extract" || admission.LogicalId==Guid.Empty || admission.ExpectedSource is {} source && (source.Native.Length is <1 or >256 || source.Native.Any(c=>c>127 || char.IsControl(c)) || source.Digest.Length!=32 || source.Digest.Any(b=>b>255))))return false;
        if(r.Limits is null || !r.Limits.Valid || r.Codepage is not (0 or 65001 or 936 or 950 or 932 or 949 or 437 or 1250 or 1251 or 1252) || r.Paths.Length is <1 or >32 || r.Paths.Any(p=>!PathValid(p)) || r.Paths.Sum(p=>Encoding.UTF8.GetByteCount(p))>131072)return false;
        if(r.Kind=="test")return r.Paths.Length==1 && r.Destination is null && r.Extract is null && r.Create is null && r.Modify is null;
        if(r.Kind=="modify")return r.Destination is null && r.Extract is null && r.Create is null && r.Readmission is null && ValidModify(r.Modify,r.Paths.Length);
        if(!PathValid(r.Destination) || r.Modify is not null)return false;
        if(r.Kind=="extract")return r.Paths.Length==1 && r.Create is null && r.Extract?.Valid==true;
        if(r.Kind!="create" || r.Extract is not null || r.Create is not {} c)return false;
        var zip=c.Format=="zip";var seven=c.Format=="seven_z";
        if(c.VolumeBytes is {} volume && (!zip && !seven || volume<65536 || volume>r.Limits.MaxOutputBytes))return false;
        if(c.Format is not ("zip" or "seven_z" or "tar" or "tgz" or "txz" or "gz" or "xz") || !NameValid(c.Name) || c.ExcludePatterns.Length>32 || c.ExcludePatterns.Any(p=>p.Contains('\0')) || c.ExcludePatterns.Sum(p=>Encoding.UTF8.GetByteCount(p))>8192 || !PasswordValid(c.Password))return false;
        if(!seven && (c.Solid.HasValue || c.EncryptHeader.HasValue) || !zip && c.Method is not null || !zip && !seven && c.Password is not null || c.EncryptHeader==true && c.Password is null)return false;
        if(c.Format=="tar" ? c.Level is not null : c.Level is not ("fast" or "normal" or "maximum") && !(zip && c.Level=="store"))return false;
        if(zip && (c.Method is not ("store" or "deflate") || (c.Method=="store")!=(c.Level=="store")))return false;
        return c.Format is not ("gz" or "xz") || r.Paths.Length==1 && !c.StripSingleRoot && c.ExcludePatterns.Length==0;
    }
    internal static WindowsCommandResult Result(bool accepted,string? code=null)=>new(accepted,code,Data:JsonSerializer.Serialize(new{accepted,code},JsonDefaults.Options));
}
internal sealed class ArchiveJobSecretConverter : JsonConverter<byte[]> {
    public override byte[]? Read(ref Utf8JsonReader reader,Type type,JsonSerializerOptions options) {
        if(reader.TokenType==JsonTokenType.Null)return null;
        if(reader.TokenType!=JsonTokenType.StartArray)throw new JsonException();
        var bytes=new List<byte>(64);
        while(reader.Read() && reader.TokenType!=JsonTokenType.EndArray){if(bytes.Count>=4096)throw new JsonException();bytes.Add(reader.GetByte());}
        if(reader.TokenType!=JsonTokenType.EndArray)throw new JsonException();return bytes.ToArray();
    }
    public override void Write(Utf8JsonWriter writer,byte[] value,JsonSerializerOptions options){writer.WriteStartArray();foreach(var b in value)writer.WriteNumberValue(b);writer.WriteEndArray();}
}
