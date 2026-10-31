using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects audit: GUID-owned package/ZIP/source fixtures, explicit current frozen worker/hash,
// original Job/browser/native one-shot publication only. No GUI/Shell/Registry/service/user files.
internal static class EditInteractionTests
{
    private static void Check(bool condition,string message){if(!condition)throw new InvalidOperationException(message);}
    private static ProtocolEnvelope Wire(ArchiveJobCommand command)=>ProtocolEnvelope.Create("platform_command",new{command="archive_job",data=command},command.OperationId);
    private static async Task<(ArchiveBrowserContext Context,ulong Ordinal)> Browse(string root,string path){
        await using var reads=ArchiveReadSession.ForOwnedTestPackage(root);var wire=ArchiveProductReadTests.Command(path:path,version:2,limits:new(){BrowserContext=true});var command=ArchiveReadWire.Parse(wire);var entries=new List<ArchiveReadEntry>();Check(reads.Dispatch(wire).Success,"original browser admission");
        while(true){var envelope=await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));var item=envelope.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;if(item.Batch is {} batch){entries.AddRange(batch.Entries);Check(reads.Dispatch(ArchiveProductReadTests.Command("ack",command.OperationId,command.SessionId,command.Generation,sequence:batch.Sequence,version:2)).Success,"ACK");}if(item.Terminal is {} terminal){Check(terminal.Status=="succeeded"&&terminal.CleanupComplete&&terminal.Browser is not null,"browser terminal");return(terminal.Browser!,entries.Single(e=>e.Name=="folder/same.txt").Ordinal);}}
    }
    internal static async Task ActualZipAskDecisionsSinglePublication(){
        var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");var expected=Environment.GetEnvironmentVariable("NXTOOLS_EDIT_INTERACTION_WORKER_SHA256");
        if(worker is null||!File.Exists(worker)||expected is not {Length:64})throw new TestSkippedException("Explicit current frozen interaction worker/hash required");
        Check(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(worker))).Equals(expected,StringComparison.OrdinalIgnoreCase),"matching frozen worker");
        foreach(var decision in new[]{"overwrite","skip","auto_rename","cancel"}){
            var root=Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!,"archive-product-"+Guid.NewGuid());Directory.CreateDirectory(root);Directory.CreateDirectory(Path.Combine(root,"archive"));File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));
            var path=Path.Combine(root,"archive","editable.zip");using(var zip=ZipFile.Open(path,ZipArchiveMode.Create)){zip.CreateEntry("folder/");using(var writer=new StreamWriter(zip.CreateEntry("folder/same.txt").Open()))writer.Write("old-content");using var keep=new StreamWriter(zip.CreateEntry("keep.txt").Open());keep.Write("keep");}
            var source=Path.Combine(root,"same.txt");File.WriteAllText(source,"new");var original=File.ReadAllBytes(path);var current=await Browse(root,path);await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);
            var command=new ArchiveJobCommand{Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=1,Action="start",Request=new(){Kind="modify",Paths=[path,source],Modify=new(current.Context,"add",[],"folder/")}};
            Check(jobs.Dispatch(Wire(command)).Success,"original Modify admission");var prompts=0;ArchiveJobTerminal? result=null;
            while(result is null){var envelope=await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));var item=envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;if(item.Prompt is {} prompt){prompts++;Check(prompt.Kind=="conflict"&&prompt.Display is {IncomingMember:"folder/same.txt",Target:"folder/same.txt"}&&prompt.ExistingBytes==11&&prompt.IncomingBytes==3,"private archive-relative exact conflict context");var reply=new ArchiveJobReply{Ticket=prompt.Ticket,Kind="conflict",Decision=decision=="cancel"?null:decision,ApplyToAll=decision!="cancel"};Check(jobs.Dispatch(Wire(command with{Action="reply",Request=null,Reply=reply})).Success,"current typed reply");}result=item.Terminal;}
            Check(prompts==1&&result.CleanupComplete,"one prompt / actual cleanup");
            if(decision is "skip" or "cancel"){Check(result.FilesCommitted==0&&File.ReadAllBytes(path).SequenceEqual(original),"zero-commit original retained");Check(decision=="skip"?result.Outcome=="succeeded_with_warnings"&&result.FilesSkipped==1:result.Outcome=="cancelled","truthful skip/cancel terminal");}
            else{Check(result.Outcome=="succeeded"&&result.FilesCommitted==1&&result.Journal is {Count:1}&&result.Journal[0] is {Replaced:true,Published:true,State:"Committed"},"one real native publication");using var zip=ZipFile.OpenRead(path);var target=decision=="overwrite"?"folder/same.txt":"folder/same (2).txt";using var reader=new StreamReader(zip.GetEntry(target)!.Open());Check(reader.ReadToEnd()=="new","independent .NET decoded incoming");using var keep=new StreamReader(zip.GetEntry("keep.txt")!.Open());Check(keep.ReadToEnd()=="keep","unchanged independent content");Check(zip.Entries.Count== (decision=="overwrite"?3:4),"exact candidate member count");}
            File.WriteAllText(Path.Combine(root,"terminal.json"),JsonSerializer.Serialize(result,JsonDefaults.Options));
            var evidence=Environment.GetEnvironmentVariable("NXTOOLS_EDIT_INTERACTION_EVIDENCE");if(evidence is not null){Directory.CreateDirectory(evidence);File.AppendAllText(Path.Combine(evidence,"roots.txt"),root+Environment.NewLine);}
        }
    }
    private static string MatchingWorker(){
        var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");var expected=Environment.GetEnvironmentVariable("NXTOOLS_EDIT_INTERACTION_WORKER_SHA256");
        if(worker is null||!File.Exists(worker)||expected is not {Length:64})throw new TestSkippedException("Explicit matching EIC worker/hash required");
        Check(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(worker))).Equals(expected,StringComparison.OrdinalIgnoreCase),"matching worker");return worker;
    }
    private static string OwnedRoot(string worker){var root=Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!,"archive-product-"+Guid.NewGuid());Directory.CreateDirectory(root);Directory.CreateDirectory(Path.Combine(root,"archive"));File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));return root;}
    private static async Task<ArchiveJobTerminal> Terminal(ArchiveJobSession jobs,ArchiveJobCommand command,string? decision=null){
        Check(jobs.Dispatch(Wire(command)).Success,"original Job admission");ArchiveJobTerminal? result=null;var prompts=0;
        while(result is null){var envelope=await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));var item=envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;if(item.Prompt is {} prompt){prompts++;Check(decision is not null&&prompt.Kind=="conflict","only original conflict prompt");Check(jobs.Dispatch(Wire(command with{Action="reply",Request=null,Reply=new(){Ticket=prompt.Ticket,Kind="conflict",Decision=decision,ApplyToAll=true}})).Success,"original reply");}result=item.Terminal;}
        Check(prompts==(decision is null?0:1),"exact prompt count");return result;
    }
    private static void Evidence(string root,ArchiveJobTerminal terminal){File.WriteAllText(Path.Combine(root,"terminal.json"),JsonSerializer.Serialize(terminal,JsonDefaults.Options));var evidence=Environment.GetEnvironmentVariable("NXTOOLS_EIC_REPAIR_EVIDENCE");if(evidence is not null){Directory.CreateDirectory(evidence);File.AppendAllText(Path.Combine(evidence,"roots.txt"),root+Environment.NewLine);}}
    internal static async Task EicAc01DirectoryMergeZeroPublication(){
        var worker=MatchingWorker();
        foreach(var mode in new[]{"implicit-skip","explicit-skip","new-empty","real-file"}){
            var root=OwnedRoot(worker);var path=Path.Combine(root,"archive","editable.zip");
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create)){if(mode=="explicit-skip")zip.CreateEntry("folder/");using(var writer=new StreamWriter(zip.CreateEntry("folder/same.txt").Open()))writer.Write("old-content");using var keep=new StreamWriter(zip.CreateEntry("untouched.txt").Open());keep.Write("keep");}
            var source=Path.Combine(root,"inputs",mode=="new-empty"?"new-empty":"folder");Directory.CreateDirectory(source);if(mode!="new-empty")File.WriteAllText(Path.Combine(source,"same.txt"),"new");if(mode=="real-file")File.WriteAllText(Path.Combine(source,"fresh.txt"),"fresh");
            var before=File.ReadAllBytes(path);var current=await Browse(root,path);await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);var flushes=0;var publications=0;
            jobs.ProbeOutputCommitForFixture(stage=>{if(stage=="before_flush")flushes++;if(stage=="native_published")publications++;});
            var command=new ArchiveJobCommand{Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=1,Action="start",Request=new(){Kind="modify",Paths=[path,source],Modify=new(current.Context,"add",[],null)}};
            var terminal=await Terminal(jobs,command,mode=="new-empty"?null:"skip");Evidence(root,terminal);Check(terminal.CleanupComplete,"actual owned cleanup");
            if(mode.EndsWith("-skip",StringComparison.Ordinal)){Check(terminal.Outcome=="succeeded_with_warnings"&&terminal.FilesSkipped==1&&terminal.FilesCommitted==0&&terminal.Journal is {Count:0}&&flushes==0&&publications==0&&File.ReadAllBytes(path).SequenceEqual(before),"directory merge/allSkip has zero candidate/native publication and exact original");}
            else{Check(terminal.FilesCommitted==1&&terminal.Journal is {Count:1}&&flushes==1&&publications==1,"one actual native publication");using var zip=ZipFile.OpenRead(path);Check(zip.Entries.Count==3&&zip.GetEntry("folder/") is null,"do not materialize existing implicit folder");var target=mode=="new-empty"?"new-empty/":"folder/fresh.txt";Check(zip.GetEntry(target) is not null,"new empty directory or real file retained");if(mode=="real-file"){using var reader=new StreamReader(zip.GetEntry(target)!.Open());Check(reader.ReadToEnd()=="fresh","independent new file bytes");}using var old=new StreamReader(zip.GetEntry("folder/same.txt")!.Open());Check(old.ReadToEnd()=="old-content","skipped file unchanged");}
        }
    }
    private static async Task SeedEdit(string root,string path,string format){
        var source=Path.Combine(root,"seed","folder");Directory.CreateDirectory(source);File.WriteAllText(Path.Combine(source,"same.txt"),"old-content");
        if(format=="Zip"){using var zip=ZipFile.Open(path,ZipArchiveMode.Create);using var writer=new StreamWriter(zip.CreateEntry("folder/same.txt").Open());writer.Write("old-content");return;}
        System.Diagnostics.ProcessStartInfo start;
        if(format=="Tar"){
            var python=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE");if(python is null||!File.Exists(python))throw new TestSkippedException("Explicit Python oracle required");start=new(python);start.ArgumentList.Add("-c");start.ArgumentList.Add("import sys,tarfile,io;t=tarfile.open(sys.argv[1],'w',format=tarfile.USTAR_FORMAT);h=tarfile.TarInfo('folder/same.txt');h.size=11;t.addfile(h,io.BytesIO(b'old-content'));t.close()");start.ArgumentList.Add(path);
        }else{
            await using(var seedJobs=ArchiveJobSession.ForOwnedTestPackage(root)){var created=await Terminal(seedJobs,new(){Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=1,Action="start",Request=new(){Kind="create",Paths=[Path.Combine(root,"seed")],Destination=Path.GetDirectoryName(path),Create=new(){Format="seven_z",Name=Path.GetFileName(path),Level="fast",Solid=false,StripSingleRoot=true,ExcludePatterns=[],TestAfterCreate=false}}});Check(created.Outcome=="succeeded"&&created.FilesCommitted==1&&created.CleanupComplete,"original qualified 7z seed route");}
            var oracle=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE");if(oracle is null||!File.Exists(oracle))throw new TestSkippedException("Explicit stock7Zip oracle required");start=new(oracle);foreach(var arg in new[]{"t","-bd",path})start.ArgumentList.Add(arg);
        }
        start.UseShellExecute=false;start.RedirectStandardOutput=true;start.RedirectStandardError=true;using var child=System.Diagnostics.Process.Start(start)!;var stdout=child.StandardOutput.ReadToEndAsync();var stderr=child.StandardError.ReadToEndAsync();await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(30));await stdout;Check(child.ExitCode==0&&(await stderr).Length==0,"independent tiny seed");
    }
    internal static async Task EicAc02NativeCancelledReplyOnly(){
        var worker=MatchingWorker();foreach(var format in new[]{"Zip","Tar","SevenZ"}){
            var root=OwnedRoot(worker);var path=Path.Combine(root,"archive","editable"+(format=="Zip"?".zip":format=="Tar"?".tar":".7z"));await SeedEdit(root,path,format);var before=File.ReadAllBytes(path);var current=await Browse(root,path);await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);var replies=0;var published=0;
            jobs.ProbeOutputCommitForFixture(stage=>{if(stage=="native_published")published++;if(stage=="edit_source_rechecked"){replies++;throw new ArchiveOutputException("cancelled",state:ArchiveCommitState.FailedBeforeCommit);}});
            var command=new ArchiveJobCommand{Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=1,Action="start",Request=new(){Kind="modify",Paths=[path],Modify=new(current.Context,"rename",[current.Ordinal],"renamed.txt")}};
            var terminal=await Terminal(jobs,command);Evidence(root,terminal);
            Check(replies==1&&published==0&&terminal.Outcome=="cancelled"&&terminal.Code=="cancelled"&&terminal.FilesCommitted==0&&terminal.CleanupComplete&&File.ReadAllBytes(path).SequenceEqual(before),"native reply-only cancellation reaches original Job without cancel control");
            Check(Directory.GetFiles(Path.GetDirectoryName(path)!,".nxpart-*").Length==0,"only owned partial removed");
        }
    }
}
