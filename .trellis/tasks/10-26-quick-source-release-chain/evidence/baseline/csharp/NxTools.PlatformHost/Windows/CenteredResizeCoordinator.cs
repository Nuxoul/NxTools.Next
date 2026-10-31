namespace NxTools.PlatformHost.Windows;

/// <summary>
/// A fixed worker budget, one in-flight operation per HWND, one coalesced next turn.
/// A hung cross-process SetWindowPos retains its slot: timeout must never create a
/// replacement worker capable of overtaking the original native operation.
/// </summary>
internal sealed class CenteredResizeCoordinator(int maximumWorkers = 2, int maximumWindows = 64)
{
    private readonly object _gate = new();
    private readonly Dictionary<IntPtr, Slot> _slots = [];
    private int _workers;
    private sealed class Slot { internal Action? Next; internal bool Running; }

    internal bool TrySchedule(IntPtr root, Action operation)
    {
        lock (_gate)
        {
            if (!_slots.TryGetValue(root, out var slot))
            {
                if (_slots.Count >= maximumWindows) return false;
                _slots[root] = slot = new Slot();
            }
            slot.Next = operation;
            StartAvailableWorkers();
            return true;
        }
    }

    private void StartAvailableWorkers()
    {
        while (_workers < maximumWorkers)
        {
            var candidate = _slots.FirstOrDefault(pair => !pair.Value.Running && pair.Value.Next is not null);
            if (candidate.Value is null) return;
            candidate.Value.Running = true;
            _workers++;
            var root = candidate.Key;
            var slot = candidate.Value;
            var worker = new Thread(() => Run(root, slot)) { IsBackground = true, Name = "NxTools.CenteredResize" };
            try { worker.Start(); }
            catch { slot.Running = false; _workers--; _slots.Remove(root); throw; }
        }
    }

    private void Run(IntPtr root, Slot slot)
    {
        while (true)
        {
            Action? operation;
            lock (_gate) { operation = slot.Next; slot.Next = null; }
            try { operation?.Invoke(); }
            catch { /* The operation boundary reports failures; retain worker/ordering integrity. */ }
            lock (_gate)
            {
                if (slot.Next is not null) continue;
                _slots.Remove(root);
                _workers--;
                StartAvailableWorkers();
                return;
            }
        }
    }
}
