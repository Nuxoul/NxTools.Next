using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// GUID-owned files, attributed streams and explicit helper/Job only. No user/Shell state.
internal static class ArchiveStreamWorkerTests
{
    private static void Require(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static async Task WithWorker(Func<DirectoryInfo, Task> action)
    {
        var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker)||!File.Exists(worker)) throw new TestSkippedException("Explicit built Rust worker required.");
        var helper=Path.Combine(AppContext.BaseDirectory,"NxTools.ArchiveWorker.exe"); Require(!File.Exists(helper),"refuse existing helper");
        var owned=Directory.CreateTempSubdirectory("archive-stream-"); var copied=false;
        try { File.Copy(worker,helper,false); copied=true; await action(owned); }
        finally { if(copied)File.Delete(helper); owned.Delete(true); }
    }
    private static async Task<ArchiveWorkerEvent> Finish(ArchiveWorkerRunner runner, ConcurrentQueue<ArchiveWorkerEvent> events)
    {
        var clock=Stopwatch.StartNew();
        while(!events.Any(e=>e.Kind=="exited")) { Require(clock.ElapsedMilliseconds<15000,"stream worker deadline"); await Task.Delay(20); }
        Require(await runner.DrainAsync(),"stream drain"); return events.Single(e=>e.Kind=="exited");
    }
    public static Task ActualGzipExtraction() => WithWorker(async owned => {
        var fixtures=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if(string.IsNullOrEmpty(fixtures))throw new TestSkippedException("Independent stream fixtures required.");
        foreach(var mode in new[]{"normal","multi","crc","budget"}) {
            var root=Directory.CreateDirectory(Path.Combine(owned.FullName,Guid.NewGuid().ToString("N")));
            var source=Path.Combine(owned.FullName,"payload.gz");
            var bytes=File.ReadAllBytes(Path.Combine(fixtures,mode=="multi"?"multi.gz":"independent.gz"));
            if(mode=="crc")bytes[^8]^=1; File.WriteAllBytes(source,bytes);
            var events=new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner=ArchiveWorkerRunner.ForPackage(events.Enqueue);
            Require(runner.Start(new(Guid.NewGuid(),"extract",source,0,new {max_entry_bytes=mode=="budget"?1:1048576},OutputRoot:root.FullName)),"stream extraction admission");
            var terminal=await Finish(runner,events); var success=mode is "normal" or "multi";
            Require(terminal.Category==(success?"success":mode=="budget"?"resource_limit":"worker_failure") && terminal.FilesCommitted==(success?1:0) && terminal.CleanupComplete,"stream extraction terminal: "+mode+"/"+terminal.Category);
            if(success) {
                var expected=mode=="multi"?"first\nsecond\n"u8.ToArray():System.Text.Encoding.UTF8.GetBytes("独立 gzip fixture\n").Concat(Enumerable.Range(0,256).Select(i=>(byte)i)).ToArray();
                Require(File.ReadAllBytes(Path.Combine(root.FullName,"payload")).SequenceEqual(expected),"independent gzip bytes");
            }
            Require(Directory.GetFiles(root.FullName,".nxpart-*").Length==0,"stream partial leak");
        }
    });
    public static Task ActualGzipCreation() => WithWorker(async owned => {
        var source=Path.Combine(owned.FullName,"source.bin");
        var expected=Enumerable.Range(0,262144).Select(i=>(byte)(i*31)).ToArray(); File.WriteAllBytes(source,expected);
        foreach(var level in new[]{"fast","normal","maximum"}) {
            var root=Directory.CreateDirectory(Path.Combine(owned.FullName,Guid.NewGuid().ToString("N")));
            var events=new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner=ArchiveWorkerRunner.ForPackage(events.Enqueue);
            Require(runner.Start(new(Guid.NewGuid(),"create_gzip",source,0,new {max_entries=100},OutputRoot:root.FullName,
                CreateName:"created.gz",CompressionLevel:level)),"gzip create admission: "+level);
            var terminal=await Finish(runner,events);
            Require(terminal.Category=="success" && terminal.FilesCommitted==1 && terminal.CommitState=="Committed" && terminal.NativeErrorCode is null && terminal.CleanupComplete,"gzip create terminal");
            using var input=File.OpenRead(Path.Combine(root.FullName,"created.gz")); using var gzip=new GZipStream(input,CompressionMode.Decompress);
            using var decoded=new MemoryStream(); await gzip.CopyToAsync(decoded);
            Require(decoded.ToArray().SequenceEqual(expected),"independent .NET gzip decode differs");
            Require(Directory.GetFiles(root.FullName,".nxpart-*").Length==0,"create partial leak");
        }
    });
    public static Task EmptyGzipXzCreationInterop() => WithWorker(async owned => {
        var python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE");
        if (string.IsNullOrEmpty(python) || !File.Exists(python)) throw new TestSkippedException("Explicit CPython gzip/lzma oracle required.");
        var source = Path.Combine(owned.FullName, "zero.bin"); File.WriteAllBytes(source, []);
        foreach (var extension in new[] { "gz", "xz" }) {
            var root = Directory.CreateDirectory(Path.Combine(owned.FullName, extension));
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var creator = ArchiveWorkerRunner.ForPackage(events.Enqueue);
            Require(creator.Start(new(Guid.NewGuid(), extension == "gz" ? "create_gzip" : "create_xz", source, 0, new { max_entries = 100 },
                OutputRoot: root.FullName, CreateName: "empty." + extension, CompressionLevel: "normal")), "empty stream normal-level creation admission");
            var created = await Finish(creator, events);
            Require(created.Category == "success" && created.FilesCommitted == 1 && created.CommitState == "Committed" && created.CleanupComplete, "empty stream native creation");
            var archive = Path.Combine(root.FullName, "empty." + extension);
            Require(new FileInfo(archive).Length > 0, "empty source still has complete stream framing");
            var start = new ProcessStartInfo(python) { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true };
            foreach (var argument in new[] { "-I", "-c", "import sys,gzip,lzma; p=sys.argv[1]; d=(gzip.open(p,'rb') if p.endswith('.gz') else lzma.open(p,'rb')).read(1); assert d==b''; print('empty:0')", archive }) start.ArgumentList.Add(argument);
            using var oracle = Process.Start(start)!;
            var stdout = await oracle.StandardOutput.ReadToEndAsync(); var stderr = await oracle.StandardError.ReadToEndAsync();
            Require(oracle.WaitForExit(5000) && oracle.ExitCode == 0 && stderr.Length == 0 && stdout.Trim() == "empty:0", "independent CPython empty decode");
            foreach (var operation in new[] { "open", "test", "extract" }) {
                var output = operation == "extract" ? Path.Combine(root.FullName, "decoded") : null;
                var readEvents = new ConcurrentQueue<ArchiveWorkerEvent>(); using var reader = ArchiveWorkerRunner.ForPackage(readEvents.Enqueue);
                Require(reader.Start(new(Guid.NewGuid(), operation, archive, 0, new { max_entries = 100 }, OutputRoot: output)), "empty stream existing read admission");
                var terminal = await Finish(reader, readEvents);
                Require(terminal.Category == "success" && terminal.CleanupComplete && terminal.FilesCommitted == (operation == "extract" ? 1 : 0), "empty stream read/extract cleanup/count");
                var frames = readEvents.Where(e => e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
                var complete = frames.Single(f => f.GetProperty("op").GetString() == "complete");
                Require(complete.GetProperty("files_verified").GetUInt64() == 1 && complete.GetProperty("bytes_verified").GetUInt64() == 0, "empty stream exact read counts");
                if (operation == "open") {
                    var entry = frames.Single(f => f.GetProperty("op").GetString() == "entry_batch").GetProperty("entries")[0];
                    Require(entry.GetProperty("size").ValueKind == System.Text.Json.JsonValueKind.Null, "listing must not invent an expanded size");
                }
                if (output is not null) Require(Directory.GetFiles(output).Length == 1 && new FileInfo(Path.Combine(output, "empty")).Length == 0, "one empty native output");
                using var released = File.Open(archive, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            }
            Require(Directory.GetFiles(root.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "empty stream owned partial cleanup");
        }
        using var releasedSource = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
    });
    public static Task CreationSafetyAndPartialRead() => WithWorker(async owned => {
        var source=Path.Combine(owned.FullName,"source.bin"); File.WriteAllBytes(source,new byte[262144]);
        foreach(var operation in new[]{"create_gzip","create_xz"})
        foreach(var mode in operation=="create_xz"?new[]{"cancel","existing","budget","work"}:new[]{"cancel","existing","budget"}) {
            var root=Directory.CreateDirectory(Path.Combine(owned.FullName,Guid.NewGuid().ToString("N")));
            var final=Path.Combine(root.FullName,operation=="create_xz"?"created.xz":"created.gz"); if(mode=="existing")File.WriteAllText(final,"user");
            var events=new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner=ArchiveWorkerRunner.ForPackage(events.Enqueue); var id=Guid.NewGuid(); var barriers=0;
            runner.SourceCheckpointProbe=stage=>{
                if(stage is "source_validated" or "source_precommit") {
                    barriers++;
                    try { using var unexpected=new FileStream(source,FileMode.Open,FileAccess.Write,FileShare.ReadWrite|FileShare.Delete); throw new InvalidOperationException("source lease writable"); }
                    catch(IOException) { }
                    if(stage=="source_precommit"&&mode=="cancel")Require(runner.Cancel(id),"create cancel");
                }
            };
            Require(runner.Start(new(id,operation,source,0,new {max_entry_bytes=mode=="budget"?1:1048576,max_working_bytes=mode=="work"?67108864:536870912},OutputRoot:root.FullName,CreateName:Path.GetFileName(final),CompressionLevel:mode=="work"?"maximum":"normal")),"safety admission");
            var terminal=await Finish(runner,events);
            Require(terminal.Category==(mode=="cancel"?"cancelled":mode is "budget" or "work"?"resource_limit":"worker_failure") && terminal.FilesCommitted==0 && terminal.CleanupComplete,"create safety terminal: "+operation+"/"+mode+"/"+terminal.Category);
            Require(barriers==(mode is "budget" or "work"?0:2),"create source barriers");
            Require(mode=="existing"?File.ReadAllText(final)=="user":!File.Exists(final),"create failure changed final");
            Require(Directory.GetFiles(root.FullName,".nxpart-*").Length==0 && File.ReadAllBytes(source).Length==262144,"create cleanup/source");
        }
        var partialRoot=Directory.CreateDirectory(Path.Combine(owned.FullName,Guid.NewGuid().ToString("N")));
        using(var session=ArchiveOutputSession.Open(partialRoot.FullName,Guid.NewGuid())) {
            var token=session.BeginFile("bounded"); session.Write(token,"abcdef"u8.ToArray());
            Require(session.ReadPartial(token,2,3).SequenceEqual("cde"u8.ToArray()) && session.ReadPartial(token,6,1).Length==0,"partial random access");
            foreach(var (key,offset,length) in new[]{(token,-1L,1),(token,7L,1),(token,0L,65537),(Guid.NewGuid(),0L,1)}) {
                try { session.ReadPartial(key,offset,length); throw new InvalidOperationException("invalid partial read accepted"); }
                catch(ArchiveOutputException failure){Require(failure.Stage is "read_limit" or "unknown_token","partial read typing");}
            }
            Require(session.Commit(token).Category=="committed","partial read disturbed write position");
            try {session.ReadPartial(token,0,1);throw new InvalidOperationException("committed token readable");}
            catch(ArchiveOutputException failure){Require(failure.Stage=="read_limit","published token read category");}
        }
        var rejects=new ConcurrentQueue<ArchiveWorkerEvent>(); using var rejectedRunner=ArchiveWorkerRunner.ForPackage(rejects.Enqueue);
        foreach(var mode in new[]{"store","password","name","self"}) {
            var request=new ArchiveWorkerRequest(Guid.NewGuid(),"create_gzip",source,0,new {max_entries=100},
                Password:mode=="password"?"synthetic-secret":null,OutputRoot:owned.FullName,
                CreateName:mode=="self"?"source.bin":mode=="name"?"../bad":"valid.gz",CompressionLevel:mode=="store"?"store":"normal");
            Require(!rejectedRunner.Start(request),"unsupported creation option admitted: "+mode);
        }
        Require(rejectedRunner.ActiveCount==0,"rejected creation launched worker");
    });
    public static Task ActualBzip2ZipExtraction() => WithWorker(async owned => {
        var fixtures=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if(string.IsNullOrEmpty(fixtures))throw new TestSkippedException("Independent Python ZIP12 required.");
        var events=new ConcurrentQueue<ArchiveWorkerEvent>();using var runner=ArchiveWorkerRunner.ForPackage(events.Enqueue);
        var root=Directory.CreateDirectory(Path.Combine(owned.FullName,"output"));
        Require(runner.Start(new(Guid.NewGuid(),"extract",Path.Combine(fixtures,"independent-bzip2.zip"),0,new {max_entries=100},OutputRoot:root.FullName)),"ZIP12 admission");
        var terminal=await Finish(runner,events);
        Require(terminal.Category=="success"&&terminal.FilesCommitted==1&&terminal.CleanupComplete,"ZIP12 native terminal");
        var expected=System.Text.Encoding.UTF8.GetBytes("独立 gzip fixture\n").Concat(Enumerable.Range(0,256).Select(i=>(byte)i)).ToArray();
        Require(File.ReadAllBytes(Path.Combine(root.FullName,"资料","payload.bin")).SequenceEqual(expected),"ZIP12 independent bytes");
        Require(Directory.GetFiles(root.FullName,".nxpart-*",SearchOption.AllDirectories).Length==0,"ZIP12 partial leak");
    });
    public static Task ActualTarExtraction() => WithWorker(async owned => {
        var fixtures=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if(string.IsNullOrEmpty(fixtures))throw new TestSkippedException("Independent Python TAR required.");
        foreach(var name in new[]{"independent.tar","unsafe.tar","collision.tar","huge-pax.tar"}) {
            var events=new ConcurrentQueue<ArchiveWorkerEvent>();using var runner=ArchiveWorkerRunner.ForPackage(events.Enqueue);
            var root=Directory.CreateDirectory(Path.Combine(owned.FullName,Guid.NewGuid().ToString("N")));
            Require(runner.Start(new(Guid.NewGuid(),"extract",Path.Combine(fixtures,name),0,new {max_entries=100},OutputRoot:root.FullName)),"TAR admission");
            var terminal=await Finish(runner,events);var success=name=="independent.tar";
            Require(terminal.Category==(success?"success":name=="huge-pax.tar"?"resource_limit":"worker_failure")&&terminal.FilesCommitted==(success?1:0)&&terminal.CleanupComplete,"TAR native terminal: "+name+"/"+terminal.Category);
            if(success){
                var expected=System.Text.Encoding.UTF8.GetBytes("独立 gzip fixture\n").Concat(Enumerable.Range(0,256).Select(i=>(byte)i)).ToArray();
                Require(File.ReadAllBytes(Path.Combine(root.FullName,"资料","payload.bin")).SequenceEqual(expected),"TAR independent bytes");
                Require(Directory.Exists(Path.Combine(root.FullName,"资料","empty"))&&!File.Exists(Path.Combine(root.FullName,"link")),"TAR empty/link policy");
            }else Require(Directory.GetFileSystemEntries(root.FullName).Length==0,"unsafe TAR output before preflight");
            Require(Directory.GetFiles(root.FullName,".nxpart-*",SearchOption.AllDirectories).Length==0,"TAR partial leak");
        }
    });
    public static Task ActualTarGzipExtraction() => WithWorker(async owned => {
        var fixtures=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if(string.IsNullOrEmpty(fixtures))throw new TestSkippedException("Independent Python TAR.GZ required.");
        foreach(var mode in new[]{"normal","crc","budget"}) {
            var source=Path.Combine(owned.FullName,mode+".tar.gz");var bytes=File.ReadAllBytes(Path.Combine(fixtures,"independent.tar.gz"));
            if(mode=="crc")bytes[^8]^=1;File.WriteAllBytes(source,bytes);
            var events=new ConcurrentQueue<ArchiveWorkerEvent>();using var runner=ArchiveWorkerRunner.ForPackage(events.Enqueue);
            var root=Directory.CreateDirectory(Path.Combine(owned.FullName,Guid.NewGuid().ToString("N")));
            Require(runner.Start(new(Guid.NewGuid(),"extract",source,0,new {max_entries=100,max_output_bytes=mode=="budget"?512:1048576},OutputRoot:root.FullName)),"TGZ admission");
            var terminal=await Finish(runner,events);var success=mode=="normal";
            Require(terminal.Category==(success?"success":mode=="budget"?"resource_limit":"worker_failure")&&terminal.FilesCommitted==(success?1:0)&&terminal.CleanupComplete,"TGZ native terminal: "+mode+"/"+terminal.Category);
            if(success){
                var expected=System.Text.Encoding.UTF8.GetBytes("独立 gzip fixture\n").Concat(Enumerable.Range(0,256).Select(i=>(byte)i)).ToArray();
                Require(File.ReadAllBytes(Path.Combine(root.FullName,"资料","payload.bin")).SequenceEqual(expected),"TGZ independent bytes");
                Require(Directory.Exists(Path.Combine(root.FullName,"资料","empty"))&&!File.Exists(Path.Combine(root.FullName,"link")),"TGZ directory/link policy");
            }else Require(Directory.GetFileSystemEntries(root.FullName).Length==0,"outer TGZ failure created output");
        }
    });
    public static Task ActualBzip2Streams() => WithWorker(async owned => {
        var fixtures=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if(string.IsNullOrEmpty(fixtures))throw new TestSkippedException("Independent Python BZip2 streams required.");
        foreach(var mode in new[]{"single","multi","tar","corrupt","truncated","budget"}) {
            var compound=mode=="tar";var source=Path.Combine(owned.FullName,mode+(compound?".tar.bz2":".bz2"));
            var bytes=File.ReadAllBytes(Path.Combine(fixtures,compound?"independent.tar.bz2":mode=="multi"?"multi.bz2":"independent.bz2"));
            if(mode=="corrupt")bytes[10]^=1;if(mode=="truncated")Array.Resize(ref bytes,bytes.Length-4);File.WriteAllBytes(source,bytes);
            var events=new ConcurrentQueue<ArchiveWorkerEvent>();using var runner=ArchiveWorkerRunner.ForPackage(events.Enqueue);
            var root=Directory.CreateDirectory(Path.Combine(owned.FullName,Guid.NewGuid().ToString("N")));
            Require(runner.Start(new(Guid.NewGuid(),"extract",source,0,new {max_entries=100,max_output_bytes=mode=="budget"?1:1048576},OutputRoot:root.FullName)),"BZip2 stream admission");
            var terminal=await Finish(runner,events);var success=mode is "single" or "multi" or "tar";
            Require(terminal.Category==(success?"success":mode=="budget"?"resource_limit":"worker_failure")&&terminal.FilesCommitted==(success?1:0)&&terminal.CleanupComplete,"BZip2 stream terminal: "+mode+"/"+terminal.Category);
            if(success){
                var expected=mode=="multi"?"first\nsecond\n"u8.ToArray():System.Text.Encoding.UTF8.GetBytes("独立 gzip fixture\n").Concat(Enumerable.Range(0,256).Select(i=>(byte)i)).ToArray();
                Require(File.ReadAllBytes(compound?Path.Combine(root.FullName,"资料","payload.bin"):Path.Combine(root.FullName,mode)).SequenceEqual(expected),"BZip2 stream bytes");
            }else Require(Directory.GetFileSystemEntries(root.FullName).Length==0,"bad BZip2 published output");
        }
    });
    public static Task ActualXzCreation() => WithWorker(async owned => {
        var oracle=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE");
        if(string.IsNullOrEmpty(oracle)||!File.Exists(oracle))throw new TestSkippedException("Explicit trusted CPython XZ oracle required.");
        var source=Path.Combine(owned.FullName,"source.bin");var expected=Enumerable.Range(0,262144).Select(i=>(byte)(i*31)).ToArray();File.WriteAllBytes(source,expected);
        foreach(var level in new[]{"fast","normal","maximum"}) {
            var root=Directory.CreateDirectory(Path.Combine(owned.FullName,Guid.NewGuid().ToString("N")));var events=new ConcurrentQueue<ArchiveWorkerEvent>();using var runner=ArchiveWorkerRunner.ForPackage(events.Enqueue);
            Require(runner.Start(new(Guid.NewGuid(),"create_xz",source,0,new {max_entries=100},OutputRoot:root.FullName,CreateName:"created.xz",CompressionLevel:level)),"XZ create admission: "+level);
            var terminal=await Finish(runner,events);Require(terminal.Category=="success"&&terminal.FilesCommitted==1&&terminal.CommitState=="Committed"&&terminal.CleanupComplete,
                $"XZ create terminal: level={level},category={terminal.Category},state={terminal.CommitState},native={terminal.NativeErrorCode},count={terminal.FilesCommitted},codec={events.Where(e=>e.Frame is {} frame&&frame.GetProperty("op").GetString()=="failure").Select(e=>e.Frame!.Value.GetProperty("error_code").GetString()).FirstOrDefault()}");
            var start=new ProcessStartInfo(oracle){UseShellExecute=false,CreateNoWindow=true,RedirectStandardOutput=true,RedirectStandardError=true};
            foreach(var argument in new[]{"-I","-c","import lzma,hashlib,sys; d=lzma.open(sys.argv[1],'rb').read(262145); print(str(len(d))+':'+hashlib.sha256(d).hexdigest())",Path.Combine(root.FullName,"created.xz")})start.ArgumentList.Add(argument);
            using var process=Process.Start(start)!;var observed=await process.StandardOutput.ReadToEndAsync();var stderr=await process.StandardError.ReadToEndAsync();
            Require(process.WaitForExit(5000)&&process.ExitCode==0&&stderr.Length==0,"independent Python decoder failure");
            Require(observed.Trim()==expected.Length+":"+Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(expected)).ToLowerInvariant(),"independent XZ byte/hash mismatch");
            Require(Directory.GetFiles(root.FullName,".nxpart-*").Length==0,"XZ partial leak");
        }
    });
    // Independent bsdtar/libzstd + Python XXH64 fixtures; GUID files and owned Job only.
    public static Task ActualZstdReads() => WithWorker(async owned => {
        var fixtures=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if(string.IsNullOrEmpty(fixtures))throw new TestSkippedException("Independent Zstd fixtures required.");
        foreach(var compound in new[]{false,true})
        foreach(var mode in new[]{"normal","multi","checksum","truncated","garbage","window","metadata","budget","cancel"}) {
            var bytes=File.ReadAllBytes(Path.Combine(fixtures,compound?"checked.tar.zst":mode=="multi"?"multi.zst":"checked.zst"));
            if(mode=="checksum")bytes[^1]^=1;
            if(mode=="truncated")Array.Resize(ref bytes,bytes.Length-2);
            if(mode=="garbage")bytes=bytes.Concat("garbage"u8.ToArray()).ToArray();
            if(mode=="window")bytes[5]=0xff;
            var source=Path.Combine(owned.FullName,"payload"+(compound?(mode=="multi"?".tar.zst":".tzst"):".zst"));
            File.WriteAllBytes(source,bytes);
            foreach(var operation in mode=="cancel"?new[]{"extract"}:new[]{"open","test","extract"}) {
                var root=Directory.CreateDirectory(Path.Combine(owned.FullName,Guid.NewGuid().ToString("N")));
                var events=new ConcurrentQueue<ArchiveWorkerEvent>();using var runner=ArchiveWorkerRunner.ForPackage(events.Enqueue);var id=Guid.NewGuid();var checks=0;
                runner.OutputCommitCheckpointProbe=stage=>{
                    if(stage!="before_flush")return;checks++;
                    try{using var writer=new FileStream(source,FileMode.Open,FileAccess.Write,FileShare.ReadWrite|FileShare.Delete);throw new InvalidOperationException("Zstd source writable");}catch(IOException){}
                    if(mode=="cancel")Require(runner.Cancel(id),"Zstd cancel");
                };
                try {
                    Require(runner.Start(new(id,operation,source,0,new {max_entries=100,max_working_bytes=67108864,max_metadata_bytes=mode=="metadata"?1:1048576,max_output_bytes=mode=="budget"?1:1048576},OutputRoot:operation=="extract"?root.FullName:null)),"Zstd admission");
                    var terminal=await Finish(runner,events);var success=mode is "normal" or "multi" || mode=="checksum"&&!compound&&operation=="open";
                    var resource=mode is "window" or "metadata" or "budget";
                    // Standalone metadata does not decode an unknown-size payload for its actual-output budget.
                    if(mode=="budget"&&!compound&&operation=="open")success=true;
                    Require(terminal.Category==(success?"success":mode=="cancel"?"cancelled":resource?"resource_limit":"worker_failure")&&terminal.FilesCommitted==(success&&operation=="extract"?1:0)&&terminal.CleanupComplete,"Zstd terminal: "+compound+"/"+mode+"/"+operation+"/"+terminal.Category);
                    var frames=events.Where(e=>e.Kind=="frame"&&e.Frame is not null).Select(e=>e.Frame!.Value).ToArray();
                    Require(!frames.Any(f=>f.GetProperty("op").GetString()=="native_call"),"Zstd private output escaped");
                    if(!success&&mode!="cancel")Require(frames.Single(f=>f.GetProperty("op").GetString()=="failure").GetProperty("error_code").GetString()==(resource?"ResourceLimit":"CorruptArchive"),"Zstd typed error");
                    if(success&&operation=="open") {
                        var entries=frames.Where(f=>f.GetProperty("op").GetString()=="entry_batch").SelectMany(f=>f.GetProperty("entries").EnumerateArray().ToArray()).ToArray();
                        Require(entries.Length==(compound?3:1),"Zstd entries");
                        if(!compound)Require(entries[0].GetProperty("size").ValueKind==System.Text.Json.JsonValueKind.Null,"Zstd unknown expanded size");
                    }
                    var expected=mode=="multi"&&!compound?"first\nsecond\n"u8.ToArray():System.Text.Encoding.UTF8.GetBytes("独立 gzip fixture\n").Concat(Enumerable.Range(0,256).Select(i=>(byte)i)).ToArray();
                    if(success&&operation!="open") {
                        var complete=frames.Single(f=>f.GetProperty("op").GetString()=="complete");
                        Require(complete.GetProperty("files_verified").GetInt64()==1&&complete.GetProperty("bytes_verified").GetInt64()==expected.Length,"Zstd verified counts");
                    }
                    var files=Directory.GetFiles(root.FullName,"*",SearchOption.AllDirectories);
                    if(success&&operation=="extract")Require(files.Length==1&&File.ReadAllBytes(files[0]).SequenceEqual(expected)&&terminal.CommitState=="Committed","Zstd independent bytes/state");
                    else Require(files.Length==0,"Zstd failure/read-only published file");
                    if(compound&&success&&operation=="extract")Require(Directory.Exists(Path.Combine(root.FullName,"资料","empty"))&&!File.Exists(Path.Combine(root.FullName,"link")),"Zstd TAR directory/link");
                    Require(mode!="cancel"||checks==1,"Zstd cancellation boundary");
                    Require(File.ReadAllBytes(source).SequenceEqual(bytes),"Zstd source changed");
                    Require(Directory.GetFiles(root.FullName,".nxpart-*",SearchOption.AllDirectories).Length==0,"Zstd partial leak");
                    using var released=new FileStream(source,FileMode.Open,FileAccess.Write,FileShare.None);
                } finally {runner.Stop();Require(await runner.DrainAsync(),"Zstd failure drain");}
            }
        }
    });
}
