namespace NxTools.PlatformHost.Windows;

// Kernel ownership survives the broker: a restarted broker's recovery child waits for any
// prior deployment child. Abandoned ownership means the prior writer has exited, not success.
internal sealed class TrustedDeploymentGate : IDisposable
{
    private readonly Mutex mutex;
    private TrustedDeploymentGate(Mutex mutex) => this.mutex = mutex;
    internal static TrustedDeploymentGate Acquire()
    {
        var mutex = new Mutex(false, @"Global\NxToolsProtectedDeploymentV1");
        try
        {
            try
            {
                if (!mutex.WaitOne(TimeSpan.FromMinutes(3)))
                    throw new TimeoutException("Protected deployment is already running.");
            }
            catch (AbandonedMutexException) { /* Previous writer exited; transaction recovery owns repair. */ }
            return new TrustedDeploymentGate(mutex);
        }
        catch { mutex.Dispose(); throw; }
    }
    public void Dispose()
    {
        try { mutex.ReleaseMutex(); }
        finally { mutex.Dispose(); }
    }
}
