using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Diagnostics;
namespace NxTools.PlatformHost.Windows.Archive;

// Read-only projection over original held source/output namespace owners. No writes/probes.
internal static class ArchiveSpacePreflight
{
    // Reuse the original detached command owner; native waits never park the pipe reader.
    internal static Task DispatchAsync(PlatformCommandSequenceDispatcher dispatcher,ArchiveSpaceCommand command,
        IPlatformLog? log,CancellationToken cancellation,Func<WindowsCommandResult,CancellationToken,Task> publish,
        Func<ulong,ulong>? capacityFixture=null) =>
        dispatcher.DispatchAsync("archive_entry",async token=> {
            var result=Run(command,log,token,capacityFixture);
            if(!token.IsCancellationRequested)await publish(result,token);
        },cancellation);

    internal static WindowsCommandResult Run(ArchiveSpaceCommand command,IPlatformLog? log,CancellationToken cancellation,
        Func<ulong,ulong>? capacityFixture=null)
    {
        var clock=Stopwatch.StartNew();
        Diagnostic(log,command,"accepted","admission",clock.ElapsedMilliseconds,null);
        if(cancellation.IsCancellationRequested) {
            Diagnostic(log,command,"wait_ended","cancelled",clock.ElapsedMilliseconds,null);
            return new(false,"cancelled");
        }
        if(BoundedStaOperationRunner.TryRun("NxTools.ArchiveSpace",TimeSpan.FromSeconds(5),2,
            token=>Observe(command,log,token,capacityFixture),out WindowsCommandResult? result,out _,cancellation))
            return result??new(false,"space_unavailable");
        var code=cancellation.IsCancellationRequested?"cancelled":"space_busy_or_timeout";
        Diagnostic(log,command,"wait_ended",code,clock.ElapsedMilliseconds,null);
        return new(false,code);
    }
    internal static WindowsCommandResult Observe(ArchiveSpaceCommand command,IPlatformLog? log=null,CancellationToken cancellation=default,
        Func<ulong,ulong>? capacityFixture=null)
    {
        var clock=Stopwatch.StartNew();var stage="validation";
        void Record(string outcome,int? native=null) => Diagnostic(log,command,outcome,stage,clock.ElapsedMilliseconds,native);
        Record("started");
        try {
            if(!command.Valid) { Record("failed");return new(false,"invalid_request"); }
            cancellation.ThrowIfCancellationRequested();stage="source";
            // Original finite family/header admission, not a decoder/size scan. Exceeding either
            // ceiling is honestly resource_limit, never permission to expand execution budgets.
            using var source=ArchiveVolumeSources.Open(command.Source,4ul<<20,100ul<<30);
            var matches=command.ExpectedSource is null || source.Identity()==command.ExpectedSource;
            if(!matches) { stage="source_changed";Record("failed");return new(false,"source_changed"); }
            cancellation.ThrowIfCancellationRequested();stage="destination";
            using var output=ArchiveOutputSession.Prepare(command.Destination,command.OperationId);
            var observed=output.ObserveSpace();
            if(command.ExpectedTarget is {} expected && expected!=observed.TargetIdentity) {
                stage="target_changed";Record("failed");return new(false,"target_changed");
            }
            source.Verify();cancellation.ThrowIfCancellationRequested();
            // Tests may replace only observed available bytes, after real held-native observation.
            var available=capacityFixture?.Invoke(observed.Available)??observed.Available;
            cancellation.ThrowIfCancellationRequested();
            var result=new ArchiveSpaceObservation(command.Version,command.OperationId,command.SessionId,command.Generation,
                available,observed.Allocation,observed.TargetIdentity,observed.VolumeIdentity,observed.Missing,matches,observed.MissingDirectories,
                command.Version==2?new ArchiveSpaceSourceSet(source.Identity(),source.PrimaryPath,source.Paths.ToArray()):null);
            if(!result.ValidFor(command))throw new ArchiveOutputException("resource_limit");
            stage="observed";Record("finished");
            return new(true,Data:JsonSerializer.Serialize(result,JsonDefaults.Options));
        } catch(OperationCanceledException) { stage="cancelled";Record("failed");return new(false,"cancelled"); }
        catch(ArchiveOutputException failure) {
            stage=FailureCode(stage,failure.Stage,failure.NativeCode);Record("failed",failure.NativeCode);
            return new(false,stage,NativeErrorCode:failure.NativeCode);
        }
        catch(UnauthorizedAccessException failure) {
            stage=FailureCode(stage,"open_component",5);Record("failed",failure.HResult&0xffff);
            return new(false,stage,NativeErrorCode:failure.HResult&0xffff);
        }
        catch(IOException failure) {
            var native=failure.HResult&0xffff;stage=FailureCode(stage,"open_component",native);Record("failed",native);
            return new(false,stage,NativeErrorCode:native);
        }
        catch { stage="observation_failed";Record("failed");return new(false,"space_unavailable"); }
    }
    private static string FailureCode(string phase,string code,int? native)
    {
        if(code=="wrong_kind")return phase=="source"?"source_wrong_kind":"target_wrong_kind";
        if(code!="open_component")return code;
        return (phase=="source",native) switch {
            (true,2 or 3)=>"source_unavailable",(true,5)=>"source_denied",(true,32 or 33)=>"source_busy",(true,267)=>"source_wrong_kind",
            (false,5)=>"target_denied",(false,32 or 33)=>"target_busy",(false,267)=>"target_wrong_kind",_=>code
        };
    }
    private static void Diagnostic(IPlatformLog? log,ArchiveSpaceCommand command,string outcome,string stage,long elapsed,int? native)
    {
        try { log?.Write("Information","archive.space."+outcome,command.OperationId,new Dictionary<string,object?> {
            ["stage"]=stage,["elapsed_ms"]=elapsed,["native_error_code"]=native,
            ["session_id"]=command.SessionId,["generation"]=command.Generation }); }
        catch { /* Observational sink failure must never change native results. */ }
    }
}
