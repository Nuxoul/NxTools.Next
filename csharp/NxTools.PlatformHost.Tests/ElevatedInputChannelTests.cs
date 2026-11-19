using System.Collections.Concurrent;
using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
using static NxTools.PlatformHost.Tests.ElevatedInputTests;

namespace NxTools.PlatformHost.Tests;

internal static class ElevatedInputChannelTests
{
    public static async Task ConfigurationReplayAndExit()
    {
        var platform = new Platform();
        using var requests = new BlockingCollection<ElevatedDirectoryHelperRequestedEvent>();
        using var client = new ElevatedDirectoryNavigationClient(requests.Add, NullPlatformLog.Instance, platform);
        client.ConfigureInput(Config(7));
        client.ConfigureInput(Config(8));
        using var first = await Peer.Connect(NextRendezvous(requests), 1001);
        var configuration = await first.Next();
        Assert(configuration is { Operation: "configure_input", InputConfiguration.Keyboard.ResetGeneration: 8 },
            "latest configuration cached before connection must replay once authenticated");
        await first.Reply(configuration, true);
        await Until(() => client.OwnsInput(1001, 8));
        Assert(!client.OwnsInput(1002, 8) && !client.OwnsInput(1001, 7), "PID and generation qualify ownership");
        client.InputUnavailable(1001, 7);
        Assert(client.OwnsInput(1001, 8), "old failure cannot clear current ownership");
        client.InputUnavailable(1001, 8);
        Assert(!client.OwnsInput(0, 8), "observed hook exit restores ordinary routing");
        client.ConfigureInput(Config(9, recording: true));
        configuration = await first.Next();
        await first.Reply(configuration, true);
        Assert(!client.OwnsInput(0, 9), "recording has no elevated gesture owner");
        client.ConfigureInput(Config(10));
        configuration = await first.Next();
        await first.Reply(configuration, true);
        await Until(() => client.OwnsInput(1001, 10));
        platform.Exit(1001);
        var reconnect = NextRendezvous(requests);
        Assert(!client.OwnsInput(0, 10), "idle actual process exit clears ownership without a request or heartbeat");
        using var second = await Peer.Connect(reconnect, 1002);
        configuration = await second.Next();
        Assert(configuration.InputConfiguration?.Keyboard.ResetGeneration == 10, "reconnect replays current config");
        await second.Reply(configuration, true);
        await Until(() => client.OwnsInput(1002, 10));
        client.Dispose();
        Assert(!client.OwnsInput(0, 10) && await second.Reader.ReadLineAsync().WaitAsync(TimeSpan.FromSeconds(2)) is null,
            "dispose closes channel and retires ownership");
    }

    public static async Task AutomaticRecoveryIsBounded()
    {
        var platform = new Platform();
        using var requests = new BlockingCollection<ElevatedDirectoryHelperRequestedEvent>();
        var log = new ConfigurationLog();
        using var client = new ElevatedDirectoryNavigationClient(requests.Add, log, platform);
        client.ConfigureInput(Config(1));
        for (var attempt = 0; attempt < 4; attempt++)
        {
            using var peer = await Peer.Connect(NextRendezvous(requests), 4001 + attempt);
            Assert((await peer.Next()).Operation == "configure_input", "configuration replay reaches each connection");
            peer.Dispose();
            await Until(() => Volatile.Read(ref log.Disconnected) >= attempt + 1);
        }
        Assert(!requests.TryTake(out _, 100) && !client.OwnsInput(0, 1),
            "four connected-channel failures exhaust automatic recovery rather than relaunch forever");
    }

    public static async Task FailureBeforeConfigurationAck()
    {
        var platform = new Platform();
        var log = new ConfigurationLog();
        using var requests = new BlockingCollection<ElevatedDirectoryHelperRequestedEvent>();
        using var client = new ElevatedDirectoryNavigationClient(requests.Add, log, platform);
        client.ConfigureInput(Config(1));
        using var peer = await Peer.Connect(NextRendezvous(requests), 3001);
        var request = await peer.Next();
        client.InputUnavailable(3001, 1);
        await peer.Reply(request, true);
        await log.Configured.Task.WaitAsync(TimeSpan.FromSeconds(2));
        Assert(!client.OwnsInput(0, 1), "late configuration ACK cannot resurrect an already failed hook");
    }

    private sealed class ConfigurationLog : IPlatformLog
    {
        public int Disconnected;
        public readonly TaskCompletionSource Configured = new(TaskCreationOptions.RunContinuationsAsynchronously);
        public int ConfiguredCount;
        public void Write(string level, string eventName, Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (eventName == "elevated_directory_helper.disconnected") Interlocked.Increment(ref Disconnected);
            if (eventName == "elevated_input.lifecycle" && fields?.GetValueOrDefault("state") is "configured")
            {
                Interlocked.Increment(ref ConfiguredCount);
                Configured.TrySetResult();
            }
        }
    }

    public static async Task PasteAndDirectoryShareChannel()
    {
        var platform = new Platform();
        using var requests = new BlockingCollection<ElevatedDirectoryHelperRequestedEvent>();
        using var client = new ElevatedDirectoryNavigationClient(requests.Add, NullPlatformLog.Instance, platform);
        client.ConfigureInput(Config(1));
        using var peer = await Peer.Connect(NextRendezvous(requests), 2001);
        var request = await peer.Next();
        await peer.Reply(request, true);
        await Until(() => client.OwnsInput(2001, 1));
        var input = new ElevatedPasteInput(10, 11, Guid.NewGuid());
        var paste = Task.Run(() => client.TryPasteInput(input));
        request = await peer.Next();
        Assert(request.Operation == "paste_input" && request.PasteInput == input &&
            request.Foreground is null && request.Command is null && request.InputConfiguration is null,
            "input-only wire carries no clipboard representation");
        await peer.Reply(request, true);
        Assert((await paste)?.Success == true, "confirmed input result returns through original channel");
        var capture = Task.Run(() => client.TryCapture(new(10, 11, 20, "fixture", "")));
        request = await peer.Next();
        Assert(request.Operation == "capture", "capture still shares serialized channel");
        await peer.Reply(request, false);
        Assert(await capture is null, "original capture failure preserved");
        platform.Elevated = false;
        Assert(client.TryPasteInput(input) is null, "ordinary target never dispatches privileged input");
        platform.Elevated = true;
        paste = Task.Run(() => client.TryPasteInput(input));
        request = await peer.Next();
        Assert(request.Operation == "paste_input", "second request admitted after prior operations");
        var result = await paste.WaitAsync(TimeSpan.FromSeconds(5));
        Assert(result is { Success: false } && result.Error!.Contains("unknown", StringComparison.Ordinal),
            "dispatched timeout must return failure, never unavailable/fallback or automatic replay");
        Assert(!client.OwnsInput(0, 1), "timeout invalidates input ownership");
    }

    public static Task UnavailableKeepsOrdinaryPath()
    {
        var platform = new Platform();
        using var client = new ElevatedDirectoryNavigationClient(_ => { }, NullPlatformLog.Instance, platform);
        client.ConfigureInput(Config(1));
        Assert(client.TryPasteInput(new(10, 11, Guid.NewGuid())) is null && !client.OwnsInput(0, 1),
            "no protected helper available: ordinary routing remains usable");
        return Task.CompletedTask;
    }

    public static async Task ActivationAndPointerUseOriginalConnection()
    {
        var platform = new Platform();
        var log = new ConfigurationLog();
        using var requests = new BlockingCollection<ElevatedDirectoryHelperRequestedEvent>();
        using var client = new ElevatedDirectoryNavigationClient(requests.Add, log, platform);
        client.BindDesktopInputPeer(55);
        client.ConfigureInput(Config(1));
        using var peer = await Peer.Connect(NextRendezvous(requests), 5001);
        var request = await peer.Next();
        Assert(request.InputConfiguration?.DesktopProcessId == 55,
            "configuration binds the actual Desktop pipe peer, not a caller window owner");
        await peer.Reply(request, true);
        await Until(() => Volatile.Read(ref log.ConfiguredCount) >= 1);
        var timestamp = unchecked((uint)Environment.TickCount64);
        Assert(!client.AcceptPointer(5002, timestamp), "another helper cannot forward a click");
        Assert(!client.AcceptPointer(5001, unchecked(timestamp - 1001)), "expired clicks are rejected");
        Assert(client.AcceptPointer(5001, timestamp), "current helper click is admitted");
        Assert(!client.AcceptPointer(5001, timestamp), "duplicate click timestamp is not replayed");
        await Task.Delay(20);
        client.ConfigureInput(Config(2, recording: true));
        Assert(!client.AcceptPointer(5001, unchecked((uint)Environment.TickCount64)),
            "configuration replacement retires pointer ownership before its ACK");
        request = await peer.Next();
        await peer.Reply(request, true);
        await Until(() => Volatile.Read(ref log.ConfiguredCount) >= 2);
        Assert(!client.AcceptPointer(5001, timestamp), "old configuration samples cannot dismiss a new popup");
        Assert(!client.OwnsInput(0, 2) && client.AcceptPointer(5001, unchecked((uint)Environment.TickCount64)),
            "keyboard recording does not disable administrator outside-click dismissal");
        await Task.Delay(100);
        var events = System.Threading.Channels.Channel.CreateBounded<WindowsPlatformEvent>(8);
        var ordinary = new WindowsPlatformAdapter.EventWindow(events.Writer, NullPlatformLog.Instance,
            _ => null, work => work(), work => work(), _ => false, client);
        var target = new System.Windows.Forms.NativeWindow();
        target.CreateHandle(new System.Windows.Forms.CreateParams { Parent = new IntPtr(-3) });
        try
        {
            var sampledTick = unchecked((uint)Environment.TickCount64 - 40);
            ordinary.HandleElevatedPointerPress(target.Handle.ToInt64(), 5001, sampledTick);
            ordinary.HandleElevatedPointerPress(target.Handle.ToInt64(), 5001, sampledTick);
            Assert(events.Reader.TryRead(out var forwarded) && forwarded is GlobalPointerPressedEvent pointer &&
                pointer.WindowHandle == target.Handle.ToInt64() && pointer.RootWindowHandle == target.Handle.ToInt64() &&
                pointer.ProcessId == (uint)Environment.ProcessId && pointer.EventSequence > 0 &&
                pointer.SampledAtUnixMs <= DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - 20 &&
                pointer.SampledAtUnixMs >= DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - 250,
                "ordinary host forwards the original sampled root/time through the real pointer-event owner");
            Assert(!events.Reader.TryRead(out _), "the original ordinary queue receives each helper press once");
        }
        finally { target.DestroyHandle(); }
        var activation = new ElevatedWindowActivation(20, Guid.NewGuid());
        var pending = Task.Run(() => client.TryActivateInput(activation, 10));
        request = await peer.Next();
        Assert(request.Operation == "activate_window" && request.Id == activation.CorrelationId &&
            request.WindowActivation == activation && request.PasteInput is null &&
            request.InputConfiguration is null && request.Foreground is null && request.Command is null,
            "activation uses the original input-only authenticated channel without clipboard data");
        await peer.Reply(request, true);
        Assert((await pending)?.Success == true, "confirmed administrator activation returns once");
        platform.Elevated = false;
        Assert(client.TryActivateInput(activation, 10) is null, "ordinary activation keeps its original native path");
        platform.Elevated = true;
        pending = Task.Run(() => client.TryActivateInput(activation with { CorrelationId = Guid.NewGuid() }, 10));
        request = await peer.Next();
        Assert(request.Operation == "activate_window", "second activation admitted");
        var terminal = await pending.WaitAsync(TimeSpan.FromSeconds(4));
        Assert(terminal is { Success: false } && terminal.Error!.Contains("unknown", StringComparison.Ordinal),
            "a dispatched timeout must fail without replaying activation on the ordinary path");
        Assert(!client.AcceptPointer(5001, unchecked((uint)Environment.TickCount64)),
            "original channel retirement also retires pointer messages");
    }

    public static async Task PointerObservationSurvivesLongIdle()
    {
        long clock = 10_000;
        var log = new ConfigurationLog();
        using var requests = new BlockingCollection<ElevatedDirectoryHelperRequestedEvent>();
        using var client = new ElevatedDirectoryNavigationClient(requests.Add, log, new Platform(), () => clock);
        client.ConfigureInput(Config(1));
        using var peer = await Peer.Connect(NextRendezvous(requests), 6001);
        var request = await peer.Next();
        await peer.Reply(request, true);
        await Until(() => Volatile.Read(ref log.ConfiguredCount) >= 1);
        Assert(client.AcceptPointer(6001, (uint)clock), "initial pointer admitted");
        clock += (long)int.MaxValue + 5;
        Assert(client.AcceptPointer(6001, unchecked((uint)clock)),
            "fresh administrator pointer must remain usable after more than 24.8 days without reconfiguration");
        clock += (long)uint.MaxValue + 200;
        Assert(client.AcceptPointer(6001, unchecked((uint)clock)),
            "fresh administrator pointer remains usable across complete native tick wrap and long idle");
        Assert(!client.AcceptPointer(6001, unchecked((uint)(clock - 2000))), "expired pointer remains rejected");
    }

    private static ElevatedInputConfiguration Config(long generation, bool recording = false) =>
        new(99, new(true, recording, QuickLaunchTriggerKind.DoubleControl, generation), Guid.NewGuid());

    private static ElevatedDirectoryHelperRequestedEvent NextRendezvous(
        BlockingCollection<ElevatedDirectoryHelperRequestedEvent> requests)
    {
        Assert(requests.TryTake(out var request, 3000), "bounded helper rendezvous missing");
        return request!;
    }

    internal static async Task Until(Func<bool> predicate)
    {
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(3));
        while (!predicate()) await Task.Delay(5, timeout.Token);
    }

    private sealed class Platform : IElevatedInputClientPlatform
    {
        private readonly ConcurrentDictionary<int, TaskCompletionSource> _exits = new();
        public bool Elevated = true;
        public bool VerifyHelper(NamedPipeServerStream pipe, int processId) => pipe.IsConnected && processId > 0;
        public bool RequiresElevatedInput(long window) => Elevated;
        public Task WaitForExitAsync(int processId, CancellationToken cancellationToken) =>
            _exits.GetOrAdd(processId, _ => new(TaskCreationOptions.RunContinuationsAsynchronously)).Task.WaitAsync(cancellationToken);
        public void Exit(int processId) =>
            _exits.GetOrAdd(processId, _ => new(TaskCreationOptions.RunContinuationsAsynchronously)).TrySetResult();
    }

    private sealed class Peer(NamedPipeClientStream pipe, StreamReader reader, StreamWriter writer) : IDisposable
    {
        public StreamReader Reader => reader;
        public static async Task<Peer> Connect(ElevatedDirectoryHelperRequestedEvent request, int processId)
        {
            var pipe = new NamedPipeClientStream(".", request.PipeName, PipeDirection.InOut, PipeOptions.Asynchronous);
            await pipe.ConnectAsync(2000);
            var reader = new StreamReader(pipe, new UTF8Encoding(false), false, leaveOpen: true);
            var writer = new StreamWriter(pipe, new UTF8Encoding(false), leaveOpen: true) { AutoFlush = true };
            await writer.WriteLineAsync(JsonSerializer.Serialize(new ElevatedDirectoryHelperHello(
                request.AuthenticationToken, processId, true), JsonDefaults.Options));
            return new(pipe, reader, writer);
        }
        public async Task<ElevatedDirectoryHelperRequest> Next() =>
            JsonSerializer.Deserialize<ElevatedDirectoryHelperRequest>(
                (await reader.ReadLineAsync().WaitAsync(TimeSpan.FromSeconds(3)))!, JsonDefaults.Options)!;
        public Task Reply(ElevatedDirectoryHelperRequest request, bool success, int? nativeError = null) =>
            writer.WriteLineAsync(JsonSerializer.Serialize(new ElevatedDirectoryHelperResponse(
                request.Id, success, null, success ? null : "fixture rejected", NativeErrorCode: nativeError), JsonDefaults.Options));
        private int _disposed;
        public void Dispose()
        {
            if (Interlocked.Exchange(ref _disposed, 1) != 0) return;
            try { writer.Dispose(); }
            catch (IOException) { }
            reader.Dispose();
            pipe.Dispose();
        }
    }
}
