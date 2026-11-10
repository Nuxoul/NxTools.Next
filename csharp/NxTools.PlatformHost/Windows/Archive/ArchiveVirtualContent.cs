using System.Runtime.InteropServices;
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
    private readonly ArchiveEntrySession.VirtualMaterialization context;
    private ulong remaining;
    private bool retired,cancellationDisposed;
    private Task? disposal;
    internal string? TerminalFailure => context.Started && context.Terminal.Task.IsCompletedSuccessfully && context.Terminal.Task.Result.Category is not (null or "success")
        ? context.Terminal.Task.Result.Category : null;
    private sealed class Pending
    {
        internal Task<ArchiveEntrySession.VirtualMember> Task=null!;
        internal Exception? Failure;
    }
    internal ArchiveVirtualContent(ArchiveEntrySession entries,ArchiveVirtualDragItem[] items,ulong totalLimit=ArchiveEntrySession.MaterializeOutputLimit,Guid? authorizationId=null,Action<int,bool,ulong>? progress=null)
        :this(entries,ArchiveVirtualPlan.Admit(items),totalLimit,authorizationId,progress) { }
    internal ArchiveVirtualContent(ArchiveEntrySession entries,ArchiveVirtualPlan plan,ulong totalLimit=ArchiveEntrySession.MaterializeOutputLimit,Guid? authorizationId=null,Action<int,bool,ulong>? progress=null,Guid? operation=null,Action<ArchiveEntrySession.VirtualMemberProgress>? memberProgress=null)
    {
        if(totalLimit==0||totalLimit>ArchiveEntrySession.MaterializeOutputLimit)throw new ArgumentOutOfRangeException(nameof(totalLimit));
        this.entries=entries;items=plan.Items;remaining=totalLimit;this.authorizationId=authorizationId;this.progress=progress;
        if(!entries.TryCaptureVirtualAuthorization(authorizationId,items,out var stop))throw new ArchiveOutputException("authorization_invalid");
        context=entries.CreateVirtualMaterialization(plan,authorizationId,operation,memberProgress);
        authorizationToken=stop;authorizationStop=stop.Register(Cancel);
    }
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
        if(request.Failure is {} failure)throw new ArchiveOutputException(failure is ArchiveOutputException output?output.Stage:"source_unavailable");
        // Cache repeats the held identity/hash check for EVERY independent native cursor. The
        // optional descriptor estimate is checked here, never used as an actual-byte substitute.
        var stream=entries.OpenVirtualStream(value,(long?)items[index].Size,linked.Token,out _);
        try { linked.Token.ThrowIfCancellationRequested(); return stream; }
        catch { Marshal.Release(stream); throw; }
    }
    private async Task<ArchiveEntrySession.VirtualMember> Materialize(int index,Pending pending)
    {
        await materialization.WaitAsync();
        try {
            cancellation.Token.ThrowIfCancellationRequested();
            if(remaining==0||items[index].Size is {} estimate&&estimate>remaining)throw new ArchiveOutputException("resource_limit");
            Report(index,false,0);
            var value=await entries.DemandVirtual(context,index,remaining);
            if(value.Size<0||(ulong)value.Size>remaining){remaining=0;throw new ArchiveOutputException("resource_limit");}
            remaining-=(ulong)value.Size;
            Report(index,true,(ulong)value.Size);
            return value;
        }catch(Exception failure){pending.Failure=failure;throw;}
        finally{materialization.Release();}
    }
    private void Report(int index,bool ready,ulong bytes){try{progress?.Invoke(index,ready,bytes);}catch{}}
    private void CancelEntry(int index) => entries.CancelVirtual(context);
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
            retired=true;
            if(pending.Values.Any(value=>!value.Task.IsCompleted))Cancel();
            disposal=DisposeCore(pending.ToArray());return new(disposal);
        }
    }
    private async Task DisposeCore(KeyValuePair<int,Pending>[] tasks)
    {
        // Domain failure/cancellation is distinct from failed native cleanup.
        foreach(var pair in tasks) { try { await pair.Value.Task; } catch { } }
        await entries.CloseVirtual(context);
        authorizationStop.Unregister();
        lock(gate){cancellationDisposed=true;cancellation.Dispose();materialization.Dispose();}
    }
}
