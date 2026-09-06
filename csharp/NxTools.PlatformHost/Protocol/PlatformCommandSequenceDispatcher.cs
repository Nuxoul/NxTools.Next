namespace NxTools.PlatformHost.Protocol;

internal sealed class PlatformCommandSequenceDispatcher
{
    private const string NativeFileDragCommand = "begin_file_drag";

    // Win32/Shell commands whose native work blocks for unbounded time (Shell
    // extension enumeration, icon extraction, dialog pickers). Running them
    // inline stalls the pipe read loop, delaying every later command —
    // including the quick-launch results these commands are meant to feed.
    private static readonly HashSet<string> DetachedCommands = new(StringComparer.Ordinal)
    {
        NativeFileDragCommand,
        "query_shell_actions",
        "query_shell_submenu",
        "invoke_shell_action",
        "extract_shell_icon",
        "pick_clipboard_archive_path",
        "pick_clipboard_storage_directory",
        "pick_directory",
        "pick_program",
        "export_browser_extension",
        // Launching a Shell target waits on process start and can take
        // seconds. Measured incident: the quick-launch panel hide that follows
        // an Enter-launch was stuck behind it in the pipe read loop, and even
        // the self-healing retry queued behind the same stall — the panel
        // stayed on screen until the launch finished.
        "open_shell_target",
    };

    private readonly object _gate = new();
    private readonly HashSet<Task> _detachedCommands = [];
    private readonly Action<Exception>? _onDetachedFailure;

    public PlatformCommandSequenceDispatcher(
        Action<Exception>? onDetachedFailure = null)
    {
        _onDetachedFailure = onDetachedFailure;
    }

    public Task DispatchAsync(
        string commandName,
        Func<Task> executeAsync)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(commandName);
        ArgumentNullException.ThrowIfNull(executeAsync);
        if (!DetachedCommands.Contains(commandName))
        {
            return executeAsync();
        }

        Task command;
        try
        {
            command = executeAsync();
        }
        catch (Exception error)
        {
            _onDetachedFailure?.Invoke(error);
            return Task.CompletedTask;
        }

        lock (_gate)
        {
            _detachedCommands.Add(command);
        }
        _ = ObserveDetachedAsync(command);
        return Task.CompletedTask;
    }

    public Task DrainAsync()
    {
        Task[] commands;
        lock (_gate)
        {
            commands = [.. _detachedCommands];
        }
        return Task.WhenAll(commands.Select(IgnoreFailureAsync));
    }

    private async Task ObserveDetachedAsync(Task command)
    {
        try
        {
            await command;
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception error)
        {
            _onDetachedFailure?.Invoke(error);
        }
        finally
        {
            lock (_gate)
            {
                _detachedCommands.Remove(command);
            }
        }
    }

    private static async Task IgnoreFailureAsync(Task command)
    {
        try
        {
            await command;
        }
        catch
        {
        }
    }
}
