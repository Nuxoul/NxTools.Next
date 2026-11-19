using System.Collections.Concurrent;
using System.ComponentModel;
using System.IO.Pipes;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using Microsoft.Web.WebView2.Core;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class MarkdownDocumentPickerTests
{
    private const string CommandName = "pick_markdown_document";
    private static readonly TimeSpan FixtureWait = TimeSpan.FromSeconds(10);

    public static async Task RejectsInvalidPeerInReader()
    {
        var log = new CaptureLog();
        await using var pipe = new PickerPipe(log: log);
        await pipe.Connect();
        var process = (uint)Environment.ProcessId;
        foreach (var request in new[]
        {
            Request(new { SourceWindowHandle = 1UL, SourceProcessId = process + 1 }),
            Request(new { SourceWindowHandle = 0UL, SourceProcessId = process }),
            Request(new { SourceWindowHandle = ulong.MaxValue, SourceProcessId = process }),
            Request(new { SourceProcessId = process })
        })
        {
            await pipe.Send(request);
            CheckFailure(await pipe.Receive(request.Id), "picker_invalid_peer", 87);
            var row = log.Rows.Single(entry => entry.Id == request.CorrelationId);
            Check(row.Name == "markdown.reader.picker.terminal" && Equals(row.Fields!["category"], "picker_invalid_peer"),
                "invalid source is rejected before picker admission or presentation");
        }
    }

    public static Task OwnedMultiselectIsDetachedAndRetired()
    {
        using var owner = new OwnedWindow();
        var request = OwnerRequest(owner.Handle);
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var pipe = new PickerPipe((dialog, hwnd) =>
        {
            var configured = hwnd.Handle == owner.Handle && Thread.CurrentThread.GetApartmentState() == ApartmentState.STA &&
                dialog.Multiselect && dialog.CheckFileExists && dialog.CheckPathExists && dialog.RestoreDirectory &&
                dialog.AutoUpgradeEnabled && dialog.Filter == "Markdown 文档 (*.md;*.markdown)|*.md;*.markdown";
            entered.Set();
            Check(release.Wait(FixtureWait), "fixture releases modal presentation");
            return configured ? [@"C:\owned\retired.md"] : throw new InvalidOperationException("wrong owner, STA or dialog options");
        });
        try
        {
            pipe.Connect().GetAwaiter().GetResult();
            pipe.Send(request).GetAwaiter().GetResult();
            Check(entered.Wait(FixtureWait), "original STA reached presentation");
            var ping = ProtocolEnvelope.Create("ping", new { SentAtUnixMs = 42 });
            pipe.Send(ping).GetAwaiter().GetResult();
            Check(pipe.Receive(ping.Id).GetAwaiter().GetResult().Type == "pong", "modal selection does not stall the serial reader");
            var competing = OwnerRequest(owner.Handle);
            pipe.Send(competing).GetAwaiter().GetResult();
            CheckFailure(pipe.Receive(competing.Id).GetAwaiter().GetResult(), "picker_busy", 170);
            owner.Dispose(); // Retire on the HWND's creating thread.
            release.Set();
            CheckFailure(pipe.Receive(request.Id).GetAwaiter().GetResult(), "picker_invalid_owner", 1400);
        }
        finally
        {
            release.Set();
            pipe.DisposeAsync().AsTask().GetAwaiter().GetResult();
        }
        return Task.CompletedTask;
    }

    public static Task ResultBoundsAndDiagnostics()
    {
        using var owner = new OwnedWindow();
        var log = new CaptureLog();
        var scenarios = new ConcurrentQueue<Func<string[]>>();
        var pipe = new PickerPipe((_, _) => scenarios.TryDequeue(out var pick)
            ? pick() : throw new InvalidOperationException("missing controlled selection"), log);
        try
        {
            pipe.Connect().GetAwaiter().GetResult();
            ProtocolEnvelope Pick(Func<string[]> show, out Guid operation)
            {
                scenarios.Enqueue(show);
                var request = OwnerRequest(owner.Handle);
                operation = request.CorrelationId!.Value;
                Check(PlatformCommandParser.ParsePickMarkdownDocument(request).SourceWindowHandle == (ulong)owner.Handle.ToInt64(),
                    "unsigned source HWND survives typed parsing");
                pipe.Send(request).GetAwaiter().GetResult();
                return pipe.Receive(request.Id).GetAwaiter().GetResult();
            }
            string[] selected = [@"C:\private-path-sentinel\读我.MD", @"C:\owned\中文.markdown"];
            Check(ReadPaths(Pick(() => selected, out var selectedId)).SequenceEqual(selected), "ordered Unicode multiselect roundtrips");
            Check(ReadPaths(Pick(() => [], out var cancelId)).Length == 0, "user cancel is success with an explicit empty vector");
            var countLimit = Enumerable.Range(0, 32).Select(i => $@"C:\owned\{i}.md").ToArray();
            Check(ReadPaths(Pick(() => countLimit, out _)).SequenceEqual(countLimit), "exact selection count is admitted");
            CheckFailure(Pick(() => [.. countLimit, @"C:\owned\overflow.md"], out _), "picker_limit");
            // ASCII apostrophes are legal in filenames and exercise JSON's widest escape expansion.
            var components = string.Concat(Enumerable.Repeat(new string('\'', 240) + "\\", 70));
            var escaped = @"C:\" + components[..16378] + ".md";
            var byteLimit = Enumerable.Repeat(escaped, 8).ToArray();
            Check(byteLimit.Sum(Encoding.UTF8.GetByteCount) == 131072, "fixture fills the raw UTF-8 byte budget");
            Check(ReadPaths(Pick(() => byteLimit, out _)).SequenceEqual(byteLimit), "escaped exact byte-limit selection fits the real serialized frame");
            var overflow = byteLimit.ToArray();
            overflow[^1] = overflow[^1].Insert(overflow[^1].Length - 3, "x");
            CheckFailure(Pick(() => overflow, out _), "picker_limit");
            CheckFailure(Pick(() => [selected[0], "relative.md"], out _), "picker_invalid_selection");
            CheckFailure(Pick(() => [selected[0], @"C:\owned\not-markdown.txt"], out _), "picker_invalid_selection");
            CheckFailure(Pick(() => throw new Win32Exception(5, "private-path-sentinel"), out var failedId), "picker_failed", 5);
            CheckLifecycle(log, selectedId, "selected", 2, null);
            CheckLifecycle(log, cancelId, "cancelled", 0, null);
            CheckLifecycle(log, failedId, "picker_failed", 0, 5);
            Check(!JsonSerializer.Serialize(log.Rows.Select(row => new { row.Name, row.Id, row.Fields })).Contains("private-path-sentinel", StringComparison.Ordinal),
                "diagnostics never expose selected paths or native exception messages");
        }
        finally { pipe.DisposeAsync().AsTask().GetAwaiter().GetResult(); }
        return Task.CompletedTask;
    }

    public static Task ExportPickerOwnsCancelAndRetirement()
    {
        using var owner = new OwnedWindow();
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var log = new CaptureLog();
        const string selected = @"C:\private-path-sentinel\报告.pdf";
        var presents = 0;
        var pipe = new PickerPipe(log: log, presentExport: (dialog, hwnd) =>
        {
            Check(hwnd.Handle == owner.Handle && Thread.CurrentThread.GetApartmentState() == ApartmentState.STA,
                "save dialog is bound to the original owner on an STA");
            Check(dialog.OverwritePrompt && dialog.CheckPathExists && dialog.AddExtension && dialog.RestoreDirectory,
                "save dialog uses native extension and overwrite handling");
            switch (++presents)
            {
                case 1:
                    Check(dialog.DefaultExt == "html" && dialog.FileName == "报告.html", "HTML defaults and suggestion");
                    return null;
                case 2:
                    Check(dialog.DefaultExt == "pdf" && dialog.FileName == "报告.pdf", "PDF defaults and suggestion");
                    return selected;
                default:
                    entered.Set();
                    Check(release.Wait(FixtureWait), "fixture releases the save dialog");
                    return selected;
            }
        });
        try
        {
            pipe.Connect().GetAwaiter().GetResult();
            ProtocolEnvelope Save(string format, uint? pid = null) => Request(new
            {
                SourceWindowHandle = (ulong)owner.Handle.ToInt64(), SourceProcessId = pid ?? (uint)Environment.ProcessId,
                Format = format, SuggestedName = "报告"
            }, "pick_markdown_export");
            ProtocolEnvelope Exchange(ProtocolEnvelope request)
            {
                pipe.Send(request).GetAwaiter().GetResult();
                return pipe.Receive(request.Id).GetAwaiter().GetResult();
            }
            CheckFailure(Exchange(Save("pdf", (uint)Environment.ProcessId + 1)), "picker_invalid_peer", 87);
            CheckFailure(Exchange(Save("docx")), "picker_invalid_peer", 87);
            var cancel = Exchange(Save("html"));
            Check(cancel.Payload.GetProperty("success").GetBoolean() && cancel.Payload.GetProperty("data").ValueKind == JsonValueKind.Null,
                "user cancel returns success with no data");
            var pick = Save("pdf");
            var confirmed = Exchange(pick);
            Check(confirmed.Payload.GetProperty("success").GetBoolean() && confirmed.Payload.GetProperty("data").GetString() == selected,
                "selected path is plain data, not JSON nested in data");
            var retired = Save("pdf");
            pipe.Send(retired).GetAwaiter().GetResult();
            Check(entered.Wait(FixtureWait), "save picker entered its real detached STA");
            var ping = ProtocolEnvelope.Create("ping", new { SentAtUnixMs = 42 });
            Check(Exchange(ping).Type == "pong", "save picker does not block the reader");
            owner.Dispose();
            release.Set();
            CheckFailure(pipe.Receive(retired.Id).GetAwaiter().GetResult(), "picker_invalid_owner", 1400);
            Check(log.Rows.Count(row => row.Id == pick.CorrelationId && row.Name.StartsWith("markdown.reader.export_picker.")) == 3,
                "picker accepted/start/terminal share logical correlation");
            Check(!JsonSerializer.Serialize(log.Rows.Select(row => row.Fields)).Contains("private-path-sentinel"),
                "picker logging omits the selected path");
        }
        finally
        {
            release.Set();
            pipe.DisposeAsync().AsTask().GetAwaiter().GetResult();
        }
        return Task.CompletedTask;
    }

    public static async Task PdfTransfersOnlySuccessfulRender()
    {
        var root = Directory.CreateTempSubdirectory("NxTools-MarkdownPdf-Fixture-");
        try
        {
            var htmlPath = Path.Combine(root.FullName, "private-path-sentinel.html");
            var outputPath = Path.Combine(root.FullName, "staged.pdf");
            await File.WriteAllTextAsync(htmlPath, "<!doctype html><p>报告</p>");
            var log = new CaptureLog();
            var scenarios = new ConcurrentQueue<Func<string, CancellationToken, Task<bool>>>();
            await using var pipe = new PickerPipe(log: log, renderPdf: (html, candidate, token) =>
            {
                Check(Thread.CurrentThread.GetApartmentState() == ApartmentState.STA, "only the dedicated STA renders PDF");
                Check(html.Length != 0 && candidate != outputPath, "renderer receives input and only a private candidate path");
                Check(scenarios.TryDequeue(out var render), "fixture renderer has exactly one configured outcome");
                return render!(candidate, token);
            });
            await pipe.Connect();
            async Task<ProtocolEnvelope> Print(string input, Guid operation)
            {
                var request = Request(new { HtmlPath = input, OutputPath = outputPath }, "print_markdown_pdf", operation);
                await pipe.Send(request);
                return await pipe.Receive(request.Id);
            }
            var correlation = Guid.NewGuid();
            CheckFailure(await Print("relative.html", correlation), "invalid_request", 87);
            scenarios.Enqueue((_, _) => throw new WebView2RuntimeNotFoundException());
            var missing = await Print(htmlPath, correlation);
            CheckFailure(missing, "runtime_missing", missing.Payload.GetProperty("native_error_code").GetInt32());
            Check(!File.Exists(outputPath), "missing runtime cannot produce a staging result");
            scenarios.Enqueue(async (path, _) => { await File.WriteAllTextAsync(path, "partial"); return false; });
            CheckFailure(await Print(htmlPath, correlation), "render_failed");
            Check(!File.Exists(outputPath) && !root.EnumerateDirectories().Any(), "private partial output is removed on renderer failure");
            await File.WriteAllTextAsync(outputPath, "existing output");
            CheckFailure(await Print(htmlPath, correlation), "output_exists", 80);
            Check(await File.ReadAllTextAsync(outputPath) == "existing output", "even an occupied staging path is not replaced");
            File.Delete(outputPath);
            scenarios.Enqueue(async (path, _) => { await File.WriteAllTextAsync(path, "%PDF-1.7\nfixture"); return true; });
            var success = await Print(htmlPath, correlation);
            Check(success.Payload.GetProperty("success").GetBoolean() && success.Payload.GetProperty("data").ValueKind == JsonValueKind.Null,
                "PDF success returns no filename or payload");
            Check(await File.ReadAllTextAsync(outputPath) == "%PDF-1.7\nfixture" && !root.EnumerateDirectories().Any(),
                "successful candidate is moved intact and the native directory is removed");
            Check(log.Rows.Where(row => row.Name.StartsWith("markdown.reader.pdf.")).All(row => row.Id == correlation) &&
                  log.Rows.Any(row => row.Name == "markdown.reader.pdf.terminal" && Equals(row.Fields!["category"], "runtime_missing")) &&
                  !JsonSerializer.Serialize(log.Rows.Select(row => row.Fields)).Contains("private-path-sentinel"),
                "PDF diagnostics preserve only correlation and safe lifecycle fields");
        }
        finally { root.Delete(recursive: true); }
    }

    public static async Task PdfDisconnectRetiresInFlightStaging()
    {
        var root = Directory.CreateTempSubdirectory("NxTools-MarkdownPdf-Fixture-");
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var log = new CaptureLog();
        try
        {
            var htmlPath = Path.Combine(root.FullName, "document.html");
            var outputPath = Path.Combine(root.FullName, "staged.pdf");
            await File.WriteAllTextAsync(htmlPath, "<!doctype html><p>Report</p>");
            await using var pipe = new PickerPipe(log: log, renderPdf: async (_, candidate, _) =>
            {
                entered.TrySetResult();
                await release.Task.WaitAsync(FixtureWait);
                // Simulate completion by a native renderer that did not observe cancellation.
                await File.WriteAllTextAsync(candidate, "%PDF-1.7\nlate render");
                return true;
            });
            await pipe.Connect();
            ProtocolEnvelope Print() => Request(new { HtmlPath = htmlPath, OutputPath = outputPath }, "print_markdown_pdf");
            var first = Print();
            await pipe.Send(first);
            await entered.Task.WaitAsync(FixtureWait);
            var ping = ProtocolEnvelope.Create("ping", new { SentAtUnixMs = 42 });
            await pipe.Send(ping);
            Check((await pipe.Receive(ping.Id)).Type == "pong", "PDF renderer does not block the pipe reader");
            var competing = Print();
            await pipe.Send(competing);
            CheckFailure(await pipe.Receive(competing.Id), "render_busy", 170);
            await pipe.Retire();
            Check(!pipe.HasReply && log.Rows.Any(row => row.Id == first.CorrelationId && row.Name == "markdown.reader.pdf.terminal" &&
                Equals(row.Fields!["category"], "render_cancelled")), "pipe retirement suppresses the original reply");
            release.TrySetResult();
            Check(SpinWait.SpinUntil(() => !root.EnumerateDirectories().Any(), FixtureWait) && !File.Exists(outputPath),
                "late renderer completion cleans the candidate and never touches the Desktop staging name");
        }
        finally
        {
            release.TrySetResult();
            root.Delete(recursive: true);
        }
    }

    public static async Task PdfUsesActualOfflineWebView()
    {
        var root = Directory.CreateTempSubdirectory("NxTools-MarkdownPdf-Native-");
        try
        {
            using var listener = new System.Net.Sockets.TcpListener(System.Net.IPAddress.Loopback, 0);
            listener.Start();
            var port = ((System.Net.IPEndPoint)listener.LocalEndpoint).Port;
            const string pixel = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAEklEQVR4nGN4J2i87r8QA4QCACpDBeOf83GEAAAAAElFTkSuQmCC";
            var html = Path.Combine(root.FullName, "report.html");
            var pdf = Path.Combine(root.FullName, "report.pdf");
            await File.WriteAllTextAsync(html, $$"""
                <!doctype html><meta charset="utf-8">
                <style>section { break-before: page } img { width: 30px; height: 30px; }</style>
                <h1>中文报告: selectable text</h1>
                <img src="data:image/png;base64,{{pixel}}" alt="embedded pixel">
                <img src="http://127.0.0.1:{{port}}/blocked">
                <script>fetch('http://127.0.0.1:{{port}}/no-script');</script>
                <section>第二页: second page</section>
                """);
            var result = MarkdownPdfPrinter.Print(new(html, pdf), Guid.NewGuid(), NullPlatformLog.Instance, CancellationToken.None);
            if (result.Error == "runtime_missing")
                throw new TestSkippedException("WebView2 Runtime is not installed");
            Check(result.Success, $"PrintToPdfAsync completed ({result.Error}, {result.NativeErrorCode})");
            var bytes = Encoding.Latin1.GetString(await File.ReadAllBytesAsync(pdf));
            Check(System.Text.RegularExpressions.Regex.Matches(bytes, @"/Type\s*/Page\b").Count >= 2 &&
                  bytes.Contains("/Subtype /Image", StringComparison.Ordinal) && bytes.Contains("/ToUnicode", StringComparison.Ordinal),
                "actual PDF contains multiple pages, an embedded image and a Unicode font text map");
            Check(!listener.Pending(), "neither document subresources nor scripts connected to the loopback listener");
            Check(!root.EnumerateDirectories().Any(), "the actual runtime profile is removed");
        }
        finally { root.Delete(recursive: true); }
    }

    private static void CheckLifecycle(CaptureLog log, Guid operation, string category, int count, int? nativeCode)
    {
        var rows = log.Rows.Where(row => row.Id == operation && row.Name.StartsWith("markdown.reader.picker.", StringComparison.Ordinal)).ToArray();
        Check(rows.Select(row => row.Name).SequenceEqual(new[] { "markdown.reader.picker.accepted", "markdown.reader.picker.start", "markdown.reader.picker.terminal" }),
            "owned picker lifecycle preserves the operation correlation");
        Check(rows.All(row => row.Fields!.ContainsKey("elapsed_ms")) && Equals(rows[^1].Fields!["category"], category) &&
              Equals(rows[^1].Fields!["count"], count) && Equals(rows[^1].Fields!["native_error_code"], nativeCode),
            "terminal diagnostic matches category, count and native code");
    }

    private static string[] ReadPaths(ProtocolEnvelope reply)
    {
        Check(reply.Type == "platform_command_result" && reply.Payload.GetProperty("success").GetBoolean(), "picker returns a normal successful command result");
        using var data = JsonDocument.Parse(reply.Payload.GetProperty("data").GetString()!);
        return data.RootElement.GetProperty("paths").Deserialize<string[]>(JsonDefaults.Options)!;
    }

    private static void CheckFailure(ProtocolEnvelope reply, string code, int? nativeCode = null)
    {
        Check(reply.Type == "platform_command_result" && !reply.Payload.GetProperty("success").GetBoolean() &&
              reply.Payload.GetProperty("error").GetString() == code &&
              reply.Payload.GetProperty("data").ValueKind == JsonValueKind.Null &&
              reply.Payload.GetProperty("native_error_code").Deserialize<int?>() == nativeCode,
            $"{code} is a typed failure with no partial result");
    }

    private static ProtocolEnvelope OwnerRequest(IntPtr owner) =>
        Request(new { SourceWindowHandle = (ulong)owner.ToInt64(), SourceProcessId = (uint)Environment.ProcessId });

    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?>? Fields)> Rows = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Rows.Enqueue((eventName, correlationId, fields));
    }

    private sealed class OwnedWindow : IDisposable
    {
        internal IntPtr Handle { get; private set; }

        internal OwnedWindow()
        {
            Handle = CreateWindowExW(0, "STATIC", "", 0, 0, 0, 1, 1, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            Check(Handle != IntPtr.Zero, "create self-owned HWND");
        }

        public void Dispose()
        {
            if (Handle == IntPtr.Zero) return;
            Check(DestroyWindow(Handle), "destroy only the self-owned HWND");
            Handle = IntPtr.Zero;
        }
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateWindowExW(uint exStyle, string className, string title, uint style,
        int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DestroyWindow(IntPtr hwnd);

    private static ProtocolEnvelope Request<T>(T data, string command = CommandName, Guid? correlation = null) =>
        ProtocolEnvelope.Create("platform_command", new { Command = command, Data = data }, correlation ?? Guid.NewGuid());

    private static void Check(bool value, string detail)
    {
        if (!value) throw new InvalidOperationException(detail);
    }

    // Exercise the real serial reader, with an explicit test peer in place of the session handshake.
    private sealed class PickerPipe : IAsyncDisposable
    {
        private readonly CancellationTokenSource _stop = new();
        private readonly PlatformCommandSequenceDispatcher _dispatcher = new();
        private readonly Channel<ProtocolEnvelope> _outgoing = Channel.CreateBounded<ProtocolEnvelope>(8);
        private readonly NamedPipeServerStream _server;
        private readonly NamedPipeClientStream _client;
        private readonly PlatformPipeClient _pipeClient;
        private StreamWriter? _writer;
        private Task? _reader;

        internal PickerPipe(Func<OpenFileDialog, IWin32Window, string[]>? present = null, IPlatformLog? log = null,
            Func<SaveFileDialog, IWin32Window, string?>? presentExport = null,
            Func<Stream, string, CancellationToken, Task<bool>>? renderPdf = null)
        {
            var name = "NxTools.MarkdownPicker.Fixture." + Guid.NewGuid();
            _server = new NamedPipeServerStream(name, PipeDirection.In, 1, PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
            _client = new NamedPipeClientStream(".", name, PipeDirection.Out, PipeOptions.Asynchronous);
            _pipeClient = new PlatformPipeClient(name, null!, log ?? NullPlatformLog.Instance, present, presentExport, renderPdf);
        }

        internal async Task Connect()
        {
            var connecting = _server.WaitForConnectionAsync(_stop.Token);
            await _client.ConnectAsync((int)FixtureWait.TotalMilliseconds, _stop.Token);
            await connecting;
            _writer = new StreamWriter(_client, new UTF8Encoding(false), 4096, leaveOpen: true) { AutoFlush = true };
            var read = typeof(PlatformPipeClient).GetMethod("ReadLoopAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
            _reader = (Task)read.Invoke(_pipeClient, [new BoundedFrameReader(_server), _outgoing.Writer, _stop.Token,
                _dispatcher, (uint)Environment.ProcessId, null!, null!, null!, null!, null!, null!])!;
        }

        internal Task Send(ProtocolEnvelope request) =>
            _writer!.WriteLineAsync(JsonSerializer.Serialize(request, JsonDefaults.Options));

        internal async Task<ProtocolEnvelope> Receive(Guid requestId)
        {
            var reply = await _outgoing.Reader.ReadAsync().AsTask().WaitAsync(FixtureWait);
            Check(reply.CorrelationId == requestId, "reply targets the original request ID");
            var frame = JsonSerializer.SerializeToUtf8Bytes(reply, JsonDefaults.Options);
            Check(frame.Length < ProtocolConstants.MaxFrameBytes, "serialized reply fits the pipe frame bound");
            var decoded = JsonSerializer.Deserialize<ProtocolEnvelope>(frame, JsonDefaults.Options)!;
            decoded.Validate();
            return decoded;
        }

        internal bool HasReply => _outgoing.Reader.TryPeek(out _);

        internal async Task Retire()
        {
            _stop.Cancel();
            if (_reader is not null)
            {
                try { await _reader.WaitAsync(FixtureWait); }
                catch (OperationCanceledException) when (_stop.IsCancellationRequested) { }
            }
            await _dispatcher.DrainAsync(FixtureWait);
        }

        public async ValueTask DisposeAsync()
        {
            try { await Retire(); }
            finally
            {
                _writer?.Dispose();
                _client.Dispose();
                _server.Dispose();
                _stop.Dispose();
            }
        }
    }
}
