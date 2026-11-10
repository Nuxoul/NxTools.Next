using System.Collections.Concurrent;
using System.IO.Compression;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text.Json;
using System.Text.Json.Nodes;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveVirtualDragTests;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveVirtualTreeTests
{
    private static ArchiveBrowserContext Synthetic()=>new(Guid.NewGuid(),"Zip","",new("v1:fixture",Enumerable.Repeat(1u,32).ToArray(),1,1));
    private static ArchiveVirtualDragCommand Command(ArchiveBrowserContext? browser=null,string path=@"C:\owned\source.zip")
    {
        var session=Guid.NewGuid();
        return new(1,Guid.NewGuid(),session,1,"start",42,[new("one.txt",3,new(){Version=1,OperationId=Guid.NewGuid(),SessionId=session,Generation=1,Action="materialize",Path=path,Ordinal=0,Browser=browser??Synthetic(),CopyMotw=false})]);
    }
    private static JsonObject Raw(ArchiveVirtualDragCommand command)=>JsonSerializer.SerializeToNode(command,JsonDefaults.Options)!.AsObject();
    private static JsonObject Item(JsonObject command,string name,ulong? ordinal,ulong? size,bool directory=false)
    {
        var item=command["items"]![0]!.DeepClone().AsObject();item["name"]=name;item["size"]=size;item["directory"]=directory;
        item["entry"]!["operation_id"]=Guid.NewGuid();item["entry"]!["ordinal"]=ordinal;return item;
    }
    private static ProtocolEnvelope Wire(JsonObject raw,string alias="archive_virtual_drag")=>ProtocolEnvelope.Create("platform_command",new{command=alias,data=raw},raw["operation_id"]!.GetValue<Guid>());
    private static ArchiveVirtualDragCommand Parse(JsonObject raw,string alias="archive_virtual_drag")=>ArchiveVirtualDragWire.Parse(Wire(raw,alias));
    private static void Reject(JsonObject raw,string reason)
    {
        try{Parse(raw);}catch(Exception failure) when(failure is InvalidDataException or JsonException){return;}
        throw new InvalidOperationException("must reject whole tree: "+reason);
    }
    internal static Task WireShapesConflictsAndBounds()
    {
        OrdinaryDirectoryWirePreservesEveryMember();
        var missing=Raw(Command());missing["items"]![0]!["name"]="parent/child";Reject(missing,"missing actual parent descriptor");
        var raw=Raw(Command());
        var unknown=Item(raw,"root/unknown.txt",1,null);unknown.Remove("size");
        raw["items"]=new JsonArray(Item(raw,"root",null,null,true),unknown,Item(raw,"empty",2,null,true),Item(raw,"known.txt",0,3));
        var parsed=Parse(raw);
        Check(parsed.Items!.Length==4,"directory and missing-size file must be accepted by original parser");
        Check(JsonSerializer.Serialize(parsed,JsonDefaults.Options)==JsonSerializer.Serialize(Parse(raw,"archive_virtual_drag_spike"),JsonDefaults.Options),"legacy alias same tree contract");
        var old=Raw(Command());old["items"]![0]!.AsObject().Remove("directory");
        Check(!JsonSerializer.Serialize(Parse(old),JsonDefaults.Options).Contains("\"directory\":false",StringComparison.Ordinal),"v1 default false omitted");
        var nullSize=raw.DeepClone().AsObject();nullSize["items"]![1]!["size"]=null;Parse(nullSize);
        var legacyEntry=parsed.Items[0].Entry;
        try{ArchiveEntryWire.Parse(ProtocolEnvelope.Create("platform_command",new{command="archive_entry",data=legacyEntry},legacyEntry.OperationId));throw new InvalidOperationException("implicit directory cannot gain file materialization authority");}catch(InvalidDataException){}
        void Bad(Action<JsonObject> mutate,string why){var bad=raw.DeepClone().AsObject();mutate(bad);Reject(bad,why);}
        Bad(b=>b["items"]![0]!["size"]=0,"directory size must be absent/null");
        Bad(b=>b["items"]![1]!["entry"]!["ordinal"]=null,"file needs real ordinal");
        Bad(b=>b["items"]![0]!["entry"]!["browser"]=null,"directory browser authority");
        Bad(b=>b["items"]![0]!["entry"]!["path"]="relative.zip","directory source path");
        Bad(b=>b["items"]![0]!["entry"]!["depth"]=4,"directory depth");
        Bad(b=>b["items"]![0]!["entry"]!["copy_motw"]=true,"MOTW inconsistent");
        Bad(b=>b["items"]![0]!["entry"]!["password_interaction"]=true,"directory cannot grant password interaction");
        Bad(b=>b["items"]![0]!["entry"]!["browser"]!["source"]!.AsObject().Remove("size"),"directory source shape preserves required zero-valued fields");
        Bad(b=>b["items"]![0]!["entry"]=null,"all descriptors retain nonnull entry");
        Bad(b=>b["items"]![1]!["entry"]!["output_limit"]=1,"wire cannot supply internal worker output authority");
        Bad(b=>b["items"]![0]!["entry"]!["browser"]!["comment"]="different","frozen browser identity inconsistent");
        Bad(b=>b["items"]![1]!["name"]="ROOT/unknown.txt","parent casing inconsistency");
        Bad(b=>b["items"]![2]!["name"]="root/UNKNOWN.txt","case-insensitive duplicate");
        Bad(b=>{b["items"]![0]!["directory"]=false;b["items"]![0]!["entry"]!["ordinal"]=10;},"file/directory prefix conflict");
        foreach(var name in new[]{"../escape","a//b","/root","a\\b","CON","a:ads",new string('x',260)})Bad(b=>b["items"]![1]!["name"]=name,"unsafe name");
        Bad(b=>b["items"]![3]!["size"]=ulong.MaxValue,"native signed stream size representation");
        var known=raw.DeepClone().AsObject();known["items"]![1]!["size"]=536870913UL;Parse(known);
        var bounds=Raw(Command());var all=new JsonArray();
        all.Add(Item(bounds,"root",null,null,true));
        for(var i=0;i<95;i++)all.Add(Item(bounds,"root/dir"+i,null,null,true));
        for(var i=0;i<32;i++)all.Add(Item(bounds,"root/file"+i,(ulong)i,null));
        bounds["items"]=all;Check(Parse(bounds).Items!.Length==128,"128 descriptors /32 files accepted");
        var extra=bounds.DeepClone().AsObject();extra["items"]!.AsArray().Add(Item(bounds,"extra",null,null,true));Check(Parse(extra).Items!.Length==129,"129 descriptors retained");
        var many=Raw(Command());var files=new JsonArray();for(var i=0;i<33;i++)files.Add(Item(many,"file"+i,(ulong)i,null));many["items"]=files;Check(Parse(many).Items!.Length==33,"33 file roots retained");
        var roots=Raw(Command());var rootItems=new JsonArray();for(var i=0;i<33;i++)rootItems.Add(Item(roots,"root"+i,null,null,true));roots["items"]=rootItems;Check(Parse(roots).Items!.Length==33,"all directory roots retained");
        var deep=Raw(Command());deep["items"]![0]!["name"]=string.Join('/',Enumerable.Repeat("a",130));Reject(deep,"missing real parent descriptors");
        var payload=bounds.DeepClone().AsObject();foreach(var item in payload["items"]!.AsArray())item!["entry"]!["browser"]!["comment"]=new string('c',3000);Parse(payload);
        foreach(var item in payload["items"]!.AsArray())item!["entry"]!["browser"]!["comment"]=new string('c',10000);Reject(payload,"actual transport frame");
        var only=Raw(Command());only["items"]=new JsonArray(Item(only,"empty",null,null,true));Parse(only);
        var implicitParents=Raw(Command());implicitParents["items"]=new JsonArray(Item(implicitParents,"Parent/one.txt",0,null),Item(implicitParents,"parent/two.txt",1,null));Reject(implicitParents,"implicit parent casing mismatch");
        var longest=Raw(Command());longest["items"]=new JsonArray(Item(longest,new string('a',128),null,null,true),Item(longest,new string('a',128)+"/"+new string('b',130),0,3));Parse(longest);
        var duplicate=Wire(Raw(Command()));
        var encoded=duplicate.Payload.GetRawText().Replace("\"size\":3,","\"size\":3,\"size\":3,",StringComparison.Ordinal);
        using(var document=JsonDocument.Parse(encoded))try{ArchiveVirtualDragWire.Parse(duplicate with{Payload=document.RootElement.Clone()});throw new InvalidOperationException("duplicate size property accepted");}catch(InvalidDataException){}
        // Root wrapper is presentation only: v1 needs no new field or materialization authority.
        var wrapped=Raw(Command());
        wrapped["items"]=new JsonArray(Item(wrapped,"Bundle",null,null,true),Item(wrapped,"Bundle/empty",null,null,true),Item(wrapped,"Bundle/member.txt",7,3));
        var rootCommand=Parse(wrapped);
        Check(rootCommand.Version==1&&rootCommand.Items![2].Entry.Ordinal==7,"root wrapper retains original ordinal and v1");
        foreach(var unsafeRoot in new[]{"CON","COM¹","LPT²","CONIN$","CONOUT$","CON .zip","../bad","a/b","bad.","bad "}) {
            var bad=wrapped.DeepClone().AsObject();bad["items"]![0]!["name"]=unsafeRoot;
            Reject(bad,"unsafe wrapper cannot enter descriptor");
        }
        foreach(var name in new[]{"\u007f","\u0085",new string('界',256)}) {
            var bad=Raw(Command());bad["items"]![0]!["name"]=name;Reject(bad,"Windows component/control/UTF-16 bounds");
        }
        foreach(var length in new[]{85,86,255}) {
            var unicode=Raw(Command());unicode["items"]![0]!["name"]=new string('界',length);Parse(unicode);
        }
        return Task.CompletedTask;
    }
    private static void LongRootSerializedRustCommandAndDescriptor()
    {
        var wire=Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!,"archive-r45-n01-long-root-wire.json");
        var envelope=JsonSerializer.Deserialize<ProtocolEnvelope>(File.ReadAllText(wire),JsonDefaults.Options)!;
        envelope.Validate();
        var command=ArchiveVirtualDragWire.Parse(envelope);
        var wrapper=new string('界',86);
        Check(System.Text.Encoding.UTF8.GetByteCount(wrapper)==258&&wrapper.Length==86,"UTF-8 transport bytes differ from Windows component UTF-16 units");
        Check(command.Plan is {Count:3}&&command.Items is {Length:3}&&command.Items[0].Name==wrapper&&command.Items[0].Directory&&
            command.Items[1].Name==wrapper+"/empty"&&command.Items[1].Directory&&command.Items[1].Entry.Ordinal==0&&
            command.Items[2].Name==wrapper+"/file"&&!command.Items[2].Directory&&command.Items[2].Entry.Ordinal==1,
            "serialized Rust Root retains unshortened wrapper, descriptors and original ordinals");
        using var data=new ArchiveVirtualFileDataObject(command.Plan!,_=>throw new InvalidOperationException("descriptors cannot demand content"));
        var request=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);
        data.GetData(ref request,out var medium);
        try{var pointer=GlobalLock(medium.unionmember);try{
            Check(Marshal.ReadInt32(pointer)==3,"Root wrapper belongs to full descriptor index array");
            for(var index=0;index<3;index++) {
                var name=index switch{0=>wrapper,1=>wrapper+@"\empty",_=>wrapper+@"\file"};
                var at=pointer+4+index*592;
                Check(Marshal.PtrToStringUni(at+72)==name,"FILEDESCRIPTORW UTF-16 name and original casing");
                Check(Marshal.ReadInt16(at+72+name.Length*2)==0,"FILEDESCRIPTORW NUL terminator stays within 260 WCHAR");
                Check(Marshal.ReadInt32(at+36)==(index==2?0x80:0x10),"directory versus file native attributes");
            }
        }finally{GlobalUnlock(medium.unionmember);}}finally{ReleaseStgMedium(ref medium);}
    }
    internal static Task SdkDescriptorsAndDirectoryRefusal()
    {
        LongRootSerializedRustCommandAndDescriptor();
        OrdinaryDirectorySdkAndNativeDemand();
        Known64BitDescriptorSize();
        // JSON keeps this regression compilable against the old nonnullable descriptor API.
        var files=JsonSerializer.Deserialize<ArchiveVirtualFile[]>("[{\"Name\":\"root\",\"Size\":null,\"Directory\":true},{\"Name\":\"root/unknown.txt\",\"Size\":null},{\"Name\":\"known.txt\",\"Size\":3},{\"Name\":\"empty\",\"Size\":null,\"Directory\":true}]")!;
        var calls=new List<int>();using var data=new ArchiveVirtualFileDataObject(files,index=>{calls.Add(index);return Memory("one"u8.ToArray());});
        var descriptor=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);data.GetData(ref descriptor,out var group);
        try{var pointer=GlobalLock(group.unionmember);try{
            Check(Marshal.ReadInt32(pointer)==4,"full descriptor count");
            for(var i=0;i<4;i++){
                var at=pointer+4+i*592;var known=i==2;
                Check(unchecked((uint)Marshal.ReadInt32(at))==(known?0x80000044u:0x80000004u),"SDK flags: no size promise for directory/unknown");
                Check(Marshal.ReadInt32(at+36)==(i is 0 or 3?0x10:0x80),"SDK attributes offset36");
                Check(Marshal.ReadInt32(at+64)==0&&Marshal.ReadInt32(at+68)==(known?3:0),"SDK size offsets64/68");
                Check(Marshal.PtrToStringUni(at+72)==files[i].Name.Replace('/','\\'),"SDK name offset72/592bytes");
            }
        }finally{GlobalUnlock(group.unionmember);}}finally{ReleaseStgMedium(ref group);}
        Check(calls.Count==0,"descriptor must never eagerly materialize");
        for(var i=0;i<260;i++){
            var directory=Format(ArchiveVirtualFileDataObject.ContentsFormat,i%2==0?0:3,TYMED.TYMED_ISTREAM);
            Check(data.QueryGetData(ref directory)<0,"directory FileContents Query rejected");
            try{data.GetData(ref directory,out _);throw new InvalidOperationException("directory GetData accepted");}catch(COMException){}
        }
        Check(calls.Count==0,"directory index must not reach original content owner or request budget");
        var contents=Format(ArchiveVirtualFileDataObject.ContentsFormat,1,TYMED.TYMED_ISTREAM);
        for(var i=0;i<257;i++){data.GetData(ref contents,out var stream);ReleaseStgMedium(ref stream);}
        Check(calls.Count==257&&calls.All(index=>index==1),"full-array lindex and independent repeated cursors");
        return Task.CompletedTask;
    }
    private static string Root()=>Directory.CreateDirectory(Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!,"archive-product-virtual-tree-"+Guid.NewGuid())).FullName;
    private static string Worker()
    {
        var worker=Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if(string.IsNullOrEmpty(worker)||!File.Exists(worker))throw new TestSkippedException("Explicit matching worker required; exploratory old-worker runs are not final matching evidence");
        return worker;
    }
    private static async Task<ArchiveBrowserContext> Browse(string root,string path,Action<ArchiveReadBatch>? observed=null)
    {
        await using var read=ArchiveReadSession.ForOwnedTestPackage(root);
        var request=ArchiveProductReadTests.Command(path:path,version:2,limits:new(){BrowserContext=true});var identity=ArchiveReadWire.Parse(request);
        Check(read.Dispatch(request).Success,"original browse admission");
        while(true){var frame=(await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(30))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if(frame.Batch is {} batch){observed?.Invoke(batch);Check(read.Dispatch(ArchiveProductReadTests.Command("ack",identity.OperationId,identity.SessionId,identity.Generation,sequence:batch.Sequence,version:2)).Success,"original read credit");}
            if(frame.Terminal is {} terminal){Check(terminal.Status=="succeeded"&&terminal.CleanupComplete&&terminal.Browser is not null,"original source binding");return terminal.Browser!;}
        }
    }
    private static void FileEntry(ZipArchive zip,string name,string value){using var writer=new StreamWriter(zip.CreateEntry(name).Open());writer.Write(value);}
    private static uint SevenzCrc(ReadOnlySpan<byte> data)
    {
        uint crc=uint.MaxValue;
        foreach(var value in data){crc^=value;for(var bit=0;bit<8;bit++)crc=(crc>>1)^((crc&1)==0?0u:0xedb88320u);}
        return ~crc;
    }
    private static void SevenzOrdinalFixture(string path)
    {
        // Raw 7z header, two independent Copy blocks and two zero-stream files. The
        // second block follows an empty entry in FilesInfo; both blocks share a name.
        var packed="onetwo"u8.ToArray();
        var header=new List<byte> {
            1,4,6,0,2,9,3,3,0, // Header/MainStreams/PackInfo: two packed streams.
            7,11,2,0,1,1,0,1,1,0,12,3,3,0, // Two single-Copy-coder blocks.
            8,0,0, // SubStreamsInfo, MainStreamsInfo end.
            5,4,0x0e,1,0x50,0x0f,1,0xc0, // Four files; indices 1/3 are empty files.
            0x11,
        };
        var names=System.Text.Encoding.Unicode.GetBytes("same.txt\0empty-a.txt\0same.txt\0empty-b.txt\0");
        Check(names.Length+1<128,"single-byte 7z property length");
        header.Add((byte)(names.Length+1));header.Add(0);header.AddRange(names);
        header.Add(0);header.Add(0); // FilesInfo and Header end.
        var signature=new byte[32];
        new byte[]{0x37,0x7a,0xbc,0xaf,0x27,0x1c,0,4}.CopyTo(signature,0);
        BitConverter.GetBytes((ulong)packed.Length).CopyTo(signature,12);
        BitConverter.GetBytes((ulong)header.Count).CopyTo(signature,20);
        BitConverter.GetBytes(SevenzCrc(header.ToArray())).CopyTo(signature,28);
        BitConverter.GetBytes(SevenzCrc(signature.AsSpan(12,20))).CopyTo(signature,8);
        using var source=File.Create(path);source.Write(signature);source.Write(packed);source.Write(header.ToArray());
    }
    private static long Stat(ref STGMEDIUM medium)
    {
        var stream=(IStream)Marshal.GetObjectForIUnknown(medium.unionmember);
        try{stream.Stat(out var stat,1);return stat.cbSize;}finally{Marshal.ReleaseComObject(stream);}
    }
    internal static async Task ActualZipTreeUnknownStreamsAndIdentity()
    {
        await OrdinaryDirectoryActualDemandAndTotals();
        var worker=Worker();var root=Root();STGMEDIUM retained=default;
        try {
            File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));var path=Path.Combine(root,"tree.zip");
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create)){zip.CreateEntry("root/");FileEntry(zip,"root/known.txt","one");zip.CreateEntry("empty/");zip.CreateEntry("root/sub/");FileEntry(zip,"root/sub/unknown.txt","two");zip.CreateEntry("root/");}
            var browser=await Browse(root,path);var raw=Raw(Command(browser,path));
            // Test-only nullable estimate published from real ZIP metadata. Codec and original
            // ordinal/source/session chain remain untouched; this is not an unknown-format matrix.
            raw["items"]=new JsonArray(Item(raw,"empty",2,null,true),Item(raw,"root",0,null,true),Item(raw,"root/sub",null,null,true),Item(raw,"root/sub/unknown.txt",4,null),Item(raw,"root/known.txt",1,3));
            var command=Parse(raw);
            await using(var entries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                await using var drag=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,escape:()=>false,loop:(data,source)=>{
                    var descriptor=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);data.GetData(ref descriptor,out var group);ReleaseStgMedium(ref group);
                    for(var i=0;i<3;i++){var directory=Format(ArchiveVirtualFileDataObject.ContentsFormat,i,TYMED.TYMED_ISTREAM);Check(data.QueryGetData(ref directory)<0,"directory lindex refused before worker");try{data.GetData(ref directory,out _);throw new InvalidOperationException("directory requested contents");}catch(COMException){}}
                    Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"tree descriptors and directory requests create no lease");
                    var unknown=Format(ArchiveVirtualFileDataObject.ContentsFormat,3,TYMED.TYMED_ISTREAM);data.GetData(ref unknown,out retained);
                    Check(Stat(ref retained)==3&&Read(ref retained,1)=="t","unknown descriptor gets actual native Stat/bytes, ordinal4 at lindex3");
                    data.GetData(ref unknown,out var repeat);try{Check(Stat(ref repeat)==3&&Read(ref repeat,3)=="two","independent repeated cursor/actual Stat");}finally{ReleaseStgMedium(ref repeat);}
                    var known=Format(ArchiveVirtualFileDataObject.ContentsFormat,4,TYMED.TYMED_ISTREAM);data.GetData(ref known,out var one);try{Check(Read(ref one,3)=="one","known original ordinal1 at lindex4");}finally{ReleaseStgMedium(ref one);}
                    Check(Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Length==2,"two distinct files materialized once, directories excluded");
                    return(0x00040100,1);
                });
                var terminal=await drag.Dispatch(Wire(raw),1).WaitAsync(TimeSpan.FromSeconds(40));Check(terminal.Success,"original tree drag terminal: "+terminal.Error);
                Check(Stat(ref retained)==3&&Read(ref retained,2)=="wo","target-held unknown native stream survives retirement");ReleaseStgMedium(ref retained);retained=default;
            }
            await using(var rootEntries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                // Same real read/worker/cache owner with a descriptor-only archive-stem wrapper.
                // The worker outputs original member paths; the wrapper MUST NOT become a cache path.
                var wrapped=Raw(Command(browser,path));
                wrapped["items"]=new JsonArray(Item(wrapped,"Bundle",null,null,true),Item(wrapped,"Bundle/root",0,null,true),Item(wrapped,"Bundle/root/known.txt",1,3),Item(wrapped,"Bundle/empty",2,null,true));
                await using var rootDrag=new ArchiveVirtualDragSession(rootEntries,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,escape:()=>false,loop:(data,source)=>{
                    var descriptor=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);data.GetData(ref descriptor,out var group);
                    try{var pointer=GlobalLock(group.unionmember);try{
                        Check(Marshal.ReadInt32(pointer)==4,"wrapper descriptor included in full index array");
                        Check(Marshal.PtrToStringUni(pointer+4+72)=="Bundle","outer folder is safe archive display stem");
                        Check(Marshal.PtrToStringUni(pointer+4+2*592+72)==@"Bundle\root\known.txt","descriptor path includes wrapper");
                    }finally{GlobalUnlock(group.unionmember);}}finally{ReleaseStgMedium(ref group);}
                    Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"wrapper/descriptor never preextracts");
                    var request=Format(ArchiveVirtualFileDataObject.ContentsFormat,2,TYMED.TYMED_ISTREAM);data.GetData(ref request,out var file);
                    try{Check(Read(ref file,3)=="one","wrapper lindex2 maps to original ordinal1");}finally{ReleaseStgMedium(ref file);}
                    var output=Directory.GetFiles(Path.Combine(root,"archive","sessions"),"known.txt",SearchOption.AllDirectories).Single();
                    Check(Path.GetFileName(Path.GetDirectoryName(output))=="root"&&!output.Split(Path.DirectorySeparatorChar).Contains("Bundle"),"actual content path remains original root/known.txt, not descriptor wrapper");
                    return(0x00040100,1);
                });
                var rootTerminal=await rootDrag.Dispatch(Wire(wrapped),1).WaitAsync(TimeSpan.FromSeconds(40));
                Check(rootTerminal.Success,"root wrapper original content chain: "+rootTerminal.Error);
                Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"root wrapper exact leases retired");
            }
            var collisionPath=Path.Combine(root,"collision.zip");
            using(var zip=ZipFile.Open(collisionPath,ZipArchiveMode.Create)){FileEntry(zip,"same.txt","one");FileEntry(zip,"SAME.txt","two");FileEntry(zip,"same.txt","tri");}
            var collisionBrowser=await Browse(root,collisionPath);
            var collision=Raw(Command(collisionBrowser,collisionPath));
            collision["items"]=new JsonArray(Item(collision,"same.txt",0,null),Item(collision,"SAME (2).txt",1,null),Item(collision,"same (3).txt",2,null));
            await using(var entries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                await using var collisionDrag=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,escape:()=>false,loop:(data,source)=>{
                    for(var index=0;index<3;index++) {
                        var request=Format(ArchiveVirtualFileDataObject.ContentsFormat,index,TYMED.TYMED_ISTREAM);
                        data.GetData(ref request,out var stream);
                        try{Check(Read(ref stream,3)==(index switch{0=>"one",1=>"two",_=>"tri"}),"renamed descriptor preserves every original colliding ordinal");}finally{ReleaseStgMedium(ref stream);}
                    }
                    return(0x00040100,1);
                });
                var result=await collisionDrag.Dispatch(Wire(collision),1).WaitAsync(TimeSpan.FromSeconds(40));
                Check(result.Success,"collisions with separate native leases must materialize all members: "+result.Error);
                Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"collision leases retire through original cleanup");
            }
            var sevenzPath=Path.Combine(root,"ordinal-collision.7z");
            SevenzOrdinalFixture(sevenzPath);
            var sevenzEntries=new List<ArchiveReadEntry>();
            var sevenzBrowser=await Browse(root,sevenzPath,batch=>sevenzEntries.AddRange(batch.Entries));
            Check(sevenzBrowser.Format=="SevenZ"&&sevenzEntries.Count==4,"real two-block 7z read binding");
            for(var index=0;index<4;index++) {
                var expectedName=index is 0 or 2?"same.txt":index==1?"empty-a.txt":"empty-b.txt";
                Check(sevenzEntries[index].Ordinal==(ulong)index&&sevenzEntries[index].Name==expectedName&&
                    sevenzEntries[index].Size==(ulong)(index is 0 or 2?3:0),"real 7z metadata keeps each original ordinal and size");
            }
            Check(sevenzEntries[0].WarningCodes.Contains("OutputCollision")&&sevenzEntries[2].WarningCodes.Contains("OutputCollision"),
                "identical 7z names remain visible as separate colliding members");
            var sevenzRaw=Raw(Command(sevenzBrowser,sevenzPath));
            // Descriptor order differs from source and decoder traversal. Empty entries
            // have no block; the second same-name file belongs to the second Copy block.
            sevenzRaw["items"]=new JsonArray(Item(sevenzRaw,"empty-b.txt",3,0),Item(sevenzRaw,"same (2).txt",2,3),
                Item(sevenzRaw,"same.txt",0,3),Item(sevenzRaw,"empty-a.txt",1,0));
            await using(var sevenzSession=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                await using var sevenzDrag=new ArchiveVirtualDragSession(sevenzSession,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,escape:()=>false,loop:(data,source)=>{
                    foreach(var (index,expected) in new[]{(0,""),(1,"two"),(2,"one"),(3,"")}) {
                        var request=Format(ArchiveVirtualFileDataObject.ContentsFormat,index,TYMED.TYMED_ISTREAM);
                        data.GetData(ref request,out var stream);
                        try{Check(Stat(ref stream)==expected.Length&&Read(ref stream,expected.Length)==expected,
                            "7z original ordinal content at native IStream lindex "+index);}finally{ReleaseStgMedium(ref stream);}
                    }
                    return(0x00040100,1);
                });
                var sevenzTerminal=await sevenzDrag.Dispatch(Wire(sevenzRaw),1).WaitAsync(TimeSpan.FromSeconds(40));
                Check(sevenzTerminal.Success,"7z colliding native leases must retain each decoded ordinal: "+sevenzTerminal.Error);
            }
            Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"7z collision leases retire after actual worker cleanup");
            await using(var entries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                var fresh=Raw(Command(browser,path));fresh["items"]=new JsonArray(Item(fresh,"known.txt",1,null));
                await using var content=new ArchiveVirtualContent(entries,Parse(fresh).Items!);
                var stream=content.Open(0,CancellationToken.None);Marshal.Release(stream);
                var output=Directory.GetFiles(Path.Combine(root,"archive","sessions"),"known.txt",SearchOption.AllDirectories).Single();
                File.WriteAllText(output,"bad");
                try{stream=content.Open(0,CancellationToken.None);Marshal.Release(stream);throw new InvalidOperationException("repeat must revalidate sealed identity/hash");}catch(ArchiveOutputException failure){Check(failure.Stage=="source_changed","changed sealed output rejected");}
            }
        }finally{if(retained.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref retained);Directory.Delete(root,true);}
    }
    internal static async Task ActualSealedSizeAccountingWithoutExtraCursor()
    {
        var worker=Worker();var root=Root();var payload=new string('a',256*1024);var noAccountingCursor=true;
        try {
            File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));var path=Path.Combine(root,"accounting.zip");
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create)){FileEntry(zip,"unknown.txt",payload);FileEntry(zip,"known.txt",payload);zip.CreateEntry("zero.txt");}
            var browser=await Browse(root,path);
            for(var trial=0;trial<3;trial++) {
                await using(var entries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                    var raw=Raw(Command(browser,path));raw["items"]=new JsonArray(Item(raw,"unknown.txt",0,null),Item(raw,"known.txt",1,(ulong)payload.Length),Item(raw,"zero.txt",2,0));
                    var command=Parse(raw);var ready=new List<ulong>();
                    await using var content=new ArchiveVirtualContent(entries,command.Items!,(ulong)payload.Length*2+1,progress:(_,isReady,bytes)=>{if(isReady)ready.Add(bytes);});
                    using var data=new ArchiveVirtualFileDataObject(command.Items!.Select(item=>new ArchiveVirtualFile(item.Name,(long?)item.Size)).ToArray(),index=>content.Open(index,CancellationToken.None));
                    var pointer=Marshal.GetComInterfaceForObject(data,typeof(System.Runtime.InteropServices.ComTypes.IDataObject));STGMEDIUM first=default;
                    var clock=System.Diagnostics.Stopwatch.StartNew();
                    try {
                        var get=NativeGetData(pointer);var request=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);
                        Check(get(pointer,ref request,out first)==0,"original native demand materializes unknown member");
                        var firstMs=clock.Elapsed.TotalMilliseconds;var firstCounts=entries.CacheVerificationForFixture;
                        Check(Stat(ref first)==payload.Length&&Read(ref first,1)=="a","unknown descriptor has actual sealed Stat and bytes");
                        noAccountingCursor&=firstCounts==new ArchiveEntryCache.VerificationCounts(payload.Length,payload.Length,1,1);
                        Check(get(pointer,ref request,out var repeat)==0,"repeated native demand");
                        try{Check(Read(ref repeat,payload.Length)==payload,"independent repeated cursor begins at zero");}finally{ReleaseStgMedium(ref repeat);}
                        Check(Read(ref first,payload.Length-1)==payload[1..],"first cursor position remains independent");
                        foreach(var index in new[]{1,2}) {
                            var next=Format(ArchiveVirtualFileDataObject.ContentsFormat,index,TYMED.TYMED_ISTREAM);
                            Check(get(pointer,ref next,out var medium)==0,"known and zero native demands");
                            try{Check(Stat(ref medium)==(index==1?payload.Length:0)&&Read(ref medium,index==1?payload.Length:0)==(index==1?payload:""),"exact known/zero output bytes");}finally{ReleaseStgMedium(ref medium);}
                        }
                        var counts=entries.CacheVerificationForFixture;
                        noAccountingCursor&=counts==new ArchiveEntryCache.VerificationCounts(payload.Length*2L,payload.Length*3L,4,4);
                        Check(ready.SequenceEqual(new[]{(ulong)payload.Length,(ulong)payload.Length,0UL}),"sealed actual bytes charge each distinct index exactly once");
                        Console.WriteLine($"OBS archive.drag.accounting operation_id={command.Items![0].Entry.OperationId} trial={trial+1} first_ms={firstMs:F3} demand_ms={clock.Elapsed.TotalMilliseconds:F3} first_seal_hash_bytes={firstCounts.SealHashBytes} first_cursor_hash_bytes={firstCounts.CursorHashBytes} first_cursor_opens={firstCounts.CursorOpens} seal_hash_bytes={counts.SealHashBytes} cursor_hash_bytes={counts.CursorHashBytes} cursor_checks={counts.CursorChecks} cursor_opens={counts.CursorOpens}");
                    }finally{if(first.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref first);Marshal.Release(pointer);}
                }
                Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any()&&!Directory.GetFiles(root,".nxpart-*",SearchOption.AllDirectories).Any()&&!Directory.GetFiles(root,"*.txt",SearchOption.AllDirectories).Any(),"native published outputs and exact leases cleaned after each trial");
            }
            Check(noAccountingCursor,"first materialization must use Seal size without a size-only cursor or full cache hash; each real cursor still hashes once");
        }finally{Directory.Delete(root,true);}
    }
    internal static async Task ActualFirstAndRepeatedCursorTamper()
    {
        var worker=Worker();var root=Root();
        try {
            File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));var path=Path.Combine(root,"tamper.zip");
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create))FileEntry(zip,"one.txt","one");
            var browser=await Browse(root,path);
            var retainedOutputs=new HashSet<string>();
            foreach(var beforeFirst in new[]{true,false}) {
                await using var entries=ArchiveEntrySession.ForOwnedTestPackage(root);
                var raw=Raw(Command(browser,path));raw["items"]=new JsonArray(Item(raw,"one.txt",0,null));
                string? outputPath=null;
                void Tamper(){outputPath=Directory.GetFiles(Path.Combine(root,"archive","sessions"),"one.txt",SearchOption.AllDirectories).Single(candidate=>!retainedOutputs.Contains(candidate));var time=File.GetLastWriteTimeUtc(outputPath);File.WriteAllText(outputPath,"bad");File.SetLastWriteTimeUtc(outputPath,time);}
                if(beforeFirst)entries.SealedCheckpointForFixture(Tamper);
                await using(var content=new ArchiveVirtualContent(entries,Parse(raw).Items!,3)) {
                    if(!beforeFirst){var first=content.Open(0,CancellationToken.None);Marshal.Release(first);Tamper();}
                    try{var stream=content.Open(0,CancellationToken.None);Marshal.Release(stream);throw new InvalidOperationException("same-size same-time tamper must not return a native cursor");}
                    catch(ArchiveOutputException failure){Check(failure.Stage=="source_changed","original consumer rejects changed sealed bytes");}
                    var counts=entries.CacheVerificationForFixture;
                    Check(counts.SealHashBytes==3&&counts.CursorHashBytes>=3&&counts.CursorChecks>=1,"Seal and actual target verification remain full-content checks");
                }
                Check(outputPath is not null&&File.ReadAllText(outputPath)=="bad","original cleanup retains changed output");retainedOutputs.Add(outputPath!);
            }
        }finally{Directory.Delete(root,true);}
    }
    internal static Task CacheCursorIntegrityAndLifetime()
    {
        var root=Root();
        try {
            using var cache=ArchiveEntryCache.ForOwnedTestCache(root);
            foreach(var repeat in new[]{false,true}) {
                var lease=cache.Create(Guid.NewGuid(),Guid.NewGuid(),1);var path=Path.Combine(lease.Root,"files","one.txt");
                var before=cache.VerificationForFixture;
                using(var output=ArchiveOutputSession.Open(Path.Combine(lease.Root,"files"),lease.Operation)) {
                    var token=output.BeginFile("one.txt");Check(output.Write(token,"one"u8.ToArray()),"original native output write");Check(output.Commit(token).Category=="committed","original native publication");
                }
                lease.Seal(path);
                if(repeat){var stream=cache.OpenVirtualStream(lease.Id,lease.Operation,lease.Session,lease.Generation,null,CancellationToken.None,out var size);Check(size==3,"real cursor actual size");Marshal.Release(stream);}
                var timestamp=File.GetLastWriteTimeUtc(path);File.WriteAllText(path,"bad");File.SetLastWriteTimeUtc(path,timestamp);
                try{var stream=cache.OpenVirtualStream(lease.Id,lease.Operation,lease.Session,lease.Generation,null,CancellationToken.None,out _);Marshal.Release(stream);throw new InvalidOperationException("same-size same-time tamper accepted");}
                catch(ArchiveOutputException failure){Check(failure.Stage=="source_changed","first/repeated cache cursor still hashes sealed bytes");}
                var after=cache.VerificationForFixture;
                Check(after.SealHashBytes-before.SealHashBytes==3&&after.CursorHashBytes-before.CursorHashBytes==(repeat?6:3)&&
                    after.CursorChecks-before.CursorChecks==(repeat?2:1)&&after.CursorOpens-before.CursorOpens==(repeat?1:0),"owner observations count full Seal/cursor hash reads separately from native cursor opens");
                Check(!cache.Release(lease.Id)&&File.ReadAllText(path)=="bad","changed output retained by original cleanup");
            }
            var clean=cache.Create(Guid.NewGuid(),Guid.NewGuid(),1);var cleanPath=Path.Combine(clean.Root,"files","clean.txt");File.WriteAllText(cleanPath,"one");clean.Seal(cleanPath);
            var first=cache.OpenVirtualStream(clean.Id,clean.Operation,clean.Session,clean.Generation,null,CancellationToken.None,out _);
            var second=cache.OpenVirtualStream(clean.Id,clean.Operation,clean.Session,clean.Generation,3,CancellationToken.None,out _);
            var a=new STGMEDIUM{tymed=TYMED.TYMED_ISTREAM,unionmember=first};var b=new STGMEDIUM{tymed=TYMED.TYMED_ISTREAM,unionmember=second};
            try{Check(Read(ref a,1)=="o"&&Read(ref b,3)=="one","native cursors independent");Check(!cache.Release(clean.Id),"target-held stream retains owned output");Check(Read(ref a,2)=="ne","transferred stream survives retirement");}
            finally{ReleaseStgMedium(ref a);ReleaseStgMedium(ref b);}
            cache.Sweep();Check(!File.Exists(cleanPath),"original bounded sweep removes released unlocked output");
            return Task.CompletedTask;
        }finally{Directory.Delete(root,true);}
    }
    internal static async Task ActualUnknownZstdReadToNativeStream()
    {
        var root=Root();STGMEDIUM held=default;
        try {
            File.Copy(Worker(),Path.Combine(root,ArchiveReadHelper.DeploymentName));var path=Path.Combine(root,"unknown.zst");
            // Independent minimal Zstd frame: no content-size field,1KiBwindow,final RAW3-byte block.
            File.WriteAllBytes(path,[0x28,0xb5,0x2f,0xfd,0,0,25,0,0,(byte)'o',(byte)'n',(byte)'e']);
            ArchiveReadEntry? original=null;
            var browser=await Browse(root,path,batch=>{Check(batch.Entries.Length==1,"one real stream member");original=batch.Entries[0];});
            Check(original is {Size:null,Encrypted:false,Kind:"File"},"original worker browse must truthfully report unknown unencrypted file");
            var raw=Raw(Command(browser,path));raw["items"]=new JsonArray(Item(raw,"unknown",original!.Ordinal,null));
            await using(var entries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                await using var drag=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,escape:()=>false,loop:(data,source)=>{
                    var descriptor=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);data.GetData(ref descriptor,out var group);
                    try{var pointer=GlobalLock(group.unionmember);try{Check(unchecked((uint)Marshal.ReadInt32(pointer+4))==0x80000004u,"real unknown omits FD_FILESIZE");}finally{GlobalUnlock(group.unionmember);}}finally{ReleaseStgMedium(ref group);}
                    Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"real unknown descriptor no work");
                    var request=Format(ArchiveVirtualFileDataObject.ContentsFormat,0,TYMED.TYMED_ISTREAM);data.GetData(ref request,out held);
                    Check(Stat(ref held)==3&&Read(ref held,1)=="o","real unknown actual Stat and bytes");
                    data.GetData(ref request,out var repeated);try{Check(Read(ref repeated,3)=="one","real unknown independent repeated cursor");}finally{ReleaseStgMedium(ref repeated);}
                    return(0x00040100,1);
                });
                var terminal=await drag.Dispatch(Wire(raw),1).WaitAsync(TimeSpan.FromSeconds(40));Check(terminal.Success,"real unknown original content chain: "+terminal.Error);
                Check(Read(ref held,2)=="ne","real unknown held stream survives retirement");ReleaseStgMedium(ref held);held=default;
            }
        }finally{if(held.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref held);Directory.Delete(root,true);}
    }
    internal static async Task SourceAdmissionIsIndependentOfRemainingOutputBudget()
    {
        var root=Root();
        try {
            File.Copy(Worker(),Path.Combine(root,ArchiveReadHelper.DeploymentName));var path=Path.Combine(root,"source-larger-than-output.zip");
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create)){FileEntry(zip,"one.txt","one");FileEntry(zip,"two.txt","two");}
            Check(new FileInfo(path).Length>6,"encoded source is larger than remaining output allowance");
            var browser=await Browse(root,path);var raw=Raw(Command(browser,path));raw["items"]=new JsonArray(Item(raw,"one.txt",0,null),Item(raw,"two.txt",1,null));
            await using(var entries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                await using var content=new ArchiveVirtualContent(entries,Parse(raw).Items!,6);
                for(var index=0;index<2;index++) {var stream=content.Open(index,CancellationToken.None);var medium=new STGMEDIUM {tymed=TYMED.TYMED_ISTREAM,unionmember=stream};try{Check(Stat(ref medium)==3&&Read(ref medium,3)==(index==0?"one":"two"),"remaining output must not shrink encoded source authority");}finally{ReleaseStgMedium(ref medium);}}
            }
            await using(var entries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                // Actual tiny materialization with a quota above512MiB, without a large allocation.
                await using var content=new ArchiveVirtualContent(entries,Parse(raw).Items!,(512UL<<20)+17);
                var stream=content.Open(0,CancellationToken.None);var medium=new STGMEDIUM{tymed=TYMED.TYMED_ISTREAM,unionmember=stream};
                try{Check(Stat(ref medium)==3&&Read(ref medium,3)=="one","original materialization admits general output quota above512MiB");}finally{ReleaseStgMedium(ref medium);}
            }
            Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"exact original leases retire");
        }finally{Directory.Delete(root,true);}
    }
    internal static async Task ActualRemainingBudgetAndCleanup()
    {
        var worker=Worker();var root=Root();
        try {
            File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));var path=Path.Combine(root,"budget.zip");
            // Output budgets remain independent of encoded source admission.
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create)){FileEntry(zip,"one.txt",new string('a',700));FileEntry(zip,"two.txt",new string('b',700));zip.CreateEntry("zero.txt");}
            var browser=await Browse(root,path);
            foreach(var mode in new[]{"known_oversize","unknown_oversize","concurrent","repeat","estimate_mismatch","zero_estimate","zero_remaining"}) {
                await using var entries=ArchiveEntrySession.ForOwnedTestPackage(root);long written=0;entries.ProbeWriteForFixture(bytes=>Interlocked.Add(ref written,bytes));
                var raw=Raw(Command(browser,path));
                var limit=mode=="zero_remaining"?700UL:1300UL;
                raw["items"]=mode is "estimate_mismatch" or "zero_estimate"?new JsonArray(Item(raw,"one.txt",0,mode=="zero_estimate"?0UL:699UL),Item(raw,"two.txt",1,null)):
                    new JsonArray(Item(raw,"one.txt",0,null),Item(raw,"two.txt",1,mode=="known_oversize"?700UL:null),Item(raw,"zero.txt",2,0));
                var command=Parse(raw);var content=new ArchiveVirtualContent(entries,command.Items!,limit);
                try {
                    using var data=new ArchiveVirtualFileDataObject(command.Items!.Select(item=>new ArchiveVirtualFile(item.Name,(long?)item.Size,item.Directory)).ToArray(),index=>content.Open(index,CancellationToken.None));
                    bool Open(int index){var request=Format(ArchiveVirtualFileDataObject.ContentsFormat,index,TYMED.TYMED_ISTREAM);STGMEDIUM medium=default;try{data.GetData(ref request,out medium);Check(Stat(ref medium)==(index==2?0:700),"native actual size");return true;}catch(ArchiveOutputException failure){Check(failure.Stage is "resource_limit" or "source_changed","bounded original failure: "+mode+" index="+index+" "+failure.Stage);Check(medium.unionmember==IntPtr.Zero,"failed original materialization returns no medium");return false;}finally{if(medium.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref medium);}}
                    if(mode=="concurrent") {
                        var results=await Task.WhenAll(Task.Run(()=>Open(0)),Task.Run(()=>Open(1))).WaitAsync(TimeSpan.FromSeconds(40));
                        Check(results.Count(ok=>ok)==1,"simultaneous distinct unknown requests cannot oversubscribe remaining budget");
                    }else if(mode is "estimate_mismatch" or "zero_estimate"){Check(!Open(0),"wrong declared estimate must not publish medium");Check(!Open(1),"wrong zero/known estimate cannot replace actual sealed budget charge");}
                    else {
                        Check(Open(0),"first unknown file fits");
                        if(mode=="repeat") {
                            for(var i=0;i<3;i++)Check(Open(0),"repeat validates without charging twice");
                            Check(Open(2),"repeat did not double-charge actual bytes; zero-byte file fits positive remaining budget");
                        }
                        if(mode=="zero_remaining")Check(!Open(2),"exhausted budget refuses even zero-byte entries without worker bypass");
                        else Check(!Open(1),"original worker remaining limit refuses second file before output");
                    }
                    Check(Interlocked.Read(ref written)==700,"only first700 bytes passed original worker output; overbudget output never written");
                }finally{await content.DisposeAsync();}
                Check(!Directory.GetFiles(root,".nxpart-*",SearchOption.AllDirectories).Any(),"original worker partials cleaned");
                Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"all successful/failed exact leases retired after actual cleanup");
                Check(!Directory.GetFiles(root,"*.txt",SearchOption.AllDirectories).Any(),"no post-write-only aggregate leftovers");
            }
        }finally{Directory.Delete(root,true);}
    }
    private static JsonObject OrdinaryDirectoryRaw(ArchiveBrowserContext? browser=null,string path=@"C:\owned\source.zip",int count=256)
    {
        var raw=Raw(Command(browser,path));var items=new JsonArray(Item(raw,"ordinary",null,null,true));
        for(var ordinal=0;ordinal<count;ordinal++)items.Add(Item(raw,$"ordinary/file-{ordinal:000}.txt",(ulong)ordinal,8));
        raw["items"]=items;return raw;
    }
    private static void OrdinaryDirectoryWirePreservesEveryMember()
    {
        Check(Parse(OrdinaryDirectoryRaw(count:33)).Items!.Count(item=>!item.Directory)==33,"33 small files are not a logical selection overflow");
        var raw=OrdinaryDirectoryRaw();var parsed=Parse(raw);var items=parsed.Items!;
        Check(items.Length==257&&items.Count(item=>!item.Directory)==256,"ordinary directory: parser preserves all 256 files plus parent");
        Check(items.Skip(1).Select(item=>item.Entry.Ordinal!.Value).SequenceEqual(Enumerable.Range(0,256).Select(index=>(ulong)index)),"all original ordinals, not a partial selection");
        foreach(var ordinal in new[]{0,128,255}) {
            var item=items[ordinal+1];
            Check(item.Name==$"ordinary/file-{ordinal:000}.txt"&&item.Size==8&&item.Entry.Path==@"C:\owned\source.zip","first/middle/last descriptor source identity and size");
        }
        Check(Parse(raw,"archive_virtual_drag_spike").Items!.Length==257,"legacy alias shares complete native tree");
        var changed=raw.DeepClone().AsObject();changed["items"]![129]!["entry"]!["browser"]!["source"]!["native"]="v1:other";
        Reject(changed,"middle member cannot change frozen source identity");
        var commented=raw.DeepClone().AsObject();
        foreach(var item in commented["items"]!.AsArray())item!["entry"]!["browser"]!["comment"]=new string('c',4096);
        var compact=Compact(commented);var compactParsed=Parse(compact);
        Check(compactParsed.Items!.Length==257&&compactParsed.Items[129].Entry.Ordinal==128&&compactParsed.Items[129].Entry.Browser!.Comment.Length==4096,"one shared source expands every original entry intent");
        Check(System.Text.Encoding.UTF8.GetByteCount(Wire(compact).Payload.GetRawText())<ProtocolConstants.MaxFrameBytes,"compact source uses existing transport frame");
        Check(ReferenceEquals(compactParsed.Items[1].Entry.Browser,compactParsed.Items[256].Entry.Browser),"native materialization shares one immutable browser source");
        var overrideSource=compact.DeepClone().AsObject();overrideSource["items"]![129]!["entry"]!["path"]=@"C:\other.zip";Reject(overrideSource,"compact member cannot override source");
        var missingSource=compact.DeepClone().AsObject();missingSource["source"]!["browser"]!["source"]!.AsObject().Remove("size");Reject(missingSource,"compact source retains required zero-valued identity fields");
        var unicode=compact.DeepClone().AsObject();unicode["source"]!["browser"]!["comment"]=new string('\u007f',65536);Parse(unicode);
    }
    private static JsonObject Compact(JsonObject original)
    {
        var raw=original.DeepClone().AsObject();var first=raw["items"]![0]!["entry"]!;
        raw["source"]=new JsonObject {
            ["path"]=first["path"]!.DeepClone(),["codepage"]=first["codepage"]!.DeepClone(),
            ["browser"]=first["browser"]!.DeepClone(),["depth"]=first["depth"]!.DeepClone(),
            ["copy_motw"]=first["copy_motw"]?.DeepClone(),
        };
        foreach(var item in raw["items"]!.AsArray()) {
            var entry=item!["entry"]!;
            item["entry"]=new JsonObject{["operation_id"]=entry["operation_id"]!.DeepClone(),["ordinal"]=entry["ordinal"]?.DeepClone()};
        }
        return raw;
    }
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]private delegate int GetDataAbi(IntPtr self,ref FORMATETC format,out STGMEDIUM medium);
    private static GetDataAbi NativeGetData(IntPtr pointer)=>Marshal.GetDelegateForFunctionPointer<GetDataAbi>(Marshal.ReadIntPtr(Marshal.ReadIntPtr(pointer),3*IntPtr.Size));
    private static void OrdinaryDirectorySdkAndNativeDemand()
    {
        var files=new[]{new ArchiveVirtualFile("ordinary",null,true)}.Concat(Enumerable.Range(0,256).Select(index=>new ArchiveVirtualFile($"ordinary/file-{index:000}.txt",8))).ToArray();
        var calls=new List<int>();
        using var data=new ArchiveVirtualFileDataObject(files,index=>{calls.Add(index);return Memory(System.Text.Encoding.UTF8.GetBytes($"file-{index-1:000}"));});
        var pointer=Marshal.GetComInterfaceForObject(data,typeof(System.Runtime.InteropServices.ComTypes.IDataObject));STGMEDIUM held=default;
        try {
            var get=NativeGetData(pointer);var descriptor=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);
            Check(get(pointer,ref descriptor,out var group)==0,"native IDataObject GetData descriptor demand");
            try{var bytes=GlobalLock(group.unionmember);try{
                Check(bytes!=IntPtr.Zero&&Marshal.ReadInt32(bytes)==257,"SDK FILEGROUPDESCRIPTORW full count, not 128");
                foreach(var index in new[]{1,129,256}) {
                    var at=bytes+4+index*592;
                    Check(unchecked((uint)Marshal.ReadInt32(at))==0x80000044u&&Marshal.ReadInt32(at+36)==0x80,"SDK file flags and attributes");
                    Check(Marshal.ReadInt32(at+64)==0&&Marshal.ReadInt32(at+68)==8,"actual known descriptor size");
                    Check(Marshal.PtrToStringUni(at+72)==files[index].Name.Replace('/','\\'),"SDK first/middle/last full-array descriptor indices");
                }
            }finally{GlobalUnlock(group.unionmember);}}finally{ReleaseStgMedium(ref group);}
            Check(calls.Count==0,"257 descriptors must not materialize any member");
            for(var index=1;index<=256;index++) {
                var format=Format(ArchiveVirtualFileDataObject.ContentsFormat,index,TYMED.TYMED_ISTREAM);
                Check(get(pointer,ref format,out var medium)==0&&medium.tymed==TYMED.TYMED_ISTREAM,"native indexed demand "+index);
                try {
                    Check(Stat(ref medium)==8,"native IStream actual size");
                    if(index==256){Check(Read(ref medium,1)=="f","retained last cursor");held=medium;medium=default;}
                    else Check(Read(ref medium,8)==$"file-{index-1:000}","actual first/middle/indexed bytes "+index);
                }finally{if(medium.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref medium);}
            }
            var repeat=Format(ArchiveVirtualFileDataObject.ContentsFormat,129,TYMED.TYMED_ISTREAM);
            Check(get(pointer,ref repeat,out var repeated)==0,"a legitimate repeated cursor after all 256 files is not request overflow");
            try{Check(Read(ref repeated,8)=="file-128","independent middle cursor after complete selection");}finally{ReleaseStgMedium(ref repeated);}
            Check(calls.Count==257&&calls.Take(256).SequenceEqual(Enumerable.Range(1,256)),"all full-array indices, then a legitimate repeat");
            data.Dispose();Check(Read(ref held,7)=="ile-255","last native stream survives data-object retirement");
        }finally{if(held.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref held);Marshal.Release(pointer);}
    }
    private static void Known64BitDescriptorSize()
    {
        const long known=(1L<<33)+17;
        using var data=new ArchiveVirtualFileDataObject([new("large.bin",known),new("unknown.bin",null)],_=>throw new InvalidOperationException("descriptor read cannot request content"));
        var format=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);data.GetData(ref format,out var group);
        try{var pointer=GlobalLock(group.unionmember);try{
            Check(unchecked((uint)Marshal.ReadInt32(pointer+4))==0x80000044u,"known size sets FD_FILESIZE");
            var actual=((ulong)unchecked((uint)Marshal.ReadInt32(pointer+4+64))<<32)|unchecked((uint)Marshal.ReadInt32(pointer+4+68));
            Check(actual==(ulong)known,"FILEDESCRIPTORW preserves all 64 size bits, never fake zero");
            Check(unchecked((uint)Marshal.ReadInt32(pointer+4+592))==0x80000004u,"unknown remains no FD_FILESIZE");
        }finally{GlobalUnlock(group.unionmember);}}finally{ReleaseStgMedium(ref group);}
    }
    private static async Task OrdinaryDirectoryActualDemandAndTotals()
    {
        var worker=Worker();var root=Root();STGMEDIUM held=default;
        try {
            File.Copy(worker,Path.Combine(root,ArchiveReadHelper.DeploymentName));var path=Path.Combine(root,"ordinary.zip");
            using(var zip=ZipFile.Open(path,ZipArchiveMode.Create))for(var ordinal=0;ordinal<256;ordinal++)FileEntry(zip,$"ordinary/file-{ordinal:000}.txt",$"file-{ordinal:000}");
            var metadata=new List<ArchiveReadEntry>();var browser=await Browse(root,path,batch=>metadata.AddRange(batch.Entries));
            Check(metadata.Count==256&&metadata.Select(item=>item.Ordinal).SequenceEqual(Enumerable.Range(0,256).Select(index=>(ulong)index)),"real ZIP read keeps total file count and original ordinals");
            var raw=Compact(OrdinaryDirectoryRaw(browser,path));var command=Parse(raw);var progress=new ConcurrentQueue<ArchiveVirtualDragProgress>();
            var demands=Enumerable.Range(0,32).Concat(new[]{128,255}).ToArray();
            await using(var entries=ArchiveEntrySession.ForOwnedTestPackage(root)) {
                await using var drag=new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,escape:()=>false,loop:(data,_)=>{
                    var capability=(IDataObjectAsyncCapability)data;var ended=false;
                    Check(capability.StartOperation(IntPtr.Zero)==0,"original async receiver negotiation");
                    var pointer=Marshal.GetComInterfaceForObject(data,typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
                    try {
                        var get=NativeGetData(pointer);var descriptor=Format(ArchiveVirtualFileDataObject.DescriptorFormat,-1,TYMED.TYMED_HGLOBAL);
                        Check(get(pointer,ref descriptor,out var group)==0,"original native descriptor demand");
                        try{var bytes=GlobalLock(group.unionmember);try{Check(Marshal.ReadInt32(bytes)==257,"native descriptor count includes all files and parent");}finally{GlobalUnlock(group.unionmember);}}finally{ReleaseStgMedium(ref group);}
                        Check(!Directory.GetFiles(root,"manifest.json",SearchOption.AllDirectories).Any(),"descriptor-only requests do not start extraction");
                        foreach(var ordinal in demands) {
                            var format=Format(ArchiveVirtualFileDataObject.ContentsFormat,ordinal+1,TYMED.TYMED_ISTREAM);
                            Check(get(pointer,ref format,out var medium)==0,"real worker/native demand past cache lease32, source ordinal "+ordinal);
                            try {
                                Check(Stat(ref medium)==8,"actual sealed size, not archive source size");
                                if(ordinal==255){Check(Read(ref medium,1)=="f","retain last actual source cursor");held=medium;medium=default;}
                                else Check(Read(ref medium,8)==$"file-{ordinal:000}","actual original ZIP member bytes "+ordinal);
                            }finally{if(medium.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref medium);}
                        }
                        var last=Format(ArchiveVirtualFileDataObject.ContentsFormat,256,TYMED.TYMED_ISTREAM);
                        Check(get(pointer,ref last,out var repeat)==0,"repeat last original member");
                        try{Check(Read(ref repeat,8)=="file-255","actual last bytes and independent cursor");}finally{ReleaseStgMedium(ref repeat);}
                        Check(capability.EndOperation(0,IntPtr.Zero,1)==0,"receiver reports COPY completion");ended=true;return(0x00040100,1);
                    }finally{if(!ended)capability.EndOperation(unchecked((int)0x80004004),IntPtr.Zero,0);Marshal.Release(pointer);}
                });
                drag.Progress=frame=>progress.Enqueue(frame.Payload.GetProperty("data").Deserialize<ArchiveVirtualDragProgress>(JsonDefaults.Options)!);
                var result=await drag.Dispatch(Wire(raw),1).WaitAsync(TimeSpan.FromSeconds(120));
                Check(result.Success,"ordinary directory must reach original native demand/cleanup: "+result.Error);
                var final=progress.Last();
                Check(progress.All(item=>item.FilesTotal==256&&item.BytesTotal==2048),"all progress preserves complete logical file count, not active work count");
                Check(final.Phase=="finished"&&final.Code=="receiver_reported_complete"&&final.CleanupComplete==true&&final.FilesDone==(ulong)demands.Length&&final.BytesDone==(ulong)demands.Length*8,"only actual distinct demands count as sealed work");
                Check(Stat(ref held)==8&&Read(ref held,7)=="ile-255","actual first/middle/last streams and retained last handle survive cleanup");
                ReleaseStgMedium(ref held);held=default;
            }
        }finally{if(held.unionmember!=IntPtr.Zero)ReleaseStgMedium(ref held);Directory.Delete(root,true);}
    }
    [DllImport("kernel32.dll",ExactSpelling=true)]private static extern IntPtr GlobalLock(IntPtr memory);
    [DllImport("kernel32.dll",ExactSpelling=true)]private static extern bool GlobalUnlock(IntPtr memory);
}
