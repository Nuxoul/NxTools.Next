using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed record ArchiveVirtualFile(string Name, long? Size, bool Directory=false);

// Synchronous delayed rendering on the dedicated OLE STA only. Every successful GetData transfers
// one native medium reference to the consumer, who must call ReleaseStgMedium. No managed IStream CCW.
[ComVisible(true), ClassInterface(ClassInterfaceType.None)]
internal sealed class ArchiveVirtualFileDataObject : System.Runtime.InteropServices.ComTypes.IDataObject, IDisposable
{
    internal static readonly short DescriptorFormat = (short)RegisterClipboardFormatW("FileGroupDescriptorW");
    internal static readonly short ContentsFormat = (short)RegisterClipboardFormatW("FileContents");
    internal static readonly short EffectFormat = (short)RegisterClipboardFormatW("Preferred DropEffect");
    private const int FormatError=unchecked((int)0x80040064), IndexError=unchecked((int)0x80040068), MediumError=unchecked((int)0x80040069);
    private readonly ArchiveVirtualFile[] files;
    private readonly Func<int, IntPtr> open;
    private int retired;
    private int contentRequests;
    internal ArchiveVirtualFileDataObject(ArchiveVirtualFile[] files, Func<int, IntPtr> open)
    {
        if(!ValidTree(files))throw new ArgumentException("invalid_virtual_files");
        this.files=files.ToArray();this.open=open;
    }
    internal static bool ValidTree(ArchiveVirtualFile[] files)
    {
        if(files.Length is <1 or >128||files.Any(file=>file is null||!ValidName(file.Name)||file.Size is <0 or >536870912||file.Directory&&file.Size is not null)||
            files.Count(file=>!file.Directory)>32||files.Sum(file=>file.Size??0)>536870912)return false;
        var names=new Dictionary<string,ArchiveVirtualFile>(StringComparer.OrdinalIgnoreCase);
        var paths=new Dictionary<string,string>(StringComparer.OrdinalIgnoreCase);
        foreach(var file in files) {
            if(!names.TryAdd(file.Name,file))return false;
            var parts=file.Name.Split('/');
            for(var i=1;i<=parts.Length;i++) {
                var path=string.Join('/',parts[..i]);
                if(paths.TryGetValue(path,out var prior)&&prior!=path)return false;
                paths[path]=path;
            }
        }
        if(files.Select(file=>file.Name.Split('/')[0]).Distinct(StringComparer.OrdinalIgnoreCase).Count()>32)return false;
        foreach(var file in files) {
            var parts=file.Name.Split('/');
            for(var i=1;i<parts.Length;i++)if(!names.TryGetValue(string.Join('/',parts[..i]),out var parent)||!parent.Directory)return false;
        }
        return true;
    }
    internal static bool ValidName(string? name)
    {
        if(string.IsNullOrEmpty(name)||name.Length>=260||name.Contains('\\'))return false;
        try {foreach(var part in name.Split('/'))ArchiveOutputSession.Validate(part);return true;}
        catch(ArchiveOutputException){return false;}
    }
    public void GetData(ref FORMATETC format, out STGMEDIUM medium)
    {
        medium=default;var result=QueryGetData(ref format);if(result<0)throw new COMException("virtual_format_rejected",result);
        if(format.cfFormat==ContentsFormat) {
            if(Interlocked.Increment(ref contentRequests)>256)throw new COMException("virtual_request_budget",unchecked((int)0x8007000E));
            var stream=open(format.lindex);
            if(stream==IntPtr.Zero)throw new COMException("virtual_stream_unavailable",unchecked((int)0x8003001E));
            if(Volatile.Read(ref retired)!=0){Marshal.Release(stream);throw new COMException("virtual_drag_retired",unchecked((int)0x800704C7));}
            medium=new(){tymed=TYMED.TYMED_ISTREAM,unionmember=stream};return;
        }
        var bytes=format.cfFormat==EffectFormat?BitConverter.GetBytes(1):Descriptors();
        medium=new(){tymed=TYMED.TYMED_HGLOBAL,unionmember=Allocate(bytes)};
    }
    private byte[] Descriptors()
    {
        // SDK FILEDESCRIPTORW:592bytes, flags0, attributes36, high/low size64/68, UTF16name72.
        var bytes=new byte[4+592*files.Length];BitConverter.GetBytes(files.Length).CopyTo(bytes,0);
        for(var index=0;index<files.Length;index++) {
            var at=4+index*592;var file=files[index];var known=!file.Directory&&file.Size is not null;
            BitConverter.GetBytes(known?0x80000044u:0x80000004u).CopyTo(bytes,at);
            BitConverter.GetBytes(file.Directory?0x10u:0x80u).CopyTo(bytes,at+36);
            if(known){BitConverter.GetBytes((uint)((ulong)file.Size!.Value>>32)).CopyTo(bytes,at+64);BitConverter.GetBytes((uint)file.Size.Value).CopyTo(bytes,at+68);}
            Encoding.Unicode.GetBytes(file.Name.Replace('/','\\')).CopyTo(bytes,at+72);
        }
        return bytes;
    }
    public int QueryGetData(ref FORMATETC format)
    {
        if(Volatile.Read(ref retired)!=0)return unchecked((int)0x800704C7);
        if(format.ptd!=IntPtr.Zero || format.dwAspect!=DVASPECT.DVASPECT_CONTENT)return FormatError;
        if(format.cfFormat==ContentsFormat){if(format.lindex<0||format.lindex>=files.Length||files[format.lindex].Directory)return IndexError;return(format.tymed&TYMED.TYMED_ISTREAM)!=0?0:MediumError;}
        if(format.cfFormat!=DescriptorFormat&&format.cfFormat!=EffectFormat)return FormatError;
        if(format.lindex!=-1)return IndexError;
        return(format.tymed&TYMED.TYMED_HGLOBAL)!=0?0:MediumError;
    }
    public IEnumFORMATETC EnumFormatEtc(DATADIR direction)
    {
        if(direction!=DATADIR.DATADIR_GET)throw new COMException("not_supported",unchecked((int)0x80004001));
        return new Formats([Format(DescriptorFormat,-1,TYMED.TYMED_HGLOBAL),Format(ContentsFormat,-1,TYMED.TYMED_ISTREAM),Format(EffectFormat,-1,TYMED.TYMED_HGLOBAL)]);
    }
    private static FORMATETC Format(short id,int index,TYMED medium)=>new(){cfFormat=id,lindex=index,dwAspect=DVASPECT.DVASPECT_CONTENT,tymed=medium};
    [ComVisible(true),ClassInterface(ClassInterfaceType.None)]
    private sealed class Formats(FORMATETC[] formats,int position=0):IEnumFORMATETC
    {
        private int cursor=position;
        public int Next(int count,FORMATETC[] values,int[]? fetched){if(count<0||values.Length<count||count!=1&&(fetched is null||fetched.Length==0))return unchecked((int)0x80070057);var read=0;while(read<count&&cursor<formats.Length)values[read++]=formats[cursor++];if(fetched is {Length:>0})fetched[0]=read;return read==count?0:1;}
        public int Skip(int count){if(count<0)return unchecked((int)0x80070057);var read=Math.Min(count,formats.Length-cursor);cursor+=read;return read==count?0:1;}
        public int Reset(){cursor=0;return 0;}
        public void Clone(out IEnumFORMATETC clone)=>clone=new Formats(formats,cursor);
    }
    public void GetDataHere(ref FORMATETC format, ref STGMEDIUM medium)=>throw new COMException("not_supported",MediumError);
    public int GetCanonicalFormatEtc(ref FORMATETC input,out FORMATETC output){output=input;output.ptd=IntPtr.Zero;return 0x00040130;}
    public void SetData(ref FORMATETC format,ref STGMEDIUM medium,bool release)=>throw new COMException("not_supported",unchecked((int)0x80004001));
    public int DAdvise(ref FORMATETC format,ADVF flags,IAdviseSink sink,out int connection){connection=0;return unchecked((int)0x80040003);}
    public void DUnadvise(int connection)=>throw new COMException("not_supported",unchecked((int)0x80040003));
    public int EnumDAdvise(out IEnumSTATDATA enumerator){enumerator=null!;return unchecked((int)0x80040003);}
    public void Dispose()=>Interlocked.Exchange(ref retired,1);
    private static IntPtr Allocate(byte[] bytes)
    {
        var memory=GlobalAlloc(0x42,(UIntPtr)bytes.Length);if(memory==IntPtr.Zero)throw new OutOfMemoryException();
        var pointer=GlobalLock(memory);if(pointer==IntPtr.Zero){GlobalFree(memory);throw new COMException("global_lock",Marshal.GetHRForLastWin32Error());}
        try{Marshal.Copy(bytes,0,pointer,bytes.Length);}finally{GlobalUnlock(memory);}return memory;
    }
    [DllImport("user32.dll",CharSet=CharSet.Unicode,ExactSpelling=true)] private static extern uint RegisterClipboardFormatW(string name);
    [DllImport("kernel32.dll",ExactSpelling=true,SetLastError=true)] private static extern IntPtr GlobalAlloc(uint flags,UIntPtr bytes);
    [DllImport("kernel32.dll",ExactSpelling=true,SetLastError=true)] private static extern IntPtr GlobalLock(IntPtr memory);
    [DllImport("kernel32.dll",ExactSpelling=true)] private static extern bool GlobalUnlock(IntPtr memory);
    [DllImport("kernel32.dll",ExactSpelling=true)] private static extern IntPtr GlobalFree(IntPtr memory);
}
