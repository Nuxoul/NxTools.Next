using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    internal Guid BindMaterialPeer(uint processId) => _windowMaterials.BindPeer(processId);
    internal SystemMaterialCapabilitiesChangedEvent? MaterialSnapshot => _windowMaterials.SnapshotEvent();

    internal Task<WindowsCommandResult> ApplyWindowMaterialAsync(
        ApplyWindowMaterialCommand command, Guid correlation, CancellationToken cancellation) =>
        ApplyWindowMaterialCoreAsync(_windowMaterials, command, correlation, cancellation);

    internal static Task<WindowsCommandResult> ApplyWindowMaterialCoreAsync(
        WindowMaterialOwner owner, ApplyWindowMaterialCommand command, Guid correlation,
        CancellationToken cancellation) => Task.Run(() =>
        {
            var completed = BoundedStaOperationRunner.TryRun("NxTools.WindowMaterial",
                TimeSpan.FromSeconds(5), token => owner.Apply(command, correlation, token),
                out WindowsCommandResult? applied, out _, cancellation);
            return completed && applied is not null ? applied : new(false, "window_material_unavailable");
        }, CancellationToken.None);
}
