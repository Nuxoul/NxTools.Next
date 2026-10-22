using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
namespace NxTools.PlatformHost.Tests;
// Owned tiny source/target/helper; original native Job/held output, no GUI/Shell/system changes.
internal static class ArchiveDestinationPreflightTests {
    private static void Check(bool value,string detail){if(!value)throw new InvalidOperationException(detail);}
    private static ProtocolEnvelope Wire(ArchiveJobCommand command)=>ProtocolEnvelope.Create("platform_command",new{command="archive_job",data=command},command.OperationId);
    private static async Task<ArchiveJobEvent> Next(ArchiveJobSession session)=>(await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(10))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
    public static async Task MissingAndLowSpaceRequireOriginalConfirmation(){
        var root=Path.Combine(Path.GetTempPath(),"archive-product-"+Guid.NewGuid());Directory.CreateDirectory(root);
        try{
            var worker=Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,"../../../../../target/debug/nxtools-archive-worker.exe"));File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));
            var source=Path.Combine(root,"tiny.zip");using(var zip=ZipFile.Open(source,ZipArchiveMode.Create)){var entry=zip.CreateEntry("a.txt");using var writer=new StreamWriter(entry.Open());writer.Write("payload");}
            foreach(var missing in new[]{true,false})foreach(var approve in new[]{false,true}){
                var output=Path.Combine(root,Guid.NewGuid().ToString(),"output");if(!missing)Directory.CreateDirectory(output);
                await using var session=ArchiveJobSession.ForOwnedTestPackage(root);if(!missing)session.ConfigureSpaceFixture(0);
                var command=new ArchiveJobCommand{Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=1,Action="start",Request=new(){Kind="extract",Paths=[source],Destination=output,Extract=new(){Strategy="here",SingleFileFolder=false,Conflict="skip",CopyMotw=false}}};
                Check(session.Dispatch(Wire(command)).Success,"preflight admission");ArchiveJobEvent next;
                do{next=await Next(session);Check(next.Terminal is null,"destination terminated before confirmation: "+next.Terminal?.Code+" native "+next.Terminal?.NativeErrorCode);}while(next.Prompt is null);
                var prompt=next.Prompt;
                Check(prompt.Kind=="destination"&&prompt.ErrorCode==(missing?"create_destination":"low_space")&&!File.Exists(Path.Combine(output,"a.txt"))&&Directory.Exists(output)==!missing,"original read-only destination prompt before create");
                Check(prompt.IncomingBytes==7 && (missing||prompt.ExistingBytes==0),"original plan must provide known payload size before output");
                var reply=new ArchiveJobReply{Ticket=prompt.Ticket,Kind="destination",Decision=approve?"overwrite":null,ApplyToAll=false};
                Check(!session.Dispatch(Wire(command with{Action="reply",Request=null,Reply=reply with{Ticket=reply.Ticket with{Generation=99}}})).Success,"stale generation cannot approve destination");
                Check(!session.Dispatch(Wire(command with{Action="reply",Request=null,SessionId=Guid.NewGuid(),Reply=reply})).Success,"stale session cannot approve destination");
                Check(session.Dispatch(Wire(command with{Action="reply",Request=null,Reply=reply})).Success,"current destination response");
                Check(!session.Dispatch(Wire(command with{Action="reply",Request=null,Reply=reply})).Success,"destination ticket consumed once");
                do{next=await Next(session);Check(next.Prompt is null,"destination approval must not create another prompt");}while(next.Terminal is null);
                var terminal=next.Terminal;
                Check(terminal.CleanupComplete,"destination terminal actual cleanup");
                if(approve)Check(terminal.Outcome=="succeeded"&&terminal.FilesCommitted==1&&terminal.BytesCommitted==7&&File.ReadAllText(Path.Combine(output,"a.txt"))=="payload","approved preflight original native publication "+terminal.Code);
                else Check(terminal.Outcome=="cancelled"&&terminal.FilesCommitted==0&&!File.Exists(Path.Combine(output,"a.txt"))&&Directory.Exists(output)==!missing,"cancel preflight zero effects");
            }
            var input=Path.Combine(root,"source.txt");File.WriteAllText(input,"payload");
            foreach(var approve in new[]{false,true}){
                var output=Path.Combine(root,"create-"+Guid.NewGuid(),"output");await using var session=ArchiveJobSession.ForOwnedTestPackage(root);
                var command=new ArchiveJobCommand{Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=1,Action="start",Request=new(){Kind="create",Paths=[input],Destination=output,Create=new(){Format="zip",Level="normal",Method="deflate",StripSingleRoot=false,ExcludePatterns=[],TestAfterCreate=false,Name="created.zip"}}};
                Check(session.Dispatch(Wire(command)).Success,"creation destination admission");ArchiveJobEvent next;do{next=await Next(session);Check(next.Terminal is null,"creation terminated before destination "+next.Terminal?.Code);}while(next.Prompt is null);
                Check(next.Prompt.Kind=="destination"&&next.Prompt.IncomingBytes is null&&!Directory.Exists(output),"creation unknown encoded size before any directory effects");
                Check(session.Dispatch(Wire(command with{Action="reply",Request=null,Reply=new(){Ticket=next.Prompt.Ticket,Kind="destination",Decision=approve?"overwrite":null,ApplyToAll=false}})).Success,"creation original destination reply");
                do{next=await Next(session);Check(next.Prompt is null,"unexpected creation prompt");}while(next.Terminal is null);
                Check(next.Terminal.CleanupComplete,"creation destination cleanup");
                if(approve){Check(next.Terminal.Outcome=="succeeded"&&next.Terminal.FilesCommitted==1,"creation destination publication "+next.Terminal.Code);using var zip=ZipFile.OpenRead(Path.Combine(output,"created.zip"));using var reader=new StreamReader(zip.Entries.Single().Open());Check(reader.ReadToEnd()=="payload","independent creation bytes");}
                else Check(next.Terminal.Outcome=="cancelled"&&next.Terminal.FilesCommitted==0&&!Directory.Exists(output),"creation cancelled destination changed output");
            }
        }finally{Directory.Delete(root,true);}
    }
}
