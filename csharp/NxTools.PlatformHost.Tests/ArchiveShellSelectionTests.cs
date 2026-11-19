using System.Runtime.InteropServices;
using System.Security.Cryptography;
using NxTools.PlatformHost.Windows.Archive;
using NxTools.ShellSelectionHelper;

namespace NxTools.PlatformHost.Tests;

// Reviewed effects: in-process COM objects/class factory on an owned STA thread, managed fake
// IShellItemArray/IShellItem, fake desktop launcher and GUID-owned temporary files only.
// No CoRegisterClassObject, Registry, CLSID registration, Explorer, process launch or GUI.
internal static class ArchiveShellSelectionTests
{
    internal static Task QuickCreateTitlesAndCompleteSelectionUseExplorerCommand() => OnSta(() =>
    {
        foreach (var (name, extension) in new[] { ("CompressZip", "zip"), ("CompressSevenZ", "7z") })
        {
            Check(Enum.TryParse<ShellVerb>(name, out var verb), "independent quick-create verb " + name);
            using var lifetime = new ServerLifetime();
            var forwarder = new RecordingForwarder();
            var factory = new ArchiveSelectionClassFactory(verb, forwarder, new RecordingLog(), lifetime);
            var iid = new Guid("a08ce4d0-fa25-44ab-b57c-c7b1c323e0b9");
            Check(factory.CreateInstance(IntPtr.Zero, ref iid, out var pointer) == 0, "official IExplorerCommand title entry");
            var paths = new[] { @"C:\selected\报告.txt", @"C:\selected\second.bin" };
            var array = new FakeArray(paths.Select(path => new FakeItem(path)).ToArray());
            var selection = Marshal.GetComInterfaceForObject(array, typeof(IShellItemArray));
            try
            {
                Check(CallExplorerTitle(pointer, selection, out var title) == 0, "selection title succeeds");
                try { Check(Marshal.PtrToStringUni(title) == $"压缩为“报告.{extension}”", "actual selection filename and fixed format"); }
                finally { Marshal.FreeCoTaskMem(title); }
                foreach (var (filename, attributes) in new[] { ("folder.with.dot", 0x20000000u), (".gitignore", 0u) })
                {
                    var query = new FakeArray([new FakeItem(@"C:\selected\" + filename) { Attributes = attributes }]);
                    var queryPointer = Marshal.GetComInterfaceForObject(query, typeof(IShellItemArray));
                    try
                    {
                        Check(CallExplorerTitle(pointer, queryPointer, out var queryTitle) == 0, "current query selection title");
                        try { Check(Marshal.PtrToStringUni(queryTitle) == $"压缩为“{filename}.{extension}”", "folder and dotfile names match original Rust draft"); }
                        finally { Marshal.FreeCoTaskMem(queryTitle); }
                    }
                    finally { Marshal.Release(queryPointer); }
                }
                Check(forwarder.Calls.Count == 0 && lifetime.LiveObjects == 1, "title query never executes or retires invocation");
                Check(CallExplorerInvoke(pointer, selection) == 0, "same original command executes");
                Check(forwarder.Calls.Single().Verb == verb && forwarder.Calls.Single().Paths.SequenceEqual(paths), "one complete ordered selection");
                Check(CallExplorerInvoke(pointer, selection) == ArchiveSelectionCommand.EIllegalMethodCall, "once-only invocation");
                Check(lifetime.LiveObjects == 0, "original lifetime retired");
            }
            finally { Marshal.Release(selection); Marshal.Release(pointer); }
            Check(factory.CreateInstance(IntPtr.Zero, ref iid, out var abandoned) == 0, "title-only object admitted");
            Marshal.Release(abandoned);
            Check(lifetime.LiveObjects == 0, "title-only final release frees original lifetime slot");
            foreach (var selected in new[] { new[] { @"C:\selected\报告.txt", @"C:\selected\second.bin" }, new[] { @"C:\selected\folder" } })
            {
                var executeIid = typeof(IExecuteCommand).GUID;
                Check(factory.CreateInstance(IntPtr.Zero, ref executeIid, out var executePointer) == 0, "classic DelegateExecute identity");
                var selectionIid = typeof(IObjectWithSelection).GUID;
                var selectedArray = Marshal.GetComInterfaceForObject(new FakeArray(selected.Select(path => new FakeItem(path)).ToArray()), typeof(IShellItemArray));
                try
                {
                    Check(Marshal.QueryInterface(executePointer, in selectionIid, out var selectedPointer) == 0, "delegate retains original selection interface");
                    try { Check(CallSetSelection(selectedPointer, selectedArray) == 0, "delegate accepts the complete selection"); }
                    finally { Marshal.Release(selectedPointer); }
                    var calls = forwarder.Calls.Count;
                    Check(CallExecute(executePointer) == 0 && forwarder.Calls.Count == calls + 1, "delegate forwards once");
                    Check(forwarder.Calls[^1].Verb == verb && forwarder.Calls[^1].Paths.SequenceEqual(selected), "ZIP/7z complete ordered file or folder selection");
                    Check(CallExecute(executePointer) == ArchiveSelectionCommand.EIllegalMethodCall && lifetime.LiveObjects == 0, "delegate cannot execute twice and retires original lifetime");
                }
                finally { Marshal.Release(selectedArray); Marshal.Release(executePointer); }
            }
        }
    });

    internal static Task QuickCreateRecommendationsAndPlans()
    {
        Check(ArchiveIntegrationIntent.RecommendedMenuCommands.TakeLast(3).SequenceEqual(["create_zip", "create_7z", "create"]), "three creation recommendations");
        foreach (var layout in new[] { "cascade", "flat" })
        {
            var plan = ArchiveSelectionHandlerPlan.Create(@"C:\NxTools\NxTools.ShellSelectionHelper.exe", 2,
                new ArchiveIntegrationIntent(true, false, []) { MenuLayout = layout, MenuCommands = ["create_zip", "create_7z", "create"] });
            Check(plan.Changes.All(change => change.Target.ValueName != "ExplorerCommandHandler"), "classic local-server commands never require in-process Explorer activation");
            Check(plan.Changes.Count(change => change.Target.ValueName == "DelegateExecute") == 6, "all three source commands use grouped delegates for files and folders");
            Check(plan.Changes.Count(change => change.Target.KeyPath.EndsWith(@"\LocalServer32")) == 3, "out-of-process helper classes");
            foreach (var command in ArchiveSelectionHandlerPlan.Commands.Where(command => ArchiveSelectionHandlerPlan.IsCreation(command.Command)))
            {
                var expected = command.Command switch { "create_zip" => "压缩为同名 .zip", "create_7z" => "压缩为同名 .7z", _ => "新建压缩包…" };
                Check(command.Label == expected, "fixed classic menu label " + command.Command);
                Check(plan.Changes.Count(change => change.Target.ValueName == "MUIVerb" && Equals(change.Value.Data, expected)) == 2, "same label on file and folder menus");
            }
            Check(plan.Changes.All(change => change.Value.Data is not string value || !value.Contains("%1")), "no per-item invocation");
            var icon = "\"C:\\Program Files\\NxTools\\NxTools.exe\",-2";
            var withIcons = ArchiveSelectionHandlerPlan.Create(@"C:\Program Files\NxTools\NxTools.ShellSelectionHelper.exe", 2,
                new ArchiveIntegrationIntent(true, false, [".zip"]) { MenuLayout = layout });
            var menus = withIcons.Changes.Where(change => change.Target.ValueName == "MUIVerb" &&
                (layout == "flat" || Equals(change.Value.Data, "NxTools"))).ToArray();
            Check(menus.Length == (layout == "cascade" ? 3 : 11), "archive, file and directory menu coverage");
            foreach (var menu in menus)
                Check(withIcons.Changes.Any(change => change.Target.KeyPath == menu.Target.KeyPath &&
                    change.Target.ValueName == "Icon" && Equals(change.Value.Data, icon)),
                    "every visible menu root has the packaged desktop archive icon");
            Check(withIcons.Changes.Where(change => change.Target.ValueName == "Icon").Count() == menus.Length,
                "cascade children do not repeat the root icon");
            var store = new ArchiveIntegrationPlanTests.FakeArchiveIntegrationStore();
            var facts = ArchiveIntegrationTransaction.Apply(store, withIcons);
            Check(facts.Values.Count(value => value.Target.ValueName == "Icon") == menus.Length,
                "icon writes share original applied-facts ownership");
            ArchiveIntegrationTransaction.Uninstall(store, facts);
            Check(store.SnapshotValues().Count == 0, "original exact-value uninstall includes menu icons");
        }
        return Task.CompletedTask;
    }

    private static unsafe int CallExplorerTitle(IntPtr instance, IntPtr selection, out IntPtr title)
    {
        IntPtr result = IntPtr.Zero;
        var status = ((delegate* unmanaged[Stdcall]<IntPtr, IntPtr, IntPtr*, int>)Marshal.ReadIntPtr(Marshal.ReadIntPtr(instance), 3 * IntPtr.Size))(instance, selection, &result);
        title = result;
        return status;
    }
    private static unsafe int CallExplorerInvoke(IntPtr instance, IntPtr selection) =>
        ((delegate* unmanaged[Stdcall]<IntPtr, IntPtr, IntPtr, int>)Marshal.ReadIntPtr(Marshal.ReadIntPtr(instance), 8 * IntPtr.Size))(instance, selection, IntPtr.Zero);

    internal static Task OneSelectionForwardsOnceInOrderThroughComVtable() => OnSta(() =>
    {
        var forwarder = new RecordingForwarder();
        var log = new RecordingLog();
        using var lifetime = new ServerLifetime();
        var factory = new ArchiveSelectionClassFactory(ShellVerb.ExtractSmart, forwarder, log, lifetime);
        var paths = new[] { @"C:\b\second.zip", @"C:\a\first.7z", @"\\server\share\third.rar" };
        var executeIid = typeof(IExecuteCommand).GUID;
        Check(factory.CreateInstance(IntPtr.Zero, ref executeIid, out var pointer) == 0 && pointer != IntPtr.Zero, "class factory creates IExecuteCommand");
        Check(lifetime.LiveObjects == 1, "live object accounted");
        var selectionIid = typeof(IObjectWithSelection).GUID;
        var array = new FakeArray(paths.Select(path => new FakeItem(path)).ToArray());
        var arrayPointer = Marshal.GetComInterfaceForObject(array, typeof(IShellItemArray));
        try
        {
            // Exact SDK vtable slots: IUnknown 0..2, IObjectWithSelection.SetSelection 3,
            // IExecuteCommand.SetKeyState 3 .. Execute 9.
            Check(Marshal.QueryInterface(pointer, in selectionIid, out var selectionPointer) == 0, "same object exposes IObjectWithSelection");
            try
            {
                Func<IntPtr, IntPtr, int> setSelection = CallSetSelection;
                Func<IntPtr, int> execute = CallExecute;
                Check(setSelection(selectionPointer, arrayPointer) == 0, "SetSelection retains the array");
                Check(execute(pointer) == 0, "Execute forwards the complete selection");
                Check(forwarder.Calls.Count == 1, "exactly one forward per selection");
                Check(forwarder.Calls[0].Verb == ShellVerb.ExtractSmart && forwarder.Calls[0].Paths.SequenceEqual(paths), "ordered unmodified paths");
                Check(execute(pointer) == ArchiveSelectionCommand.EIllegalMethodCall && forwarder.Calls.Count == 1, "second Execute never forwards again");
                Check(setSelection(selectionPointer, arrayPointer) == ArchiveSelectionCommand.EIllegalMethodCall, "selection is frozen after Execute");
                Check(lifetime.LiveObjects == 0, "retired after the single terminal");
                Check(array.Reads == paths.Length, "array read exactly once per item");
                Check(log.Entries.All(entry => entry.Fields.Values.All(value => value is not string text || !text.Contains('\\'))), "diagnostics carry no paths");
                Check(log.Entries.Any(entry => entry.Event == "shell_selection.terminal" && Equals(entry.Fields["count"], 3)), "terminal carries bounded count");
            }
            finally
            {
                Marshal.Release(selectionPointer);
            }
        }
        finally
        {
            Marshal.Release(arrayPointer);
            Marshal.Release(pointer);
        }        var badIid = Guid.NewGuid();
        Check(factory.CreateInstance(IntPtr.Zero, ref badIid, out _) == ArchiveSelectionCommand.ENoInterface, "unknown interface rejected");
        Check(factory.CreateInstance(new IntPtr(1), ref executeIid, out _) == ArchiveSelectionClassFactory.ClassENoAggregation, "aggregation rejected");
        Check(lifetime.LiveObjects == 0, "rejected creations do not leak lifetime");
    });

    internal static Task PartialFailuresRejectWholeSelectionWithoutDrops() => OnSta(() =>
    {
        var cases = new (string Name, IShellItemArray? Array, SelectionFailure Expected)[]
        {
            ("null", null, SelectionFailure.NoSelection),
            ("empty", new FakeArray([]), SelectionFailure.Empty),
            ("count", new FakeArray([new FakeItem(@"C:\a.zip")]) { CountResult = unchecked((int)0x80070005) }, SelectionFailure.CountFailed),
            ("item", new FakeArray([new FakeItem(@"C:\a.zip"), new FakeItem(@"C:\b.zip")]) { FailItemAt = 1 }, SelectionFailure.ItemFailed),
            ("virtual", new FakeArray([new FakeItem(@"C:\a.zip"), new FakeItem(null)]), SelectionFailure.NotFileSystem),
            ("device", new FakeArray([new FakeItem(@"\\.\pipe\x")]), SelectionFailure.InvalidPath),
            ("relative", new FakeArray([new FakeItem(@"relative.zip")]), SelectionFailure.InvalidPath),
            ("too many", new FakeArray(Enumerable.Range(0, ShellSelectionReader.MaxPaths + 1).Select(i => new FakeItem($@"C:\{i}.zip")).ToArray()), SelectionFailure.TooMany),
            ("long", new FakeArray([new FakeItem(@"C:\" + new string('a', ShellSelectionReader.MaxPathBytes))]), SelectionFailure.PathTooLong),
            ("bytes", new FakeArray(Enumerable.Range(0, 10).Select(i => new FakeItem($@"C:\{i}\" + new string('b', 30_000))).ToArray()), SelectionFailure.TooManyBytes),
        };
        foreach (var (name, array, expected) in cases)
        {
            var forwarder = new RecordingForwarder();
            using var lifetime = new ServerLifetime();
            Check(lifetime.TryAddObject(), "object slot");
            var command = new ArchiveSelectionCommand(ShellVerb.Compress, forwarder, new RecordingLog(), lifetime.Cancellation, lifetime.ReleaseObject);
            Check(((IObjectWithSelection)command).SetSelection(array) == 0, name + " retained");
            Check(((IExecuteCommand)command).Execute() < 0, name + " is an explicit failure");
            Check(command.Failure == expected, $"{name}: {command.Failure} == {expected}");
            Check(forwarder.Calls.Count == 0, name + " never forwards a partial selection");
            Check(lifetime.LiveObjects == 0, name + " retires");
        }
        var exact = new FakeArray(Enumerable.Range(0, ShellSelectionReader.MaxPaths).Select(i => new FakeItem($@"C:\{i}.zip")).ToArray());
        var sink = new RecordingForwarder();
        using var bounded = new ServerLifetime();
        Check(bounded.TryAddObject(), "bounded slot");
        var maxCommand = new ArchiveSelectionCommand(ShellVerb.Compress, sink, new RecordingLog(), bounded.Cancellation, bounded.ReleaseObject);
        ((IObjectWithSelection)maxCommand).SetSelection(exact);
        Check(((IExecuteCommand)maxCommand).Execute() == 0 && sink.Calls.Single().Paths.Count == ShellSelectionReader.MaxPaths, "exact 256 bound accepted whole");
        foreach (var verb in new[] { ShellVerb.Open, ShellVerb.Test })
        {
            var arity = new RecordingForwarder();
            var single = new ArchiveSelectionCommand(verb, arity, new RecordingLog(), CancellationToken.None, () => { });
            ((IObjectWithSelection)single).SetSelection(new FakeArray([new FakeItem(@"C:\a.zip"), new FakeItem(@"C:\b.zip")]));
            Check(((IExecuteCommand)single).Execute() < 0 && arity.Calls.Count == 0, verb + " multi-selection rejected, not truncated");
        }
    });

    internal static Task IdentityQueueCancellationAndLifetimeFailures() => OnSta(() =>
    {
        var directory = Path.Combine(Path.GetTempPath(), "nxtools-shell-helper-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        try
        {
            var desktop = Path.Combine(directory, "NxTools.exe");
            File.WriteAllBytes(desktop, [1, 2, 3, 4]);
            var hash = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(desktop)));
            var launcher = new FakeLauncher { ExitCode = 0 };
            var paths = new[] { @"C:\x.zip", @"C:\y.zip" };

            Check(new DesktopIntentForwarder(new DesktopIdentity(directory, ""), launcher)
                .Forward(Guid.NewGuid(), ShellVerb.ExtractHere, paths, CancellationToken.None).Status == ForwardStatus.IdentityRejected,
                "unpackaged empty release identity refuses delivery");
            Check(new DesktopIntentForwarder(new DesktopIdentity(directory, new string('0', 64)), launcher)
                .Forward(Guid.NewGuid(), ShellVerb.ExtractHere, paths, CancellationToken.None).Status == ForwardStatus.IdentityRejected,
                "hash mismatch refuses delivery");
            Check(launcher.Calls.Count == 0, "rejected identity never launches");

            var forwarder = new DesktopIntentForwarder(new DesktopIdentity(directory, hash), launcher);
            var delivered = forwarder.Forward(Guid.NewGuid(), ShellVerb.ExtractHere, paths, CancellationToken.None);
            Check(delivered.Status == ForwardStatus.Delivered && launcher.Calls.Count == 1, "approved identity forwards once");
            Check(string.Equals(launcher.Calls[0].Executable, desktop, StringComparison.OrdinalIgnoreCase), "exact sibling desktop");
            Check(launcher.Calls[0].Arguments.SequenceEqual(["--archive-extract", "here", @"C:\x.zip", @"C:\y.zip"]), "existing ingress vocabulary, ordered argv paths");
            foreach (var verb in new[] { ShellVerb.CompressZip, ShellVerb.CompressSevenZ })
            {
                var quickLauncher = new FakeLauncher { ExitCode = 0 };
                var correlationId = Guid.NewGuid();
                Check(new DesktopIntentForwarder(new DesktopIdentity(directory, hash), quickLauncher)
                    .Forward(correlationId, verb, paths, CancellationToken.None).Status == ForwardStatus.Delivered, "quick handoff accepted");
                Check(quickLauncher.Calls.Single().Arguments.SequenceEqual(new[] { "--archive-correlation-id", correlationId.ToString("D") }
                    .Concat(HelperIdentity.IntentArguments(verb)).Concat(paths)), "one correlation and complete argv selection");
            }

            foreach (var (exit, status) in new (int?, ForwardStatus)[] { (1, ForwardStatus.Unavailable), (2, ForwardStatus.Rejected), (null, ForwardStatus.HandedToNewOwner) })
            {
                var typed = new DesktopIntentForwarder(new DesktopIdentity(directory, hash), new FakeLauncher { ExitCode = exit })
                    .Forward(Guid.NewGuid(), ShellVerb.Compress, paths, CancellationToken.None);
                Check(typed.Status == status, $"exit {exit} maps to {status}");
            }
            Check(new DesktopIntentForwarder(new DesktopIdentity(directory, hash), new FakeLauncher { Throw = new System.ComponentModel.Win32Exception(2) })
                .Forward(Guid.NewGuid(), ShellVerb.Compress, paths, CancellationToken.None).Status == ForwardStatus.Unavailable, "launch failure is typed");

            using var cancelled = new CancellationTokenSource();
            cancelled.Cancel();
            var cancelledLauncher = new FakeLauncher { ExitCode = 0 };
            Check(new DesktopIntentForwarder(new DesktopIdentity(directory, hash), cancelledLauncher)
                .Forward(Guid.NewGuid(), ShellVerb.Compress, paths, cancelled.Token).Status == ForwardStatus.Cancelled && cancelledLauncher.Calls.Count == 0,
                "cancelled before launch never forwards");

            using var lifetime = new ServerLifetime();
            var factory = new ArchiveSelectionClassFactory(ShellVerb.Compress, new RecordingForwarder(), new RecordingLog(), lifetime);
            var iid = typeof(IObjectWithSelection).GUID;
            var created = new List<IntPtr>();
            for (var index = 0; index < ServerLifetime.MaxLiveObjects; index++)
            {
                Check(factory.CreateInstance(IntPtr.Zero, ref iid, out var pointer) == 0, "bounded creation");
                created.Add(pointer);
            }
            Check(factory.CreateInstance(IntPtr.Zero, ref iid, out _) == ArchiveSelectionCommand.EFail, "live-object queue bound enforced");
            Check(factory.LockServer(true) == 0 && !lifetime.Idle.WaitOne(0), "LockServer keeps server alive");
            foreach (var pointer in created)
            {
                var executeIid = typeof(IExecuteCommand).GUID;
                Check(Marshal.QueryInterface(pointer, in executeIid, out var executePointer) == 0, "same object exposes IExecuteCommand");
                Check(CallExecute(executePointer) < 0, "never-selected command fails explicitly");
                Marshal.Release(executePointer);
                Marshal.Release(pointer);
            }
            Check(lifetime.LiveObjects == 0 && !lifetime.Idle.WaitOne(0), "lock still held after objects retire");
            factory.LockServer(false);
            Check(lifetime.Idle.WaitOne(0), "idle only after objects and locks reach zero");
            lifetime.Shutdown();
            Check(factory.CreateInstance(IntPtr.Zero, ref iid, out _) == ArchiveSelectionCommand.EFail, "shutdown refuses new objects");
            var late = new RecordingForwarder();
            var lateCommand = new ArchiveSelectionCommand(ShellVerb.Compress, late, new RecordingLog(), lifetime.Cancellation, () => { });
            ((IObjectWithSelection)lateCommand).SetSelection(new FakeArray([new FakeItem(@"C:\late.zip")]));
            Check(((IExecuteCommand)lateCommand).Execute() < 0 && late.Calls.Count == 0 && lateCommand.Result?.Status == ForwardStatus.Cancelled,
                "shutdown cancellation stops forwarding");
        }
        finally
        {
            Directory.Delete(directory, true);
        }

        Check(NxTools.ShellSelectionHelper.Program.IsComActivation(["-Embedding"]) && NxTools.ShellSelectionHelper.Program.IsComActivation(["/Embedding"]), "COM activation contract");
        Check(!NxTools.ShellSelectionHelper.Program.IsComActivation([]) && !NxTools.ShellSelectionHelper.Program.IsComActivation(["-Embedding", @"C:\x"]), "no other entry");
        Check(NxTools.ShellSelectionHelper.Program.ExpectedDesktopSha256 == NxTools.PlatformHost.Generated.ReleaseBinaryManifest.DesktopSha256,
            "helper and Host must share the same Desktop build identity");
    });

    // F-AC01: a Shell-created command that is finally Released without Execute must free its slot.
    internal static Task FinalReleaseWithoutExecuteRetiresSlotAndReadmits() => OnSta(() =>
    {
        using var lifetime = new ServerLifetime();
        var forwarder = new RecordingForwarder();
        var factory = new ArchiveSelectionClassFactory(ShellVerb.Compress, forwarder, new RecordingLog(), lifetime);
        var iid = typeof(IExecuteCommand).GUID;
        // Round A: the caller never forces GC; the real CreateInstance admission boundary must readmit.
        for (var index = 0; index < ServerLifetime.MaxLiveObjects; index++)
        {
            Check(factory.CreateInstance(IntPtr.Zero, ref iid, out var pointer) == 0, "bounded creation");
            Marshal.Release(pointer); // final external COM reference, no Execute/SetSelection
        }
        Check(factory.CreateInstance(IntPtr.Zero, ref iid, out var ninth) == 0, $"ninth creation readmitted after final Releases (live={lifetime.LiveObjects})");
        Marshal.Release(ninth);
        // Round B: after final Release and collection every unexecuted object is retired and the server idles.
        for (var index = 1; index < ServerLifetime.MaxLiveObjects; index++)
        {
            Check(factory.CreateInstance(IntPtr.Zero, ref iid, out var pointer) == 0, "bounded creation");
            Marshal.Release(pointer);
        }
        for (var attempt = 0; attempt < 20 && lifetime.LiveObjects > 0; attempt++)
        {
            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();
        }
        Check(lifetime.LiveObjects == 0, $"final Release retires every unexecuted object (live={lifetime.LiveObjects})");
        Check(lifetime.Idle.WaitOne(0), "server becomes idle when all objects are released");
        Check(forwarder.Calls.Count == 0, "unexecuted objects never forward");

        // Executed objects retire exactly once: no double decrement when their final Release follows.
        var held = new List<IntPtr>();
        Check(factory.CreateInstance(IntPtr.Zero, ref iid, out var executed) == 0, "executed creation");
        for (var index = 1; index < ServerLifetime.MaxLiveObjects; index++)
        {
            Check(factory.CreateInstance(IntPtr.Zero, ref iid, out var live) == 0, "live slot");
            held.Add(live);
        }
        Check(CallExecute(executed) < 0 && lifetime.LiveObjects == ServerLifetime.MaxLiveObjects - 1, "Execute retires once");
        Marshal.Release(executed);
        for (var attempt = 0; attempt < 5; attempt++)
        {
            GC.Collect();
            GC.WaitForPendingFinalizers();
        }
        Check(lifetime.LiveObjects == ServerLifetime.MaxLiveObjects - 1, "final Release after Execute does not decrement again");
        Check(factory.CreateInstance(IntPtr.Zero, ref iid, out var refill) == 0, "freed executed slot readmits");
        Check(factory.CreateInstance(IntPtr.Zero, ref iid, out _) == ArchiveSelectionCommand.EFail, "8 live interfaces still bound");
        Check(CallExecute(held[0]) < 0, "held interfaces stay valid after other objects retire");
        foreach (var pointer in held) Marshal.Release(pointer);
        Marshal.Release(refill);
    });

    // F-AC02: a held LockServer(true) keeps the pump alive past the initial idle bound.
    internal static Task LockServerKeepsPumpAliveUntilUnlocked() => OnSta(() =>
    {
        using var locked = new ServerLifetime();
        locked.Lock();
        var unlocker = new Thread(() =>
        {
            Thread.Sleep(900);
            locked.Unlock();
        }) { IsBackground = true };
        var clock = System.Diagnostics.Stopwatch.StartNew();
        unlocker.Start();
        LocalServer.PumpUntilIdle(locked, TimeSpan.FromMilliseconds(250), TimeSpan.FromSeconds(10));
        var elapsed = clock.ElapsedMilliseconds;
        unlocker.Join();
        Check(elapsed >= 850, $"pump honoured the held server lock past initial idle (elapsed {elapsed}ms)");
        Check(elapsed < 5000, $"pump exits promptly once objects and locks reach zero (elapsed {elapsed}ms)");

        using var fresh = new ServerLifetime();
        clock.Restart();
        LocalServer.PumpUntilIdle(fresh, TimeSpan.FromMilliseconds(200), TimeSpan.FromSeconds(10));
        Check(clock.ElapsedMilliseconds < 3000, "never-activated server still exits at the initial idle bound");

        using var stuck = new ServerLifetime();
        stuck.Lock();
        clock.Restart();
        LocalServer.PumpUntilIdle(stuck, TimeSpan.FromMilliseconds(100), TimeSpan.FromMilliseconds(700));
        Check(clock.ElapsedMilliseconds < 4000, "explicit hard lifetime still bounds a held lock");
        stuck.Unlock();
    });
    internal static Task FutureRegistrationPlanMatchesHelperAndStaysUnapplied()
    {
        var helperClasses = HelperIdentity.VerbClasses;
        foreach (var (verb, clsid, _) in ArchiveSelectionHandlerPlan.Verbs)
        {
            var expected = verb switch
            {
                "open" => ShellVerb.Open,
                "extract-smart" => ShellVerb.ExtractSmart,
                "extract-here" => ShellVerb.ExtractHere,
                "extract-name" => ShellVerb.ExtractName,
                _ => ShellVerb.Test,
            };
            Check(helperClasses[expected] == clsid, verb + " CLSID equals helper registration");
        }
        Check(ArchiveSelectionHandlerPlan.HelperExecutableName == HelperIdentity.ExecutableName, "helper executable identity");
        var helper = @"C:\Program Files\NxTools\NxTools.ShellSelectionHelper.exe";
        var plan = ArchiveSelectionHandlerPlan.Create(helper, 3, [".zip", ".7Z"]);
        Check(plan.AppliedFacts is null && plan.Scope == ArchiveIntegrationScope.CurrentUserClasses, "pure unapplied HKCU plan");
        Check(plan.Changes.All(change => change.Target.KeyPath.StartsWith(@"Software\Classes\", StringComparison.OrdinalIgnoreCase)
            && !change.Target.KeyPath.Contains("UserChoice", StringComparison.OrdinalIgnoreCase)), "current-user Classes only");
        Check(plan.Changes.Count(change => change.Target.KeyPath.EndsWith(@"\LocalServer32", StringComparison.OrdinalIgnoreCase)
            && (string)change.Value.Data == "\"" + helper + "\"") == ArchiveSelectionHandlerPlan.Verbs.Count, "quoted exact helper LocalServer32");
        Check(plan.Changes.Count(change => change.Target.ValueName == "DelegateExecute") == 2 * ArchiveSelectionHandlerPlan.Verbs.Count, "DelegateExecute per extension and verb");
        Check(plan.Changes.All(change => change.Value.Data is not string text || !text.Contains("%1", StringComparison.Ordinal)), "grouped verbs never use per-file %1");
        foreach (var change in plan.Changes) WindowsArchiveIntegrationStore.Validate(change.Target);
        foreach (var invalid in new[] { "relative.exe", @"C:\Other\NxTools.exe", "C:\\x\"\\NxTools.ShellSelectionHelper.exe" })
        {
            try { ArchiveSelectionHandlerPlan.Create(invalid, 1, [".zip"]); throw new InvalidOperationException("accepted " + invalid); }
            catch (ArgumentException) { }
        }
        var store = new ArchiveIntegrationPlanTests.FakeArchiveIntegrationStore();
        var facts = ArchiveIntegrationTransaction.Apply(store, plan);
        Check(ArchiveIntegrationTransaction.Uninstall(store, facts).RemovedInstalledValues == plan.Changes.Count
            && store.SnapshotValues().Count == 0, "future plan reuses exact compensation/uninstall on a fake store");
        return Task.CompletedTask;
    }

    // Exact SDK vtable slots called as real unmanaged stdcall function pointers.
    private static unsafe int CallSetSelection(IntPtr instance, IntPtr selection) =>
        ((delegate* unmanaged[Stdcall]<IntPtr, IntPtr, int>)Marshal.ReadIntPtr(Marshal.ReadIntPtr(instance), 3 * IntPtr.Size))(instance, selection);

    private static unsafe int CallExecute(IntPtr instance) =>
        ((delegate* unmanaged[Stdcall]<IntPtr, int>)Marshal.ReadIntPtr(Marshal.ReadIntPtr(instance), 9 * IntPtr.Size))(instance);

    private static Task OnSta(Action body)
    {
        Exception? failure = null;
        var thread = new Thread(() => { try { body(); } catch (Exception error) { failure = error; } }) { IsBackground = true };
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        if (!thread.Join(TimeSpan.FromSeconds(30))) throw new TimeoutException("STA fixture did not finish.");
        if (failure is not null) throw new InvalidOperationException(failure.Message, failure);
        return Task.CompletedTask;
    }

    private static void Check(bool value, string detail)
    {
        if (!value) throw new InvalidOperationException(detail);
    }

    [ComVisible(true), ClassInterface(ClassInterfaceType.None)]
    private sealed class FakeItem(string? path) : IShellItem
    {
        internal uint Attributes { get; init; }
        public int BindToHandler(IntPtr bindContext, ref Guid handler, ref Guid iid, out IntPtr result) { result = IntPtr.Zero; return ArchiveSelectionCommand.EFail; }
        public int GetParent(out IShellItem parent) { parent = null!; return ArchiveSelectionCommand.EFail; }
        public int GetDisplayName(uint sigdnName, out IntPtr name)
        {
            name = IntPtr.Zero;
            if (sigdnName != ShellSelectionReader.SigdnFileSysPath) return ArchiveSelectionCommand.EFail;
            if (path is null) return unchecked((int)0x80070057);
            name = Marshal.StringToCoTaskMemUni(path);
            return 0;
        }
        public int GetAttributes(uint mask, out uint attributes) { attributes = Attributes & mask; return 0; }
        public int Compare(IShellItem other, uint hint, out int order) { order = 0; return 0; }
    }

    [ComVisible(true), ClassInterface(ClassInterfaceType.None)]
    private sealed class FakeArray(FakeItem[] items) : IShellItemArray
    {
        internal int CountResult { get; init; }
        internal int FailItemAt { get; init; } = -1;
        internal int Reads { get; private set; }
        public int BindToHandler(IntPtr bindContext, ref Guid handler, ref Guid iid, out IntPtr result) { result = IntPtr.Zero; return ArchiveSelectionCommand.EFail; }
        public int GetPropertyStore(int flags, ref Guid iid, out IntPtr result) { result = IntPtr.Zero; return ArchiveSelectionCommand.EFail; }
        public int GetPropertyDescriptionList(IntPtr keyType, ref Guid iid, out IntPtr result) { result = IntPtr.Zero; return ArchiveSelectionCommand.EFail; }
        public int GetAttributes(int attributeFlags, uint mask, out uint attributes) { attributes = 0; return 0; }
        public int GetCount(out uint count) { count = (uint)items.Length; return CountResult; }
        public int GetItemAt(uint index, out IShellItem item)
        {
            Reads++;
            item = null!;
            if (index == FailItemAt) return unchecked((int)0x80004005);
            item = items[index];
            return 0;
        }
        public int EnumItems(out IntPtr enumerator) { enumerator = IntPtr.Zero; return ArchiveSelectionCommand.EFail; }
    }

    private sealed class RecordingForwarder : ISelectionForwarder
    {
        internal List<(ShellVerb Verb, IReadOnlyList<string> Paths)> Calls { get; } = [];
        public ForwardResult Forward(Guid correlationId, ShellVerb verb, IReadOnlyList<string> paths, CancellationToken cancellation)
        {
            Calls.Add((verb, paths.ToArray()));
            return new(ForwardStatus.Delivered, 0);
        }
    }

    private sealed class RecordingLog : IHelperLog
    {
        internal List<(string Event, IReadOnlyDictionary<string, object?> Fields)> Entries { get; } = [];
        public void Write(string eventName, Guid correlationId, IReadOnlyDictionary<string, object?> fields) =>
            Entries.Add((eventName, fields));
    }

    private sealed class FakeLauncher : IDesktopLauncher
    {
        internal int? ExitCode { get; init; }
        internal Exception? Throw { get; init; }
        internal List<(string Executable, IReadOnlyList<string> Arguments)> Calls { get; } = [];
        public int? Run(string executable, IReadOnlyList<string> arguments, TimeSpan timeout, CancellationToken cancellation)
        {
            Calls.Add((executable, arguments.ToArray()));
            if (Throw is not null) throw Throw;
            return ExitCode;
        }
    }
}
