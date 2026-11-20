namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    private readonly InstalledApplicationQuery _installedApplications = new();

    public WindowsCommandResult QueryInstalledApplications(Guid correlationId, CancellationToken cancellationToken) =>
        _installedApplications.ExecuteBounded(correlationId, _log, cancellationToken);
}
