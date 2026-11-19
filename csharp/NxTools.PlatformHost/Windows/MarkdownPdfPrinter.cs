using System.ComponentModel;
using System.Diagnostics;
using System.Windows.Interop;
using System.Windows.Threading;
using Microsoft.Web.WebView2.Core;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class MarkdownPdfPrinter
{
    private static readonly TimeSpan Budget = TimeSpan.FromSeconds(30);
    private const long MaxHtmlBytes = 64 * 1024 * 1024;
    private const long MaxPdfBytes = 128 * 1024 * 1024;
    private const string CandidateName = "render.pdf";
    private const string DocumentUrl = "https://markdown-export.invalid/document.html";
    private const string DocumentHeaders = "Content-Type: text/html; charset=utf-8\r\n" +
        "Content-Security-Policy: default-src 'none'; img-src data:; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'\r\n" +
        "X-DNS-Prefetch-Control: off";

    internal static WindowsCommandResult Print(PrintMarkdownPdfCommand command, Guid correlationId,
        IPlatformLog log, CancellationToken sessionToken,
        Func<Stream, string, CancellationToken, Task<bool>>? render = null)
    {
        var started = Stopwatch.GetTimestamp();
        void Diagnostic(string stage, string? category = null, int? nativeCode = null) =>
            WriteDiagnostic(log, correlationId, stage, category, nativeCode, Stopwatch.GetElapsedTime(started).TotalMilliseconds);

        void Clean(string directory)
        {
            try { Directory.Delete(directory, recursive: true); }
            catch (DirectoryNotFoundException) { }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException)
            {
                Diagnostic("cleanup_failed", "native_temp", error.HResult);
            }
        }

        WindowsCommandResult Execute()
        {
            if (LocalPath(command.HtmlPath) is not { } html || LocalPath(command.OutputPath) is not { } output ||
                string.Equals(html, output, StringComparison.OrdinalIgnoreCase))
                return new(false, "invalid_request", NativeErrorCode: 87);
            if (sessionToken.IsCancellationRequested)
                return new(false, "render_cancelled", NativeErrorCode: 995);

            using var deadline = new CancellationTokenSource(Budget);
            using var cancellation = CancellationTokenSource.CreateLinkedTokenSource(sessionToken, deadline.Token);
            using var staged = new StagedCandidate(Clean);
            var ran = BoundedStaOperationRunner.TryRun("NxTools Markdown PDF", Timeout.InfiniteTimeSpan,
                token => Prepare(html, output, staged, token), out WindowsCommandResult? prepared, out _, cancellation.Token);
            if (sessionToken.IsCancellationRequested)
                return new(false, "render_cancelled", NativeErrorCode: 995);
            if (deadline.IsCancellationRequested)
                return new(false, "render_timeout", NativeErrorCode: 1460);
            if (!ran || prepared is null)
                return new(false, "render_busy", NativeErrorCode: 170);
            if (!prepared.Success)
                return prepared;

            // Only this still-current synchronous result owner can write the Desktop staging name.
            // The STA can never create it, even if a native callback survives a timeout.
            try
            {
                File.Move(Path.Combine(staged.Directory!, CandidateName), output, overwrite: false);
                return new(true);
            }
            catch (IOException error) when ((error.HResult & 0xffff) is 80 or 183)
            {
                return new(false, "output_exists", NativeErrorCode: error.HResult & 0xffff);
            }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException)
            {
                return new(false, "output_failed", NativeErrorCode: error.HResult);
            }
        }

        WindowsCommandResult Prepare(string html, string output, StagedCandidate staged, CancellationToken token)
        {
            if (File.Exists(output) || Directory.Exists(output))
                return new(false, "output_exists", NativeErrorCode: 80);
            var parent = Path.GetDirectoryName(output)!;
            if (!Directory.Exists(parent))
                return new(false, "output_failed", NativeErrorCode: 3);

            FileStream source;
            try { source = new FileStream(html, FileMode.Open, FileAccess.Read, FileShare.Read); }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException)
            {
                return new(false, "input_unavailable", NativeErrorCode: error.HResult);
            }
            using (source)
            {
                if (source.Length == 0)
                    return new(false, "invalid_request", NativeErrorCode: 87);
                if (source.Length > MaxHtmlBytes)
                    return new(false, "input_limit", NativeErrorCode: 223);
                token.ThrowIfCancellationRequested();
                var work = Path.Combine(parent, ".nxtools-pdf-" + Guid.NewGuid().ToString("N"));
                try { Directory.CreateDirectory(work); }
                catch (Exception error) when (error is IOException or UnauthorizedAccessException)
                {
                    return new(false, "output_failed", NativeErrorCode: error.HResult);
                }

                var transferred = false;
                try
                {
                    Diagnostic("start");
                    var pdf = Path.Combine(work, CandidateName);
                    var printed = Pump(() => render is null
                        ? RenderOffline(source, pdf, token, () => Diagnostic("document_ready"))
                        : render(source, pdf, token));
                    token.ThrowIfCancellationRequested();
                    if (!printed)
                        return new(false, "render_failed");
                    using (var candidate = new FileStream(pdf, FileMode.Open, FileAccess.Read, FileShare.Read))
                    {
                        if (candidate.Length > MaxPdfBytes)
                            return new(false, "output_limit", NativeErrorCode: 223);
                        Span<byte> signature = stackalloc byte[5];
                        if (candidate.Read(signature) != signature.Length || !signature.SequenceEqual("%PDF-"u8))
                            return new(false, "render_failed");
                    }
                    transferred = staged.TryTransfer(work);
                    return transferred ? new(true) : new(false, "render_cancelled", NativeErrorCode: 995);
                }
                catch (WebView2RuntimeNotFoundException error)
                {
                    return new(false, "runtime_missing", NativeErrorCode: error.HResult);
                }
                catch (OperationCanceledException) when (token.IsCancellationRequested) { throw; }
                catch (Exception error)
                {
                    return new(false, "render_failed", NativeErrorCode: error is Win32Exception native ? native.NativeErrorCode : error.HResult);
                }
                finally
                {
                    if (!transferred)
                        Clean(work);
                }
            }
        }

        Diagnostic("accepted");
        var result = Execute();
        Diagnostic("terminal", result.Success ? "staged" : result.Error, result.NativeErrorCode);
        return result;
    }

    internal static void WriteDiagnostic(IPlatformLog log, Guid id, string stage,
        string? category, int? nativeCode, double elapsedMs)
    {
        try
        {
            log.Write("info", "markdown.reader.pdf." + stage, id, new Dictionary<string, object?>
            {
                ["category"] = category, ["native_error_code"] = nativeCode, ["elapsed_ms"] = elapsedMs
            });
        }
        catch { /* Diagnostics cannot determine the export result. */ }
    }

    private static string? LocalPath(string? path)
    {
        if (path is null || path.Length >= 32767 || path.Contains('\0'))
            return null;
        if (path.StartsWith(@"\\?\", StringComparison.Ordinal))
            path = path[4..];
        // Desktop stages on a local drive, even if the user ultimately saves to a UNC path.
        if (path.Length < 4 || !char.IsAsciiLetter(path[0]) || path[1] != ':' || path[2] is not ('\\' or '/') ||
            path.AsSpan(2).Contains(':'))
            return null;
        try
        {
            var normalized = Path.GetFullPath(path);
            return Path.EndsInDirectorySeparator(normalized) ? null : normalized;
        }
        catch (Exception error) when (error is ArgumentException or IOException or NotSupportedException) { return null; }
    }

    private static T Pump<T>(Func<Task<T>> operation)
    {
        var dispatcher = Dispatcher.CurrentDispatcher;
        var previous = SynchronizationContext.Current;
        SynchronizationContext.SetSynchronizationContext(new DispatcherSynchronizationContext(dispatcher));
        try
        {
            var task = operation();
            if (!task.IsCompleted)
            {
                var frame = new DispatcherFrame();
                task.GetAwaiter().OnCompleted(() => frame.Continue = false);
                Dispatcher.PushFrame(frame);
            }
            return task.GetAwaiter().GetResult();
        }
        finally { SynchronizationContext.SetSynchronizationContext(previous); }
    }

    private static async Task<bool> RenderOffline(Stream html, string pdf, CancellationToken token, Action documentReady)
    {
        var profile = Path.Combine(Path.GetDirectoryName(pdf)!, "profile");
        var options = new CoreWebView2EnvironmentOptions
        {
            ExclusiveUserDataFolderAccess = true,
            ReleaseChannels = CoreWebView2ReleaseChannels.Stable,
            AdditionalBrowserArguments = "--disable-background-networking --no-pings --host-resolver-rules=\"MAP * ~NOTFOUND\""
        };
        // Runtime discovery is performed by the SDK. No installer or browser fallback is launched.
        var environment = await CoreWebView2Environment.CreateAsync(null, profile, options);
        token.ThrowIfCancellationRequested();
        var exited = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        void OnBrowserExited(object? sender, CoreWebView2BrowserProcessExitedEventArgs args) => exited.TrySetResult();
        environment.BrowserProcessExited += OnBrowserExited;
        using var parent = new HwndSource(new HwndSourceParameters("NxTools PDF renderer", 1024, 768)
        {
            WindowStyle = unchecked((int)0x80000000) // WS_POPUP, without WS_VISIBLE
        });
        CoreWebView2Controller? controller = null;
        try
        {
            controller = await environment.CreateCoreWebView2ControllerAsync(parent.Handle);
            token.ThrowIfCancellationRequested();
            controller.Bounds = new System.Drawing.Rectangle(0, 0, 1024, 768);
            var web = controller.CoreWebView2;
            web.Settings.IsScriptEnabled = false;
            web.Settings.IsWebMessageEnabled = false;
            web.Settings.AreHostObjectsAllowed = false;
            web.Settings.AreDevToolsEnabled = false;
            web.Settings.IsBuiltInErrorPageEnabled = false;
            var ready = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
            ulong? navigation = null;
            web.NavigationStarting += (_, args) =>
            {
                if (navigation is null && args.Uri == DocumentUrl)
                    navigation = args.NavigationId;
                else
                    args.Cancel = true;
            };
            web.FrameNavigationStarting += (_, args) => args.Cancel = true;
            web.NewWindowRequested += (_, args) => args.Handled = true;
            web.LaunchingExternalUriScheme += (_, args) => args.Cancel = true;
            web.DownloadStarting += (_, args) => { args.Cancel = true; args.Handled = true; };
            web.NavigationCompleted += (_, args) =>
            {
                if (navigation == args.NavigationId)
                    ready.TrySetResult(args.IsSuccess);
            };
            web.ProcessFailed += (_, _) => ready.TrySetResult(false);
            web.AddWebResourceRequestedFilter("*", CoreWebView2WebResourceContext.All, CoreWebView2WebResourceRequestSourceKinds.All);
            var documentServed = false;
            web.WebResourceRequested += (_, args) =>
            {
                if (!documentServed && args.ResourceContext == CoreWebView2WebResourceContext.Document &&
                    args.Request.Method == "GET" && args.Request.Uri == DocumentUrl)
                {
                    documentServed = true;
                    args.Response = environment.CreateWebResourceResponse(html, 200, "OK", DocumentHeaders);
                }
                else if (args.ResourceContext != CoreWebView2WebResourceContext.Image ||
                         !args.Request.Uri.StartsWith("data:image/", StringComparison.OrdinalIgnoreCase))
                {
                    args.Response = environment.CreateWebResourceResponse(null, 403, "Forbidden", string.Empty);
                }
            };
            web.Navigate(DocumentUrl);
            if (!await ready.Task.WaitAsync(token) || !documentServed)
                return false;
            documentReady();
            var settings = environment.CreatePrintSettings();
            settings.ShouldPrintBackgrounds = true;
            settings.ShouldPrintHeaderAndFooter = false;
            return await web.PrintToPdfAsync(pdf, settings).WaitAsync(token);
        }
        finally
        {
            if (controller is not null)
            {
                controller.Close();
                // The runtime can retain profile handles briefly after its last controller closes.
                await Task.WhenAny(exited.Task, Task.Delay(TimeSpan.FromSeconds(2)));
            }
            environment.BrowserProcessExited -= OnBrowserExited;
        }
    }

    // The worker owns cleanup until transfer. A timeout/disconnect may retire the result
    // owner before the worker exits; in that case transfer is refused and the worker cleans up.
    private sealed class StagedCandidate(Action<string> cleanup) : IDisposable
    {
        private readonly object _gate = new();
        private bool _retired;
        internal string? Directory { get; private set; }

        internal bool TryTransfer(string directory)
        {
            lock (_gate)
            {
                if (_retired) return false;
                Directory = directory;
                return true;
            }
        }

        public void Dispose()
        {
            string? directory;
            lock (_gate)
            {
                _retired = true;
                directory = Directory;
                Directory = null;
            }
            if (directory is not null) cleanup(directory);
        }
    }
}
