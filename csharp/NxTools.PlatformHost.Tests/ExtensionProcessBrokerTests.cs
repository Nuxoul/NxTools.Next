using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.ExtensionHost;
using NxTools.PlatformHost.Windows.Extensions;
using static NxTools.PlatformHost.Tests.ExtensionProcessTests;

namespace NxTools.PlatformHost.Tests;

internal static class ExtensionProcessBrokerTests
{
    public static async Task ControlReaderStreamsBeyondInt32WithoutLifetimeBudget()
    {
        const long expectedFrames = 8193;
        var sourceFrame = LargeJsonFrame();
        var expectedBytes = checked(sourceFrame.LongLength * expectedFrames);
        Assert(expectedBytes == 2147745792L && expectedBytes > int.MaxValue,
            "stream fixture must exceed the former cumulative Int32 budget");
        using var source = new RepeatingFrameStream(sourceFrame, expectedFrames);
        var reader = ExtensionHostBroker.CreateControlReader(source);
        long frames = 0;
        var expectedChars = 0;
        while (await reader.ReadAsync() is { } frame)
        {
            frames++;
            if (frames == 1)
            {
                var command = ExtensionProcessProtocol.Parse(frame);
                Assert(command.Control is { Op: "write", Data: not null } && command.Control.Data.Length > 80000,
                    "generated control frame must satisfy the real JSON protocol");
                expectedChars = frame.Length;
            }
            Assert(frame.Length == expectedChars && frame[0] == '{' && frame[^1] == ' ',
                "stream frame changed while generating bounded chunks");
        }
        Assert(frames == expectedFrames, "unlimited broker reader stopped before all generated frames");
        Assert(source.BytesGenerated == expectedBytes, "unlimited broker reader did not consume the complete generated stream");
    }

    public static async Task FiniteReaderBudgetsAndFrameErrors()
    {
        using (var finite = new MemoryStream(ExtensionProcessProtocol.Utf8.GetBytes("{}\n{}\n"), writable: false))
        {
            var reader = new ExtensionLineReader(finite, 5, "finite");
            Assert(await reader.ReadAsync() == "{}", "finite reader must preserve an in-budget frame");
            await ExpectReaderFailure(reader, "finite_total_limit");
        }

        var maximumLine = new byte[ExtensionProcessProtocol.FrameLimit + 1];
        Array.Fill(maximumLine, (byte)'x');
        maximumLine[^1] = 10;
        using (var stream = new MemoryStream(maximumLine, writable: false))
        {
            var reader = new ExtensionLineReader(stream, maximumLine.Length, "finite");
            Assert((await reader.ReadAsync())?.Length == ExtensionProcessProtocol.FrameLimit,
                "finite reader must preserve the 256 KiB line boundary");
            Assert(await reader.ReadAsync() is null, "finite reader must reach EOF after the maximum line");
        }

        var longLine = new byte[ExtensionProcessProtocol.FrameLimit + 2];
        Array.Fill(longLine, (byte)'x');
        longLine[^1] = 10;
        using (var stream = new MemoryStream(longLine, writable: false))
            await ExpectReaderFailure(new ExtensionLineReader(stream, longLine.Length, "finite"), "finite_line_limit");
        using (var stream = new MemoryStream(new byte[] { 0xc3, 0x28, 10 }, writable: false))
            await ExpectReaderFailure(new ExtensionLineReader(stream, 3, "finite"), "finite_invalid_utf8");
        using (var stream = new MemoryStream(ExtensionProcessProtocol.Utf8.GetBytes("{}"), writable: false))
            await ExpectReaderFailure(new ExtensionLineReader(stream, 2, "finite"), "finite_truncated_line");
    }

    public static async Task ControlResponsivenessAndParentEof()
    {
        using var broker = Broker();
        try
        {
            var run = Request("duplex"); await Send(broker, run);
            await Event(broker, e => e.Op == "stdout" && e.Data == "host-call");
            var invalid = Guid.NewGuid();
            await Send(broker, new ExtensionControlRequest(1, "cancel", invalid));
            var rejected = await Event(broker, e => e.RunId == invalid);
            Assert(rejected.Reason == "unknown_run", "control reader must respond while child waits for host call");
            await Send(broker, new ExtensionControlRequest(1, "write", run.RunId, "reply"));
            await Event(broker, e => e.Op == "stdout" && e.Data == "reply");
            await Event(broker, e => e.Op == "exited");
            var sleeping = Request("sleep"); await Send(broker, sleeping);
            var started = await Event(broker, e => e.Op == "started");
            using var child = Process.GetProcessById((int)started.ProcessId!);
            broker.StandardInput.Close(); await broker.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(10));
            Assert(child.HasExited, "control EOF must terminate owned job before broker exit");
        }
        finally { KillOwned(broker); }
    }

    public static async Task BrokerCrashAndActualParentDeath()
    {
        using (var broker = Broker())
        {
            try
            {
                await Send(broker, Request("sleep"));
                var started = await Event(broker, e => e.Op == "started");
                using var child = Process.GetProcessById((int)started.ProcessId!);
                broker.Kill(); await broker.WaitForExitAsync();
                await Until(() => child.HasExited);
            }
            finally { KillOwned(broker); }
        }
        var root = Path.Combine(Path.GetTempPath(), "NxTools-extension-" + Guid.NewGuid()); Directory.CreateDirectory(root);
        Process? ownedBroker = null;
        using var parent = ExtensionProcessFixture.Start("--extension-fixture-parent", Path.Combine(root, "broker"),
            JsonSerializer.Serialize(Request("tree", Path.Combine(root, "child")), ExtensionProcessProtocol.Json));
        try
        {
            await Until(() => File.Exists(Path.Combine(root, "child")));
            ownedBroker = Process.GetProcessById(int.Parse(File.ReadAllText(Path.Combine(root, "broker"))));
            var descendantPid = int.Parse(File.ReadAllText(Path.Combine(root, "child")).Trim());
            parent.Kill(); await parent.WaitForExitAsync();
            await ownedBroker.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(10));
            Assert(Gone(descendantPid), "authenticated parent death must kill tree even without control EOF");
        }
        finally { KillOwned(parent); if (ownedBroker is not null) { KillOwned(ownedBroker); ownedBroker.Dispose(); } Directory.Delete(root, true); }
    }

    public static async Task ParentIdentityAndMalformedControl()
    {
        using (var denied = ExtensionProcessFixture.Start("--extension-fixture-broker", Environment.ProcessId.ToString(),
            Path.Combine(AppContext.BaseDirectory, "not-the-parent.exe")))
        {
            try
            {
                var rejected = await Event(denied, e => e.Op == "rejected");
                Assert(rejected.Reason == "parent_image_mismatch", "real executable policy must reject before reading start");
                await denied.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(10));
            }
            finally { KillOwned(denied); }
        }
        using var broker = Broker();
        try
        {
            var id = Guid.NewGuid();
            await broker.StandardInput.WriteLineAsync($"{{\"version\":1,\"op\":\"cancel\",\"run_id\":\"{id}\",\"op\":\"write\"}}");
            await broker.StandardInput.FlushAsync();
            Assert((await Event(broker, e => e.Op == "rejected")).Reason == "duplicate_control_key", "duplicate JSON keys must not select another op");
            await broker.StandardInput.BaseStream.WriteAsync(new byte[] { 0xc3, 0x28, 10 });
            await broker.StandardInput.BaseStream.FlushAsync();
            Assert((await Event(broker, e => e.Op == "rejected")).Reason == "control_invalid_utf8", "real broker control is strict UTF8");
            await broker.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(10));
        }
        finally { KillOwned(broker); }
    }

    private static async Task ExpectReaderFailure(ExtensionLineReader reader, string reason)
    {
        try
        {
            while (await reader.ReadAsync() is not null) { }
        }
        catch (ExtensionFailure failure)
        {
            Assert(failure.Reason == reason, "reader returned the wrong bounded failure");
            return;
        }
        throw new InvalidOperationException("reader accepted a bounded protocol violation");
    }

    private static byte[] LargeJsonFrame()
    {
        var frame = new byte[ExtensionProcessProtocol.FrameLimit];
        var prefix = ExtensionProcessProtocol.Utf8.GetBytes(
            "{\"version\":1,\"op\":\"write\",\"run_id\":\"00000000-0000-0000-0000-000000000001\",\"data\":\"");
        var suffix = ExtensionProcessProtocol.Utf8.GetBytes("\"}");
        prefix.CopyTo(frame, 0);
        var offset = prefix.Length;
        ReadOnlySpan<byte> value = [0xe4, 0xb8, 0xad];
        while (offset + value.Length + suffix.Length + 2 <= frame.Length)
        {
            value.CopyTo(frame.AsSpan(offset));
            offset += value.Length;
        }
        suffix.CopyTo(frame, offset);
        offset += suffix.Length;
        while (offset < frame.Length - 1) frame[offset++] = (byte)' ';
        frame[^1] = 10;
        return frame;
    }

    private sealed class RepeatingFrameStream(byte[] frame, long frameCount) : Stream
    {
        private readonly long length = checked(frame.LongLength * frameCount);
        private long remaining = checked(frame.LongLength * frameCount);
        private int frameOffset;

        internal long BytesGenerated { get; private set; }
        public override bool CanRead => true;
        public override bool CanSeek => false;
        public override bool CanWrite => false;
        public override long Length => length;
        public override long Position { get => BytesGenerated; set => throw new NotSupportedException(); }
        public override void Flush() { }
        public override int Read(byte[] buffer, int offset, int count) => ReadCore(buffer.AsSpan(offset, count));
        public override int Read(Span<byte> buffer) => ReadCore(buffer);
        public override Task<int> ReadAsync(byte[] buffer, int offset, int count, CancellationToken cancellationToken)
        {
            cancellationToken.ThrowIfCancellationRequested();
            return Task.FromResult(ReadCore(buffer.AsSpan(offset, count)));
        }
        public override ValueTask<int> ReadAsync(Memory<byte> buffer, CancellationToken cancellationToken = default)
        {
            cancellationToken.ThrowIfCancellationRequested();
            return ValueTask.FromResult(ReadCore(buffer.Span));
        }
        private int ReadCore(Span<byte> buffer)
        {
            var target = (int)Math.Min(buffer.Length, remaining);
            var written = 0;
            while (written < target)
            {
                var count = Math.Min(target - written, frame.Length - frameOffset);
                frame.AsSpan(frameOffset, count).CopyTo(buffer[written..]);
                written += count;
                frameOffset += count;
                if (frameOffset == frame.Length) frameOffset = 0;
            }
            remaining -= written;
            BytesGenerated += written;
            return written;
        }
        public override long Seek(long offset, SeekOrigin origin) => throw new NotSupportedException();
        public override void SetLength(long value) => throw new NotSupportedException();
        public override void Write(byte[] buffer, int offset, int count) => throw new NotSupportedException();
    }

    private static Process Broker() => ExtensionProcessFixture.Start("--extension-fixture-broker", Environment.ProcessId.ToString(), Environment.ProcessPath!);
    private static async Task Send(Process broker, object value)
    { await broker.StandardInput.WriteLineAsync(JsonSerializer.Serialize(value, ExtensionProcessProtocol.Json)); await broker.StandardInput.FlushAsync(); }
    private static async Task<ExtensionProcessEvent> Event(Process broker, Func<ExtensionProcessEvent, bool> match)
    {
        while (true)
        {
            var line = await broker.StandardOutput.ReadLineAsync().WaitAsync(TimeSpan.FromSeconds(10));
            Assert(line is not null, "broker protocol unexpectedly closed");
            var value = JsonSerializer.Deserialize<ExtensionProcessEvent>(line!, ExtensionProcessProtocol.Json)!;
            if (match(value)) return value;
        }
    }
    private static void KillOwned(Process process)
    { if (!process.HasExited) { process.Kill(entireProcessTree: true); process.WaitForExit(5000); } }
}
