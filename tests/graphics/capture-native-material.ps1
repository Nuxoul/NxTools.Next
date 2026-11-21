param(
    [Parameter(Mandatory=$true)][long]$WindowHandle,
    [Parameter(Mandatory=$true)][int]$ExpectedProcessId,
    [Parameter(Mandatory=$true)][string]$Destination
)
$ErrorActionPreference='Stop'
if (-not ('NativeMaterialCapture' -as [type])) {
    Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;
public static class NativeMaterialCapture {
    [StructLayout(LayoutKind.Sequential)] private struct Rect { public int Left,Top,Right,Bottom; }
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr window,out Rect rect);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window,out uint processId);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] private static extern int GetWindowText(IntPtr window,StringBuilder text,int count);
    public static void Capture(long handle,int expectedProcessId,string destination) {
        IntPtr window=new IntPtr(handle);
        uint owner;
        GetWindowThreadProcessId(window,out owner);
        var title=new StringBuilder(128);
        GetWindowText(window,title,title.Capacity);
        if(owner!=(uint)expectedProcessId || title.ToString()!="NxTools \u2014 Native Material Preview (F2 / Esc)") throw new InvalidOperationException("Preview identity mismatch");
        if(GetForegroundWindow()!=window) throw new InvalidOperationException("Preview is not foreground; no desktop capture was taken");
        Rect rect;
        if(!GetWindowRect(window,out rect)) throw new InvalidOperationException("Preview bounds unavailable");
        using(var bitmap=new Bitmap(rect.Right-rect.Left,rect.Bottom-rect.Top,PixelFormat.Format32bppArgb)) {
            using(var graphics=Graphics.FromImage(bitmap)) graphics.CopyFromScreen(rect.Left,rect.Top,0,0,bitmap.Size,CopyPixelOperation.SourceCopy);
            bitmap.Save(destination,ImageFormat.Png);
        }
    }
}
'@
}
[NativeMaterialCapture]::Capture($WindowHandle,$ExpectedProcessId,$Destination)
Write-Output 'Captured foreground-owned native GPUI/DWM window'
