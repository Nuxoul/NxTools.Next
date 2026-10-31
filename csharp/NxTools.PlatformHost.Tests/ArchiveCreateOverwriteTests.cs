using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: GUID-owned tiny sources/targets and fixed helper copy only; original session/Job/native output.
// No Shell, GUI, source deletion, settings, registry or service operations.
internal static class ArchiveCreateOverwriteTests {
    private static void Check(bool value,string detail) { if(!value)throw new InvalidOperationException(detail); }
    private static string Package() {
        var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if(string.IsNullOrEmpty(worker)||!File.Exists(worker))throw new TestSkippedException("Explicit matching built worker required.");
        var root=Path.Combine(Path.GetTempPath(),"archive-product-"+Guid.NewGuid());Directory.CreateDirectory(root);
        File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));return root;
    }
    private static ProtocolEnvelope Wire(ArchiveJobCommand c)=>ProtocolEnvelope.Create("platform_command",new{command="archive_job",data=c},c.OperationId);
    private static ArchiveJobCommand Start(string root)=>new(){Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=7,Action="start",Request=new(){Kind="create",Paths=[Path.Combine(root,"source.txt")],Destination=Path.Combine(root,"out"),Create=new(){Format="zip",Level="normal",Method="deflate",StripSingleRoot=false,ExcludePatterns=[],TestAfterCreate=false,Name="created.zip"}}};
    private static ArchiveJobCommand Reply(ArchiveJobCommand start,ArchiveJobPrompt prompt,string? decision)=>start with{Action="reply",Request=null,Reply=new(){Ticket=prompt.Ticket,Kind="create_overwrite",Decision=decision,ApplyToAll=false}};
    private static async Task<ArchiveJobEvent> Next(ArchiveJobSession session)=>(await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
    private static async Task<ArchiveJobPrompt> Prompt(ArchiveJobSession session,ArchiveJobCommand command) {
        while(true) {
            var item=await Next(session);Check(item.Terminal is null,"expected create overwrite prompt, got terminal "+item.Terminal?.Code);
            Check(item.Kind!="started","worker started before create approval");
            if(item.Prompt is not {} prompt)continue;
            Check(item.OperationId==command.OperationId && item.SessionId==command.SessionId && item.Generation==command.Generation && prompt.Kind=="create_overwrite" && prompt.Ticket.OperationId==command.OperationId && prompt.Ticket.Generation==command.Generation && prompt.Ticket.PromptId!=Guid.Empty && prompt.ExistingBytes==8,"typed current native observation");
            var target=Path.Combine(command.Request!.Destination!,command.Request.Create!.Name);
            Check(prompt.Display is {IncomingMember:null} && prompt.Display.Target==target,"current immutable create target");
            Check(!prompt.ToString().Contains(target) && !prompt.Display!.ToString().Contains(target),"create prompt diagnostic redaction");return prompt;
        }
    }
    private static async Task<ArchiveJobTerminal> Finish(ArchiveJobSession session,bool mayStart=true) {
        while(true) {var item=await Next(session);Check(mayStart||item.Kind!="started","cancelled preflight launched worker");if(item.Terminal is {} terminal){Check(terminal.CleanupComplete,"actual cleanup required");return terminal;}}
    }
    private static void AssertOnlyTarget(string root,string bytes) {
        var output=Path.Combine(root,"out");Check(File.ReadAllText(Path.Combine(output,"created.zip"))==bytes,"prior/changed target altered");
        Check(Directory.GetFileSystemEntries(output).Length==1,"unexpected output/owned partial");Check(File.ReadAllText(Path.Combine(root,"source.txt"))=="incoming-payload","source changed");
    }
    public static async Task ActualConfirmationCancelOverwriteChangedTarget() {
        var root=Package();try {
            File.WriteAllText(Path.Combine(root,"source.txt"),"incoming-payload");Directory.CreateDirectory(Path.Combine(root,"out"));var target=Path.Combine(root,"out","created.zip");File.WriteAllText(target,"original");
            await using var session=ArchiveJobSession.ForOwnedTestPackage(root);
            var cancel=Start(root);Check(session.Dispatch(Wire(cancel)).Success,"create cancel admission");var first=await Prompt(session,cancel);
            var stale=Reply(cancel,first,"overwrite");Check(!session.Dispatch(Wire(stale with{Reply=stale.Reply! with{Ticket=first.Ticket with{Generation=8}}})).Success,"stale ticket admitted");
            Check(!session.Dispatch(Wire(stale with{Generation=8})).Success && !session.Dispatch(Wire(stale with{SessionId=Guid.NewGuid()})).Success,"stale job identity admitted");
            foreach(var invalid in new[]{stale.Reply! with{Decision="skip"},stale.Reply! with{Decision="auto_rename"},stale.Reply! with{ApplyToAll=true},stale.Reply! with{ApprovedLimit=1},stale.Reply! with{Password=[65]}})Check(!session.Dispatch(Wire(stale with{Reply=invalid})).Success,"unsupported create overwrite authority admitted");
            AssertOnlyTarget(root,"original");Check(session.Dispatch(Wire(Reply(cancel,first,null))).Success,"typed cancel reply");var cancelled=await Finish(session,false);Check(cancelled.Outcome=="cancelled" && cancelled.FilesCommitted==0 && cancelled.Journal is {Count:0},"create cancellation truth "+cancelled.Code);AssertOnlyTarget(root,"original");
            Check(!session.Dispatch(Wire(stale)).Success && !session.Events.TryRead(out _),"cancel terminal/ticket revived");
            var overwrite=Start(root);Check(session.Dispatch(Wire(overwrite)).Success,"overwrite admission");var second=await Prompt(session,overwrite);Check(session.Dispatch(Wire(Reply(overwrite,second,"overwrite"))).Success,"original target approval");var done=await Finish(session);
            Check(done.Outcome=="succeeded" && done.FilesCommitted==1 && done.Journal is {Count:1} && done.Journal[0] is {Replaced:true,State:"Committed"},"native atomic replacement facts "+done.Code);
            using(var zip=ZipFile.OpenRead(target)){Check(zip.Entries.Count==1,"independent entry count");using var reader=new StreamReader(zip.Entries[0].Open());Check(reader.ReadToEnd()=="incoming-payload","independent replacement bytes");}
            Check(!session.Dispatch(Wire(Reply(overwrite,second,"overwrite"))).Success && !session.Events.TryRead(out _),"overwrite terminal duplicated");
            File.WriteAllText(target,"original");var changed=Start(root);Check(session.Dispatch(Wire(changed)).Success,"changed target admission");var third=await Prompt(session,changed);
            File.Move(target,Path.Combine(root,"retained-original.zip"));File.WriteAllText(target,"original"); // Same bytes, different native object.
            Check(session.Dispatch(Wire(Reply(changed,third,"overwrite"))).Success,"bound original confirmation");var refused=await Finish(session);
            Check(refused.Outcome=="failed" && refused.Code=="target_changed" && refused.FilesCommitted==0 && refused.Journal is {Count:0},"changed target must refuse "+refused.Code);AssertOnlyTarget(root,"original");
            Check(!session.Events.TryRead(out _),"multiple actual terminals");
        }finally{Directory.Delete(root,true);}
    }
    public static async Task ActualDetachedPreflightCancellationRetainsSlot() {
        var root=Package();using var release=new ManualResetEventSlim();var entered=new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        try {
            File.WriteAllText(Path.Combine(root,"source.txt"),"incoming-payload");Directory.CreateDirectory(Path.Combine(root,"out"));File.WriteAllText(Path.Combine(root,"out","created.zip"),"original");
            await using var first=ArchiveJobSession.ForOwnedTestPackage(root);await using var second=ArchiveJobSession.ForOwnedTestPackage(root);await using var third=ArchiveJobSession.ForOwnedTestPackage(root);
            first.ProbeCreateOverwritePreflightForFixture(()=>{entered.TrySetResult();Check(release.Wait(TimeSpan.FromSeconds(10)),"preflight release deadline");});
            try {
                var blocked=Start(root);var receipt=await Task.Run(()=>first.Dispatch(Wire(blocked))).WaitAsync(TimeSpan.FromSeconds(2));Check(receipt.Success,"detached admission");await entered.Task.WaitAsync(TimeSpan.FromSeconds(2));
                Check(first.Dispatch(Wire(blocked with{Action="cancel",Request=null})).Success,"cancel while native preflight is blocked");
                while(first.Events.TryRead(out var item))Check(item.Payload.GetProperty("data").GetProperty("terminal").ValueKind==JsonValueKind.Null,"fabricated terminal before preflight cleanup");
                var other=Start(root);Check(second.Dispatch(Wire(other)).Success,"second original reservation");var prompt=await Prompt(second,other);
                Check(!third.Dispatch(Wire(Start(root))).Success,"cancelled blocked preflight must retain GLOBAL max2 slot");
                release.Set();var cancelled=await Finish(first,false);Check(cancelled.Outcome=="cancelled" && cancelled.FilesCommitted==0,"preflight cancellation result");AssertOnlyTarget(root,"original");
                Check(second.Dispatch(Wire(other with{Action="cancel",Request=null})).Success,"cancel original prompt operation");Check(!second.Dispatch(Wire(Reply(other,prompt,"overwrite"))).Success,"cancelled prompt cannot revive");Check((await Finish(second,false)).Outcome=="cancelled","second actual cleanup");
                Check(!first.Events.TryRead(out _) && !second.Events.TryRead(out _),"preflight once-only terminal");
            }finally{release.Set();}
        }finally{release.Set();Directory.Delete(root,true);}
    }
}
