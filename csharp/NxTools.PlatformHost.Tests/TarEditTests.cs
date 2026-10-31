using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects audit: GUID-owned temporary archives and original owned-package Job/worker/native
// output only. Explicit frozen worker/hash and independent Python oracle. No product/Explorer,
// Registry/service/ACL/UserChoice/Recycle operations. Unknown candidate deliberately retained.
internal static class TarEditTests
{
    private static void Check(bool value,string detail) { if(!value)throw new InvalidOperationException(detail); }
    private static ProtocolEnvelope Wire(ArchiveJobCommand command)=>ProtocolEnvelope.Create("platform_command",new {command="archive_job",data=command},command.OperationId);
    private static ArchiveJobCommand Start(ArchiveJobRequest request)=>new(){Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=1,Action="start",Request=request};
    internal static Task StrictWireAndCapabilities()
    {
        foreach(var format in new[]{"Tar","TarGz","TarXz"}) {
            var browser=new ArchiveBrowserContext(Guid.NewGuid(),format,"",new("v1:fixture",new uint[32],100,0));
            foreach(var action in new[]{"add","delete","replace_content","rename","new_directory"}) {
                var edit=new ArchiveJobModify(browser,action,action is "delete" or "replace_content" or "rename"?[1]:[],action is "rename" or "new_directory"?"new/":null);
                var paths=action is "add" or "replace_content"?new[]{@"C:\owned\source.tar",@"C:\owned\input"}:new[]{@"C:\owned\source.tar"};
                var request=new ArchiveJobRequest{Kind="modify",Paths=paths,Modify=edit};
                Check(ArchiveJobWire.Parse(Wire(Start(request))).Request!.Modify!.Browser.Format==format,"TAR wire roundtrip");
            }
            Check(!ArchiveJobWire.ValidModify(new(browser,"comment",[],"forbidden"),1),"TAR comment refused");
            Check(ArchiveWorkerRunner.ModificationFormatMatches("modify_tar",format),"actual TAR route");
        }
        foreach(var format in new[]{"TarBz2","TarZstd","Gzip","Xz","Rar5","Zip","SevenZ"})Check(!ArchiveWorkerRunner.ModificationFormatMatches("modify_tar",format),"no suffix/read grant");
        var capabilities=new[]{"source_lease_v1","budget_reservations_v1","source_tree_v1","partial_read_v1","tar_modify_v1"};
        Check(ArchiveWorkerRunner.SupportsCapabilities("modify_tar",JsonSerializer.SerializeToElement(capabilities)),"TAR capabilities");
        foreach(var required in capabilities)Check(!ArchiveWorkerRunner.SupportsCapabilities("modify_tar",JsonSerializer.SerializeToElement(capabilities.Where(c=>c!=required))),"fail-closed missing capability");
        return Task.CompletedTask;
    }
    private sealed class Log:NxTools.PlatformHost.Diagnostics.IPlatformLog {
        internal readonly System.Collections.Concurrent.ConcurrentQueue<string> Lines=new();
        public void Write(string level,string eventName,Guid? correlationId=null,IReadOnlyDictionary<string,object?>? fields=null)=>Lines.Enqueue(JsonSerializer.Serialize(new{eventName,correlationId,fields}));
    }
    private sealed class Fixture
    {
        internal readonly string Root,Archive,Format;
        internal readonly Log Diagnostics=new();
        private int sequence;
        internal Fixture(string format) {
            Format=format;
            var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
            var expected=Environment.GetEnvironmentVariable("NXTOOLS_TAR_EDIT_WORKER_SHA256");
            if(worker is null||!File.Exists(worker)||expected is not {Length:64})throw new TestSkippedException("Explicit matching frozen TAR worker and hash required");
            Check(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(worker))).Equals(expected,StringComparison.OrdinalIgnoreCase),"frozen worker identity");
            Root=Path.Combine(Path.GetTempPath(),"archive-product-"+Guid.NewGuid());Directory.CreateDirectory(Root);
            Directory.CreateDirectory(Path.Combine(Root,"archive"));Directory.CreateDirectory(Path.Combine(Root,"evidence"));
            File.Copy(worker,Path.Combine(Root,ArchiveReadHelper.DeploymentName));
            Archive=Path.Combine(Root,"archive",format switch{"TarGz"=>"editable.tgz","TarXz"=>"editable.txz",_=>"editable.tar"});
            File.WriteAllText(Path.Combine(Root,"evidence","worker.sha256"),expected);
            var evidence=Environment.GetEnvironmentVariable("NXTOOLS_TAR_EDIT_EVIDENCE");if(evidence is not null){Directory.CreateDirectory(evidence);File.AppendAllText(Path.Combine(evidence,"roots.txt"),Root+Environment.NewLine);}
        }
        internal ArchiveJobSession Jobs()=>ArchiveJobSession.ForOwnedTestPackage(Root,Diagnostics);
        internal async Task<string> Python(params string[] args) {
            var python=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE");
            var script=Environment.GetEnvironmentVariable("NXTOOLS_TAR_EDIT_ORACLE");
            if(python is null||!File.Exists(python)||script is null||!File.Exists(script))throw new TestSkippedException("Explicit independent CPython TAR oracle required");
            var start=new ProcessStartInfo(python){UseShellExecute=false,RedirectStandardOutput=true,RedirectStandardError=true};start.ArgumentList.Add(script);foreach(var arg in args)start.ArgumentList.Add(arg);
            using var child=Process.Start(start)!;var output=child.StandardOutput.ReadToEndAsync();var error=child.StandardError.ReadToEndAsync();await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(30));var text=await output;Check(child.ExitCode==0,"independent TAR oracle: "+await error);return text;
        }
        internal async Task Seed()=>await Python("seed",Archive,Format);
        internal string Input(string name,string content){var path=Path.Combine(Root,"inputs",name);Directory.CreateDirectory(Path.GetDirectoryName(path)!);File.WriteAllText(path,content,new UTF8Encoding(false));return path;}
        internal async Task<(ArchiveBrowserContext Context,ArchiveReadEntry[] Entries)> Browse(){
            await using var reads=ArchiveReadSession.ForOwnedTestPackage(Root,Diagnostics);
            var wire=ArchiveProductReadTests.Command(path:Archive,version:2,limits:new(){BrowserContext=true});var command=ArchiveReadWire.Parse(wire);var entries=new List<ArchiveReadEntry>();Check(reads.Dispatch(wire).Success,"browser admission");
            while(true){var envelope=await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));var item=envelope.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                if(item.Batch is {} batch){entries.AddRange(batch.Entries);Check(reads.Dispatch(ArchiveProductReadTests.Command("ack",command.OperationId,command.SessionId,command.Generation,sequence:batch.Sequence,version:2)).Success,"metadata ACK");}
                if(item.Terminal is not {} terminal)continue;Check(terminal.Status=="succeeded"&&terminal.CleanupComplete&&terminal.Browser?.Format==Format,"browser terminal: "+terminal.Code);return(terminal.Browser!,entries.ToArray());}
        }
        internal ArchiveJobRequest Edit(ArchiveBrowserContext context,string action,ulong[]? ids=null,string? value=null,string[]? additions=null,string? prefix=null)=>new(){Kind="modify",Paths=new[]{Archive}.Concat(additions??[]).ToArray(),Modify=new(context,action,ids??[],value,prefix)};
        internal async Task<ArchiveJobTerminal> Run(ArchiveJobSession jobs,ArchiveJobRequest request,Action<ArchiveJobCommand>? before=null,string? conflict=null,Action? after=null){
            var index=++sequence;var original=File.ReadAllBytes(Archive);File.WriteAllBytes(Path.Combine(Root,"evidence",index+".before"),original);
            var command=Start(request);before?.Invoke(command);Check(jobs.Dispatch(Wire(command)).Success,"job admission");var phases=new List<string>();
            while(true){var envelope=await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));var item=envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
                if(item.Progress is {} progress&&!phases.Contains(progress.Phase))phases.Add(progress.Phase);
                if(item.Prompt is {} prompt){Check(prompt.Kind=="conflict"&&conflict is not null,"TAR must not prompt password");Check(jobs.Dispatch(Wire(command with{Action="reply",Request=null,Reply=new(){Ticket=prompt.Ticket,Kind="conflict",Decision=conflict,ApplyToAll=true}})).Success,"conflict reply");}
                if(item.Terminal is not {} terminal)continue;
                after?.Invoke();
                if(terminal.FilesCommitted==1){Check(terminal.Journal is {Count:1}&&terminal.Journal[0].Replaced,"one same-path replacement");Check(new[]{"edit_planning","edit_reading","edit_reencoding","edit_verifying","edit_committing"}.All(phases.Contains),"real fixed phases: "+string.Join(',',phases));}
                Check(Diagnostics.Lines.All(line=>!line.Contains(Root)&&!line.Contains("kept-data")),"diagnostic privacy");
                File.WriteAllText(Path.Combine(Root,"evidence",index+".terminal.json"),JsonSerializer.Serialize(terminal,JsonDefaults.Options));File.WriteAllLines(Path.Combine(Root,"evidence","diagnostics.jsonl"),Diagnostics.Lines);File.WriteAllText(Path.Combine(Root,"evidence",index+".after.sha256"),Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(Archive))));return terminal;
            }
        }
        internal async Task Oracle(Dictionary<string,string?> expected,bool oldMetadata=true){var text=await Python("inspect",Archive);File.WriteAllText(Path.Combine(Root,"evidence","oracle-"+sequence+".json"),text);using var parsed=JsonDocument.Parse(text);var actual=parsed.RootElement;Check(actual.EnumerateObject().Count()==expected.Count,"exact member count");foreach(var pair in expected){Check(actual.TryGetProperty(pair.Key,out var item),"exact member");Check(item.GetProperty("content").GetString()==pair.Value,"exact content");if(oldMetadata&&pair.Key=="keep.txt"){Check(item.GetProperty("uid").GetInt32()==123&&item.GetProperty("gid").GetInt32()==456&&item.GetProperty("mode").GetInt32()==0x1a0,"preserved identity/mode");Check(item.GetProperty("pax").GetProperty("mtime").GetString()=="1700000000.123456789","exact PAX precision");}}}
        internal void Unchanged(byte[] before,ArchiveJobTerminal terminal){Check(terminal.FilesCommitted==0&&terminal.CleanupComplete&&File.ReadAllBytes(Archive).SequenceEqual(before),"original retained/cleanup");Check(Directory.GetFiles(Path.GetDirectoryName(Archive)!,".nxpart-*").Length==0,"owned unpublished partial cleanup");}
    }
    private static Dictionary<string,string?> Manifest()=>new(){["keep.txt"]="kept-data",["folder/"]=null,["folder/child.txt"]="child-data",["implicit/child"]="implicit-data",["empty/"]=null,["zero"]=""};
    private static void Published(ArchiveJobTerminal terminal)=>Check(terminal.Outcome=="succeeded"&&terminal.FilesCommitted==1&&terminal.CleanupComplete&&terminal.Journal is {Count:1}&&terminal.Journal[0] is {State:"Committed",Published:true},"native publication: "+terminal.Code);
    internal static async Task ActualActionsAndEmpty(){
        foreach(var format in new[]{"Tar","TarGz","TarXz"}){
            var f=new Fixture(format);await f.Seed();await using var jobs=f.Jobs();var expected=Manifest();await f.Oracle(expected);
            var current=await f.Browse();Published(await f.Run(jobs,f.Edit(current.Context,"rename",[current.Entries.Single(e=>e.Name.TrimEnd('/')=="folder").Ordinal],"改名/")));expected.Remove("folder/");expected.Remove("folder/child.txt");expected["改名/"]=null;expected["改名/child.txt"]="child-data";await f.Oracle(expected);
            current=await f.Browse();Published(await f.Run(jobs,f.Edit(current.Context,"rename",[current.Entries.Single(e=>e.Name=="implicit/child").Ordinal],"implicit-new/",prefix:"implicit/")));expected.Remove("implicit/child");expected["implicit-new/child"]="implicit-data";await f.Oracle(expected);
            current=await f.Browse();Published(await f.Run(jobs,f.Edit(current.Context,"replace_content",[current.Entries.Single(e=>e.Name=="keep.txt").Ordinal],additions:[f.Input("replacement","replacement")])));expected["keep.txt"]="replacement";await f.Oracle(expected);
            current=await f.Browse();Published(await f.Run(jobs,f.Edit(current.Context,"add",additions:[f.Input("added","new")])));expected["added"]="new";await f.Oracle(expected);
            current=await f.Browse();Published(await f.Run(jobs,f.Edit(current.Context,"new_directory",value:"new/")));expected["new/"]=null;await f.Oracle(expected);
            current=await f.Browse();Published(await f.Run(jobs,f.Edit(current.Context,"delete",current.Entries.Select(e=>e.Ordinal).ToArray())));await f.Oracle(new());Check((await f.Browse()).Entries.Length==0,"valid empty TAR");
        }
    }
    internal static async Task ActualSourceCancellationAndHandoff(){
        var f=new Fixture("Tar");await f.Seed();await using var jobs=f.Jobs();var current=await f.Browse();var before=File.ReadAllBytes(f.Archive);
        FileStream? writer=null;try{var refused=await f.Run(jobs,f.Edit(current.Context,"delete",[0]),_=>writer=new(f.Archive,FileMode.Open,FileAccess.ReadWrite,FileShare.ReadWrite),after:()=>{writer?.Dispose();writer=null;});Check(refused.Code=="source_changed","writer conflict");f.Unchanged(before,refused);}finally{writer?.Dispose();}
        var seen=false;var cancelled=await f.Run(jobs,f.Edit(current.Context,"new_directory",value:"cancelled/"),command=>jobs.ProbeOutputCommitForFixture(stage=>{if(stage!="edit_source_rechecked")return;seen=true;try{using var blocked=File.Open(f.Archive,FileMode.Open,FileAccess.Write,FileShare.ReadWrite|FileShare.Delete);throw new InvalidOperationException("source FILE released too early");}catch(IOException){}try{Directory.Move(Path.GetDirectoryName(f.Archive)!,Path.Combine(f.Root,"moved"));throw new InvalidOperationException("parent released");}catch(IOException){}Check(jobs.Dispatch(Wire(command with{Action="cancel",Request=null})).Success,"cancel receipt");}));Check(seen&&cancelled.Outcome=="cancelled","final held-lease cancellation");f.Unchanged(before,cancelled);
        jobs.ProbeOutputCommitForFixture(_=>{});var stale=current.Context;Published(await f.Run(jobs,f.Edit(current.Context,"rename",[0],"renamed")));before=File.ReadAllBytes(f.Archive);var changed=await f.Run(jobs,f.Edit(stale,"delete",[0]));Check(changed.Code=="source_changed","stale source");f.Unchanged(before,changed);
    }
    internal static async Task ActualNativeFailureUnknownAndLateCancel(){
        foreach(var code in new[]{5,64}){var f=new Fixture("TarGz");await f.Seed();await using var jobs=f.Jobs();var current=await f.Browse();var before=File.ReadAllBytes(f.Archive);var count=0;jobs.ProbeOutputCommitForFixture(stage=>{if(stage=="commit_in_flight"){count++;throw new ArchiveOutputException("replace_file",code);}});var terminal=await f.Run(jobs,f.Edit(current.Context,"new_directory",value:"new/"));Check(count==1&&terminal.FilesCommitted==0&&terminal.NativeErrorCode==code&&File.ReadAllBytes(f.Archive).SequenceEqual(before),"native one-shot failure facts");if(code==5){Check(terminal.CommitState=="FailedBeforeCommit","denied truth");f.Unchanged(before,terminal);}else{Check(terminal.CommitState=="CommitOutcomeUnknown"&&terminal.Code=="commit_outcome_unknown"&&terminal.Journal is {Count:1}&&terminal.Journal[0].Published is null,"unknown no retry");Check(Directory.GetFiles(Path.GetDirectoryName(f.Archive)!,".nxpart-*").Length==1,"unknown evidence retained");}}
        var late=new Fixture("TarXz");await late.Seed();await using var lateJobs=late.Jobs();var selected=await late.Browse();var observed=false;var result=await late.Run(lateJobs,late.Edit(selected.Context,"new_directory",value:"published/"),command=>lateJobs.ProbeOutputCommitForFixture(stage=>{if(stage=="native_published"){observed=true;Check(lateJobs.Dispatch(Wire(command with{Action="cancel",Request=null})).Success,"late cancel receipt");}}));Check(observed&&result.FilesCommitted==1&&result.Journal is {Count:1}&&result.Journal[0].Published==true,"late cancel preserves actual publication");var expected=Manifest();expected["published/"]=null;await late.Oracle(expected);
    }
    internal static async Task ActualConflictAndInjectedLowSpace(){
        var f=new Fixture("Tar");await f.Seed();await using var jobs=f.Jobs();var expected=Manifest();
        var same=f.Input("keep.txt","incoming-conflict");var current=await f.Browse();var before=File.ReadAllBytes(f.Archive);
        var skipped=await f.Run(jobs,f.Edit(current.Context,"add",additions:[same]),conflict:"skip");Check(skipped.Outcome=="succeeded_with_warnings"&&skipped.FilesSkipped==1&&skipped.FilesCommitted==0,"original all-skipped warning");f.Unchanged(before,skipped);await f.Oracle(expected);
        current=await f.Browse();Published(await f.Run(jobs,f.Edit(current.Context,"add",additions:[same]),conflict:"auto_rename"));expected["keep (2).txt"]="incoming-conflict";await f.Oracle(expected);
        current=await f.Browse();Published(await f.Run(jobs,f.Edit(current.Context,"add",additions:[same]),conflict:"overwrite"));expected["keep.txt"]="incoming-conflict";await f.Oracle(expected);
        current=await f.Browse();before=File.ReadAllBytes(f.Archive);var observed=false;
        jobs.ProbeOutputCommitForFixture(stage=>{if(stage=="before_flush"){observed=true;throw new ArchiveOutputException("disk_full",112);}});
        var low=await f.Run(jobs,f.Edit(current.Context,"new_directory",value:"lowspace/"));Check(observed&&low.Outcome=="failed"&&low.NativeErrorCode==112&&low.FilesCommitted==0,"injected native disk-full before FILE release");f.Unchanged(before,low);
    }
}
