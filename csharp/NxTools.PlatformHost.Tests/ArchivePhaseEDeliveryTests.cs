using System.Diagnostics;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: exact built worker copy, GUID-owned scratch archives/Jobs/native output only.
internal static class ArchivePhaseEDeliveryTests
{
    private static void Check(bool value,string detail){if(!value)throw new InvalidOperationException(detail);}
    private static string Root()
    {
        var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if(string.IsNullOrEmpty(worker)||!File.Exists(worker))throw new TestSkippedException("Explicit built matching archive worker required.");
        var scratch=Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")??throw new InvalidOperationException("Scratch required");
        var root=Path.Combine(scratch,"phase-e-delivery-fixtures","archive-product-"+Guid.NewGuid());Directory.CreateDirectory(root);
        File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));return root;
    }
    private static void FinishRoot(string root,bool passed){if(passed)Directory.Delete(root,true);else Console.Error.WriteLine("PRESERVED FIXTURE "+root);}
    private static ProtocolEnvelope Wire(ArchiveJobCommand command)=>ProtocolEnvelope.Create("platform_command",new{command="archive_job",data=command},command.OperationId);
    private static ArchiveJobCommand Start(ArchiveJobRequest request)=>new(){Version=1,OperationId=Guid.NewGuid(),SessionId=Guid.NewGuid(),Generation=1,Action="start",Request=request};
    private static async Task<ArchiveJobTerminal> Finish(ArchiveJobSession jobs)
    {
        while(true){var envelope=await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));var e=envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            if(e.Prompt is not null)throw new InvalidOperationException("Unexpected prompt "+e.Prompt.Kind);
            if(e.Terminal is {} terminal)return terminal;}
    }
    private static async Task<ArchiveJobTerminal> Run(ArchiveJobSession jobs,ArchiveJobRequest request)
    {var command=Start(request);Check(jobs.Dispatch(Wire(command)).Success,"job admission "+request.Kind);return await Finish(jobs);}
    private static void Zip(string path)
    {
        using var zip=ZipFile.Open(path,ZipArchiveMode.Create);
        foreach(var pair in new[]{("z-last.txt","keep"),("a-first.txt","remove")}){var e=zip.CreateEntry(pair.Item1);e.LastWriteTime=new DateTimeOffset(2020,1,2,3,4,6,TimeSpan.Zero);using var writer=new StreamWriter(e.Open());writer.Write(pair.Item2);}
    }
    private static async Task<(ArchiveBrowserContext Context,ArchiveReadEntry[] Entries)> Browse(string root,string path)
    {
        await using var read=ArchiveReadSession.ForOwnedTestPackage(root);
        var wire=ArchiveProductReadTests.Command(path:path,version:2,limits:new(){BrowserContext=true});var identity=ArchiveReadWire.Parse(wire);
        Check(read.Dispatch(wire).Success,"production browse admission");var entries=new List<ArchiveReadEntry>();
        while(true){var frame=(await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if(frame.Batch is {} batch){entries.AddRange(batch.Entries);Check(read.Dispatch(ArchiveProductReadTests.Command("ack",identity.OperationId,identity.SessionId,identity.Generation,sequence:batch.Sequence,version:2)).Success,"metadata credit");}
            if(frame.Terminal is {} terminal){Check(terminal.Status=="succeeded"&&terminal.CleanupComplete&&terminal.Browser is not null,"browse terminal "+terminal.Code);return(terminal.Browser!,entries.ToArray());}}
    }
    private static ArchiveJobRequest Edit(string path,ArchiveBrowserContext context,string action,ulong[]? ids=null,string? value=null,string[]? additions=null)=>
        new(){Kind="modify",Paths=new[]{path}.Concat(additions??[]).ToArray(),Modify=new(context,action,ids??[],value)};
    private static void Published(ArchiveJobTerminal terminal)=>Check(terminal.Outcome=="succeeded"&&terminal.FilesCommitted==1&&terminal.Journal is {Count:1}&&terminal.Journal[0].Replaced&&terminal.Journal[0].State=="Committed"&&terminal.CleanupComplete,"actual edit publication "+terminal.Code);
    internal static async Task ActualZipAllEditsAndStaleBinding()
    {
        var root=Root();var passed=false;try{
            var archive=Path.Combine(root,"editable.zip");Zip(archive);await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);
            const string zone="[ZoneTransfer]\r\nZoneId=3\r\n";File.WriteAllText(archive+":Zone.Identifier",zone);File.SetAttributes(archive,FileAttributes.Hidden|FileAttributes.Archive);
            var first=await Browse(root,archive);var selected=first.Entries.Single(e=>e.Name=="z-last.txt").Ordinal;
            Published(await Run(jobs,Edit(archive,first.Context,"rename",[selected],"renamed.txt")));
            Check(File.Exists(archive+":Zone.Identifier")&&File.ReadAllText(archive+":Zone.Identifier")==zone&&(File.GetAttributes(archive)&FileAttributes.Hidden)!=0,"edit preserves source MOTW and outer hidden attribute");
            using(var zip=ZipFile.OpenRead(archive)){Check(zip.GetEntry("renamed.txt") is {} renamed,"independent rename");using var reader=new StreamReader(zip.GetEntry("renamed.txt")!.Open());Check(reader.ReadToEnd()=="keep","original ID not sorted row");}
            var stale=await Run(jobs,Edit(archive,first.Context,"delete",[selected]));Check(stale.Code=="source_changed"&&stale.FilesCommitted==0&&stale.CleanupComplete,"old revision refused "+stale.Code);
            var current=await Browse(root,archive);Published(await Run(jobs,Edit(archive,current.Context,"delete",[current.Entries.Single(e=>e.Name=="a-first.txt").Ordinal])));
            current=await Browse(root,archive);Published(await Run(jobs,Edit(archive,current.Context,"new_directory",value:"empty/")));
            var source=Path.Combine(root,"added");Directory.CreateDirectory(source);File.WriteAllText(Path.Combine(source,"data.txt"),"added-data");
            current=await Browse(root,archive);Published(await Run(jobs,Edit(archive,current.Context,"add",additions:[source])));
            current=await Browse(root,archive);Published(await Run(jobs,Edit(archive,current.Context,"comment",value:"phase-e-comment")));
            using(var zip=ZipFile.OpenRead(archive)){Check(zip.Entries.Select(e=>e.FullName).Order().SequenceEqual(new[]{"renamed.txt","empty/","added/","added/data.txt"}.Order()),"all edit entries independent .NET");using var reader=new StreamReader(zip.GetEntry("added/data.txt")!.Open());Check(reader.ReadToEnd()=="added-data","recursive add bytes");}
            var browsed=await Browse(root,archive);Check(browsed.Context.Comment=="phase-e-comment","explicit new snapshot comment");
            passed=true;
        }finally{FinishRoot(root,passed);}
    }
    private static ArchiveJobRequest Create(string source,string destination,string format,string name)=>new(){Kind="create",Paths=[source],Destination=destination,Create=new(){Format=format,Level="fast",Method=format=="zip"?"deflate":null,Solid=format=="seven_z"?false:null,EncryptHeader=format=="seven_z"?false:null,StripSingleRoot=false,ExcludePatterns=[],TestAfterCreate=false,Name=name,VolumeBytes=65536}};
    internal static async Task ActualZipSevenVolumesAndOldFamilyRefusal()
    {
        var root=Root();var passed=false;try{
            var bytes=new byte[150000];new Random(7123).NextBytes(bytes);var source=Path.Combine(root,"small.bin");File.WriteAllBytes(source,bytes);
            foreach(var format in new[]{"zip","seven_z"}){
                var destination=Path.Combine(root,format);Directory.CreateDirectory(destination);var name=format=="zip"?"created.zip":"created.7z";
                await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);var done=await Run(jobs,Create(source,destination,format,name));
                Check(done.Outcome=="succeeded"&&done.FilesCommitted>=2&&done.CleanupComplete&&done.Journal is {} journal&&(ulong)journal.Count==done.FilesCommitted&&journal.All(j=>j.State=="Committed"&&!j.Replaced&&j.Published==true&&j.Cleanup=="retained"&&j.NativeIdentity is {Length:>10})&&journal.Select(j=>j.NativeIdentity).Distinct().Count()==journal.Count,"real volume journal identity/cleanup "+format+" "+done.Code);
                var primary=Path.Combine(destination,format=="zip"?name:name+".001");var browse=await Browse(root,primary);Check(browse.Entries.Single().Size==(ulong)bytes.Length,"production split browse "+format);
                var tested=await Run(jobs,new(){Kind="test",Paths=[primary]});Check(tested.Outcome=="succeeded"&&tested.BytesVerified==(ulong)bytes.Length,"actual split test "+format+" "+tested.Code);
                var extracted=Path.Combine(root,format+"-native");Directory.CreateDirectory(extracted);
                var nativeExtract=await Run(jobs,new(){Kind="extract",Paths=[primary],Destination=extracted,Extract=new(){Strategy="here",Conflict="skip",SingleFileFolder=false,CopyMotw=false}});
                Check(nativeExtract.Outcome=="succeeded"&&nativeExtract.FilesCommitted==1&&nativeExtract.CleanupComplete&&File.ReadAllBytes(Path.Combine(extracted,"small.bin")).SequenceEqual(bytes),"actual native split extraction "+format+" "+nativeExtract.Code);
                if(format=="zip") {var uppercase=await Browse(root,Path.Combine(destination,"created.Z01"));Check(uppercase.Entries.Single().Size==(ulong)bytes.Length,"production uppercase first ZIP member route");}
                await Oracle(primary,Path.Combine(root,format+"-oracle"),bytes);
                var before=Directory.GetFiles(destination).ToDictionary(path=>Path.GetFileName(path)!,File.ReadAllBytes);
                var conflict=await Run(jobs,Create(source,destination,format,name));Check(conflict.FilesCommitted==0&&conflict.Outcome=="failed"&&conflict.CleanupComplete,"old family refused");
                Check(before.All(pair=>File.ReadAllBytes(Path.Combine(destination,pair.Key!)).SequenceEqual(pair.Value)),"old family bytes unchanged");
            }
            passed=true;
        }finally{FinishRoot(root,passed);}
    }
    private static async Task Oracle(string primary,string extracted,byte[] expected)
    {
        var oracle=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE");
        if(string.IsNullOrEmpty(oracle)||!File.Exists(oracle))throw new TestSkippedException("Explicit independent 7Zip oracle required");
        var start=new ProcessStartInfo(oracle){UseShellExecute=false,RedirectStandardOutput=true,RedirectStandardError=true};foreach(var arg in new[]{"x","-y","-o"+extracted,primary})start.ArgumentList.Add(arg);
        using var child=Process.Start(start)!;var output=child.StandardOutput.ReadToEndAsync();var failure=child.StandardError.ReadToEndAsync();await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(20));await output;await failure;Check(child.ExitCode==0,"independent7Zip split extraction");Check(File.ReadAllBytes(Path.Combine(extracted,"small.bin")).SequenceEqual(expected),"independent split exact bytes");
    }
    internal static async Task ActualPartialVolumePublicationNoOverwrite()
    {
        var root=Root();var passed=false;try{
            var source=Path.Combine(root,"small.bin");var bytes=new byte[150000];new Random(13).NextBytes(bytes);File.WriteAllBytes(source,bytes);var destination=Path.Combine(root,"out");Directory.CreateDirectory(destination);
            await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);var published=0;
            jobs.ProbeOutputCommitForFixture(stage=>{if(stage=="native_published"&&++published==1)File.WriteAllText(Path.Combine(destination,"raced.zip"),"foreign-race");});
            var result=await Run(jobs,Create(source,destination,"zip","raced.zip"));
            Check(result.FilesCommitted>0&&result.Outcome=="failed"&&result.CleanupComplete&&result.Journal!.Any(j=>j.State=="Committed"),"partial published facts survive later race "+result.Code);
            Check(result.Journal!.Count>(int)result.FilesCommitted&&result.Journal.Any(j=>j.State=="FailedBeforeCommit"),"unpublished failed volume facts survive actual cleanup");
            Check(result.Journal.All(j=>j.NativeIdentity is {Length:>10})&&result.Journal.Where(j=>j.Published==true).All(j=>j.Cleanup=="retained")&&result.Journal.Where(j=>j.Published==false).All(j=>j.Cleanup=="partial_removed"),"per-volume terminal identity/published/unpublished/cleanup truth");
            Check(File.ReadAllText(Path.Combine(destination,"raced.zip"))=="foreign-race","no-replace race never overwrites");
            Check(Directory.GetFiles(destination,"*.z*").Length>0&&!Directory.GetFiles(destination,".nxpart-*").Any(),"published volumes retained and owned partials cleaned");passed=true;
        }finally{FinishRoot(root,passed);}
    }
    internal static async Task ActualRarFourFiveRealFamilies()
    {
        var fixtureRoot=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_RAR_VOLUME_FIXTURES");
        if(string.IsNullOrEmpty(fixtureRoot)||!Directory.Exists(fixtureRoot))throw new TestSkippedException("Explicit independently verified genuine RAR volumes required");
        var root=Root();var passed=false;try{
            var families=new[]{("test_rar_multivolume_single_file.part","1",3,0), ("test_read_format_rar5_multiarchive_solid.part","01",4,1)};
            foreach(var (stem,first,count,family) in families){
                var sourceRoot=Path.Combine(root,"rar"+family);Directory.CreateDirectory(sourceRoot);
                for(var index=1;index<=count;index++){var name=stem+index.ToString(first.Length==2?"D2":"D1")+".rar";File.Copy(Path.Combine(fixtureRoot,name),Path.Combine(sourceRoot,name));}
                var path=Path.Combine(sourceRoot,stem+first+".rar");await using var jobs=ArchiveJobSession.ForOwnedTestPackage(root);
                var tested=await Run(jobs,new(){Kind="test",Paths=[path]});
                Check(tested.Outcome=="succeeded"&&tested.BytesVerified==(family==0?20111UL:117398UL)&&tested.CleanupComplete,"actual RAR family test "+family+" "+tested.Code);
                var output=Path.Combine(root,"rar-output"+family);Directory.CreateDirectory(output);
                var extracted=await Run(jobs,new(){Kind="extract",Paths=[path],Destination=output,Extract=new(){Strategy="here",Conflict="skip",SingleFileFolder=false,CopyMotw=false}});
                Check(extracted.Outcome=="succeeded"&&extracted.CleanupComplete&&extracted.FilesCommitted==(family==0?1UL:9UL),"actual RAR family extraction "+family+" "+extracted.Code);
                var expected=Path.Combine(fixtureRoot,"extracted-"+family);
                foreach(var file in Directory.GetFiles(expected,"*",SearchOption.AllDirectories)){
                    var relative=Path.GetRelativePath(expected,file);var actual=Path.Combine(output,relative);
                    Check(File.Exists(actual)&&SHA256.HashData(File.ReadAllBytes(file)).SequenceEqual(SHA256.HashData(File.ReadAllBytes(actual))),"RAR independent exact output");
                }
                var later=Path.Combine(sourceRoot,stem+count.ToString(first.Length==2?"D2":"D1")+".rar");File.Move(later,later+".missing");
                var missing=await Run(jobs,new(){Kind="test",Paths=[path]});Check(missing.Outcome=="failed"&&missing.FilesCommitted==0&&missing.CleanupComplete,"missing RAR member rejected");
            }
            passed=true;
        }finally{FinishRoot(root,passed);}
    }
}
