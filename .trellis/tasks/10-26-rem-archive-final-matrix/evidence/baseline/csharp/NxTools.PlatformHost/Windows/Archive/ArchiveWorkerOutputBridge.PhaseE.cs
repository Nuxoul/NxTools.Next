using System.Text;
using System.Text.Json;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveWorkerOutputBridge
{
    private readonly bool editing;
    private ArchiveSourceLease? editSource;
    private string? editTargetIdentity;
    private ArchiveSourceLease.EditReadCompletion? editCompletion;
    private ArchiveWorkerRequest? volumeRequest;
    private bool volumeVerified;
    private readonly HashSet<Guid> flushedVolumes=[];
    private readonly HashSet<Guid> verifiedVolumeTokens=[];
    private readonly Dictionary<Guid,string> outputNames=[];
    internal void BindEditSource(ArchiveSourceLease source,string expected)
    {
        if(!editing || editSource is not null || files!=0)throw new ArchiveOutputException("invalid_edit_request");
        source.VerifyEditSource(expected);editSource=source;editTargetIdentity=expected;
    }
    internal void ConfigureVolumeOutput(ArchiveWorkerRequest request)
    {
        if(editing || request.VolumeBytes is null || volumeRequest is not null || files!=0 || overwrites.Count!=0)throw new ArchiveOutputException("invalid_output_request");
        volumeRequest=request;
    }
    private bool AllowedOutputName(string relative)
    {
        if(volumeRequest is null)return files==1 && string.Equals(relative,createName,StringComparison.Ordinal);
        if(volumeVerified || files>1025)return false;
        if(files==1)return relative==createName; // Original encoder's unpublished temporary token only.
        return VolumeOrdinal(relative) is not null;
    }
    private int? VolumeOrdinal(string name)
    {
        if(volumeRequest is null || createName is null)return null;
        if(volumeRequest.Operation=="create_zip")
        {
            if(name==createName)return 1024; // Last ZIP coordinate is validated by ordered set below.
            var stem=Path.GetFileNameWithoutExtension(createName);var prefix=stem+".z";
            if(!name.StartsWith(prefix,StringComparison.Ordinal))return null;
            var number=name[prefix.Length..];
            return int.TryParse(number,out var ordinal) && ordinal is >=1 and <=1023 && number==ordinal.ToString("D2")?ordinal:null;
        }
        var sevenPrefix=createName+".";
        if(!name.StartsWith(sevenPrefix,StringComparison.Ordinal))return null;
        var digits=name[sevenPrefix.Length..];
        return int.TryParse(digits,out var index) && index is >=1 and <=1024 && digits==index.ToString("D3")?index:null;
    }
    private bool HandleExtendedCall(string action,JsonElement call,Guid operationId,Guid callId,out object? reply)
    {
        reply=null;
        if(action=="edit_reads_complete")
        {
            if(!editing || editSource is null || editCompletion is not null || active.Count!=1)throw new ArchiveOutputException("edit_reads_incomplete");
            var token=call.GetProperty("token").GetGuid();RequireToken(token);
            session.MarkCandidateVerified(token);
            editCompletion=editSource.CompleteEditReads();
        }
        else if(action=="flush_volume")
        {
            if(volumeRequest is null || volumeVerified)throw new ArchiveOutputException("invalid_output_request");
            var token=call.GetProperty("token").GetGuid();RequireToken(token);
            if(!outputNames.TryGetValue(token,out var name)||VolumeOrdinal(name) is null || !flushedVolumes.Add(token))throw new ArchiveOutputException("invalid_output_request");
            session.MarkCandidateVerified(token);
        }
        else if(action=="volumes_verified")
        {
            if(volumeRequest is null || volumeVerified || active.Count is <1 or >1024 || flushedVolumes.Count!=active.Count)throw new ArchiveOutputException("volumes_not_verified");
            var bytes=call.GetProperty("data").EnumerateArray().Select(value=>value.GetByte()).ToArray();
            if(bytes.Length>65536)throw new ArchiveOutputException("write_limit");
            var tokens=JsonSerializer.Deserialize<Guid[]>(bytes)??throw new ArchiveOutputException("volumes_not_verified");
            if(tokens.Length!=active.Count || tokens.Distinct().Count()!=tokens.Length || tokens.Any(token=>!active.Contains(token)||!flushedVolumes.Contains(token)))throw new ArchiveOutputException("volumes_not_verified");
            for(var index=0;index<tokens.Length;index++)
            {
                var expected=volumeRequest.Operation=="create_zip" && index+1==tokens.Length?createName!:
                    volumeRequest.Operation=="create_zip"?Path.GetFileNameWithoutExtension(createName)+".z"+(index+1).ToString("D2"):createName+"."+(index+1).ToString("D3");
                if(outputNames[tokens[index]]!=expected)throw new ArchiveOutputException("volumes_not_verified");
                verifiedVolumeTokens.Add(tokens[index]);
            }
            volumeVerified=true;
        }
        else return false;
        reply=new {version=1,operation_id=operationId,op="native_result",call_id=callId,success=true};return true;
    }
}
