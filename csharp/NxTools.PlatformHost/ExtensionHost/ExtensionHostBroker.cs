using System.Runtime.InteropServices;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Extensions;

namespace NxTools.PlatformHost.ExtensionHost;

internal sealed record ExtensionParentPolicy(string ExpectedImage)
{
    internal static ExtensionParentPolicy Production => new(Path.Combine(AppContext.BaseDirectory, "NxTools.exe"));
}

// Demand-only stdio broker: no normal adapter, windows, hooks, trust/catalog or feature state.
internal static class ExtensionHostBroker
{
    // Once self-admitted this must live until OS process exit, not be closed by a using/finalizer:
    // closing a self-containing KILL_ON_JOB_CLOSE Job would terminate the broker itself.
    private static ExtensionKernelHandle? outerJob;

    internal static bool IsMode(IReadOnlyList<string> args) => args.Contains("--extension-host");
    internal static bool TryParent(IReadOnlyList<string> args, out uint pid)
    {
        pid = 0;
        return args.Count == 3 && args[0] == "--extension-host" && args[1] == "--extension-parent-pid" &&
            uint.TryParse(args[2], out pid) && pid != 0 && pid != Environment.ProcessId;
    }
    internal static async Task<int> RunAsync(uint parentPid, Stream input, Stream output,
        IPlatformLog? log = null, ExtensionParentPolicy? policy = null, ICredentialStore? credentialStore = null)
    {
        var stop = new CancellationTokenSource(); // Fixed control/writer tasks may retain it until process exit.
        var outgoing = Channel.CreateBounded<byte[]>(new BoundedChannelOptions(32)
            { FullMode = BoundedChannelFullMode.Wait, SingleReader = true });
        void Publish(ExtensionProcessEvent value)
        {
            var bytes = JsonSerializer.SerializeToUtf8Bytes(value, ExtensionProcessProtocol.Json);
            if (bytes.Length + 1 > ExtensionProcessProtocol.FrameLimit)
            {
                bytes = JsonSerializer.SerializeToUtf8Bytes(new ExtensionProcessEvent("rejected", value.RunId,
                    false, "broker_output_frame_limit"), ExtensionProcessProtocol.Json);
                stop.Cancel();
            }
            if (!outgoing.Writer.TryWrite(bytes)) stop.Cancel();
        }
        void PublishCredential(CredentialResult value)
        {
            var bytes = JsonSerializer.SerializeToUtf8Bytes(value, ExtensionProcessProtocol.Json);
            if (bytes.Length + 1 > ExtensionProcessProtocol.FrameLimit || !outgoing.Writer.TryWrite(bytes)) stop.Cancel();
        }
        using var credentials = new CredentialDispatcher(new CredentialAdapter(credentialStore ?? new CredentialWindowsStore(), log), PublishCredential);
        using var runner = new ExtensionProcessRunner(Publish, log);
        ExtensionKernelHandle? parent = null;
        Task monitor = Task.CompletedTask;
        var writer = Task.Run(async () =>
        {
            try
            {
                await foreach (var bytes in outgoing.Reader.ReadAllAsync(stop.Token))
                { await output.WriteAsync(bytes, stop.Token); await output.WriteAsync(new byte[] { 10 }, stop.Token); await output.FlushAsync(stop.Token); }
            }
            catch (Exception) { stop.Cancel(); }
        });
        var result = 0;
        try
        {
            parent = ValidateParent(parentPid, policy ?? ExtensionParentPolicy.Production);
            AdmitOuter(); // Fail closed before any start may be read.
            monitor = Task.Run(async () =>
            {
                try
                {
                    while (!stop.IsCancellationRequested)
                    {
                        var wait = ExtensionNative.Wait(parent, 0);
                        if (wait != 258) { stop.Cancel(); break; }
                        await Task.Delay(50, stop.Token);
                    }
                }
                catch (OperationCanceledException) when (stop.IsCancellationRequested) { }
            });
            // One fixed reader task: Console's redirected synchronous pipe may not interrupt
            // ReadAsync on cancellation. Parent/output failure still stops trees independently.
            var controls = Task.Run(async () =>
            {
                var reader = new ExtensionLineReader(input, int.MaxValue, "control");
                while (!stop.IsCancellationRequested && await reader.ReadAsync(stop.Token) is { } frame)
                {
                    DispatchControl(frame, runner, credentials, Publish, PublishCredential);
                }
            });
            await Task.WhenAny(controls, Task.Delay(Timeout.Infinite, stop.Token));
            if (controls.IsCompleted) await controls;
        }
        catch (OperationCanceledException) when (stop.IsCancellationRequested) { }
        catch (ExtensionFailure e) { result = 1; Publish(new("rejected", Guid.Empty, false, e.Reason, e.NativeCode)); }
        catch (Exception) { result = 1; Publish(new("rejected", Guid.Empty, false, "broker_failure")); }
        finally
        {
            runner.Stop();
            credentials.Dispose();
            if (!await runner.DrainAsync()) result = 1;
            outgoing.Writer.TryComplete();
            // Neither blocked control IO nor a blocked writer delays owned-tree termination.
            await Task.WhenAny(writer, Task.Delay(500));
            stop.Cancel(); await monitor; parent?.Dispose();
        }
        return result;
    }

    internal static void DispatchControl(string frame, ExtensionProcessRunner runner, CredentialDispatcher credentials,
        Action<ExtensionProcessEvent> process, Action<CredentialResult> credential)
    {
        try
        {
            var command = ExtensionProcessProtocol.Parse(frame);
            if (command.Start is { } start) runner.Start(start);
            else if (command.Control is { } control)
            {
                if (control.Op == "cancel") runner.Cancel(control.RunId);
                else runner.Write(control.RunId, control.Data!);
            }
            else if (command.Credential is { } request) credentials.TrySchedule(request);
        }
        catch (ExtensionFailure failure) { PublishControlFailure(frame, failure, process, credential); }
    }

    // Shared actual control error boundary; malformed credential controls correlate by request_id,
    // never by copying raw frame/secret/error text into the response or diagnostics.
    internal static void PublishControlFailure(string frame, ExtensionFailure failure,
        Action<ExtensionProcessEvent> process, Action<CredentialResult> credential)
    {
        try
        {
            using var doc = JsonDocument.Parse(frame);
            if (doc.RootElement.TryGetProperty("request_id", out var field))
            {
                credential(new(field.TryGetGuid(out var id) ? id : Guid.Empty, false, failure.Reason, failure.NativeCode));
                return;
            }
        }
        catch (Exception e) when (e is JsonException or InvalidOperationException) { }
        process(new("rejected", FrameRunId(frame), false, failure.Reason, failure.NativeCode));
    }

    private static Guid FrameRunId(string frame)
    {
        try
        {
            using var doc = JsonDocument.Parse(frame);
            return doc.RootElement.TryGetProperty("run_id", out var value) && value.TryGetGuid(out var id) ? id : Guid.Empty;
        }
        catch (JsonException) { return Guid.Empty; }
    }
    internal static ExtensionKernelHandle ValidateParent(uint pid, ExtensionParentPolicy policy)
    {
        ExtensionNative.Ordinary(ExtensionNative.CurrentProcess());
        var parent = ExtensionNative.OpenProcess(ExtensionNative.Synchronize | ExtensionNative.QueryLimited, false, pid);
        try
        {
            if (parent.IsInvalid) throw new ExtensionFailure("open_parent", Marshal.GetLastWin32Error());
            if (ActualParentPid() != pid) throw new ExtensionFailure("parent_pid_mismatch");
            var own = ExtensionNative.Identity(ExtensionNative.CurrentProcess());
            var peer = ExtensionNative.Identity(parent.DangerousGetHandle());
            ExtensionNative.Ordinary(parent.DangerousGetHandle());
            if (own.Sid != peer.Sid || own.Session != peer.Session) throw new ExtensionFailure("parent_identity_mismatch");
            if (!string.Equals(Path.GetFullPath(ExtensionNative.ProcessImage(parent)), Path.GetFullPath(policy.ExpectedImage), StringComparison.OrdinalIgnoreCase))
                throw new ExtensionFailure("parent_image_mismatch");
            if (ExtensionNative.Wait(parent, 0) != 258) throw new ExtensionFailure("parent_exited");
            return parent;
        }
        catch { parent.Dispose(); throw; }
    }
    private static void AdmitOuter()
    {
        if (outerJob is not null) throw new ExtensionFailure("broker_already_admitted");
        var job = ExtensionNative.MakeJob();
        try { ExtensionNative.Require(ExtensionNative.AssignJob(job, ExtensionNative.CurrentProcess()), "assign_outer_job"); }
        catch { job.Dispose(); throw; }
        outerJob = job;
    }
    [StructLayout(LayoutKind.Sequential)] private struct ProcessBasicInformation
    { internal IntPtr Reserved, Peb, Reserved2, Reserved3, Pid, ParentPid; }
    [DllImport("ntdll.dll", EntryPoint = "NtQueryInformationProcess", ExactSpelling = true)]
    private static extern int QueryProcess(IntPtr process, int kind, out ProcessBasicInformation information, int size, out int returned);
    private static uint ActualParentPid()
    {
        var status = QueryProcess(ExtensionNative.CurrentProcess(), 0, out var basic, Marshal.SizeOf<ProcessBasicInformation>(), out _);
        if (status != 0) throw new ExtensionFailure("query_parent", status);
        return checked((uint)basic.ParentPid.ToInt64());
    }
}
