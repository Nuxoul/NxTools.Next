using System.Runtime.InteropServices;
using System.Text;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;
using ComData = System.Runtime.InteropServices.ComTypes.IDataObject;

namespace NxTools.PlatformHost.Tests;

internal static class FloatingAssistantDropTests
{
    internal static Task CapturesOnceAndRejectsInactive() => FloatingAssistantInputTests.OnSta(() =>
    {
        var active = true;
        var received = new List<FloatingAssistantDropData>();
        var log = new DropLog();
        var target = new FloatingAssistantDropTarget(() => active, value => { received.Add(value); return true; }, log);
        var data = new System.Windows.Forms.DataObject();
        data.SetData(System.Windows.Forms.DataFormats.UnicodeText, false, "fixture 文本 https://example.invalid/image.png");
        data.SetData("UniformResourceLocatorW", false, new MemoryStream(Encoding.Unicode.GetBytes("https://example.invalid/image.png\0")));
        data.SetData(System.Windows.Forms.DataFormats.Html, false, "<img src=\"https://example.invalid/image.png\">");
        var png = Convert.FromBase64String("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1kAAAAASUVORK5CYII=");
        data.SetData("PNG", false, new MemoryStream(png));
        uint effect = 1;
        var com = (ComData)data;
        Assert(target.DragEnter(com, 1, default, ref effect) == 0 && effect == 1, "supported OLE offer accepted");
        Assert(received.Count == 0, "DragEnter must not consume payload or dispatch action");
        Assert(target.Drop(com, 0, default, ref effect) == 0 && effect == 1 && received.Count == 1, "one accepted drop");
        var value = received[0];
        Assert(value.Text == "fixture 文本 https://example.invalid/image.png" &&
            value.Url == "https://example.invalid/image.png" && value.Html == "<img src=\"https://example.invalid/image.png\">" &&
            value.ImageBytes!.SequenceEqual(png) && value.ImageFormat == "png", "independent original representations retained exactly");
        Assert(value.CorrelationId != Guid.Empty &&
            log.Entries.Any(entry => entry.Phase == "floating_assistant.drop.accepted" && entry.Id == value.CorrelationId) &&
            log.Entries.Any(entry => entry.Phase == "floating_assistant.drop.terminal" && entry.Id == value.CorrelationId),
            "OLE admission, terminal diagnostic and delivered snapshot carry one correlation");
        effect = 1;
        target.Drop(com, 0, default, ref effect);
        Assert(effect == 0 && received.Count == 1, "duplicate Drop without a new Enter cannot dispatch again");
        active = false;
        effect = 1;
        target.DragEnter(com, 1, default, ref effect);
        Assert(effect == 0, "hidden/click-through receiver refuses OLE admission");
        target.Drop(com, 0, default, ref effect);
        Assert(received.Count == 1, "inactive receiver cannot capture data");
    });

    internal static Task NativeComAbiAndBoundedFailures() => FloatingAssistantInputTests.OnSta(() =>
    {
        var received = new List<FloatingAssistantDropData>();
        var target = new FloatingAssistantDropTarget(() => true, value => { received.Add(value); return true; });
        var value = new System.Windows.Forms.DataObject();
        value.SetText("原生 COM fixture", System.Windows.Forms.TextDataFormat.UnicodeText);
        var pointer = Marshal.GetComInterfaceForObject(target, typeof(IFloatingAssistantDropTarget));
        var source = Marshal.GetComInterfaceForObject(value, typeof(ComData));
        try
        {
            var table = Marshal.ReadIntPtr(pointer);
            var enter = Marshal.GetDelegateForFunctionPointer<DropAbi>(Marshal.ReadIntPtr(table, 3 * IntPtr.Size));
            var drop = Marshal.GetDelegateForFunctionPointer<DropAbi>(Marshal.ReadIntPtr(table, 6 * IntPtr.Size));
            uint effect = 1;
            Assert(enter(pointer, source, 1, default, ref effect) == 0 && effect == 1, "native IDropTarget slot3 ABI");
            Assert(drop(pointer, source, 0, default, ref effect) == 0 && effect == 1 && received.Single().Text == "原生 COM fixture",
                "native IDropTarget slot6 exact text ABI");
        }
        finally { Marshal.Release(source); Marshal.Release(pointer); }
        var oversized = new System.Windows.Forms.DataObject();
        oversized.SetText(new string('x', 40000), System.Windows.Forms.TextDataFormat.UnicodeText);
        uint result = 1;
        target.DragEnter((ComData)oversized, 1, default, ref result);
        target.Drop((ComData)oversized, 0, default, ref result);
        Assert(result == 0 && received.Count == 1, "oversize text rejected before consumer");
        result = 1; target.DragEnter((ComData)value, 1, default, ref result); target.DragLeave();
        target.Drop((ComData)value, 0, default, ref result);
        Assert(result == 0 && received.Count == 1, "leave cancels original admitted offer");
        var files = new System.Windows.Forms.DataObject();
        files.SetFileDropList(new System.Collections.Specialized.StringCollection { @"C:\synthetic\image.png" });
        result = 1; target.DragEnter((ComData)files, 1, default, ref result); target.Drop((ComData)files, 0, default, ref result);
        Assert(result == 1 && received[1].Files.SequenceEqual(new[] { @"C:\synthetic\image.png" }), "native file drop preserves path without reading file");
        using var bitmap = new System.Drawing.Bitmap(2, 2);
        var image = new System.Windows.Forms.DataObject(); image.SetImage(bitmap);
        result = 1; target.DragEnter((ComData)image, 1, default, ref result); target.Drop((ComData)image, 0, default, ref result);
        Assert(result == 1 && received[2].ImageBytes is { Length: > 0 }, "actual GDI bitmap captured without clipboard");
    });

    private sealed class DropLog : IPlatformLog
    {
        internal List<(string Phase, Guid? Id)> Entries { get; } = [];
        public void Write(string level, string eventName, Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null) => Entries.Add((eventName, correlationId));
    }

    internal static Task ThrowingOfferGateReturnsOleFailure() => FloatingAssistantInputTests.OnSta(() =>
    {
        var active = true;
        var target = new FloatingAssistantDropTarget(() => active, _ => throw new InvalidOperationException("fixture"));
        var value = new System.Windows.Forms.DataObject();
        value.SetText("fixture", System.Windows.Forms.TextDataFormat.UnicodeText);
        uint effect = 1;
        var data = (ComData)value;
        Assert(target.DragEnter(data, 0, default, ref effect) == 0 && effect == 1, "normal OLE admission");
        active = false;
        Assert(target.DragOver(0, default, ref effect) == 0 && effect == 0,
            "inactive DragOver terminates admission");
        var failGate = false;
        var gate = new FloatingAssistantDropTarget(() => failGate ? throw new InvalidOperationException("fixture") : true, _ => true);
        effect = 1;
        Assert(gate.DragEnter(data, 0, default, ref effect) == 0 && effect == 1, "callback failure fixture admits first offer");
        failGate = true;
        Assert(gate.DragOver(0, default, ref effect) == 0 && effect == 0,
            "failing native offer gate cancels without crossing COM");
        effect = 1;
        Assert(gate.DragEnter(data, 0, default, ref effect) == 0 && effect == 0,
            "failing DragEnter active callback never crosses COM");
        failGate = false;
        effect = 1;
        Assert(gate.DragEnter(data, 0, default, ref effect) == 0 && effect == 1, "new offer admitted after failure");
        failGate = true;
        Assert(gate.Drop(data, 0, default, ref effect) == 0 && effect == 0,
            "failing Drop active callback never crosses COM");
        active = true;
        effect = 1;
        Assert(target.DragEnter(data, 0, default, ref effect) == 0 && effect == 1, "fresh offer admitted");
        Assert(target.Drop(data, 0, default, ref effect) == 0 && effect == 0,
            "receiver failure never unwinds COM or claims a successful drop");
    });
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate int DropAbi(IntPtr self, IntPtr data, uint keys, FloatingAssistantDropPoint point, ref uint effect);

    private static void Assert(bool condition, string message)
    { if (!condition) throw new InvalidOperationException(message); }
}
