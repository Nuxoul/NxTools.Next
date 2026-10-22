using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;
namespace NxTools.PlatformHost.Tests;

// Effects: GUID-owned tiny sources/outputs/helper copy, original Windows Job/stdio/transactions.
// No Shell, recycle, settings, GUI, registry, service or user-owned path.
internal static class ArchiveJobTests {
    private static void Check(bool value,string detail){if(!value)throw new InvalidOperationException(detail);}
    private static string Package(){var root=Path.Combine(Path.GetTempPath(),"archive-product-"+Guid.NewGuid());Directory.CreateDirectory(root);var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE")??Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,"../../../../../target/debug/nxtools-archive-worker.exe"));Check(Path.IsPathFullyQualified(worker)&&File.Exists(worker),"parent-built worker required");File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));return root;}
    private static ProtocolEnvelope Wire(ArchiveJobCommand c)=>ProtocolEnvelope.Create("platform_command",new{command="archive_job",data=c},c.OperationId);
    private static ArchiveJobCommand Start(ArchiveJobRequest request)=>new(){Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=1,Action="start",Request=request};
    private static ArchiveJobCommand Control(ArchiveJobCommand original,string action,ArchiveJobReply? reply=null)=>original with{Action=action,Request=null,Reply=reply};
    private static async Task<ArchiveJobEvent> Next(ArchiveJobSession session)=>(await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
    private static async Task<ArchiveJobTerminal> Finish(ArchiveJobSession session){while(true){var e=await Next(session);if(e.Terminal is {} terminal)return terminal;}}
    private static ArchiveJobRequest Extract(string source,string destination,string policy="skip")=>new(){Kind="extract",Paths=[source],Destination=destination,Extract=new(){Strategy="smart",SingleFileFolder=false,Conflict=policy,CopyMotw=false}};
    public static Task StrictWireAndPickerCancellation(){
        var command=Start(new(){Kind="test",Paths=["C:\\owned\\tiny.zip"]});Check(ArchiveJobWire.Parse(Wire(command)).Request!.Kind=="test","roundtrip");
        var value=JsonSerializer.SerializeToNode(Wire(command),JsonDefaults.Options)!;value["payload"]!["data"]!["request"]!["delete_source"]=true;
        try{ArchiveJobWire.Parse(value.Deserialize<ProtocolEnvelope>(JsonDefaults.Options)!);throw new InvalidOperationException("unknown authority accepted");}catch(JsonException){}
        var create=Start(new(){Kind="create",Paths=["C:\\owned\\input"],Destination="C:\\owned\\out",Create=new(){Format="tar",Level="normal",StripSingleRoot=false,ExcludePatterns=[],TestAfterCreate=false,Name="out.tar"}});
        try{ArchiveJobWire.Parse(Wire(create));throw new InvalidOperationException("hidden level accepted");}catch(InvalidDataException){}
        var secret=new ArchiveJobCreate{Format="zip",Level="normal",Method="deflate",Password=[97,98,99],StripSingleRoot=false,ExcludePatterns=[],TestAfterCreate=false,Name="out.zip"};
        var serialized=JsonSerializer.SerializeToElement(secret,JsonDefaults.Options);Check(serialized.GetProperty("password").ValueKind==JsonValueKind.Array && !secret.ToString().Contains("abc"),"private array/redaction");
        var picker=new ArchivePickerCommand{Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=7,OwnerWindowHandle=1,Purpose="files"};
        using var cancelled=new CancellationTokenSource();cancelled.Cancel();var result=ArchiveJobPicker.Pick(picker,0,cancelled.Token);Check(result.Cancelled && result.Paths.Length==0 && result.Generation==7,"pre-cancel must not create dialog");
        return Task.CompletedTask;
    }
    public static async Task ActualCreateExtractTestJournal(){
        var root=Package();try{
            var source=Path.Combine(root,"source.txt");File.WriteAllText(source,"tiny-private-payload");var output=Path.Combine(root,"out");Directory.CreateDirectory(output);
            await using var session=ArchiveJobSession.ForOwnedTestPackage(root);
            var create=Start(new(){Kind="create",Paths=[source],Destination=output,Create=new(){Format="zip",Level="normal",Method="deflate",StripSingleRoot=false,ExcludePatterns=[],TestAfterCreate=true,Name="created.zip"}});
            Check(session.Dispatch(Wire(create)).Success,"native create admission");var made=await Finish(session);Check(made.Outcome=="succeeded" && made.CleanupComplete && made.FilesCommitted==1 && made.BytesCommitted>0 && made.Journal is {Count:1} && made.Journal[0].State=="Committed","native create terminal "+made.Code);
            var archive=Path.Combine(output,"created.zip");using(var zip=ZipFile.OpenRead(archive)){Check(zip.Entries.Count==1,"independent entry count");using var reader=new StreamReader(zip.Entries[0].Open());Check(reader.ReadToEnd()=="tiny-private-payload","independent created bytes");}
            var test=Start(new(){Kind="test",Paths=[archive]});Check(session.Dispatch(Wire(test)).Success,"test admission");var tested=await Finish(session);Check(tested.Outcome=="succeeded" && tested.FilesVerified==1 && tested.BytesVerified==20 && tested.FilesCommitted==0 && tested.CleanupComplete,"real test counts "+tested.Code);
            var extracted=Path.Combine(root,"extracted");Directory.CreateDirectory(extracted);var extract=Start(Extract(archive,extracted));Check(session.Dispatch(Wire(extract)).Success,"extract admission");var done=await Finish(session);Check(done.Outcome=="succeeded" && done.FilesCommitted==1 && done.BytesCommitted==20 && done.Journal is {Count:1} && done.CleanupComplete,"real extract "+done.Code);Check(File.ReadAllText(Path.Combine(extracted,"source.txt"))=="tiny-private-payload","exact extracted bytes");
            Check(!session.Dispatch(Wire(extract)).Success && !session.Events.TryRead(out _),"once-only terminal duplicate");
            using var lease=File.Open(archive,FileMode.Open,FileAccess.ReadWrite,FileShare.None);File.Delete(Path.Combine(root,ArchiveReadHelper.DeploymentName));
        }finally{Directory.Delete(root,true);}
    }
    public static async Task ActualConflictGlobalAdmissionCancel(){
        var root=Package();try{
            var archive=Path.Combine(root,"tiny.zip");using(var zip=ZipFile.Open(archive,ZipArchiveMode.Create)){var entry=zip.CreateEntry("a.txt");using var writer=new StreamWriter(entry.Open());writer.Write("incoming");}
            var a=Path.Combine(root,"a");var b=Path.Combine(root,"b");Directory.CreateDirectory(a);Directory.CreateDirectory(b);File.WriteAllText(Path.Combine(a,"a.txt"),"existing-a");File.WriteAllText(Path.Combine(b,"a.txt"),"existing-b");
            await using var first=ArchiveJobSession.ForOwnedTestPackage(root);await using var second=ArchiveJobSession.ForOwnedTestPackage(root);await using var third=ArchiveReadSession.ForOwnedTestPackage(root);
            var start=Start(Extract(archive,a,"ask"));Check(first.Dispatch(Wire(start)).Success,"first job");ArchiveJobEvent prompt;do{prompt=await Next(first);Check(prompt.Terminal is null,"expected conflict not terminal "+prompt.Terminal?.Code);}while(prompt.Prompt is null);
            Check(first.Dispatch(Wire(Control(start,"pause"))).Success,"real pause control");Check(first.Dispatch(Wire(Control(start,"resume"))).Success,"real resume control");
            var other=Start(Extract(archive,b,"ask"));Check(second.Dispatch(Wire(other)).Success,"second job");ArchiveJobEvent otherPrompt;do{otherPrompt=await Next(second);Check(otherPrompt.Terminal is null,"second expected prompt");}while(otherPrompt.Prompt is null);
            var read=ArchiveProductReadTests.Command("test",path:archive);Check(!third.Dispatch(read).Success,"jobs and reads must share GLOBAL two reservations");
            var reply=new ArchiveJobReply{Ticket=prompt.Prompt!.Ticket with{Generation=99},Kind="conflict",Decision="overwrite",ApplyToAll=false};Check(!first.Dispatch(Wire(Control(start,"reply",reply))).Success,"stale ticket cannot approve");
            Check(first.Dispatch(Wire(Control(start,"cancel"))).Success,"cancel original prompt");var cancelled=await Finish(first);Check(cancelled.Outcome=="cancelled" && cancelled.FilesCommitted==0 && cancelled.CleanupComplete,"cancel cleanup "+cancelled.Code);Check(File.ReadAllText(Path.Combine(a,"a.txt"))=="existing-a","prior target preserved");
            var approve=new ArchiveJobReply{Ticket=otherPrompt.Prompt!.Ticket,Kind="conflict",Decision="skip",ApplyToAll=true};Check(second.Dispatch(Wire(Control(other,"reply",approve))).Success,"original finite conflict port");var skipped=await Finish(second);Check(skipped.Outcome=="succeeded_with_warnings" && skipped.FilesSkipped==1 && skipped.FilesCommitted==0 && skipped.CleanupComplete,"warning projection "+skipped.Code);
            Check(!first.Dispatch(Wire(Control(start,"reply",reply))).Success,"retired prompt cannot revive");
        }finally{Directory.Delete(root,true);}
    }
    private static async Task<(ArchiveBrowserContext Context,ArchiveReadEntry[] Entries)> Browse(string root,string path) {
        await using var read=ArchiveReadSession.ForOwnedTestPackage(root);
        var wire=ArchiveProductReadTests.Command(path:path,version:2,limits:new(){BrowserContext=true});var identity=ArchiveReadWire.Parse(wire);Check(read.Dispatch(wire).Success,"browser admission");var entries=new List<ArchiveReadEntry>();
        while(true){var frame=(await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if(frame.Batch is {} batch){entries.AddRange(batch.Entries);Check(read.Dispatch(ArchiveProductReadTests.Command("ack",identity.OperationId,identity.SessionId,identity.Generation,sequence:batch.Sequence,version:2)).Success,"bounded browser credit");}
            if(frame.Terminal is {} terminal){Check(terminal.Status=="succeeded" && terminal.Browser is not null && terminal.CleanupComplete,"browser context "+terminal.Code);return (terminal.Browser!,entries.ToArray());}
        }
    }
    public static async Task ActualSelectedZipSevenzRarAndChangedSource(){
        var root=Package();try {
            var archive=Path.Combine(root,"selection.zip");using(var zip=ZipFile.Open(archive,ZipArchiveMode.Create)){foreach(var name in new[]{"folder/","folder/a.txt","other.txt"}){var e=zip.CreateEntry(name);if(!name.EndsWith('/')){using var w=new StreamWriter(e.Open());w.Write(name);}}}
            var sources=new List<string>{archive};
            var fixtures=Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,"../../../../../rust/crates/nxtools-archive/tests/fixtures/upstream"));
            foreach(var fixture in new[]{"non-solid.7z","rar4.rar","rar5.rar"}){var path=Path.Combine(root,fixture);File.Copy(Path.Combine(fixtures,fixture),path);sources.Add(path);}
            await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);
            foreach(var path in sources){
                var browser=await Browse(root,path);var selected=path==archive?browser.Entries.Single(e=>e.Kind=="Directory").Ordinal:browser.Entries.First(e=>e.Kind=="File").Ordinal;
                var destination=Path.Combine(root,"selected-"+Guid.NewGuid());Directory.CreateDirectory(destination);
                var options=new ArchiveJobExtract{Strategy="name",SingleFileFolder=false,Conflict="skip",CopyMotw=false,Selection=new(browser.Context,[selected])};var command=Start(new(){Kind="extract",Paths=[path],Destination=destination,Extract=options});
                Check(jobs.Dispatch(Wire(command)).Success,"selected admission");var terminal=await Finish(jobs);Check(terminal.Outcome=="succeeded" && terminal.FilesCommitted>=1 && terminal.CleanupComplete,"selected native terminal "+terminal.Code+" format "+browser.Context.Format);
                var files=Directory.GetFiles(destination,"*",SearchOption.AllDirectories);Check(files.Length==(int)terminal.FilesCommitted,"selected committed files only");Check(files.All(file=>Path.GetRelativePath(destination,file).StartsWith(Path.GetFileNameWithoutExtension(path)+Path.DirectorySeparatorChar)),"original name strategy prefix");
                if(path==archive){Check(files.Length==1 && File.ReadAllText(files[0])=="folder/a.txt","directory expands descendants not unrelated members");}
            }
            var original=await Browse(root,archive);File.Delete(archive);using(var replacement=ZipFile.Open(archive,ZipArchiveMode.Create)){replacement.CreateEntry("replacement.txt");}
            var changed=Path.Combine(root,"changed");Directory.CreateDirectory(changed);var stale=Start(new(){Kind="extract",Paths=[archive],Destination=changed,Extract=new(){Strategy="here",SingleFileFolder=false,Conflict="skip",CopyMotw=false,Selection=new(original.Context,[1])}});Check(jobs.Dispatch(Wire(stale)).Success,"stale admitted before native identity");var denied=await Finish(jobs);Check(denied.Code=="source_changed" && denied.FilesCommitted==0 && denied.CleanupComplete && Directory.GetFiles(changed,"*",SearchOption.AllDirectories).Length==0,"native stale binding refusal "+denied.Code);
        }finally{Directory.Delete(root,true);}
    }
    public static async Task ActualReadmissionShortfallNoOutput(){
        var root=Package();try {
            var archive=Path.Combine(root,"budget.zip");using(var zip=ZipFile.Open(archive,ZipArchiveMode.Create)){var e=zip.CreateEntry("a.txt");using var w=new StreamWriter(e.Open());w.Write("payload");}
            var fixtures=Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,"../../../../../rust/crates/nxtools-archive/tests/fixtures"));
            foreach(var source in new[]{archive,Path.Combine(fixtures,"independent.tar"),Path.Combine(fixtures,"independent.tar.gz"),Path.Combine(fixtures,"upstream","lzma.7z"),Path.Combine(fixtures,"upstream","rar5.rar")}){
                var destination=Path.Combine(root,"absent-"+Guid.NewGuid(),"output");await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);
                var request=Extract(source,destination) with {Limits=new(){MaxEntryBytes=1,MaxOutputBytes=1},Readmission=new(Guid.NewGuid(),null)};var command=Start(request);
                Check(jobs.Dispatch(Wire(command)).Success,"finite preflight admission");var terminal=await Finish(jobs);Check(terminal.Code=="resource_limit" && terminal.OutputShortfall is {Required.OutputBytes:>1} && terminal.FilesCommitted==0 && terminal.CleanupComplete && !Directory.Exists(destination),"native preflight no output or retry "+terminal.Code);
                var shortage=terminal.OutputShortfall!;Directory.CreateDirectory(destination);
                var approved=Start(request with{Limits=request.Limits with{MaxEntryBytes=shortage.Required.EntryBytes,MaxOutputBytes=shortage.Required.OutputBytes},Readmission=request.Readmission! with{ExpectedSource=shortage.Source}});
                Check(jobs.Dispatch(Wire(approved)).Success,"fresh source-bound output-only admission");var done=await Finish(jobs);
                Check(done.Outcome is "succeeded" or "succeeded_with_warnings" && done.FilesCommitted>0 && done.BytesCommitted==shortage.Required.OutputBytes && done.CleanupComplete,"approved native source-bound output "+done.Code);
            }
        }finally{Directory.Delete(root,true);}
    }
    private sealed class JobLog: NxTools.PlatformHost.Diagnostics.IPlatformLog {
        internal bool Throw;
        internal readonly System.Collections.Concurrent.ConcurrentQueue<string> Records=new();
        public void Write(string level,string name,Guid? correlationId=null,IReadOnlyDictionary<string,object?>? fields=null){Records.Enqueue(name+" "+correlationId+" "+JsonSerializer.Serialize(fields));if(Throw)throw new IOException("sink fixture");}
    }
    public static async Task ActualPasswordDiagnosticsAndNativeFailure(){
        var root=Package();try {
            var fixture=Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,"../../../../../rust/crates/nxtools-archive/tests/fixtures/combo-ae2-store-256.zip"));var path=Path.Combine(root,"private-sentinel.zip");File.Copy(fixture,path);
            var log=new JobLog();await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root,log);var destination=Path.Combine(root,"out");Directory.CreateDirectory(destination);var start=Start(Extract(path,destination));Check(jobs.Dispatch(Wire(start)).Success,"private interaction admission");
            ArchiveJobEvent prompt;do{prompt=await Next(jobs);Check(prompt.Terminal is null,"password preparation terminal "+prompt.Terminal?.Code);}while(prompt.Prompt is null);Check(prompt.Prompt.Kind=="password","original password prompt");
            var cancel=new ArchiveJobReply{Ticket=prompt.Prompt.Ticket,Kind="password",ApplyToAll=false};Check(jobs.Dispatch(Wire(Control(start,"reply",cancel))).Success,"null secret cancels original");var terminal=await Finish(jobs);Check(terminal.Outcome=="cancelled" && terminal.CleanupComplete && terminal.FilesCommitted==0,"private prompt cancel cleanup");
            var retry=Start(Extract(path,destination));Check(jobs.Dispatch(Wire(retry)).Success,"encrypted retry job");ArchiveJobEvent current;do{current=await Next(jobs);Check(current.Terminal is null,"encrypted preparation failed");}while(current.Prompt is null);
            var answer=new ArchiveJobReply{Ticket=current.Prompt.Ticket,Kind="password",ApplyToAll=false,Password=System.Text.Encoding.UTF8.GetBytes("synthetic-private-ZIP-secret")};Check(jobs.Dispatch(Wire(Control(retry,"reply",answer))).Success,"actual private password reply");var verified=await Finish(jobs);Check(verified.Outcome=="succeeded" && verified.FilesCommitted>0 && verified.CleanupComplete,"private encrypted extraction "+verified.Code);
            Check(log.Records.All(r=>!r.Contains("synthetic-private-ZIP-secret")),"secret leaked to production diagnostics");
            Check(log.Records.Count(r=>r.StartsWith("archive.job.terminal "))==2 && log.Records.Any(r=>r.StartsWith("archive.job.accepted ")) && log.Records.Any(r=>r.StartsWith("archive.job.started ")) && log.Records.All(r=>!r.Contains(root) && !r.Contains("private-sentinel") && !r.Contains("资料")),"correlated low-noise diagnostics privacy");
            log.Throw=true;var missing=Start(new(){Kind="test",Paths=[Path.Combine(root,"absent-sentinel.zip")]});Check(jobs.Dispatch(Wire(missing)).Success,"native failure admission");var denied=await Finish(jobs);Check(denied.Outcome=="failed" && denied.Code=="source_unavailable" && denied.NativeErrorCode==2 && denied.CleanupComplete,"native owner code survives worker failure and throwing sink "+denied.Code);
        }finally{Directory.Delete(root,true);}
    }
    public static async Task ActualCancelAfterPublicationRetainsNativeFacts(){
        var root=Package();try {
            var archive=Path.Combine(root,"publication.zip");using(var zip=ZipFile.Open(archive,ZipArchiveMode.Create)){var e=zip.CreateEntry("a.txt");using var w=new StreamWriter(e.Open());w.Write("published");}
            var destination=Path.Combine(root,"out");Directory.CreateDirectory(destination);await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);var start=Start(Extract(archive,destination));var cancel=false;
            jobs.ProbeOutputCommitForFixture(stage=>{if(stage=="native_published"&&!cancel){cancel=true;Check(jobs.Dispatch(Wire(Control(start,"cancel"))).Success,"late cancel accepted");}});
            Check(jobs.Dispatch(Wire(start)).Success,"publication admission");var terminal=await Finish(jobs);Check(cancel && terminal.Outcome=="cancelled" && terminal.Code=="cancelled_with_committed_output" && terminal.FilesCommitted==1 && terminal.BytesCommitted==9 && terminal.CleanupComplete,"native committed truth outranks cancellation "+terminal.Code);Check(File.ReadAllText(Path.Combine(destination,"a.txt"))=="published","publication retained");
        }finally{Directory.Delete(root,true);}
    }
    public static async Task ActualAllCreateFormatsAndLinkWarnings(){
        var root=Package();try {
            var source=Path.Combine(root,"a.txt");File.WriteAllText(source,"seven-bytes");var destination=Path.Combine(root,"created");Directory.CreateDirectory(destination);await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);
            foreach(var format in new[]{"seven_z","tar","tgz","txz","gz","xz"}){
                var command=Start(new(){Kind="create",Paths=[source],Destination=destination,Create=new(){Format=format,Level=format=="tar"?null:"fast",Solid=format=="seven_z"?false:null,EncryptHeader=format=="seven_z"?false:null,StripSingleRoot=false,ExcludePatterns=[],TestAfterCreate=false,Name="out."+format}});Check(jobs.Dispatch(Wire(command)).Success,"format create admission "+format);var terminal=await Finish(jobs);Check(terminal.Outcome=="succeeded" && terminal.FilesCommitted==1 && terminal.CleanupComplete,"format native create "+format+" "+terminal.Code);
                var test=Start(new(){Kind="test",Paths=[Path.Combine(destination,"out."+format)]});Check(jobs.Dispatch(Wire(test)).Success,"format test admission");var verified=await Finish(jobs);Check(verified.Outcome=="succeeded" && verified.FilesVerified==1 && verified.BytesVerified==11 && verified.CleanupComplete,"created format test "+format+" "+verified.Code);
            }
            var tar=Path.Combine(root,"links.tar");File.Copy(Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,"../../../../../rust/crates/nxtools-archive/tests/fixtures/independent.tar")),tar);
            var output=Path.Combine(root,"tar-output");Directory.CreateDirectory(output);var extract=Start(Extract(tar,output));Check(jobs.Dispatch(Wire(extract)).Success,"TAR link admission");var warning=await Finish(jobs);Check(warning.Outcome=="succeeded_with_warnings" && warning.FilesSkipped==1 && warning.FilesCommitted==1 && warning.CleanupComplete && !File.Exists(Path.Combine(output,"links","link")),"special/link warning retained "+warning.Code);
            var empty=Path.Combine(root,"empty.zip");using(var zip=ZipFile.Open(empty,ZipArchiveMode.Create)){}var absent=Path.Combine(root,"empty-absent");var none=Start(Extract(empty,absent));Check(jobs.Dispatch(Wire(none)).Success,"empty admission");var done=await Finish(jobs);Check(done.Outcome=="succeeded" && done.FilesCommitted==0 && done.CleanupComplete && !Directory.Exists(absent),"empty archive created spurious output");
        }finally{Directory.Delete(root,true);}
    }
    public static async Task ActualRiskAndPausePromptTimeout(){
        var root=Package();try {
            var archive=Path.Combine(root,"risk.zip");using(var zip=ZipFile.Open(archive,ZipArchiveMode.Create)){var e=zip.CreateEntry("a.bin",CompressionLevel.SmallestSize);using var output=e.Open();output.Write(new byte[8192]);}
            var destination=Path.Combine(root,"out");Directory.CreateDirectory(destination);await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);jobs.ConfigureControlFixture(2000,1024,2);var start=Start(Extract(archive,destination));Check(jobs.Dispatch(Wire(start)).Success,"risk admission");ArchiveJobEvent prompt;do{prompt=await Next(jobs);Check(prompt.Terminal is null,"risk terminal before original prompt "+prompt.Terminal?.Code);}while(prompt.Prompt is null);Check(prompt.Prompt.Kind=="risk" && prompt.Prompt.ApprovedLimit==(32ul<<30),"exact immutable risk ceiling");
            Check(jobs.Dispatch(Wire(Control(start,"pause"))).Success,"pause original worker");await Task.Delay(2300);Check(jobs.Dispatch(Wire(Control(start,"resume"))).Success,"pause/prompt wait incorrectly consumed running timeout");
            var answer=new ArchiveJobReply{Ticket=prompt.Prompt.Ticket,Kind="risk",ApplyToAll=false,ApprovedLimit=prompt.Prompt.ApprovedLimit};Check(jobs.Dispatch(Wire(Control(start,"reply",answer))).Success,"risk original reply");var terminal=await Finish(jobs);Check(terminal.Outcome=="succeeded" && terminal.FilesCommitted==1 && terminal.BytesCommitted==8192 && terminal.ElapsedMs>=2300 && terminal.CleanupComplete,"risk/pause running-time semantics "+terminal.Code);
            var seven=Path.Combine(root,"risk.7z");File.Copy(Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,"../../../../../rust/crates/nxtools-archive/tests/fixtures/interop/x86-bcj-lzma2.7z")),seven);
            var sevenOutput=Path.Combine(root,"seven-output");Directory.CreateDirectory(sevenOutput);await using var sevenJobs=ArchiveJobSession.ForOwnedTestPackage(root);sevenJobs.ConfigureControlFixture(2000,1,1);
            var sevenStart=Start(Extract(seven,sevenOutput));Check(sevenJobs.Dispatch(Wire(sevenStart)).Success,"7z risk admission");ArchiveJobEvent sevenPrompt;do{sevenPrompt=await Next(sevenJobs);Check(sevenPrompt.Terminal is null,"7z original risk prompt "+sevenPrompt.Terminal?.Code);}while(sevenPrompt.Prompt is null);
            Check(sevenPrompt.Prompt.Kind=="risk"&&sevenPrompt.Prompt.ApprovedLimit==(32ul<<30)&&Directory.GetFiles(sevenOutput,"*",SearchOption.AllDirectories).Length==0,"7z meaningful independent-block ratio before output");
            var sevenReply=new ArchiveJobReply{Ticket=sevenPrompt.Prompt.Ticket,Kind="risk",ApplyToAll=false,ApprovedLimit=sevenPrompt.Prompt.ApprovedLimit};Check(sevenJobs.Dispatch(Wire(Control(sevenStart,"reply",sevenReply))).Success,"7z original finite risk reply");
            var sevenDone=await Finish(sevenJobs);Check(sevenDone.Outcome=="succeeded"&&sevenDone.FilesCommitted==1&&sevenDone.BytesCommitted==4096&&sevenDone.CleanupComplete,"7z actual native risk publication "+sevenDone.Code);
        }finally{Directory.Delete(root,true);}
    }
    public static async Task ActualUniqueDestinationFolder(){
        var root=Package();try{
            var archive=Path.Combine(root,"named.zip");using(var zip=ZipFile.Open(archive,ZipArchiveMode.Create)){foreach(var name in new[]{"a.txt","b.txt"}){var entry=zip.CreateEntry(name);using var writer=new StreamWriter(entry.Open());writer.Write(name);}}
            var output=Path.Combine(root,"out");Directory.CreateDirectory(Path.Combine(output,"named"));File.WriteAllText(Path.Combine(output,"named","foreign.txt"),"keep");
            await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);
            foreach(var strategy in new[]{"name","smart"}){var command=Start(Extract(archive,output) with{Extract=new(){Strategy=strategy,SingleFileFolder=false,Conflict="skip",CopyMotw=false}});Check(jobs.Dispatch(Wire(command)).Success,"unique destination admission");var terminal=await Finish(jobs);Check(terminal.Outcome=="succeeded" && terminal.FilesCommitted==2 && terminal.CleanupComplete,"unique destination terminal "+terminal.Code);}
            Check(!File.Exists(Path.Combine(output,"named","a.txt")) && File.ReadAllText(Path.Combine(output,"named","foreign.txt"))=="keep","Name/Smart must not merge existing directory");
            Check(File.ReadAllText(Path.Combine(output,"named (2)","a.txt"))=="a.txt" && File.ReadAllText(Path.Combine(output,"named (3)","b.txt"))=="b.txt","finite unique folders preserve all outputs");
        }finally{Directory.Delete(root,true);}
    }
}
