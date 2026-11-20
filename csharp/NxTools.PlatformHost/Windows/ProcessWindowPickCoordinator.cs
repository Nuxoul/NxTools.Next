using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed class ProcessWindowPickCoordinator : IDisposable
{
    private readonly object _gate = new();
    private readonly Queue<(Action Run, Action Fail)> _pending = new();
    private readonly ProcessWindowPickController _controller;
    private readonly Func<Action, bool>? _fixtureEnqueue;
    private Thread? _thread;
    private bool _disposed, _exited;

    internal ProcessWindowPickCoordinator(Func<ProcessWindowPickChangedEvent, bool> publish, IPlatformLog log)
    { _controller = new(new ProcessWindowPickNative(), publish, log); }

    internal ProcessWindowPickCoordinator(ProcessWindowPickController controller, Func<Action, bool> enqueue)
    { _controller = controller; _fixtureEnqueue = enqueue; }

    internal Task<WindowsCommandResult> ExecuteAsync(ProcessWindowPickCommand command, uint peer, CancellationToken cancellation)
    {
        var completion = new TaskCompletionSource<WindowsCommandResult>(TaskCreationOptions.RunContinuationsAsynchronously);
        void Run()
        {
            try { completion.TrySetResult(_controller.Execute(command, peer, cancellation)); }
            catch (Exception) { completion.TrySetResult(new(false, "window_pick_owner_failed")); }
        }
        void Fail() => completion.TrySetResult(new(false, "window_pick_unavailable"));
        lock (_gate)
        {
            if (_disposed || _exited) { Fail(); return completion.Task; }
            if (_fixtureEnqueue is not null) { if (!_fixtureEnqueue(Run)) Fail(); return completion.Task; }
            if (_pending.Count >= 32) { completion.SetResult(new(false, "window_pick_busy")); return completion.Task; }
            _pending.Enqueue((Run, Fail));
            if (_thread is null)
            {
                _thread = new Thread(Loop) { IsBackground = true, Name = "NxTools.ProcessWindowPick" };
                _thread.SetApartmentState(ApartmentState.STA);
                _thread.Start();
            }
        }
        return completion.Task;
    }

    private void Loop()
    {
        try
        {
            using var context = new ApplicationContext();
            using var timer = new System.Windows.Forms.Timer { Interval = 25 };
            timer.Tick += (_, _) =>
            {
                // A fixed owner and bounded FIFO preserve start/cancel order without blocking
                // the command reader or the shared clipboard/hotkey event thread.
                for (var i = 0; i < 32; i++)
                {
                    (Action Run, Action Fail) pending;
                    bool retired;
                    lock (_gate) { if (!_pending.TryDequeue(out pending)) break; retired = _disposed; }
                    if (retired) { _controller.Stop(); pending.Fail(); }
                    else pending.Run();
                }
                bool stopping;
                lock (_gate) stopping = _disposed;
                if (stopping) _controller.Stop();
                _controller.Tick();
                if (stopping && !_controller.Occupied) context.ExitThread();
            };
            timer.Start();
            Application.Run(context);
        }
        finally
        {
            _controller.Stop();
            lock (_gate)
            {
                _exited = true;
                while (_pending.TryDequeue(out var pending)) pending.Fail();
            }
        }
    }

    public void Dispose()
    {
        lock (_gate)
        {
            _disposed = true;
            if (_fixtureEnqueue is not null) _fixtureEnqueue(_controller.Stop);
        }
        // The original owner keeps pumping intercepted releases after session retirement.
        // Do not join it from the serial reader or create a replacement.
    }
}
