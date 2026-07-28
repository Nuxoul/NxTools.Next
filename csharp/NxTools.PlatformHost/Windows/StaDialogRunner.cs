namespace NxTools.PlatformHost.Windows;

internal static class StaDialogRunner
{
    public static WindowsCommandResult Run(
        string threadName,
        Func<WindowsCommandResult> showDialog)
    {
        WindowsCommandResult? result = null;
        var thread = new Thread(() => result = showDialog())
        {
            IsBackground = true,
            Name = threadName
        };
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        thread.Join();
        return result ?? new WindowsCommandResult(false, "Windows dialog did not return a result.");
    }
}
