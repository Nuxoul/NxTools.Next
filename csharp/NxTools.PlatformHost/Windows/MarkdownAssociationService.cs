using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal readonly record struct MarkdownAssociationTarget(string KeyPath, string ValueName);
internal sealed record MarkdownAssociationValue(RegistryValueKind Kind, string? Text);

internal interface IMarkdownAssociationStore
{
    MarkdownAssociationValue? Read(MarkdownAssociationTarget target);
    void Write(MarkdownAssociationTarget target, MarkdownAssociationValue? value);
}

internal sealed class MarkdownAssociationService(
    IMarkdownAssociationStore store,
    Func<string, string?> effectiveProgId,
    Func<string?> approvedDesktopPath,
    Action notifyChanged,
    bool supportsRegisteredAppSettings,
    Action<ProcessStartInfo> launchSettings,
    IPlatformLog log)
{
    private const string ProgId = "NxTools.Markdown";
    private const string ApplicationName = "NxTools Markdown Reader";
    private const string Classes = @"Software\Classes\";
    private const string Capabilities = @"Software\NxTools\MarkdownReader\Capabilities";
    private const string CommandSuffix = "\" --markdown-open \"%1\"";
    private const string Md = ".md";
    private const string Markdown = ".markdown";
    private const string DefaultAppsUri = "ms-settings:defaultapps";
    private static readonly MarkdownAssociationTarget OpenCommand = new(Classes + ProgId + @"\shell\open\command", "");
    private static readonly RegistrationEntry RegisteredApplication = new(new(@"Software\RegisteredApplications", ApplicationName), Capabilities);
    private static readonly RegistrationEntry[] Entries =
    [
        new(new(Classes + ProgId, ""), "NxTools Markdown Document"),
        new(OpenCommand, null),
        new(new(Capabilities, "ApplicationName"), ApplicationName),
        new(new(Capabilities, "ApplicationDescription"), "Read Markdown documents with NxTools."),
        new(new(Classes + Md + @"\OpenWithProgids", ProgId), "", Md),
        new(new(Capabilities + @"\FileAssociations", Md), ProgId, Md),
        new(new(Classes + Markdown + @"\OpenWithProgids", ProgId), "", Markdown),
        new(new(Capabilities + @"\FileAssociations", Markdown), ProgId, Markdown),
        RegisteredApplication,
    ];

    internal static MarkdownAssociationService CreateProduct(IPlatformLog log) => new(new CurrentUserStore(),
        QueryEffectiveProgId, WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath, NotifyAssociationsChanged,
        OperatingSystem.IsWindowsVersionAtLeast(10, 0, 22000),
        static startInfo => { using var launched = Process.Start(startInfo); }, log);

    // Keep all read/write/notification work off the serial reader. The single reservation also keeps
    // a timed-out native call from being overtaken by another transaction in this PlatformHost.
    internal WindowsCommandResult ExecuteBounded(MarkdownAssociationCommand command, Guid correlation, CancellationToken cancellation)
    {
        var started = Stopwatch.GetTimestamp();
        if (cancellation.IsCancellationRequested) return Execute(command, correlation, cancellation);
        if (BoundedStaOperationRunner.TryRun("NxTools.MarkdownAssociation", TimeSpan.FromSeconds(15),
            token => Execute(command, correlation, token), out WindowsCommandResult? result, out _, cancellation) && result is not null)
            return result;
        var unconfirmed = Failure("native_operation_unconfirmed");
        WriteDiagnostic("unconfirmed", command, correlation, started, unconfirmed);
        return unconfirmed;
    }

    internal WindowsCommandResult Execute(MarkdownAssociationCommand command, Guid correlation, CancellationToken cancellation)
    {
        var started = Stopwatch.GetTimestamp();
        WriteDiagnostic("accepted", command, correlation, started);
        WindowsCommandResult result;
        try
        {
            cancellation.ThrowIfCancellationRequested();
            WriteDiagnostic("started", command, correlation, started);
            result = ExecuteCore(command, correlation, started, cancellation);
        }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { result = Failure("cancelled"); }
        catch (Exception error) { result = Failure("registry_failed", error); }
        WriteDiagnostic("terminal", command, correlation, started, result);
        return result;
    }

    private WindowsCommandResult ExecuteCore(MarkdownAssociationCommand command, Guid correlation, long started, CancellationToken cancellation)
    {
        if (command.Action == MarkdownAssociationAction.Query)
            return Observe(ReadRegistration());
        if (command.Action == MarkdownAssociationAction.OpenDefaults)
            return OpenDefaults(command, correlation, started, cancellation);

        var registering = command.Action == MarkdownAssociationAction.Register;
        string? openCommand = null;
        if (registering)
        {
            string? desktopPath;
            try { desktopPath = approvedDesktopPath(); }
            catch (Exception error) { return Failure("identity_unavailable", error); }
            if (desktopPath is null || !IsDesktopPath(desktopPath)) return Failure("identity_unavailable");
            openCommand = '"' + desktopPath + CommandSuffix;
        }
        else
        {
            try { if (IsDefault(Md) || IsDefault(Markdown)) return Failure("still_default"); }
            catch (Exception error) { return Failure("query_failed", error); }
        }

        var snapshot = ReadRegistration();
        if (snapshot.Any(item => item.Value is not null && !item.Entry.Matches(item.Value)))
            return Failure("registration_conflict");
        // Publish the RegisteredApplications value last; retire it first on removal.
        var changes = (registering ? snapshot : snapshot.Reverse())
            .Select(item => (item.Entry, Before: item.Value, After: registering ? item.Entry.Value(openCommand) : null))
            .Where(item => item.Before != item.After).ToArray();
        var attempted = 0;
        try
        {
            foreach (var change in changes)
            {
                cancellation.ThrowIfCancellationRequested();
                attempted++; // Include native writes which performed the mutation before throwing.
                store.Write(change.Entry.Target, change.After);
            }
            cancellation.ThrowIfCancellationRequested();
        }
        catch (Exception failure)
        {
            Exception? rollbackError = null;
            while (attempted > 0)
            {
                var change = changes[--attempted];
                try { store.Write(change.Entry.Target, change.Before); }
                catch (Exception error) { rollbackError ??= error; }
            }
            if (rollbackError is not null)
            {
                WriteDiagnostic("rollback_incomplete", command, correlation, started, Failure("registry_failed", rollbackError));
                Notify(command, correlation, started);
                return Failure("registry_failed", failure is OperationCanceledException ? rollbackError : failure);
            }
            throw;
        }
        if (changes.Length != 0)
        {
            WriteDiagnostic("registration_changed", command, correlation, started);
            Notify(command, correlation, started);
        }
        return Observe(ReadRegistration());
    }

    private WindowsCommandResult OpenDefaults(MarkdownAssociationCommand command, Guid correlation, long started, CancellationToken cancellation)
    {
        var targeted = supportsRegisteredAppSettings && RegisteredApplication.Matches(store.Read(RegisteredApplication.Target));
        var uri = targeted
            ? DefaultAppsUri + "?registeredAppUser=" + Uri.EscapeDataString(RegisteredApplication.Target.ValueName)
            : DefaultAppsUri;
        WriteDiagnostic(targeted ? "opening_registered_app_settings" : "opening_general_settings", command, correlation, started);
        cancellation.ThrowIfCancellationRequested();
        try
        {
            launchSettings(new ProcessStartInfo(uri) { UseShellExecute = true });
            // Shell acceptance does not confirm the Settings UI or a change to defaults.
            return new(true, Data: "opened");
        }
        catch (Exception error) { return Failure("open_defaults_failed", error); }
    }

    private (RegistrationEntry Entry, MarkdownAssociationValue? Value)[] ReadRegistration() =>
        Entries.Select(entry => (entry, store.Read(entry.Target))).ToArray();

    private WindowsCommandResult Observe((RegistrationEntry Entry, MarkdownAssociationValue? Value)[] snapshot)
    {
        bool Registered(string extension) => snapshot.Where(item => item.Entry.Extension is null || item.Entry.Extension == extension)
            .All(item => item.Entry.Matches(item.Value));
        try
        {
            return new(true, Data: JsonSerializer.Serialize(new
            {
                md = new { registered = Registered(Md), is_default = IsDefault(Md) },
                markdown = new { registered = Registered(Markdown), is_default = IsDefault(Markdown) },
            }));
        }
        catch (Exception error) { return Failure("query_failed", error); }
    }

    private bool IsDefault(string extension) => string.Equals(effectiveProgId(extension), ProgId, StringComparison.OrdinalIgnoreCase);

    private static bool IsDesktopPath(string path) => !path.Contains('"') && Path.IsPathFullyQualified(path) &&
        string.Equals(Path.GetFileName(path), "NxTools.exe", StringComparison.OrdinalIgnoreCase);

    private static bool IsOwnedCommand(string? text) => text is not null && text.Length > CommandSuffix.Length + 1 &&
        text.StartsWith('"') && text.EndsWith(CommandSuffix, StringComparison.Ordinal) && IsDesktopPath(text[1..^CommandSuffix.Length]);

    private sealed record RegistrationEntry(MarkdownAssociationTarget Target, string? Text, string? Extension = null)
    {
        internal bool Matches(MarkdownAssociationValue? value) => value is { Kind: RegistryValueKind.String } &&
            (Text is not null ? value.Text == Text : IsOwnedCommand(value.Text));
        internal MarkdownAssociationValue Value(string? command) => new(RegistryValueKind.String, Text ?? command);
    }

    private static WindowsCommandResult Failure(string kind, Exception? error = null) => new(false, kind switch
    {
        "still_default" => "Change the Windows default app for Markdown before removing NxTools registration.",
        "identity_unavailable" => "The installed NxTools executable could not be verified.",
        "registration_conflict" => "A conflicting Markdown registration was left unchanged.",
        "query_failed" => "Windows could not report the effective Markdown default app.",
        "open_defaults_failed" => "Windows default apps settings could not be opened.",
        "cancelled" => "The Markdown association operation was cancelled.",
        "native_operation_unconfirmed" => "The Markdown association operation could not be confirmed. Refresh the registration status.",
        _ => "Windows could not access the Markdown registration.",
    }, NativeErrorCode: error is Win32Exception win32 ? win32.NativeErrorCode : error?.HResult, Data: kind);

    private void Notify(MarkdownAssociationCommand command, Guid correlation, long started)
    {
        try { notifyChanged(); }
        catch (Exception error) { WriteDiagnostic("notification_failed", command, correlation, started, Failure("registry_failed", error)); }
    }

    private void WriteDiagnostic(string stage, MarkdownAssociationCommand command, Guid correlation, long started, WindowsCommandResult? result = null)
    {
        try
        {
            log.Write(result is { Success: false } ? "warning" : "info", "markdown.association." + stage, correlation,
                new Dictionary<string, object?>
                {
                    ["action"] = command.Action.ToString(),
                    ["state"] = result is null ? stage : result.Success && command.Action != MarkdownAssociationAction.OpenDefaults ? "observed" : result.Data,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                    ["native_error_code"] = result?.NativeErrorCode,
                });
        }
        catch (Exception) { /* Observational diagnostics never change the operation. */ }
    }

    private const uint AssocStrProgId = 20;
    private const int NoAssociation = unchecked((int)0x80070483); // HRESULT_FROM_WIN32(ERROR_NO_ASSOCIATION)
    private const int PointerError = unchecked((int)0x80004003);
    private const uint AssocfNoTruncate = 0x20;
    private const int AssocChanged = 0x08000000;
    private const uint Flush = 0x1000;

    private static string? QueryEffectiveProgId(string extension)
    {
        var output = new StringBuilder(256);
        var length = (uint)output.Capacity;
        var hr = AssocQueryString(AssocfNoTruncate, AssocStrProgId, extension, null, output, ref length);
        if (hr == PointerError && length > output.Capacity && length <= 32768)
        {
            output = new StringBuilder((int)length);
            hr = AssocQueryString(AssocfNoTruncate, AssocStrProgId, extension, null, output, ref length);
        }
        return hr switch
        {
            0 => output.ToString(),
            NoAssociation => null,
            _ => throw new COMException("Effective association lookup failed.", hr),
        };
    }

    private static void NotifyAssociationsChanged() => SHChangeNotify(AssocChanged, Flush, IntPtr.Zero, IntPtr.Zero);

    [DllImport("shlwapi.dll", EntryPoint = "AssocQueryStringW", ExactSpelling = true, CharSet = CharSet.Unicode)]
    private static extern int AssocQueryString(uint flags, uint str, string association, string? extra, StringBuilder output, ref uint length);

    [DllImport("shell32.dll")]
    private static extern void SHChangeNotify(int eventId, uint flags, IntPtr item1, IntPtr item2);

    private sealed class CurrentUserStore : IMarkdownAssociationStore
    {
        public MarkdownAssociationValue? Read(MarkdownAssociationTarget target)
        {
            using var key = Registry.CurrentUser.OpenSubKey(target.KeyPath, false);
            if (key is null) return null;
            var value = key.GetValue(target.ValueName, null, RegistryValueOptions.DoNotExpandEnvironmentNames);
            if (value is null && !key.GetValueNames().Contains(target.ValueName, StringComparer.OrdinalIgnoreCase)) return null;
            return new(key.GetValueKind(target.ValueName), value as string);
        }

        public void Write(MarkdownAssociationTarget target, MarkdownAssociationValue? value)
        {
            if (value is null)
            {
                using var key = Registry.CurrentUser.OpenSubKey(target.KeyPath, true);
                key?.DeleteValue(target.ValueName, false);
                return;
            }
            using var writable = Registry.CurrentUser.CreateSubKey(target.KeyPath, true)
                ?? throw new IOException("Markdown registration key could not be opened for writing.");
            writable.SetValue(target.ValueName, value.Text!, value.Kind);
        }
    }
}
