using System.Diagnostics;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class ElevatedDirectoryNavigationClient
{
    private readonly IElevatedInputClientPlatform _inputPlatform;
    private readonly Func<long> _inputClock;
    private int _automaticReconnects;
    private ElevatedInputConfiguration? _inputConfiguration;
    private bool _inputWorkerRunning;
    private int _inputOwnerProcess;
    private long _inputOwnerGeneration = long.MinValue;
    private int _failedInputProcess;
    private long _failedInputGeneration = long.MinValue;
    private int _pointerOwnerProcess;
    private long _pointerOwnerGeneration = long.MinValue;
    private long _pointerConfigurationTick;
    private long _lastPointerTick;
    private bool _hasPointerTick;

    public void ConfigureInput(ElevatedInputConfiguration configuration)
    {
        lock (_connectionGate)
        {
            if (_disposed) return;
            _inputConfiguration = configuration with { DesktopProcessId = _desktopInputPeer };
            _failedInputProcess = 0;
            _failedInputGeneration = long.MinValue;
            ClearInputOwner();
            _pointerOwnerProcess = 0;
            _pointerOwnerGeneration = long.MinValue;
            _pointerConfigurationTick = _inputClock();
            _hasPointerTick = false;
        }
        StartInputConfiguration();
    }

    public bool OwnsInput(int helperProcessId, long generation)
    {
        lock (_connectionGate)
            return !_disposed && _connection is not null && _inputOwnerProcess != 0 &&
                (helperProcessId == 0 || _inputOwnerProcess == helperProcessId) &&
                _inputOwnerGeneration == generation;
    }

    public void InputUnavailable(int helperProcessId, long generation)
    {
        lock (_connectionGate)
        {
            if (_connection?.HelperProcessId == helperProcessId &&
                _inputConfiguration?.Keyboard.ResetGeneration == generation)
            {
                _failedInputProcess = helperProcessId;
                _failedInputGeneration = generation;
                ClearInputOwner();
            }
        }
    }

    public bool AcceptPointer(int helperProcessId, uint timestamp)
    {
        lock (_connectionGate)
        {
            var now = _inputClock();
            var age = unchecked((int)((uint)now - timestamp));
            var sampledAt = now - age;
            if (_disposed || _connection?.HelperProcessId != helperProcessId ||
                helperProcessId == 0 || _pointerOwnerProcess != helperProcessId ||
                _inputConfiguration?.Keyboard.ResetGeneration != _pointerOwnerGeneration ||
                age < 0 || age > 1000 || sampledAt < _pointerConfigurationTick ||
                (_hasPointerTick && sampledAt <= _lastPointerTick)) return false;
            _lastPointerTick = sampledAt;
            _hasPointerTick = true;
            return true;
        }
    }

    private void ClearInputOwner()
    {
        _inputOwnerProcess = 0;
        _inputOwnerGeneration = long.MinValue;
    }

    private void StartInputConfiguration()
    {
        lock (_connectionGate)
        {
            if (_disposed || _connection is null || _inputConfiguration is null || _inputWorkerRunning) return;
            _inputWorkerRunning = true;
        }
        _ = Task.Run(ApplyInputConfigurations);
    }

    private void ApplyInputConfigurations()
    {
        while (true)
        {
            ElevatedInputConfiguration? configuration;
            HelperConnection? connection;
            ElevatedDirectoryHelperResponse? response;
            lock (_requestGate)
            {
                lock (_connectionGate)
                {
                    configuration = _inputConfiguration;
                    connection = _connection;
                    if (_disposed || configuration is null || connection is null)
                    {
                        _inputWorkerRunning = false;
                        return;
                    }
                }
                response = SendRequest(new(configuration.CorrelationId, "configure_input", null, null,
                    InputConfiguration: configuration), TimeSpan.FromSeconds(5));
            }
            lock (_connectionGate)
            {
                if (_disposed) { _inputWorkerRunning = false; return; }
                if (!ReferenceEquals(configuration, _inputConfiguration) ||
                    !ReferenceEquals(connection, _connection)) continue;
                if (response?.Success == true && configuration.Keyboard is
                    { Enabled: true, Recording: false, Trigger: QuickLaunchTriggerKind.DoubleControl } &&
                    !(_failedInputProcess == connection.HelperProcessId &&
                      _failedInputGeneration == configuration.Keyboard.ResetGeneration))
                {
                    _inputOwnerProcess = connection.HelperProcessId;
                    _inputOwnerGeneration = configuration.Keyboard.ResetGeneration;
                }
                if (response?.Success == true)
                {
                    _pointerOwnerProcess = connection.HelperProcessId;
                    _pointerOwnerGeneration = configuration.Keyboard.ResetGeneration;
                }
                _inputWorkerRunning = false;
            }
            InputDiagnostic(configuration.CorrelationId, response?.Success == true ? "configured" : "unavailable",
                nativeError: response?.NativeErrorCode);
            return;
        }
    }

    public WindowsCommandResult? TryPasteInput(ElevatedPasteInput input)
    {
        if (!_inputPlatform.RequiresElevatedInput(input.TargetWindowHandle)) return null;
        var started = Stopwatch.GetTimestamp();
        InputDiagnostic(input.CorrelationId, "paste_started");
        var response = SendRequest(new(input.CorrelationId, "paste_input", null, null,
            PasteInput: input), TimeSpan.FromSeconds(3), out var dispatched);
        InputDiagnostic(input.CorrelationId,
            response?.Success == true ? "paste_completed" : dispatched ? "paste_failed" : "paste_unavailable",
            Stopwatch.GetElapsedTime(started).TotalMilliseconds, response?.NativeErrorCode);
        return response is not null
            ? new WindowsCommandResult(response.Success, response.Error, NativeErrorCode: response.NativeErrorCode)
            : dispatched
                ? new WindowsCommandResult(false, "The elevated paste result is unknown; input was not replayed.")
                : null;
    }

    private void ObserveHelperExit(HelperConnection connection)
    {
        _ = Task.Run(async () =>
        {
            try
            {
                await _inputPlatform.WaitForExitAsync(connection.HelperProcessId, _shutdown.Token);
                InvalidateExitedHelper(connection);
            }
            catch (ArgumentException) { InvalidateExitedHelper(connection); }
            catch (OperationCanceledException) { }
            catch (InvalidOperationException) { }
        });
    }
    private void InvalidateExitedHelper(HelperConnection connection) =>
        InvalidateConnection(connection, "the elevated helper process exited");

    private void InputDiagnostic(Guid correlationId, string state, double elapsed = 0, int? nativeError = null)
    {
        try
        {
            _log.Write(state.EndsWith("failed", StringComparison.Ordinal) || state.EndsWith("unavailable", StringComparison.Ordinal)
                ? "warning" : "info", "elevated_input.lifecycle", correlationId,
                new Dictionary<string, object?> { ["process_role"] = "ordinary_platform_host",
                    ["state"] = state, ["elapsed_ms"] = elapsed, ["native_error_code"] = nativeError });
        }
        catch { /* Diagnostics do not change input results. */ }
    }
}
