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
        "archive_entry",
        "raw_numbered_join",
        "read_clipboard_file_drop",
        "read_clipboard_text",
        "show_context_menu",
        "process_window_pick",
        "set_clipboard_capture_paused",
        "present_fixed_window_over_anchor",
        "prepare_notification_window",
        "prepare_main_window",
        "activate_window",
        "query_installed_applications",
        "query_shell_actions",
        "query_shell_submenu",
        "invoke_shell_action",
        "extract_shell_icon",
        "pick_clipboard_archive_path",
        "pick_archive_path",
        "pick_clipboard_storage_directory",
        "pick_directory",
        "pick_program",
        "pick_markdown_document",
        "pick_markdown_export",
        "print_markdown_pdf",
        "pick_extension_manifest",
        "pick_extension_runtime",
        "pick_extension_setting_path",
        "pick_browser_extension_export_path",
        "inspect_extension_runtime",
        "export_browser_extension",
        // Launching a Shell target waits on process start and can take
        // seconds. Measured incident: the quick-launch panel hide that follows
        // an Enter-launch was stuck behind it in the pipe read loop, and even
        // the self-healing retry queued behind the same stall — the panel
        // stayed on screen until the launch finished.
        "open_shell_target",
        "launch_process",
        "ensure_indexer_service",
        "open_default_apps_settings",
        "markdown_association",
        "shutdown_system",
        "suspend_system",
        "archive_shell_integration",
        "navigate_directory",
        "open_file_in_dialog",
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
        Func<Task> executeAsync) =>
        DispatchAsync(commandName, _ => executeAsync(), CancellationToken.None);

    public Task DispatchAsync(
        string commandName,
        Func<Task> executeAsync,
        CancellationToken cancellationToken) =>
        DispatchAsync(commandName, _ => executeAsync(), cancellationToken);

    public Task DispatchAsync(
        string commandName,
        Func<CancellationToken, Task> executeAsync,
        CancellationToken cancellationToken)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(commandName);
        ArgumentNullException.ThrowIfNull(executeAsync);
        if (!DetachedCommands.Contains(commandName))
        {
            return executeAsync(cancellationToken);
        }

        // Menu admission is synchronous and bounded; its own STA performs all modal work.
        // Admit before reading exact cancel so a following cancel cannot overtake session creation.
        // Other detached delegates may enter blocking native APIs before returning a Task.
        var command = commandName is "show_context_menu" or "set_clipboard_capture_paused" or "process_window_pick"
            ? executeAsync(cancellationToken)
            : Task.Run(() => executeAsync(cancellationToken), cancellationToken);
        lock (_gate)
        {
            _detachedCommands.Add(command);
        }
        _ = ObserveDetachedAsync(command);
        return Task.CompletedTask;
    }

    public async Task DrainAsync(TimeSpan? timeout = null)
    {
        var budget = timeout ?? TimeSpan.FromSeconds(2);
        if (budget < TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(timeout));
        }

        var deadline = DateTime.UtcNow + budget;
        while (true)
        {
            Task[] commands;
            lock (_gate)
            {
                commands = [.. _detachedCommands];
            }
            if (commands.Length == 0)
            {
                return;
            }

            var remaining = deadline - DateTime.UtcNow;
            if (remaining <= TimeSpan.Zero)
            {
                return;
            }

            var drain = Task.WhenAll(commands.Select(IgnoreFailureAsync));
            try
            {
                await drain.WaitAsync(remaining);
            }
            catch (TimeoutException)
            {
                return;
            }
        }
    }

    public void BeginDrain(TimeSpan? timeout = null)
    {
        _ = ObserveDrainAsync(timeout);
    }

    private async Task ObserveDrainAsync(TimeSpan? timeout)
    {
        try
        {
            await DrainAsync(timeout);
        }
        catch
        {
            // Teardown observation must never create an unobserved task fault.
        }
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
            try
            {
                _onDetachedFailure?.Invoke(error);
            }
            catch
            {
                // Detached diagnostics must not create an unobserved task fault.
            }
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
