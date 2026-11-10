namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveOutputSession
{
    // Called only after exclusive creation reports an existing name. Conflict presentation
    // needs size, not a content fingerprint or a target-version replacement transaction.
    internal long? ExistingExtractionSize(string relative)
    {
        var target = ResolveParent(relative);
        try
        {
            using var file = OutputNative.Open(target.Parent, target.Name, directory: false, readOnly: true, anyKind: true);
            return RandomAccess.GetLength(file);
        }
        catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3) { return null; }
        catch (ArchiveOutputException failure)
        {
            throw new ArchiveOutputException(failure.Stage == "reparse_component" ? "target_reparse" :
                failure.Stage == "wrong_kind" ? "target_directory" : failure.NativeCode is 32 or 33 ? "target_busy" :
                failure.NativeCode == 5 ? "target_denied" : failure.Stage, failure.NativeCode);
        }
    }
}
