using System.Diagnostics;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class ElevatedDirectoryNavigationClient
{
    private uint _desktopInputPeer;

    public void BindDesktopInputPeer(uint processId)
    {
        lock (_connectionGate)
        {
            if (_disposed || _desktopInputPeer == processId) return;
            _desktopInputPeer = processId;
            if (_inputConfiguration is { } configuration)
                ConfigureInput(configuration with { DesktopProcessId = processId, CorrelationId = Guid.NewGuid() });
        }
    }

    public WindowsCommandResult? TryActivateInput(ElevatedWindowActivation input, long foregroundWindow)
    {
        if (!_inputPlatform.RequiresElevatedInput(foregroundWindow)) return null;
        var started = Stopwatch.GetTimestamp();
        InputDiagnostic(input.CorrelationId, "activation_started");
        var response = SendRequest(new(input.CorrelationId, "activate_window", null, null,
            WindowActivation: input), TimeSpan.FromSeconds(2), out var dispatched);
        InputDiagnostic(input.CorrelationId,
            response?.Success == true ? "activation_completed" : dispatched ? "activation_failed" : "activation_unavailable",
            Stopwatch.GetElapsedTime(started).TotalMilliseconds, response?.NativeErrorCode);
        return response is not null
            ? new WindowsCommandResult(response.Success, response.Error, NativeErrorCode: response.NativeErrorCode)
            : dispatched
                ? new WindowsCommandResult(false, "The elevated activation result is unknown; activation was not replayed.")
                : null;
    }
}
