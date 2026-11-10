using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Windows.Archive;
using NxTools.PlatformHost.Windows.Extensions;

namespace NxTools.PlatformHost.Tests;

internal static class NativeAllocationTests
{
    private sealed class Allocations(int failAt = 0) : IDisposable
    {
        private readonly HashSet<IntPtr> live = [];
        internal readonly List<IntPtr> Allocated = [], Freed = [];
        internal readonly OutOfMemoryException Failure = new("fixture native allocation failure");
        internal int Attempts;
        internal IntPtr Allocate(int bytes)
        {
            if (++Attempts == failAt) throw Failure;
            var pointer = Marshal.AllocHGlobal(bytes);
            live.Add(pointer); Allocated.Add(pointer);
            return pointer;
        }
        internal void Free(IntPtr pointer)
        {
            Assert(pointer != IntPtr.Zero && live.Remove(pointer), "only a live successful allocation may be freed once");
            Freed.Add(pointer);
            Marshal.FreeHGlobal(pointer);
        }
        internal void AssertFreed(int expected)
        {
            Assert(Allocated.Count == expected && Freed.Count == expected && live.Count == 0,
                "every successful native allocation must be freed exactly once");
        }
        public void Dispose()
        {
            // Teardown owns any pointer retained when a regression assertion fails.
            foreach (var pointer in live) Marshal.FreeHGlobal(pointer);
            live.Clear();
        }
    }

    public static Task ArchiveFirstAllocationFailure() => ArchiveAllocationFailure(1);
    public static Task ArchiveSecondAllocationFailure() => ArchiveAllocationFailure(2);

    private static Task ArchiveAllocationFailure(int failAt)
    {
        using var allocations = new Allocations(failAt);
        OutOfMemoryException? failure = null;
        try { using var handle = OutputNative.Open(null, "fixture", false, nativeAllocate: allocations.Allocate, nativeFree: allocations.Free); }
        catch (OutOfMemoryException error) { failure = error; }
        Assert(ReferenceEquals(failure, allocations.Failure), "original archive allocation exception must propagate unchanged");
        Assert(allocations.Attempts == failAt, "requested archive allocation was the controlled failure");
        allocations.AssertFreed(failAt - 1);
        return Task.CompletedTask;
    }

    public static Task SpawnFirstAllocationFailure() => SpawnAllocationFailure(1);
    public static Task SpawnSecondAllocationFailure() => SpawnAllocationFailure(2);

    private static Task SpawnAllocationFailure(int failAt)
    {
        using var allocations = new Allocations(failAt);
        using var input = new SafeFileHandle(IntPtr.Zero, ownsHandle: false);
        using var output = new SafeFileHandle(IntPtr.Zero, ownsHandle: false);
        using var errorHandle = new SafeFileHandle(IntPtr.Zero, ownsHandle: false);
        OutOfMemoryException? failure = null;
        try { ExtensionNative.Spawn("unused.exe", [], Environment.CurrentDirectory, input, output, errorHandle,
            nativeAllocate: allocations.Allocate, nativeFree: allocations.Free); }
        catch (OutOfMemoryException error) { failure = error; }
        Assert(ReferenceEquals(failure, allocations.Failure), "original spawn allocation exception must propagate unchanged");
        Assert(allocations.Attempts == failAt, "requested spawn allocation was the controlled failure");
        allocations.AssertFreed(failAt - 1);
        Assert(!input.IsClosed && !output.IsClosed && !errorHandle.IsClosed, "failed spawn must not close borrowed stdio handles");
        return Task.CompletedTask;
    }

    public static Task ArchiveNativeOutcomes()
    {
        var root = Directory.CreateTempSubdirectory("native-allocation-");
        try
        {
            File.WriteAllBytes(Path.Combine(root.FullName, "payload.bin"), [1, 2, 3]);
            using var parent = OutputNative.Open(null, @"\??\" + root.FullName, directory: true);
            using var allocations = new Allocations();
            using (var file = OutputNative.Open(parent, "payload.bin", directory: false, readOnly: true,
                nativeAllocate: allocations.Allocate, nativeFree: allocations.Free))
            {
                allocations.AssertFreed(2);
                Span<byte> bytes = stackalloc byte[3];
                Assert(!file.IsClosed && RandomAccess.Read(file, bytes, 0) == 3 && bytes.SequenceEqual(new byte[] { 1, 2, 3 }),
                    "returned file handle must remain caller-owned and readable after temporary buffers are freed");
            }
            ArchiveOutputException? failure = null;
            try { using var missing = OutputNative.Open(parent, "missing.bin", directory: false,
                nativeAllocate: allocations.Allocate, nativeFree: allocations.Free); }
            catch (ArchiveOutputException error) { failure = error; }
            Assert(failure is { Stage: "open_component", NativeCode: 2 }, "native missing-file stage and code must be preserved");
            allocations.AssertFreed(4);

            var checkpointFailure = new IOException("fixture after native create");
            IOException? observed = null;
            try { using var created = OutputNative.Open(parent, "created", directory: true, create: true, exclusive: true,
                createdCheckpoint: () => throw checkpointFailure, nativeAllocate: allocations.Allocate, nativeFree: allocations.Free); }
            catch (IOException error) { observed = error; }
            Assert(ReferenceEquals(observed, checkpointFailure), "post-create exception must propagate unchanged");
            allocations.AssertFreed(6);
            Directory.Delete(Path.Combine(root.FullName, "created"));
            Assert(!parent.IsClosed, "Open must not close its borrowed parent handle");
        }
        finally { root.Delete(true); }
        return Task.CompletedTask;
    }

    public static Task SpawnNativeFailure()
    {
        var root = Directory.CreateTempSubdirectory("native-allocation-");
        try
        {
            var stdin = ExtensionNative.Pipe(reads: false);
            using var stdinOurs = stdin.Ours; using var stdinChild = stdin.Child;
            var stdout = ExtensionNative.Pipe(reads: true);
            using var stdoutOurs = stdout.Ours; using var stdoutChild = stdout.Child;
            var stderr = ExtensionNative.Pipe(reads: true);
            using var stderrOurs = stderr.Ours; using var stderrChild = stderr.Child;
            using var allocations = new Allocations();
            ExtensionFailure? failure = null;
            try
            {
                var spawned = ExtensionNative.Spawn(Path.Combine(root.FullName, "missing.exe"), [], root.FullName,
                    stdinChild, stdoutChild, stderrChild, nativeAllocate: allocations.Allocate, nativeFree: allocations.Free);
                using var process = spawned.Process; using var thread = spawned.Thread;
                ExtensionNative.TerminateProcess(process, 1);
                ExtensionNative.Wait(process, 5000);
                throw new InvalidOperationException("missing executable unexpectedly spawned");
            }
            catch (ExtensionFailure error) { failure = error; }
            Assert(failure is { Reason: "create_process", NativeCode: 2 }, "native process failure stage and code must be preserved");
            allocations.AssertFreed(2);
            Assert(!stdinChild.IsClosed && !stdoutChild.IsClosed && !stderrChild.IsClosed,
                "native failure must leave caller-owned stdio handles open");
            byte[] sent = [7, 8, 9];
            using var writer = new FileStream(stdinOurs, FileAccess.Write);
            using var reader = new FileStream(stdinChild, FileAccess.Read);
            writer.Write(sent); writer.Flush();
            var received = new byte[3]; reader.ReadExactly(received);
            Assert(received.SequenceEqual(sent), "borrowed stdin pipe must remain usable after failed spawn cleanup");
        }
        finally { root.Delete(true); }
        return Task.CompletedTask;
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
