using System.Text;
using System.Text.Json;
using System.Security.Cryptography;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

// Translation only: the original runner, Job, source lease, interactions and native output own execution.
internal sealed partial class ArchiveJobSession : IAsyncDisposable {
    private readonly object gate=new();
    private readonly Channel<ProtocolEnvelope> events=Channel.CreateBounded<ProtocolEnvelope>(16);
    private readonly Channel<ProtocolEnvelope> progress=Channel.CreateBounded<ProtocolEnvelope>(new BoundedChannelOptions(1){FullMode=BoundedChannelFullMode.DropOldest,SingleReader=true});
    private readonly Dictionary<Guid,Operation> active=[];
    private readonly HashSet<Guid> retired=[];
    private readonly Queue<Guid> recent=[];
    private readonly ArchiveWorkerRunner runner;
    private readonly IPlatformLog? log;
    private readonly bool largeCount;
    private int timeoutMs=30000;
    private bool stopped;
    internal ChannelReader<ProtocolEnvelope> Progress=>progress.Reader;
    internal ChannelReader<ProtocolEnvelope> Events=>events.Reader;
    private sealed class Operation(ArchiveJobCommand command) {
        internal readonly ArchiveJobCommand Command=command;
        internal ArchiveJobPrompt? Prompt;
        internal TaskCompletionSource<bool>? CreateOverwriteReply;
        internal TaskCompletionSource<bool>? DestinationReply;
        internal ulong Files,Bytes;
        internal ArchiveJobOutputShortfall? Shortfall;
        internal bool Complete;
        internal string? State;
        internal string? Failure;
        internal long LastProgress;
        internal string? LastProgressPhase;
        internal ulong ProgressSequence;
        internal bool BatchPasswordTried;
        internal bool BatchPasswordRetired;
    }
    internal ArchiveJobSession(IPlatformLog? log=null,bool largeCount=false):this(ArchiveReadHelper.Production(),log,largeCount){}
    internal void ProbeOutputCommitForFixture(Action<string> probe) {
        if(!string.Equals(Path.GetFileName(Environment.ProcessPath),"NxTools.PlatformHost.Tests.exe",StringComparison.OrdinalIgnoreCase))throw new InvalidOperationException("Owned test apphost required.");
        runner.OutputCommitCheckpointProbe=probe;
    }
    // HZ4 fixture connector only; reuses original owners and cannot be enabled in Release.
    internal void ProbeLifecycleForFixture(Action<string> probe) {
#if DEBUG
        if(!string.Equals(Path.GetFileName(Environment.ProcessPath),"NxTools.PlatformHost.Tests.exe",StringComparison.OrdinalIgnoreCase))throw new InvalidOperationException("Owned test apphost required.");
        ProbeOutputCommitForFixture(probe);
        runner.SourceCheckpointProbe=probe;
        runner.OutputWriteCheckpointProbe=_=>probe("candidate_write");
        runner.CleanupCheckpointProbe=()=>probe("before_cleanup");
#else
        throw new InvalidOperationException("Release fixture control is disabled.");
#endif
    }
    internal void ProbeCreateOverwritePreflightForFixture(Action probe) {
        if(!string.Equals(Path.GetFileName(Environment.ProcessPath),"NxTools.PlatformHost.Tests.exe",StringComparison.OrdinalIgnoreCase))throw new InvalidOperationException("Owned test apphost required.");
        runner.CreateOverwritePreflightProbe=probe;
    }
    internal void ConfigureControlFixture(int timeout,ulong riskBytes=0,ulong riskRatio=0) {
        if(!string.Equals(Path.GetFileName(Environment.ProcessPath),"NxTools.PlatformHost.Tests.exe",StringComparison.OrdinalIgnoreCase) || timeout<200 || timeout>30000)throw new InvalidOperationException("Owned test control required.");
        timeoutMs=timeout;
#if DEBUG
        if(riskBytes>0)runner.RiskThresholdFixture=(riskBytes,riskRatio);
#endif
    }
    private ArchiveJobSession(ArchiveReadHelper helper,IPlatformLog? log,bool largeCount) {this.log=log;this.largeCount=largeCount;runner=ArchiveWorkerRunner.ForProductJob(helper,OnRunnerEvent,OnFrame,log);runner.ConfirmDestination=ConfirmDestination;}
    internal static ArchiveJobSession ForOwnedTestPackage(string root,IPlatformLog? log=null,bool largeCount=false)=>new(ArchiveReadHelper.OwnedPackage(root),log,largeCount);
    internal WindowsCommandResult Dispatch(ProtocolEnvelope envelope) {
        ArchiveJobCommand command;
        try {command=ArchiveJobWire.Parse(envelope);}catch(ArchiveReadVersionException){return ArchiveJobWire.Result(false,"incompatible_version");}catch{return ArchiveJobWire.Result(false,"invalid_request");}
        try { lock(gate) {
            if(command.Action=="retire_batch") {RetireBatchPassword(command.BatchId);Diagnostic("batch_password_retired",command);return ArchiveJobWire.Result(true);}
            if(command.Action!="start") {
                if(stopped || !active.TryGetValue(command.OperationId,out var op) || op.Command.SessionId!=command.SessionId || op.Command.Generation!=command.Generation || op.Command.BatchId!=command.BatchId)return ArchiveJobWire.Result(false,"stale_operation");
                bool accepted;
                if(command.Action=="cancel") {RetireBatchSecret(op.Command.BatchId);accepted=runner.Cancel(command.OperationId);if(accepted)State(op,"cancelling");}
                else if(command.Action is "pause" or "resume") {accepted=runner.SetPaused(command.OperationId,command.Action=="pause");if(accepted)State(op,command.Action=="pause"?"paused":"running");}
                else accepted=Reply(op,command.Reply!);
                return ArchiveJobWire.Result(accepted,accepted?null:"stale_prompt");
            }
            if(active.ContainsKey(command.OperationId)||retired.Contains(command.OperationId))return ArchiveJobWire.Result(false,"duplicate_operation");
            if(command.Generation!=1)RetireBatchPassword(command.BatchId);
            var operation=new Operation(command);active.Add(command.OperationId,operation);
            if(stopped || active.Count>2) {Finish(operation,new("rejected",stopped?"disconnected":"busy",Journal:[]));return ArchiveJobWire.Result(false,stopped?"disconnected":"busy");}
            var request=command.Request!;var limits=request.Limits;var create=request.Create;
            var kind=request.Kind=="modify"?(request.Modify!.Browser.Format switch {"SevenZ"=>"modify_7z","Tar" or "TarGz" or "TarXz"=>"modify_tar",_=>"modify_zip"}):request.Kind=="create" ? create!.Format switch {"seven_z"=>"create_7z","gz"=>"create_gzip",var format=>"create_"+format} : request.Kind;
            var largeExtract=largeCount && request.Kind=="extract" && request.Extract?.Reclamation is null;
            string? password=null;
            try {
                if(create?.Password is {} secret)password=new UTF8Encoding(false,true).GetString(secret);
                var tree=kind is "create_7z" or "create_tar" or "create_tgz" or "create_txz";
                var admitted=runner.Start(new(command.OperationId,kind,request.Paths[0],(int)request.Codepage,
                    new ArchiveAdmittedLimits(limits.MaxEntries,limits.MaxMetadataBytes,limits.MaxWorkingBytes,limits.MaxEntryBytes,limits.MaxOutputBytes),
                    Password:password,OutputRoot:ArchiveWorkerRunner.IsModification(kind)?Path.GetDirectoryName(request.Paths[0]):request.Destination,CreateName:create?.Name,CompressionLevel:create?.Level,
                    TimeoutMs:largeExtract?0:timeoutMs,
                    ZipOptions:kind=="create_zip"?new(create!.Method!,password is null?"none":"aes256",create.StripSingleRoot,create.ExcludePatterns):null,
                    SourcePaths:ArchiveWorkerRunner.IsModification(kind) && request.Modify?.Action is ("add" or "replace_content")?request.Paths.Skip(1).ToArray():ArchiveWorkerRunner.IsTreeCreation(kind)?request.Paths:null,
                    TreeOptions:tree?new(create!.StripSingleRoot,create.ExcludePatterns):null,
                    SevenzOptions:kind=="create_7z"?new(create!.Solid??false,password is null?"none":"aes256",create.EncryptHeader??false):null,
                    ExtractOptions:request.Extract is {} extract?new(extract.CopyMotw,extract.Reclamation):null,
                    PasswordInteraction:request.Kind is "extract" or "test" or "modify",ConflictPolicy:ArchiveWorkerRunner.IsModification(kind)?"ask":request.Extract?.Conflict,
                    JobExecution:true,JobExtract:request.Extract,OutputReadmission:request.Readmission is {} readmission?JsonSerializer.SerializeToElement(readmission,JsonDefaults.Options):null,
                    VolumeBytes:create?.VolumeBytes,JobModify:request.Modify,JobLargeCount:largeExtract,
                    ParallelExtraction:kind=="extract" && request.Extract?.Reclamation is null),
                    confirmCreateOverwrite:request.Kind=="create" && create?.VolumeBytes is null?(target,token)=>ConfirmCreateOverwrite(operation,target,token):null);
                return ArchiveJobWire.Result(admitted,admitted?null:operation.Failure??"io_failure");
            } finally { if(create?.Password is {} secret)CryptographicOperations.ZeroMemory(secret); }
        }} finally {if(command.Reply?.Password is {} replySecret)CryptographicOperations.ZeroMemory(replySecret);if(command.Request?.Create?.Password is {} createSecret)CryptographicOperations.ZeroMemory(createSecret);}
    }
    private bool Reply(Operation op,ArchiveJobReply reply) {
        if(op.State=="cancelling" || op.Prompt is not {} prompt || prompt.Ticket!=reply.Ticket || prompt.Kind!=reply.Kind)return false;
        if(reply.RememberBatchPassword && !BatchPasswordEligible(op))return false;
        if(reply.Kind=="create_overwrite") {
            if(op.State=="cancelling" || op.CreateOverwriteReply is not {} pending || !pending.TrySetResult(reply.Decision=="overwrite"))return false;
            op.CreateOverwriteReply=null;op.Prompt=null;
            runner.SetPromptWaiting(op.Command.OperationId,false);State(op,reply.Decision=="overwrite"?"running":"cancelling");return true;
        }
        if(reply.Kind=="destination"){
            if(op.State=="cancelling"||op.DestinationReply is not {} pending||!pending.TrySetResult(reply.Decision=="overwrite"))return false;
            op.DestinationReply=null;op.Prompt=null;runner.SetPromptWaiting(op.Command.OperationId,false);State(op,reply.Decision=="overwrite"?"running":"cancelling");return true;
        }
        var ticket=new ArchivePasswordTicket(reply.Ticket.OperationId,reply.Ticket.PromptId,reply.Ticket.Generation);
        bool accepted;
        try {accepted=reply.Kind switch {
            "password"=>runner.RespondPasswordBytes(op.Command.OperationId,ticket,reply.Password),
            "conflict"=>runner.RespondConflict(op.Command.OperationId,ticket,reply.Decision,reply.ApplyToAll),
            "risk"=>runner.RespondRisk(op.Command.OperationId,ticket,reply.ApprovedLimit),_=>false};
            if(accepted && reply.Kind=="password") {op.BatchPasswordTried=true;if(reply.Password is null)RetireBatchSecret(op.Command.BatchId);else RememberBatchSecret(op,reply);}
        }finally{if(reply.Password is {} secret)CryptographicOperations.ZeroMemory(secret);}
        if(accepted){op.Prompt=null;runner.SetPromptWaiting(op.Command.OperationId,false);State(op,"running");}
        return accepted;
    }
    private async Task<bool> ConfirmCreateOverwrite(Operation op,(string Signature,long Size) target,CancellationToken token) {
        TaskCompletionSource<bool> pending;
        lock(gate) {
            token.ThrowIfCancellationRequested();
            if(stopped || !active.TryGetValue(op.Command.OperationId,out var current) || current!=op || op.Prompt is not null)return false;
            pending=new(TaskCreationOptions.RunContinuationsAsynchronously);op.CreateOverwriteReply=pending;
            var request=op.Command.Request!;
            var display=new ArchiveJobPromptDisplay(null,DisplayTarget(request.Destination!,request.Create!.Name));
            var prompt=new ArchiveJobPrompt(new(op.Command.OperationId,Guid.NewGuid(),op.Command.Generation),"create_overwrite",ExistingBytes:checked((ulong)target.Size),Display:display);
            op.Prompt=prompt;runner.SetPromptWaiting(op.Command.OperationId,true);State(op,"awaiting_conflict");
            Publish(ArchiveJobEvent.For(op.Command,"prompt",prompt:prompt));
        }
        return await pending.Task.WaitAsync(token);
    }
    private async Task<bool> ConfirmDestination(Guid id,bool missing,ulong? required,ulong available,CancellationToken token){
        TaskCompletionSource<bool> pending;
        lock(gate){
            token.ThrowIfCancellationRequested();
            if(stopped||!active.TryGetValue(id,out var op)||op.Prompt is not null)return false;
            pending=new(TaskCreationOptions.RunContinuationsAsynchronously);op.DestinationReply=pending;
            op.Prompt=new ArchiveJobPrompt(new(id,Guid.NewGuid(),op.Command.Generation),"destination",ErrorCode:missing?"create_destination":"low_space",ExistingBytes:available,IncomingBytes:required);
            runner.SetPromptWaiting(id,true);State(op,"awaiting_conflict");Publish(ArchiveJobEvent.For(op.Command,"prompt",prompt:op.Prompt));
        }
        return await pending.Task.WaitAsync(token);
    }
    internal void ConfigureSpaceFixture(ulong available){
        if(!string.Equals(Path.GetFileName(Environment.ProcessPath),"NxTools.PlatformHost.Tests.exe",StringComparison.OrdinalIgnoreCase))throw new InvalidOperationException("Owned test apphost required.");
        runner.AvailableSpaceFixture=available;
    }
    private void OnRunnerEvent(ArchiveWorkerEvent item) {
        lock(gate) {
            if(!active.TryGetValue(item.OperationId,out var op))return;
            if(item.Kind is "accepted" or "started") {Publish(ArchiveJobEvent.For(op.Command,item.Kind));Diagnostic(item.Kind,op.Command);return;}
            if(item.Kind=="job_state" && item.Category is {} phase){State(op,phase);return;}
            if(item.Kind=="rejected") {op.Failure=Map(item.Category);Finish(op,new("rejected",op.Failure,NativeErrorCode:item.NativeErrorCode,Journal:[]));return;}
            if(item.Kind!="exited")return;
            // Native publication, uncertainty, cleanup and timeout cause always outrank a late reply/cancel.
            var authoritative=item.Category is "commit_outcome_unknown" or "target_changed_after_commit" or "committed_but_verification_failed" or "cancelled_with_committed_output";
            var nativeCause=item.HasNativeFailure || item.Category is "source_unavailable" or "source_changed" or "target_changed" or "target_missing" or "target_directory" or "target_reparse" or "target_busy" or "target_denied" or "overwrite_not_supported" || item.Category?.StartsWith("metadata_",StringComparison.Ordinal)==true || item.Category?.StartsWith("motw_",StringComparison.Ordinal)==true;
            var code=!item.CleanupComplete?"cleanup_incomplete":authoritative || nativeCause?item.Category:item.Category=="timeout"?"timeout":op.Failure??(item.Category is "success" or "succeeded_with_warnings"?op.Complete?null:"protocol_violation":Map(item.Category));
            var outcome=code is null?(item.FilesSkipped>0 || item.Category=="succeeded_with_warnings"?"succeeded_with_warnings":"succeeded"):code is "cancelled" or "cancelled_with_committed_output"?"cancelled":"failed";
            var addResult=(outcome is "succeeded" or "succeeded_with_warnings") && item.CleanupComplete && item.OutputPublished==true &&
                item.CommitState=="Committed" && item.FilesCommitted==1 && item.Journal is { Count: 1 } journal &&
                journal[0] is { State: "Committed", Published: true } ? item.AddResult : null;
            Finish(op,new(outcome,code,(ulong)item.FilesCommitted,item.BytesCommitted,op.Files,op.Bytes,item.FilesSkipped,item.CleanupComplete,item.CommitState,(ulong)item.ElapsedMs,item.NativeErrorCode,item.Journal??[],op.Shortfall,item.ResultLocation,item.OutputPublished,addResult,largeCount?item.JournalSummary:null));
        }
    }
    private Task<string?> OnFrame(Guid id,JsonElement frame,CancellationToken token) {
        lock(gate) {
            if(stopped || !active.TryGetValue(id,out var op))return Task.FromResult<string?>("disconnected");
            try {
                var kind=frame.GetProperty("op").GetString();
                switch(kind) {
                    case "accepted":State(op,"running");break;
                    case "progress":
                        var now=Environment.TickCount64;
                        var progress=frame.GetProperty("progress").Deserialize<ArchiveJobProgress>(ArchiveJobWire.Strict)??throw new InvalidDataException();
                        if(!progress.Valid)throw new InvalidDataException();
                        var changed=progress.Phase!=op.LastProgressPhase;
                        var editStage=op.Command.Request?.Modify?.Browser.Format is ("SevenZ" or "Tar" or "TarGz" or "TarXz") && progress.Phase is ("edit_planning" or "edit_reading" or "edit_reencoding" or "edit_verifying" or "edit_committing");
                        if(now-op.LastProgress>=100 || changed) {
                            op.LastProgress=now;op.LastProgressPhase=progress.Phase;
                            // Both forwarding channels share this operation-local publication order.
                            progress=progress with { Sequence=checked(++op.ProgressSequence) };
                            Publish(ArchiveJobEvent.For(op.Command,"progress",progress:progress),lossless:editStage&&changed);
                        }break;
                    case "password_state":
                        var password=frame.GetProperty("password_state");
                        if(password.TryGetProperty("error_code",out var passwordError) && passwordError.GetString()=="WrongPassword")InvalidateBatchPassword(op);
                        if(password.GetProperty("state").GetString()=="awaiting_password")Prompt(op,password,"password",password.TryGetProperty("error_code",out var failure)?failure.GetString():null,password.GetProperty("attempt").GetUInt32());break;
                    case "conflict_prompt":Prompt(op,frame.GetProperty("conflict_prompt"),"conflict");break;
                    case "risk_prompt":Prompt(op,frame.GetProperty("risk_prompt"),"risk");break;
                    case "complete":op.Complete=true;op.Files=frame.GetProperty("files_verified").GetUInt64();op.Bytes=frame.GetProperty("bytes_verified").GetUInt64();break;
                    case "output_budget_shortfall":op.Shortfall=frame.GetProperty("output_budget_shortfall").Deserialize<ArchiveJobOutputShortfall>(ArchiveJobWire.Strict)??throw new InvalidDataException();break;
                    case "failure":op.Failure=Map(frame.GetProperty("error_code").GetString());if(op.Failure=="wrong_password")InvalidateBatchPassword(op);break;
                }
                return Task.FromResult<string?>(null);
            }catch{return Task.FromResult<string?>(op.Failure="protocol_violation");}
        }
    }
    private void Prompt(Operation op,JsonElement value,string kind,string? code=null,uint attempt=0) {
        var ticket=value.GetProperty("ticket").Deserialize<ArchiveJobTicket>(ArchiveJobWire.Strict)??throw new InvalidDataException();
        if(ticket.OperationId!=op.Command.OperationId || ticket.PromptId==Guid.Empty || ticket.Generation==0 || op.Prompt is not null)throw new InvalidDataException();
        ulong? Size(string name)=>value.TryGetProperty(name,out var size)&&size.ValueKind==JsonValueKind.Number?size.GetUInt64():null;
        ArchiveJobPromptDisplay? display=null;
        if(kind=="conflict" && value.TryGetProperty("display",out var context) && context.ValueKind!=JsonValueKind.Null) {
            display=context.Deserialize<ArchiveJobPromptDisplay>(ArchiveJobWire.Strict)??throw new InvalidDataException();
            if(!display.Valid(kind))throw new InvalidDataException();
            if(op.Command.Request!.Kind!="modify")display=display with{Target=DisplayTarget(op.Command.Request.Destination!,display.Target)};
            if(!display.Valid(kind))throw new InvalidDataException();
        }
        var prompt=new ArchiveJobPrompt(ticket,kind,code,attempt,Size("existing_bytes"),Size("incoming_bytes")??Size("declared_bytes"),Size("admitted_entry_bytes"),display);op.Prompt=prompt;
        if(kind=="password" && TryReuseBatchSecret(op,prompt))return;
        runner.SetPromptWaiting(op.Command.OperationId,true);State(op,"awaiting_"+kind);Publish(ArchiveJobEvent.For(op.Command,"prompt",prompt:prompt));
    }
    // String presentation only: no native path normalization, observation or authority is added.
    private static string DisplayTarget(string root,string relative)=>root.TrimEnd('\\')+"\\"+relative.Replace('/','\\');
    private void State(Operation op,string state){if(!active.TryGetValue(op.Command.OperationId,out var current) || current!=op || op.State==state)return;op.State=state;Publish(ArchiveJobEvent.For(op.Command,"state",state));Diagnostic(state,op.Command);}
    private void Finish(Operation op,ArchiveJobTerminal terminal) {
        if(!active.ContainsKey(op.Command.OperationId))return;
        if(terminal.Outcome is "cancelled" or "rejected" || terminal.Code is "WrongPassword" or "wrong_password" || !terminal.CleanupComplete)RetireBatchSecret(op.Command.BatchId);
        op.CreateOverwriteReply=null;op.DestinationReply=null;op.Prompt=null;
        active.Remove(op.Command.OperationId);retired.Add(op.Command.OperationId);recent.Enqueue(op.Command.OperationId);if(recent.Count>4096)retired.Remove(recent.Dequeue());
        Publish(ArchiveJobEvent.For(op.Command,"terminal",terminal:terminal));Diagnostic("terminal",op.Command,terminal);
    }
    private void Publish(ArchiveJobEvent item,bool lossless=true) {
        if(stopped)return;
        if(!lossless){progress.Writer.TryWrite(item.Envelope());return;}
        if(events.Writer.TryWrite(item.Envelope()))return;
        stopped=true;RetireBatchSecrets();runner.Stop("consumer_queue_full");events.Writer.TryComplete(new InvalidDataException("Archive job consumer queue full."));
    }
    private void Diagnostic(string phase,ArchiveJobCommand c,ArchiveJobTerminal? t=null){try{log?.Write("info","archive.job."+phase,c.OperationId,new Dictionary<string,object?>{["generation"]=c.Generation,["kind"]=c.Request?.Kind,["category"]=t?.Code,["elapsed_ms"]=t?.ElapsedMs??0,["native_error_code"]=t?.NativeErrorCode,["cleanup_complete"]=t?.CleanupComplete,["batch_secret_count"]=batchSecrets.Count});}catch{}}
    private static string Map(string? code)=>code?.StartsWith("reclamation_",StringComparison.Ordinal)==true?code:code is "success" or "succeeded_with_warnings"?code:code is "target_changed" or "target_missing" or "target_directory" or "target_reparse" or "target_busy" or "target_denied" or "overwrite_not_supported"?code:ArchiveReadSession.Map(code);
    internal void Stop(){lock(gate){stopped=true;RetireBatchSecrets();runner.Stop();events.Writer.TryComplete();progress.Writer.TryComplete();}}
    public async ValueTask DisposeAsync(){Stop();await runner.DrainAsync();runner.Dispose();}
}
