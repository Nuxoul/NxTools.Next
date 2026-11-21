param(
    [Parameter(Mandatory=$true)][int]$PreviewProcessId,
    [switch]$AssertClientOnly
)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not ('NativeMaterialPreviewWindow' -as [type])) {
    Add-Type -Path (Join-Path $root 'scripts\preview-native-material-window.cs')
}
if (-not ('NativeMaterialFrameGate' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public sealed class NativeMaterialFrameSnapshot {
    public int WindowWidth,WindowHeight,ClientWidth,ClientHeight;
    public int LeftInset,TopInset,RightInset,BottomInset;
    public long Style,ExtendedStyle;
    public bool ClientOnly { get { return Math.Abs(LeftInset)<=1 && Math.Abs(TopInset)<=1 && Math.Abs(RightInset)<=1 && Math.Abs(BottomInset)<=1; } }
}
public static class NativeMaterialFrameGate {
    [StructLayout(LayoutKind.Sequential)] private struct Rect { public int Left,Top,Right,Bottom; }
    [StructLayout(LayoutKind.Sequential)] private struct Point { public int X,Y; }
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr window,out Rect rect);
    [DllImport("user32.dll")] private static extern bool GetClientRect(IntPtr window,out Rect rect);
    [DllImport("user32.dll")] private static extern bool ClientToScreen(IntPtr window,ref Point point);
    [DllImport("user32.dll",EntryPoint="GetWindowLongPtrW")] private static extern IntPtr GetWindowLongPtr(IntPtr window,int index);
    [DllImport("dwmapi.dll")] private static extern int DwmGetWindowAttribute(IntPtr window,int attribute,out Rect rect,int size);
    public static NativeMaterialFrameSnapshot Read(long handle) {
        IntPtr window=new IntPtr(handle); Rect outer,client,visible; Point origin=new Point();
        if(!GetWindowRect(window,out outer)||!GetClientRect(window,out client)||!ClientToScreen(window,ref origin)) throw new InvalidOperationException("Native frame bounds unavailable");
        int result=DwmGetWindowAttribute(window,9,out visible,Marshal.SizeOf(typeof(Rect)));
        if(result!=0) throw new InvalidOperationException("Native visible frame readback failed");
        return new NativeMaterialFrameSnapshot { WindowWidth=outer.Right-outer.Left,WindowHeight=outer.Bottom-outer.Top,ClientWidth=client.Right,ClientHeight=client.Bottom,LeftInset=origin.X-visible.Left,TopInset=origin.Y-visible.Top,RightInset=visible.Right-origin.X-client.Right,BottomInset=visible.Bottom-origin.Y-client.Bottom,Style=GetWindowLongPtr(window,-16).ToInt64(),ExtendedStyle=GetWindowLongPtr(window,-20).ToInt64() };
    }
}
'@
}
$window=[NativeMaterialPreviewWindow]::FindWindow($PreviewProcessId)
$frame=[NativeMaterialFrameGate]::Read($window)
$frame | ConvertTo-Json -Compress | Write-Output
if ($AssertClientOnly -and -not $frame.ClientOnly) {
    throw 'Native material has an external non-client collar outside the GPUI client surface.'
}
