using System.Runtime.InteropServices;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows.Archive;

// One native drag retains immutable Rust entry intents, including the async receiver phase.
// Original ArchiveEntrySession/worker owns materialization and actual cleanup before release.
internal sealed class ArchiveVirtualContent:IAsyncDisposable
{
    private readonly ArchiveEntrySession entries;
    private readonly ArchiveVirtualDragItem[] items;
    private readonly Dictionary<int,Pending> pending=[];
    private readonly object gate=new();
    private readonly CancellationTokenSource cancellation=new();
    // Serialize distinct materializations through verified accounting, not merely through worker
    // admission. Concurrent/repeated opens cannot spend the same remaining output authority twice.
    private readonly SemaphoreSlim materialization=new(1,1);
    private readonly Guid? authorizationId;
    private readonly Action<int,bool,ulong>? progress;
    private readonly CancellationTokenRegistration authorizationStop;
    private readonly CancellationToken authorizationToken;
    private readonly ArchiveSourceActivity.Reservation sourceActivity;
    private ulong remaining;
    private bool retired,cancellationDisposed;
    private Task? disposal;
    private sealed class Pending
    {
        internal Task<ArchiveEntryResult> Task=null!;
        internal Exception? Failure;
    }
    internal ArchiveVirtualContent(ArchiveEntrySession entries,ArchiveVirtualDragItem[] items,ulong totalLimit=ArchiveEntrySession.MaterializeOutputLimit,Guid? authorizationId=null,Action<int,bool,ulong>? progress=null)
    {
        if(totalLimit==0||totalLimit>ArchiveEntrySession.MaterializeOutputLimit)throw new ArgumentOutOfRangeException(nameof(totalLimit));
        this.entries=entries;this.items=items.ToArray();remaining=totalLimit;this.authorizationId=authorizationId;this.progress=progress;
        if(!entries.TryCaptureVirtualAuthorization(authorizationId,this.items,out var stop))throw new ArchiveOutputException("authorization_invalid");
        sourceActivity = ArchiveSourceLease.ReserveActivity(this.items[0].Entry.Path!, false, this.items[0].Entry.OperationId);
        authorizationToken=stop;authorizationStop=stop.Register(Cancel);
    }
    private static ProtocolEnvelope Wire(ArchiveEntryCommand command)=>ProtocolEnvelope.Create("platform_command",new{command="archive_entry",data=command},command.OperationId);
    internal IntPtr Open(int index,CancellationToken stop)
    {
        Pending request;
        CancellationTokenSource linked;
        if(!entries.TryCaptureVirtualAuthorization(authorizationId,items,out _)) { Cancel(); throw new ArchiveOutputException("authorization_invalid"); }
        lock(gate) {
            if(retired||index<0||index>=items.Length)throw new OperationCanceledException();
            if(items[index].Directory)throw new ArchiveOutputException("invalid_request");
            if(!pending.TryGetValue(index,out request!)) {
                request=new();pending[index]=request;
                request.Task=Task.Run(()=>Materialize(index,request));
            }
            linked=CancellationTokenSource.CreateLinkedTokenSource(stop,cancellation.Token,authorizationToken);
        }
        using var linkedLifetime=linked;
        using var registration=linked.Token.Register(()=>CancelEntry(index));
        var value=request.Task.WaitAsync(linked.Token).GetAwaiter().GetResult();
        linked.Token.ThrowIfCancellationRequested();
        if(!value.Success)throw new ArchiveOutputException(value.Code??"materialization_failed",value.NativeErrorCode);
        if(request.Failure is {} failure)throw new ArchiveOutputException(failure is ArchiveOutputException output?output.Stage:"source_unavailable");
        // Cache repeats the held identity/hash check for EVERY independent native cursor. The
        // optional descriptor estimate is checked here, never used as an actual-byte substitute.
        var stream=entries.OpenVirtualStream(value,(long?)items[index].Size,linked.Token,out _);
        try { linked.Token.ThrowIfCancellationRequested(); return stream; }
        catch { Marshal.Release(stream); throw; }
    }
    private async Task<ArchiveEntryResult> Materialize(int index,Pending pending)
    {
        await materialization.WaitAsync();
        try {
            var command=items[index].Entry;
            ArchiveEntryResult Refuse(string code)=>new(1,command.OperationId,command.SessionId,command.Generation,false,null,null,code);
            if(cancellation.IsCancellationRequested)return Refuse("cancelled");
            // Zero remaining also refuses zero-byte entries: original worker requires positive
            // budgets; never invent a public zero-budget bypass or write first then reject.
            if(remaining==0||items[index].Size is {} estimate&&estimate>remaining)return Refuse("resource_limit");
            Report(index,false,0); // Actual demand at original materialization admission, never descriptor IO.
            var dispatched=entries.DispatchVirtualMaterialize(command,remaining,authorizationId);
            if(cancellation.IsCancellationRequested)CancelEntry(index);
            var result=await dispatched;
            var value=JsonSerializer.Deserialize<ArchiveEntryResult>(result.Data??"",JsonDefaults.Options)??throw new InvalidDataException();
            if(value.Success) {
                try {
                    var stream=entries.OpenVirtualStream(value,null,CancellationToken.None,out var actual);
                    try {
                        if(actual<0||(ulong)actual>remaining)throw new ArchiveOutputException("resource_limit");
                        remaining-=(ulong)actual;
                        Report(index,true,(ulong)actual); // Reliable sealed bytes once per distinct index.
                    }finally{Marshal.Release(stream);}
                }catch(Exception failure){pending.Failure=failure;remaining=0;}
            }else if(value.Retained||!value.CleanupComplete) {
                // Failed output without proved retirement must not finance another distinct file.
                remaining=0;
            }
            return value; // Even verification failure retains the exact lease for original release.
        }finally{materialization.Release();}
    }
    private void Report(int index,bool ready,ulong bytes){try{progress?.Invoke(index,ready,bytes);}catch{}}
    private void CancelEntry(int index)
    {
        _ = entries.Dispatch(Wire(items[index].Entry with{Action="cancel",Path=null,Browser=null,Ordinal=null,Codepage=0,Depth=0,CopyMotw=null,PasswordInteraction=false}));
    }
    internal void Cancel()
    {
        lock(gate) {
            if(cancellationDisposed)return;
            cancellation.Cancel();
            foreach(var index in pending.Keys)CancelEntry(index);
        }
    }
    public ValueTask DisposeAsync()
    {
        lock(gate) {
            if(disposal is not null)return new(disposal);
            retired=true;Cancel();
            disposal=DisposeCore(pending.ToArray());return new(disposal);
        }
    }
    private async Task DisposeCore(KeyValuePair<int,Pending>[] tasks)
    {
        var failures=new List<Exception>();
        foreach(var pair in tasks) {
            try {
                var result=await pair.Value.Task;
                if(result.LeaseId is {} lease&&result.CleanupComplete) {
                    var original=items[pair.Key].Entry;
                    var release=await entries.Dispatch(Wire(new(){Version=1,OperationId=original.OperationId,SessionId=original.SessionId,Generation=original.Generation,Action="release",LeaseId=lease}));
                    if(!release.Success)throw new ArchiveOutputException("lease_release_failed");
                }
                if(!result.CleanupComplete)throw new ArchiveOutputException("cleanup_incomplete");
            }catch(Exception failure){failures.Add(failure);}
        }
        // Serialize CTS retirement with Cancel/Open admission. Existing returned native streams
        // remain target-owned; disposing this owner never force-closes them.
        authorizationStop.Unregister();
        lock(gate){cancellationDisposed=true;cancellation.Dispose();materialization.Dispose();}
        if (failures.Count == 0) sourceActivity.Dispose();
        if(failures.Count>0)throw new AggregateException("virtual_cleanup_incomplete",failures);
    }
}
