using System.Diagnostics;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal interface IElevatedDirectoryNavigationChannel : IDisposable
{
    DirectoryNavigationContext? TryCapture(ForegroundContext foreground);

    WindowsCommandResult Navigate(NavigateDirectoryCommand command);
}

internal sealed class ElevatedDirectoryNavigationClient :
    IElevatedDirectoryNavigationChannel
{
    internal const string ContextPrefix = "elevated:";
    private static readonly TimeSpan CaptureTimeout =
        TimeSpan.FromMilliseconds(180);
    private static readonly TimeSpan NavigationTimeout =
        TimeSpan.FromMilliseconds(2_500);
    private static readonly TimeSpan RendezvousTimeout =
        TimeSpan.FromSeconds(5);
    private const int MaximumRendezvousAttempts = 4;
    private readonly object _connectionGate = new();
    private readonly object _requestGate = new();
    private readonly Action<ElevatedDirectoryHelperRequestedEvent> _requestHelper;
    private readonly IPlatformLog _log;
    private readonly CancellationTokenSource _shutdown = new();
    private HelperConnection? _connection;
    private bool _rendezvousInProgress;
    private int _rendezvousFailures;
    private bool _disposed;

    public ElevatedDirectoryNavigationClient(
        Action<ElevatedDirectoryHelperRequestedEvent> requestHelper,
        IPlatformLog log)
    {
        _requestHelper = requestHelper;
        _log = log;
        TryBeginRendezvous();
    }

    public DirectoryNavigationContext? TryCapture(ForegroundContext foreground)
    {
        var response = SendRequest(
            new ElevatedDirectoryHelperRequest(
                Guid.NewGuid(),
                "capture",
                foreground,
                null),
            CaptureTimeout);
        return response?.Success == true
            ? response.Context
            : null;
    }

    public WindowsCommandResult Navigate(NavigateDirectoryCommand command)
    {
        if (!IsElevatedContext(command.HostContextId))
        {
            return new WindowsCommandResult(
                false,
                "The elevated directory context is invalid.");
        }
        var response = SendRequest(
            new ElevatedDirectoryHelperRequest(
                Guid.NewGuid(),
                "navigate",
                null,
                command),
            NavigationTimeout);
        return response is null
            ? new WindowsCommandResult(
                false,
                "The same-integrity directory helper is unavailable.")
            : new WindowsCommandResult(response.Success, response.Error);
    }

    internal static bool IsElevatedContext(string? hostContextId) =>
        hostContextId?.StartsWith(
            ContextPrefix,
            StringComparison.Ordinal) == true;

    private ElevatedDirectoryHelperResponse? SendRequest(
        ElevatedDirectoryHelperRequest request,
        TimeSpan timeout)
    {
        lock (_requestGate)
        {
            HelperConnection? connection;
            lock (_connectionGate)
            {
                connection = _connection;
            }
            if (connection is null)
            {
                EnsureRendezvous();
                return null;
            }
            try
            {
                connection.Writer.WriteLine(
                    JsonSerializer.Serialize(request, JsonDefaults.Options));
                var responseTask = BoundedTextFrameReader.ReadLineAsync(
                    connection.Reader,
                    64 * 1024,
                    _shutdown.Token);
                if (!responseTask.Wait(timeout) ||
                    string.IsNullOrWhiteSpace(responseTask.Result))
                {
                    InvalidateConnection(
                        connection,
                        "the elevated directory helper response timed out");
                    return null;
                }
                var response = JsonSerializer.Deserialize<ElevatedDirectoryHelperResponse>(
                    responseTask.Result,
                    JsonDefaults.Options);
                if (response is null || response.Id != request.Id)
                {
                    InvalidateConnection(
                        connection,
                        "the elevated directory helper returned an invalid response");
                    return null;
                }
                if (response.Fatal)
                {
                    InvalidateConnection(
                        connection,
                        response.Error ??
                        "the elevated directory helper requested a restart");
                }
                return response;
            }
            catch (Exception error) when (
                error is IOException or
                InvalidOperationException or
                JsonException or
                AggregateException)
            {
                InvalidateConnection(connection, error.Message);
                return null;
            }
        }
    }

    private void TryBeginRendezvous()
    {
        lock (_connectionGate)
        {
            if (_disposed ||
                _connection is not null ||
                _rendezvousInProgress ||
                _rendezvousFailures >= MaximumRendezvousAttempts)
            {
                return;
            }
            _rendezvousInProgress = true;
        }
        try
        {
            BeginRendezvous();
        }
        catch (Exception error) when (
            error is IOException or
            UnauthorizedAccessException or
            InvalidOperationException)
        {
            HandleRendezvousFailure(error);
        }
    }

    private void EnsureRendezvous()
    {
        lock (_connectionGate)
        {
            if (!_disposed &&
                _connection is null &&
                !_rendezvousInProgress &&
                _rendezvousFailures >= MaximumRendezvousAttempts)
            {
                _rendezvousFailures = 0;
            }
        }
        TryBeginRendezvous();
    }

    private void BeginRendezvous()
    {
        if (_disposed)
        {
            return;
        }
        var pipeName = $"nxtools-next-elevated-nav-{Guid.NewGuid():N}";
        var authenticationToken = Convert.ToHexString(
            RandomNumberGenerator.GetBytes(32)).ToLowerInvariant();
        var server = new NamedPipeServerStream(
            pipeName,
            PipeDirection.InOut,
            1,
            PipeTransmissionMode.Byte,
            PipeOptions.Asynchronous | PipeOptions.CurrentUserOnly);
        _requestHelper(new ElevatedDirectoryHelperRequestedEvent(
            pipeName,
            authenticationToken));
        _ = Task.Run(
            () => AcceptHelperAsync(
                server,
                authenticationToken,
                _shutdown.Token));
    }

    private async Task AcceptHelperAsync(
        NamedPipeServerStream server,
        string authenticationToken,
        CancellationToken cancellationToken)
    {
        try
        {
            using var acceptTimeout =
                CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            acceptTimeout.CancelAfter(RendezvousTimeout);
            await server.WaitForConnectionAsync(acceptTimeout.Token);
            var reader = new StreamReader(
                server,
                new UTF8Encoding(false),
                detectEncodingFromByteOrderMarks: false,
                leaveOpen: true);
            var writer = new StreamWriter(
                server,
                new UTF8Encoding(false),
                leaveOpen: true)
            {
                AutoFlush = true,
                NewLine = "\n"
            };
            using var helloTimeout =
                CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            helloTimeout.CancelAfter(TimeSpan.FromSeconds(3));
            var line = await BoundedTextFrameReader.ReadLineAsync(
                reader,
                4_096,
                helloTimeout.Token);
            var hello = string.IsNullOrWhiteSpace(line)
                ? null
                : JsonSerializer.Deserialize<ElevatedDirectoryHelperHello>(
                    line,
                    JsonDefaults.Options);
            if (hello is null ||
                string.IsNullOrWhiteSpace(hello.AuthenticationToken) ||
                !CryptographicOperations.FixedTimeEquals(
                    Encoding.ASCII.GetBytes(hello.AuthenticationToken),
                    Encoding.ASCII.GetBytes(authenticationToken)) ||
                !hello.Elevated ||
                !ElevatedDirectoryHelperPeerVerifier.IsPackagedHelper(
                    server,
                    hello.ProcessId))
            {
                throw new InvalidDataException(
                    "the elevated directory helper handshake was rejected");
            }
            var connection = new HelperConnection(server, reader, writer);
            lock (_connectionGate)
            {
                if (_disposed)
                {
                    connection.Dispose();
                    return;
                }
                _connection?.Dispose();
                _connection = connection;
                _rendezvousInProgress = false;
                _rendezvousFailures = 0;
            }
            _log.Write("info", "elevated_directory_helper.connected", fields:
                new Dictionary<string, object?>
                {
                    ["process_id"] = hello.ProcessId
                });
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            server.Dispose();
            lock (_connectionGate)
            {
                _rendezvousInProgress = false;
            }
        }
        catch (Exception error) when (
            error is IOException or
            InvalidDataException or
            JsonException or
            OperationCanceledException)
        {
            server.Dispose();
            HandleRendezvousFailure(error);
        }
    }

    private void HandleRendezvousFailure(Exception error)
    {
        int failureCount;
        var shouldRetry = false;
        lock (_connectionGate)
        {
            _rendezvousInProgress = false;
            if (_disposed)
            {
                return;
            }
            failureCount = ++_rendezvousFailures;
            shouldRetry = failureCount < MaximumRendezvousAttempts;
        }
        _log.Write("warning", "elevated_directory_helper.connection_failed", fields:
            new Dictionary<string, object?>
            {
                ["message"] = error.Message,
                ["attempt"] = failureCount
            });
        if (shouldRetry)
        {
            _ = Task.Run(async () =>
            {
                try
                {
                    await Task.Delay(
                        TimeSpan.FromMilliseconds(100 * failureCount),
                        _shutdown.Token);
                    TryBeginRendezvous();
                }
                catch (OperationCanceledException)
                {
                }
            });
        }
    }

    private void InvalidateConnection(
        HelperConnection connection,
        string reason)
    {
        var restart = false;
        lock (_connectionGate)
        {
            if (ReferenceEquals(_connection, connection))
            {
                _connection = null;
                _rendezvousFailures = 0;
                restart = !_disposed;
            }
        }
        connection.Dispose();
        _log.Write("warning", "elevated_directory_helper.disconnected", fields:
            new Dictionary<string, object?>
            {
                ["reason"] = reason
            });
        if (restart)
        {
            TryBeginRendezvous();
        }
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }
        _disposed = true;
        _shutdown.Cancel();
        lock (_connectionGate)
        {
            _connection?.Dispose();
            _connection = null;
        }
        _shutdown.Dispose();
    }

    private sealed class HelperConnection(
        NamedPipeServerStream pipe,
        StreamReader reader,
        StreamWriter writer) : IDisposable
    {
        public StreamReader Reader { get; } = reader;
        public StreamWriter Writer { get; } = writer;

        public void Dispose()
        {
            Writer.Dispose();
            Reader.Dispose();
            pipe.Dispose();
        }
    }
}

internal sealed class ElevatedDirectoryNavigationSessionRegistry
{
    private const int MaximumSessions = 64;
    private static readonly TimeSpan SessionLifetime =
        TimeSpan.FromMinutes(10);
    private readonly object _gate = new();
    private readonly Dictionary<string, CapturedSession> _sessions =
        new(StringComparer.Ordinal);
    private readonly LinkedList<string> _sessionOrder = new();
    private readonly Dictionary<string, LinkedListNode<string>> _sessionOrderNodes =
        new(StringComparer.Ordinal);
    private readonly Func<ForegroundContext, DirectoryNavigationCaptureAttempt>
        _capture;
    private readonly Func<NavigateDirectoryCommand, WindowsCommandResult> _navigate;

    public ElevatedDirectoryNavigationSessionRegistry()
        : this(
            DirectoryNavigationAdapter.TryCaptureLocalAttempt,
            DirectoryNavigationAdapter.NavigateLocal)
    {
    }

    internal ElevatedDirectoryNavigationSessionRegistry(
        Func<ForegroundContext, DirectoryNavigationCaptureAttempt> capture,
        Func<NavigateDirectoryCommand, WindowsCommandResult> navigate)
    {
        _capture = capture;
        _navigate = navigate;
    }

    public DirectoryNavigationCaptureAttempt TryCapture(
        ForegroundContext foreground)
    {
        var capture = _capture(foreground);
        var context = capture.Context;
        if (context is null)
        {
            return capture;
        }
        var publicContextId =
            $"{ElevatedDirectoryNavigationClient.ContextPrefix}{Guid.NewGuid():N}";
        lock (_gate)
        {
            Cleanup(DateTimeOffset.UtcNow);
            _sessions[publicContextId] = new CapturedSession(
                context,
                DateTimeOffset.UtcNow);
            _sessionOrderNodes[publicContextId] =
                _sessionOrder.AddLast(publicContextId);
            TrimToCapacity();
        }
        return new DirectoryNavigationCaptureAttempt(
            context with { HostContextId = publicContextId });
    }

    public WindowsCommandResult Navigate(NavigateDirectoryCommand command)
    {
        CapturedSession? session;
        lock (_gate)
        {
            Cleanup(DateTimeOffset.UtcNow);
            if (string.IsNullOrWhiteSpace(command.HostContextId) ||
                !_sessions.TryGetValue(command.HostContextId, out session) ||
                session.Context.Host != command.Host ||
                session.Context.WindowHandle != command.WindowHandle)
            {
                return new WindowsCommandResult(
                    false,
                    "The captured elevated directory host is no longer available.");
            }
        }
        var result = _navigate(command with
        {
            HostContextId = session.Context.HostContextId
        });
        if (result.Success)
        {
            lock (_gate)
            {
                RemoveSession(command.HostContextId!);
            }
        }
        return result;
    }

    internal int SessionCount
    {
        get
        {
            lock (_gate)
            {
                return _sessions.Count;
            }
        }
    }

    internal int SessionOrderCount
    {
        get
        {
            lock (_gate)
            {
                return _sessionOrder.Count;
            }
        }
    }

    private void Cleanup(DateTimeOffset now)
    {
        foreach (var (contextId, session) in _sessions.ToArray())
        {
            if (now - session.CapturedAt > SessionLifetime)
            {
                RemoveSession(contextId);
            }
        }
        TrimToCapacity();
    }

    private void TrimToCapacity()
    {
        while (_sessions.Count > MaximumSessions &&
               _sessionOrder.First is { } oldest)
        {
            RemoveSession(oldest.Value);
        }
    }

    private void RemoveSession(string contextId)
    {
        _sessions.Remove(contextId);
        if (_sessionOrderNodes.Remove(contextId, out var node))
        {
            _sessionOrder.Remove(node);
        }
    }

    private sealed record CapturedSession(
        DirectoryNavigationContext Context,
        DateTimeOffset CapturedAt);
}

internal static class BoundedTextFrameReader
{
    public static async Task<string?> ReadLineAsync(
        TextReader reader,
        int maximumCharacters,
        CancellationToken cancellationToken)
    {
        var frame = new StringBuilder(Math.Min(maximumCharacters, 512));
        var character = new char[1];
        while (frame.Length <= maximumCharacters)
        {
            var read = await reader.ReadAsync(character.AsMemory(), cancellationToken);
            if (read == 0)
            {
                return frame.Length == 0 ? null : frame.ToString();
            }
            if (character[0] == '\n')
            {
                return TrimTrailingCarriageReturn(frame);
            }
            frame.Append(character[0]);
        }
        throw new InvalidDataException(
            $"The helper frame exceeded {maximumCharacters} characters.");
    }

    public static string? ReadLine(TextReader reader, int maximumCharacters)
    {
        var frame = new StringBuilder(Math.Min(maximumCharacters, 512));
        while (frame.Length <= maximumCharacters)
        {
            var character = reader.Read();
            if (character < 0)
            {
                return frame.Length == 0 ? null : frame.ToString();
            }
            if (character == '\n')
            {
                return TrimTrailingCarriageReturn(frame);
            }
            frame.Append((char)character);
        }
        throw new InvalidDataException(
            $"The helper frame exceeded {maximumCharacters} characters.");
    }

    private static string TrimTrailingCarriageReturn(StringBuilder frame)
    {
        if (frame.Length > 0 && frame[^1] == '\r')
        {
            frame.Length--;
        }
        return frame.ToString();
    }
}

internal static class ElevatedDirectoryNavigationHelper
{
    public static int Run(string pipeName, string authenticationToken)
    {
        if (!ElevatedHelperProcessBroker.IsValidRendezvous(
                pipeName,
                authenticationToken) ||
            !ElevatedDirectoryHelperPeerVerifier.IsCurrentProcessElevated())
        {
            return 2;
        }
        ThirdPartyFileManagerAdapter.WarmUp();
        try
        {
            using var pipe = new NamedPipeClientStream(
                ".",
                pipeName,
                PipeDirection.InOut,
                PipeOptions.None);
            pipe.Connect(5_000);
            using var reader = new StreamReader(
                pipe,
                new UTF8Encoding(false),
                detectEncodingFromByteOrderMarks: false,
                leaveOpen: true);
            using var writer = new StreamWriter(
                pipe,
                new UTF8Encoding(false),
                leaveOpen: true)
            {
                AutoFlush = true,
                NewLine = "\n"
            };
            writer.WriteLine(JsonSerializer.Serialize(
                new ElevatedDirectoryHelperHello(
                    authenticationToken,
                    Environment.ProcessId,
                    true),
                JsonDefaults.Options));
            var sessions = new ElevatedDirectoryNavigationSessionRegistry();
            var foregroundProvider = new ForegroundContextProvider();
            while (pipe.IsConnected)
            {
                var line = BoundedTextFrameReader.ReadLine(
                    reader,
                    64 * 1024);
                if (line is null)
                {
                    return 0;
                }
                var request = JsonSerializer.Deserialize<ElevatedDirectoryHelperRequest>(
                    line,
                    JsonDefaults.Options);
                if (request is null)
                {
                    return 3;
                }
                ElevatedDirectoryHelperResponse response;
                var terminateAfterResponse = false;
                if (request.Operation == "capture" &&
                    request.Foreground is not null)
                {
                    var recaptured = foregroundProvider.Capture(
                        new IntPtr(request.Foreground.WindowHandle));
                    var sameTarget =
                        recaptured.WindowHandle == request.Foreground.WindowHandle &&
                        recaptured.ProcessId == request.Foreground.ProcessId;
                    var captureTarget = sameTarget
                        ? PreserveRequestedFocus(
                            request.Foreground,
                            recaptured,
                            FocusBelongsToRequestedTarget)
                        : null;
                    var capture = captureTarget is not null
                        ? sessions.TryCapture(captureTarget)
                        : new DirectoryNavigationCaptureAttempt(
                            null,
                            new WindowsCommandResult(
                                false,
                                "The foreground host or its summon-time focus changed before the same-integrity capture."));
                    terminateAfterResponse =
                        capture.Failure?.Fatal == true;
                    response = new ElevatedDirectoryHelperResponse(
                        request.Id,
                        capture.Context is not null,
                        capture.Context,
                        capture.Failure?.Error,
                        capture.Failure?.Fatal == true);
                }
                else if (request.Operation == "navigate" &&
                         request.Command is not null)
                {
                    var result = sessions.Navigate(request.Command);
                    terminateAfterResponse = result.Fatal;
                    response = new ElevatedDirectoryHelperResponse(
                        request.Id,
                        result.Success,
                        null,
                        result.Error,
                        result.Fatal);
                }
                else
                {
                    response = new ElevatedDirectoryHelperResponse(
                        request.Id,
                        false,
                        null,
                        "The elevated directory helper request is invalid.");
                }
                writer.WriteLine(JsonSerializer.Serialize(
                    response,
                    JsonDefaults.Options));
                if (terminateAfterResponse)
                {
                    return 4;
                }
            }
            return 0;
        }
        catch (Exception error) when (
            error is IOException or
            TimeoutException or
            JsonException or
            InvalidOperationException or
            UnauthorizedAccessException)
        {
            Console.Error.WriteLine(error.Message);
            return 1;
        }
    }

    internal static ForegroundContext? PreserveRequestedFocus(
        ForegroundContext requested,
        ForegroundContext recaptured,
        Func<ForegroundContext, bool> focusIsValid)
    {
        if (!focusIsValid(requested))
        {
            return null;
        }
        return recaptured with
        {
            FocusWindowHandle = requested.FocusWindowHandle
        };
    }

    private static bool FocusBelongsToRequestedTarget(
        ForegroundContext requested)
    {
        var focus = new IntPtr(requested.FocusWindowHandle);
        var target = new IntPtr(requested.WindowHandle);
        if (focus == IntPtr.Zero ||
            target == IntPtr.Zero ||
            !IsWindow(focus) ||
            GetWindowThreadProcessId(focus, out var processId) == 0 ||
            processId != requested.ProcessId)
        {
            return false;
        }
        var root = GetAncestor(focus, GetAncestorRoot);
        return focus == target || root == target;
    }

    private const uint GetAncestorRoot = 2;

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);

    [DllImport("user32.dll")]
    private static extern IntPtr GetAncestor(
        IntPtr window,
        uint flags);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(
        IntPtr window,
        out uint processId);
}

internal static class ElevatedDirectoryHelperPeerVerifier
{
    private const uint ProcessQueryLimitedInformation = 0x1000;
    private const uint TokenQuery = 0x0008;
    private const int TokenElevation = 20;

    public static bool IsPackagedHelper(
        NamedPipeServerStream pipe,
        int claimedProcessId)
    {
        if (!GetNamedPipeClientProcessId(
                pipe.SafePipeHandle.DangerousGetHandle(),
                out var actualProcessId) ||
            actualProcessId != (uint)claimedProcessId)
        {
            return false;
        }
        var actualPath = TryGetProcessPath(actualProcessId);
        return actualPath is not null &&
            IsSameSession(actualProcessId, (uint)Environment.ProcessId) &&
            IndexerServiceInstaller.IsApprovedProtectedPlatformHost(actualPath) &&
            IsProcessElevated(actualProcessId);
    }

    public static bool IsCurrentProcessElevated() =>
        IsProcessElevated((uint)Environment.ProcessId);

    internal static bool IsSameSession(
        uint processId,
        uint currentProcessId) =>
        ProcessIdToSessionId(processId, out var processSession) &&
        ProcessIdToSessionId(currentProcessId, out var currentSession) &&
        processSession == currentSession;

    private static string? TryGetProcessPath(uint processId)
    {
        var process = OpenProcess(
            ProcessQueryLimitedInformation,
            false,
            processId);
        if (process == IntPtr.Zero)
        {
            return null;
        }
        try
        {
            var value = new StringBuilder(32_768);
            var length = value.Capacity;
            return QueryFullProcessImageName(
                process,
                0,
                value,
                ref length)
                ? value.ToString()
                : null;
        }
        finally
        {
            CloseHandle(process);
        }
    }

    internal static bool IsProcessElevated(uint processId)
    {
        var process = OpenProcess(
            ProcessQueryLimitedInformation,
            false,
            processId);
        if (process == IntPtr.Zero)
        {
            return false;
        }
        try
        {
            if (!OpenProcessToken(process, TokenQuery, out var token))
            {
                return false;
            }
            try
            {
                return GetTokenInformation(
                        token,
                        TokenElevation,
                        out var elevation,
                        Marshal.SizeOf<TokenElevationData>(),
                        out _) &&
                    elevation.TokenIsElevated != 0;
            }
            finally
            {
                CloseHandle(token);
            }
        }
        finally
        {
            CloseHandle(process);
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct TokenElevationData
    {
        public int TokenIsElevated;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr OpenProcess(
        uint desiredAccess,
        [MarshalAs(UnmanagedType.Bool)] bool inheritHandle,
        uint processId);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool QueryFullProcessImageName(
        IntPtr process,
        uint flags,
        StringBuilder value,
        ref int size);

    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool OpenProcessToken(
        IntPtr process,
        uint desiredAccess,
        out IntPtr token);

    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetTokenInformation(
        IntPtr token,
        int tokenInformationClass,
        out TokenElevationData tokenInformation,
        int tokenInformationLength,
        out int returnLength);

    [DllImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(IntPtr handle);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetNamedPipeClientProcessId(
        IntPtr pipe,
        out uint clientProcessId);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ProcessIdToSessionId(
        uint processId,
        out uint sessionId);
}

internal sealed record ElevatedDirectoryHelperHello(
    string AuthenticationToken,
    int ProcessId,
    bool Elevated);

internal sealed record ElevatedDirectoryHelperRequest(
    Guid Id,
    string Operation,
    ForegroundContext? Foreground,
    NavigateDirectoryCommand? Command);

internal sealed record ElevatedDirectoryHelperResponse(
    Guid Id,
    bool Success,
    DirectoryNavigationContext? Context,
    string? Error,
    bool Fatal = false);
