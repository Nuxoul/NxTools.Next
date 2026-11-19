using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class MarkdownAssociationTests
{
    private const string Desktop = @"C:\Program Files\NxTools 中文\NxTools.exe";
    private const string Upgraded = @"D:\Apps\NxTools\NxTools.exe";
    private static readonly MarkdownAssociationTarget OpenCommand = new(@"Software\Classes\NxTools.Markdown\shell\open\command", "");
    private static readonly MarkdownAssociationTarget MdOpenWith = new(@"Software\Classes\.md\OpenWithProgids", "NxTools.Markdown");
    private static readonly MarkdownAssociationTarget MarkdownOpenWith = new(@"Software\Classes\.markdown\OpenWithProgids", "NxTools.Markdown");

    internal static async Task WireAndDetachedReader()
    {
        foreach (var (name, action) in new[] { ("query", MarkdownAssociationAction.Query),
            ("register", MarkdownAssociationAction.Register), ("unregister", MarkdownAssociationAction.Unregister),
            ("open_defaults", MarkdownAssociationAction.OpenDefaults) })
        {
            Check(PlatformCommandParser.ParseMarkdownAssociation(Envelope(JsonSerializer.Serialize(new { action = name }))).Action == action,
                "accept exact action");
        }
        foreach (var data in new[] { "null", "[]", "{}", "[\"query\"]", "{\"action\":0}", "{\"action\":\"set_default\"}",
            "{\"action\":\"Register\"}", "{\"action\":\"query\",\"action\":\"register\"}",
            "{\"action\":\"register\",\"path\":\"foreign.exe\"}",
            "{\"action\":\"open_defaults\",\"uri\":\"ms-settings:other\"}",
            "{\"action\":\"open_defaults\",\"registeredAppUser\":\"Other App\"}" })
        {
            try { PlatformCommandParser.ParseMarkdownAssociation(Envelope(data)); }
            catch (InvalidDataException) { continue; }
            throw new InvalidOperationException("association accepted unexpected wire payload");
        }
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var native = new Fixture { Lookup = _ => { entered.Set(); release.Wait(TimeSpan.FromSeconds(5)); return null; } };
        try
        {
            var accepted = dispatcher.DispatchAsync("markdown_association", () =>
            {
                native.Service.ExecuteBounded(new(MarkdownAssociationAction.Query), Guid.NewGuid(), CancellationToken.None);
                return Task.CompletedTask;
            });
            Check(accepted.IsCompletedSuccessfully && entered.Wait(TimeSpan.FromSeconds(2)), "Shell query must detach before entering the bounded native worker");
            var overlap = native.Service.ExecuteBounded(new(MarkdownAssociationAction.Query), Guid.NewGuid(), CancellationToken.None);
            Check(!overlap.Success && overlap.Data == "native_operation_unconfirmed", "a held native operation retains the only worker slot");
            var later = false;
            await dispatcher.DispatchAsync("later", () => { later = true; return Task.CompletedTask; });
            Check(later && !release.IsSet, "later reader commands remain responsive");
        }
        finally { release.Set(); await dispatcher.DrainAsync(TimeSpan.FromSeconds(5)); }
    }

    internal static Task RegistrationAndEffectiveDefault()
    {
        var f = new Fixture();
        var priorDefault = new MarkdownAssociationTarget(@"Software\Classes\.md", "");
        var foreignOpenWith = new MarkdownAssociationTarget(MdOpenWith.KeyPath, "Other.Markdown");
        var foreignApp = new MarkdownAssociationTarget(@"Software\RegisteredApplications", "Other App");
        f.Values[priorDefault] = Value("Other.Markdown");
        f.Values[foreignOpenWith] = Value("");
        f.Values[foreignApp] = Value(@"Software\Other\Capabilities");
        f.Effective[".md"] = "Other.Markdown";
        CheckState(f.Run(MarkdownAssociationAction.Query), false, false, false, false);
        Check(f.Writes == 0 && f.Notifications == 0 && f.IdentityChecks == 0, "query does not modify registration or require identity");
        CheckState(f.Run(MarkdownAssociationAction.Register), true, false, true, false);
        Check(f.Writes == 9 && f.Notifications == 1 && f.Values[OpenCommand] == Value($"\"{Desktop}\" --markdown-open \"%1\""), "register only exact owned values with quoted Unicode executable and document");
        const string capabilities = @"Software\NxTools\MarkdownReader\Capabilities";
        Check(f.Values.Keys.ToHashSet().SetEquals([priorDefault, foreignOpenWith, foreignApp, OpenCommand, MdOpenWith, MarkdownOpenWith,
            new(@"Software\Classes\NxTools.Markdown", ""), new(capabilities, "ApplicationName"), new(capabilities, "ApplicationDescription"),
            new(capabilities + @"\FileAssociations", ".md"), new(capabilities + @"\FileAssociations", ".markdown"),
            new(@"Software\RegisteredApplications", "NxTools Markdown Reader")]), "no other association values are created");
        Check(f.Values[priorDefault] == Value("Other.Markdown") && f.Values[foreignOpenWith] == Value("") && f.Values[foreignApp] == Value(@"Software\Other\Capabilities"),
            "preserve the existing extension default, other handler, and application");
        CheckState(f.Run(MarkdownAssociationAction.Register), true, false, true, false);
        Check(f.Writes == 9 && f.Notifications == 1, "idempotent registration does not write or notify");
        f.Executable = Upgraded;
        CheckState(f.Run(MarkdownAssociationAction.Register), true, false, true, false);
        Check(f.Writes == 10 && f.Notifications == 2 && f.Values[OpenCommand] == Value($"\"{Upgraded}\" --markdown-open \"%1\""), "registration refresh replaces the old owned desktop path only");
        f.Values.Remove(MdOpenWith);
        CheckState(f.Run(MarkdownAssociationAction.Query), false, false, true, false);
        f.Effective[".md"] = "NxTools.Markdown";
        CheckState(f.Run(MarkdownAssociationAction.Query), false, true, true, false);
        Check(f.Run(MarkdownAssociationAction.Unregister) is { Success: false, Data: "still_default" } && f.Writes == 10,
            "a default is protected even if one owned registration value is missing");
        f.Effective[".md"] = "Other.Markdown";
        f.Effective[".markdown"] = "nxtools.markdown";
        CheckState(f.Run(MarkdownAssociationAction.Query), false, false, true, true);
        Check(f.Run(MarkdownAssociationAction.Unregister) is { Success: false, Data: "still_default" } && f.Writes == 10 && f.Notifications == 2,
            "even a single effective Markdown default blocks all removal");
        f.Effective[".markdown"] = "Other.Markdown";
        f.Executable = null;
        CheckState(f.Run(MarkdownAssociationAction.Unregister), false, false, false, false);
        Check(f.Values.Count == 3 && f.Values.ContainsKey(priorDefault) && f.Values.ContainsKey(foreignOpenWith) && f.Values.ContainsKey(foreignApp) && f.Notifications == 3,
            "removal needs no executable and deletes only owned values after user changes defaults");
        return Task.CompletedTask;
    }

    internal static Task ConflictsCancellationAndFailures()
    {
        var f = new Fixture { Executable = null };
        Check(f.Run(MarkdownAssociationAction.Register) is { Success: false, Data: "identity_unavailable" } && f.Writes == 0,
            "unapproved desktop fails before writes");
        f.Executable = Desktop;
        f.Values[OpenCommand] = Value("\"C:\\Other.exe\" --markdown-open \"%1\"");
        foreach (var action in new[] { MarkdownAssociationAction.Register, MarkdownAssociationAction.Unregister })
            Check(f.Run(action) is { Success: false, Data: "registration_conflict" } && f.Writes == 0, "foreign value in an owned location is preserved");
        f.Values.Clear();
        f.Lookup = _ => throw new COMException("secret path", unchecked((int)0x80004005));
        Check(f.Run(MarkdownAssociationAction.Unregister) is { Success: false, Data: "query_failed", NativeErrorCode: unchecked((int)0x80004005) } && f.Writes == 0,
            "failed Shell lookup must not be mistaken for a non-default");
        f.Lookup = null;
        using var cancel = new CancellationTokenSource();
        cancel.Cancel();
        Check(f.Run(MarkdownAssociationAction.Register, cancel.Token) is { Success: false, Data: "cancelled" } && f.Writes == 0,
            "pre-cancel never mutates registry");
        using var duringWrite = new CancellationTokenSource();
        f.AfterWrite = call => { if (call == 2) duringWrite.Cancel(); };
        Check(f.Run(MarkdownAssociationAction.Register, duringWrite.Token) is { Success: false, Data: "cancelled" } && f.Values.Count == 0 && f.Notifications == 0,
            "mid-transaction cancellation compensates all attempted values without notifying on no change");
        f.Writes = 0;
        f.AfterWrite = call => { if (call == 6) throw new Win32Exception(5, "secret path"); };
        var correlation = Guid.NewGuid();
        var failure = f.Service.Execute(new(MarkdownAssociationAction.Register), correlation, CancellationToken.None);
        Check(failure is { Success: false, Data: "registry_failed", NativeErrorCode: 5 } && !failure.Error!.Contains("secret") &&
            f.Values.Count == 0 && f.Notifications == 0, "write-then-throw rolls back current value too and returns exact native code safely");
        var entries = f.Log.Entries.Where(entry => entry.Correlation == correlation).ToArray();
        Check(entries.Select(entry => entry.Event).SequenceEqual(["markdown.association.accepted", "markdown.association.started", "markdown.association.terminal"]) &&
            entries.All(entry => entry.Fields.ContainsKey("elapsed_ms") && entry.Fields.ContainsKey("native_error_code") &&
                entry.Fields.Values.All(value => value is not string text || !text.Contains("secret"))), "correlated lifecycle contains only safe diagnostic fields");
        f.AfterWrite = null;
        f.Log.Throw = true;
        f.AfterNotify = () => throw new COMException("secret path", unchecked((int)0x80004005));
        CheckState(f.Run(MarkdownAssociationAction.Register), true, false, true, false);
        Check(f.Values.ContainsKey(MarkdownOpenWith) && f.Notifications == 1, "notification or diagnostic failure cannot undo registration or prevent readback");
        var noReadback = new Fixture { Lookup = _ => throw new Win32Exception(5, "secret path") };
        Check(noReadback.Run(MarkdownAssociationAction.Register) is { Success: false, Data: "query_failed", NativeErrorCode: 5 } &&
            noReadback.Writes == 9 && noReadback.Notifications == 1, "successful writes are not reported as observed Windows state when Shell readback fails");
        noReadback.Lookup = null;
        CheckState(noReadback.Run(MarkdownAssociationAction.Query), true, false, true, false);
        return Task.CompletedTask;
    }

    internal static Task OpensRegisteredAppSettings()
    {
        const string generic = "ms-settings:defaultapps";
        const string targeted = generic + "?registeredAppUser=NxTools%20Markdown%20Reader";
        var app = new MarkdownAssociationTarget(@"Software\RegisteredApplications", "NxTools Markdown Reader");
        var f = new Fixture();
        CheckState(f.Run(MarkdownAssociationAction.Register), true, false, true, false);
        f.Lookup = _ => throw new InvalidOperationException("settings launch must not query defaults");
        var command = PlatformCommandParser.ParseMarkdownAssociation(Envelope("{\"action\":\"open_defaults\"}"));
        var correlation = Guid.NewGuid();
        Check(f.Service.ExecuteBounded(command, correlation, CancellationToken.None) is { Success: true, Data: "opened" } &&
            f.SettingsLaunches.Single() is { FileName: targeted, UseShellExecute: true },
            "the native launch boundary gets the escaped HKCU registration name, not the generic Settings URI or ProgID");
        var entries = f.Log.Entries.Where(entry => entry.Correlation == correlation).ToArray();
        Check(entries.Select(entry => entry.Event).SequenceEqual(["markdown.association.accepted", "markdown.association.started",
            "markdown.association.opening_registered_app_settings", "markdown.association.terminal"]) &&
            Equals(entries[^1].Fields["state"], "opened"), "registered-app launch has a correlated terminal distinct from readback");

        f.SupportsRegisteredAppSettings = false;
        Check(f.Run(command.Action) is { Success: true, Data: "opened" } && f.SettingsLaunches[^1].FileName == generic,
            "pre-Windows 11 uses general Default Apps even if registered");
        f.SupportsRegisteredAppSettings = true;
        f.Values.Remove(app);
        Check(f.Run(command.Action) is { Success: true, Data: "opened" } && f.SettingsLaunches[^1].FileName == generic,
            "an absent registration falls back to general Default Apps");
        f.Values[app] = Value(@"Software\Other\Capabilities");
        Check(f.Run(command.Action) is { Success: true, Data: "opened" } && f.SettingsLaunches[^1].FileName == generic,
            "a conflicting registration is not used as the targeted application");
        Check(f.Writes == 9 && f.Notifications == 1 && f.IdentityChecks == 1,
            "opening Settings never registers, writes defaults, notifies, or resolves executable identity");

        using var cancelled = new CancellationTokenSource();
        cancelled.Cancel();
        Check(f.Run(command.Action, cancelled.Token) is { Success: false, Data: "cancelled" } && f.SettingsLaunches.Count == 4,
            "a cancelled settings request does not launch");
        f.Values[app] = Value(@"Software\NxTools\MarkdownReader\Capabilities");
        f.AfterSettingsLaunch = () => throw new Win32Exception(5, "private settings launch message");
        Check(f.Run(command.Action) is { Success: false, Data: "open_defaults_failed", NativeErrorCode: 5 } failure &&
            !failure.Error!.Contains("private") && f.SettingsLaunches.Count == 5 && f.SettingsLaunches[^1].FileName == targeted,
            "a Shell launch failure keeps its native code without leaking its message or reporting a generic retry as success");
        return Task.CompletedTask;
    }

    private static ProtocolEnvelope Envelope(string data) => ProtocolEnvelope.Create("platform_command",
        JsonSerializer.Deserialize<JsonElement>("{\"command\":\"markdown_association\",\"data\":" + data + "}"));
    private static MarkdownAssociationValue Value(string text) => new(RegistryValueKind.String, text);
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }

    private static void CheckState(WindowsCommandResult result, bool mdRegistered, bool mdDefault, bool markdownRegistered, bool markdownDefault)
    {
        Check(result.Success && result.Data is not null, "native success contains observed state");
        using var json = JsonDocument.Parse(result.Data!);
        var md = json.RootElement.GetProperty("md");
        var markdown = json.RootElement.GetProperty("markdown");
        Check(md.GetProperty("registered").GetBoolean() == mdRegistered && md.GetProperty("is_default").GetBoolean() == mdDefault &&
            markdown.GetProperty("registered").GetBoolean() == markdownRegistered && markdown.GetProperty("is_default").GetBoolean() == markdownDefault,
            "per-extension registered and effective-default readback are independent");
    }

    private sealed class Fixture : IMarkdownAssociationStore
    {
        internal readonly Dictionary<MarkdownAssociationTarget, MarkdownAssociationValue> Values = [];
        internal readonly Dictionary<string, string?> Effective = [];
        internal readonly RecordingLog Log = new();
        internal string? Executable = Desktop;
        internal Action<int>? AfterWrite;
        internal Action? AfterNotify;
        internal bool SupportsRegisteredAppSettings = true;
        internal readonly List<ProcessStartInfo> SettingsLaunches = [];
        internal Action? AfterSettingsLaunch;
        internal Func<string, string?>? Lookup;
        internal int Writes;
        internal int Notifications;
        internal int IdentityChecks;
        internal MarkdownAssociationService Service => new(this,
            extension => Lookup is null ? Effective.GetValueOrDefault(extension) : Lookup(extension),
            () => { IdentityChecks++; return Executable; },
            () => { Notifications++; AfterNotify?.Invoke(); }, SupportsRegisteredAppSettings,
            startInfo => { SettingsLaunches.Add(startInfo); AfterSettingsLaunch?.Invoke(); }, Log);
        internal WindowsCommandResult Run(MarkdownAssociationAction action, CancellationToken token = default) => Service.Execute(new(action), Guid.NewGuid(), token);
        public MarkdownAssociationValue? Read(MarkdownAssociationTarget target) => Values.TryGetValue(target, out var value) ? value : null;
        public void Write(MarkdownAssociationTarget target, MarkdownAssociationValue? value)
        {
            if (value is null) Values.Remove(target);
            else Values[target] = value;
            Writes++;
            AfterWrite?.Invoke(Writes);
        }
    }

    private sealed class RecordingLog : IPlatformLog
    {
        internal bool Throw;
        internal readonly List<(string Event, Guid? Correlation, IReadOnlyDictionary<string, object?> Fields)> Entries = [];
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (Throw) throw new IOException();
            lock (Entries) Entries.Add((eventName, correlationId, fields!));
        }
    }
}
