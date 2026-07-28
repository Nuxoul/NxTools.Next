namespace NxTools.PlatformHost.Protocol;

internal sealed class PlatformCommandSequenceDispatcher
{
    private const string NativeFileDragCommand = "begin_file_drag";
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
        if (!string.Equals(
                commandName,
                NativeFileDragCommand,
                StringComparison.Ordinal))
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
