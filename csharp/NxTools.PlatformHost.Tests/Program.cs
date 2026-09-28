using System.Threading.Channels;
using System.Text;
using System.Text.Json;
using System.Runtime.InteropServices;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Threading;
using System.Windows.Forms;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class Program
{
    public static async Task<int> Main(string[] args)
    {
        var tests = new (string Name, Func<Task> Run)[]
        {
            ("envelope uses protocol v1 wire names", EnvelopeUsesWireNames),
            ("command result preserves native error code", CommandResultPreservesNativeErrorCode),
            ("parser exceptions produce correlated failure results", ParserExceptionsProduceCorrelatedFailureResults),
            ("detached sync commands schedule before invocation", DetachedSyncCommandsScheduleBeforeInvocation),
            ("full clipboard event queues clean staging and report once", FullClipboardEventQueueCleansStagingAndReportsOnce),
            ("empty clipboard payloads are ignored even with native formats", EmptyClipboardPayloadsAreIgnored),
            ("fallback sequence recheck prevents emptying changed clipboard", FallbackSequenceRecheckPreventsEmptyingChangedClipboard),
            ("platform command diagnostics prefer the operation correlation",
                PlatformCommandDiagnosticsPreferOperationCorrelation),
            ("paste command preserves text and target window", PasteCommandPreservesPayload),
            ("image and files preserve capture and paste payloads", MultiFormatPayloadsRoundTrip),
            ("native clipboard format command preserves staged payload", NativeFormatPayloadRoundTrips),
            ("clipboard configuration preserves hotkey, exclusions and capture limits", ClipboardConfigurationRoundTrips),
            ("clipboard capture limits clamp sizes without payload data", ClipboardCaptureLimitsClampAndReject),
            ("quick-launch configuration and panel request preserve Windows context", QuickLaunchContractsRoundTrip),
            ("auto quick switch only consumes an Explorer source after a dialog is known", AutoQuickSwitchRequiresKnownDialog),
            ("auto quick switch invalidates stale asynchronous candidates", AutoQuickSwitchInvalidatesStaleCandidates),
            ("auto quick switch accepts supported third-party directory sources", AutoQuickSwitchAcceptsThirdPartySources),
            ("fixed file-dialog search retains directory sources without automatic switching", FileDialogSearchRetainsSourcesWithoutAutoSwitch),
            ("file-dialog initialization retries preserve the pending Explorer source", FileDialogInitializationRetriesPreserveExplorerSource),
            ("file-dialog recognition survives a transient foreground handoff", FileDialogRecognitionSurvivesTransientForegroundHandoff),
            ("file-dialog child creation immediately re-arms pending recognition", FileDialogStructureChangeRearmsPendingRecognition),
            ("file-dialog movement ignores children and coalesces root geometry", FileDialogMovementCoalescesRootGeometry),
            ("visible file-dialog foreground repeats reuse the recognized host", VisibleFileDialogForegroundRepeatsReuseRecognizedHost),
            ("elevated directory helper request preserves its opaque rendezvous", ElevatedDirectoryHelperRequestRoundTrips),
            ("deferred directory capture expires with its summon budget", DeferredDirectoryCaptureExpires),
            ("Explorer navigation keeps the captured host window", ExplorerNavigationKeepsCapturedWindow),
            ("desktop navigation recognizes only the Shell desktop and opens the target", DesktopNavigationUsesShellDesktop),
            ("desktop identity requires Shell structure and Explorer ownership", DesktopIdentityRequiresShellEvidence),
            ("file-dialog navigation keeps the captured host window", FileDialogNavigationKeepsCapturedWindow),
            ("third-party file manager navigation keeps the captured host and active pane", ThirdPartyFileManagerNavigationKeepsCapturedHost),
            ("Directory Opus adapter requires the real lister and preserves the active pane", DirectoryOpusAdapterPreservesActivePane),
            ("Directory Opus frame focus keeps captured pane navigable", DirectoryOpusFrameFocusKeepsCapturedPaneNavigable),
            ("Directory Opus pane selection uses title and geometry before cached pane", DirectoryOpusPaneSelectionUsesReferenceEvidence),
            ("Directory source recognition failure clears stale source", DirectorySourceRecognitionFailureClearsStaleSource),
            ("Directory source recognition survives return to a known file dialog", DirectorySourceRecognitionSurvivesReturnToKnownFileDialog),
            ("pending directory source recognition is bounded and superseded safely", PendingDirectorySourceRecognitionIsBoundedAndSuperseded),
            ("Directory Opus localized display paths canonicalize to parsing paths", DirectoryOpusDisplayPathsCanonicalize),
            ("known folder display aliases cover the desktop folder", KnownFolderDisplayAliasesCoverDesktop),
            ("Total Commander adapter uses the captured source panel", TotalCommanderAdapterPreservesSourcePanel),
            ("OneCommander adapter freezes its UI Automation pane before the popup opens", OneCommanderAdapterPreservesAutomationPane),
            ("Files identity matches the Listary process and UIA root contract", FilesIdentityMatchesListaryContract),
            ("Files adapter requires its real window and confirms navigation on the captured host", FilesAdapterPreservesCapturedWindow),
            ("Files navigation succeeds only after its path control confirms the target", FilesNavigationRequiresReadback),
            ("bounded STA operations cancel before a stalled provider can perform late work", BoundedStaOperationsCancelLateWork),
            ("canceled STA dialog drops its late result", CanceledStaDialogDropsLateResult),
            ("canceled detached drain does not block reconnect", CanceledDetachedDrainDoesNotBlockReconnect),
            ("detached cancellation observes late faults", DetachedCancellationObservesLateFaults),
            ("canceled Shell worker is not reused from cache", CanceledShellWorkerIsNotReusedFromCache),
            ("elevated directory contexts route only through the same-integrity channel", ElevatedDirectoryContextsUseSameIntegrityChannel),
            ("elevated helper sessions preserve and consume the captured local context", ElevatedHelperSessionsPreserveCapturedContext),
            ("elevated capture preserves summon-time focus instead of delayed focus", ElevatedCapturePreservesSummonTimeFocus),
            ("elevated helper rendezvous accepts only bounded random names and tokens", ElevatedHelperRendezvousIsBounded),
            ("elevated helper diagnostics keep host and helper PIDs distinct", ElevatedHelperDiagnosticsKeepPidsDistinct),
            ("real Directory Opus lister exposes a third-party navigation context", RealDirectoryOpusListerExposesNavigationContext),
            ("real Files window exposes a third-party navigation context", RealFilesWindowExposesNavigationContext),
            ("file-dialog recognition rejects unrelated #32770 windows", FileDialogRecognitionUsesStructuralMarkers),
            ("file-dialog navigation writes a directory and restores its edit focus", FileDialogNavigationUsesCapturedControls),
            ("modern file-dialog navigation uses its address edit without replacing the file name", ModernFileDialogNavigationUsesAddressEdit),
            ("file-dialog search submits a selected file through the captured dialog", FileDialogSearchSubmitsSelectedFile),
            ("file-dialog hotkey capture normalizes paths without filesystem access", FileDialogPathCaptureAvoidsFilesystemValidation),
            ("real Explorer Shell window resolves its exact HWND and self-navigates", RealExplorerShellWindowResolvesExactHwnd),
            ("double-control gesture requires two released taps inside the activation window", DoubleControlGestureRequiresTwoReleasedTaps),
            ("double-control gesture rejects key repeat and intervening keys", DoubleControlGestureRejectsRepeatAndInterveningKeys),
            ("hotkey recording releases and restores the registered shortcut", HotKeyRecordingReleasesAndRestoresShortcut),
            ("clipboard hotkey uses 350ms debounce", ClipboardHotKeyUses350MsDebounce),
            ("clipboard exclusions match process names safely", ClipboardExclusionsMatchProcessNames),
            ("topmost command preserves native window", TopmostCommandPreservesNativeWindow),
            ("minimized command shows a hidden native window minimized without focus", MinimizedCommandUsesRealNativeWindow),
            ("no-activate command preserves native window", NoActivateCommandPreservesNativeWindow),
            ("window-bounds command preserves physical placement", WindowBoundsCommandPreservesPlacement),
            ("centered rectangle preserves the anchor visual center", CenteredRectanglePreservesAnchorCenter),
            ("reconnect failure logging is rate limited", ReconnectFailureLoggingIsRateLimited),
            ("taskbar progress command preserves native window and aggregate progress", TaskbarProgressCommandPreservesPayload),
            ("taskbar overlay placement preserves anchor and logical offsets", TaskbarOverlayPlacementPreservesPayload),
            ("taskbar overlay visibility preserves its native target", TaskbarOverlayVisibilityPreservesPayload),
            ("taskbar overlay edit finish preserves its native target", TaskbarOverlayEditFinishPreservesPayload),
            ("taskbar overlay bounds scale and clamp inside the taskbar", TaskbarOverlayBoundsScaleAndClamp),
            ("taskbar overlay applies passive styles and a real hide-show lifecycle", TaskbarOverlayUsesPassiveNativeLifecycle),
            ("taskbar progress activates a supported shell interface on STA", TaskbarProgressUsesSupportedShellInterface),
            ("taskbar progress diagnostics log state and milestones without flooding", TaskbarProgressDiagnosticsAreBounded),
            ("high-frequency taskbar updates skip generic command lifecycle logs", TaskbarProgressSkipsGenericCommandLogs),
            ("native file drag command preserves the completed artifact path", FileDragCommandPreservesPayload),
            ("native file drag drops on release and cancels on escape", NativeFileDragTracksPointerState),
            ("native file drag input queues attach for the OLE scope and detach once", NativeFileDragInputQueuesUseOneLease),
            ("native file drag runs on a dedicated STA worker", NativeFileDragRunsOnDedicatedStaWorker),
            ("native file drag diagnostics preserve correlation and state fields", NativeFileDragDiagnosticsPreserveCorrelation),
            ("anchored window placement centers visible content against its live owner", AnchoredWindowPlacementCentersVisibleContent),
            ("anchored window follow reuses calibrated frame insets", AnchoredWindowFollowReusesCalibratedInsets),
            ("file-dialog search host preserves physical placement and visibility", FileDialogSearchHostPreservesPlacement),
            ("visibility command preserves native window", VisibilityCommandPreservesNativeWindow),
            ("desktop shell and tray contracts preserve bounded intent", DesktopShellAndTrayContractsRoundTrip),
            ("startup registration and tray actions use real Windows boundaries", DesktopShellUsesRealWindowsBoundaries),
            ("startup registration resolves the current desktop release path", StartupRegistrationResolvesCurrentDesktopPath),
            ("desktop tray rejects a missing product executable icon", DesktopTrayRequiresProductIcon),
            ("tray factory failure disposes partially created resources", TrayFactoryFailureDisposesPartialResources),
            ("titlebar move reaches the real native window boundary", TitlebarMoveUsesRealNativeWindow),
            ("titlebar maximize toggle reaches the real native window boundary", TitlebarMaximizeToggleUsesRealNativeWindow),
            ("window position, resize and shell commands preserve payload", WindowPositionResizeAndShellCommandsPreservePayload),
            ("completion sound and shutdown use the typed Windows adapter", CompletionActionsUseTypedWindowsAdapter),
            ("typed process launch preserves arguments and Windows flags", TypedProcessLaunchPreservesArgumentsAndFlags),
            ("shell action session commands preserve payload", ShellActionSessionCommandsPreservePayload),
            ("shell action session enumerates a real filesystem target", ShellActionSessionEnumeratesFilesystemTarget),
            ("shell action bitmap is staged as a GPUI image", ShellActionBitmapIsStaged),
            ("shell icon extraction command preserves target and size", ShellIconCommandPreservesPayload),
            ("shell icon extraction runs on a dedicated STA worker", ShellIconExtractionRunsOnDedicatedStaWorker),
            ("shell icon extraction allows bounded parallel STA workers", ShellIconExtractionAllowsBoundedParallelWorkers),
            ("icon cache reuses identical content keys", IconCacheReusesIdenticalKeys),
            ("icon cache separates size and selector keys", IconCacheSeparatesSizeAndSelectorKeys),
            ("icon cache bounds the memory LRU", IconCacheBoundsMemoryLru),
            ("icon cache regenerates a missing disk file", IconCacheRegeneratesMissingDiskFile),
            ("icon cache deduplicates concurrent generation", IconCacheDeduplicatesConcurrentGeneration),
            ("icon cache generation does not hold the metadata lock", IconCacheGenerationDoesNotHoldMetadataLock),
            ("missing files fall back to their Windows file type icon", MissingFileUsesTypeIcon),
            ("shortcut icon resolves to its target without the link overlay", ShortcutIconResolvesWithoutOverlay),
            ("shortcut extraction preserves distinct application icons", ShortcutExtractionPreservesApplicationIcons),
            ("shortcut navigation resolves the real target directory", ShortcutNavigationResolvesRealTargetDirectory),
            ("clipboard archive picker preserves mode and suggested name", ClipboardArchivePickerPreservesPayload),
            ("clipboard storage picker preserves current directory", ClipboardStoragePickerPreservesPayload),
            ("generic directory picker preserves title and current directory", DirectoryPickerPreservesPayload),
            ("program picker preserves current path", ProgramPickerPreservesPayload),
            ("shell target policy accepts registry and safe tools only", ShellTargetPolicyAcceptsSafeQuickTargets),
            ("Windows adapter does not own the GPUI window shape", WindowChromeUsesNativeContracts),
            ("single image path is promoted to image capture", SingleImagePathIsPromoted),
            ("pure image capture requires PNG and file path compatibility fallbacks", PureImageCaptureRequiresCompatibilityFallbacks),
            ("file drop fallback uses a valid Unicode DROPFILES payload", FileDropFallbackUsesUnicodeDropFilesPayload),
            ("pure image native formats gain PNG and file path fallbacks", PureImageFormatsGainBothFallbacks),
            ("history image exposes PNG, bitmap and file path formats", HistoryImageExposesCompatibilityFormats),
            ("rich image target can omit duplicate file drop format", RichImageTargetOmitsFileDropFormat),
            ("image path fallback cleanup is bounded and protects the current file", ImagePathFallbackCleanupIsBounded),
            ("paste retries transient target activation", PasteRetriesTransientTargetActivation),
            ("copy-only writes clipboard without activating target", CopyOnlyWritesWithoutActivatingTarget),
            ("four clipboard actions reach the expected Windows outcome", FourActionsReachExpectedWindowsOutcome),
            ("native input layout matches Windows ABI", NativeInputLayoutMatchesWindowsAbi),
            ("paste diagnostics preserve correlation and Win32 error", PasteDiagnosticsPreserveCorrelationAndWin32Error),
            ("platform log writes structured JSON lines", PlatformLogWritesStructuredJsonLines),
            ("process exception diagnostics preserve evidence without payload text", ProcessExceptionDiagnosticsPreserveEvidence),
            ("unverified fallback is not reported as paste success", UnverifiedFallbackIsNotReportedAsPasteSuccess),
            ("pressed modifiers keep paste result unconfirmed", PressedModifiersKeepPasteResultUnconfirmed),
            ("incomplete input keeps paste result unconfirmed", IncompleteInputKeepsPasteResultUnconfirmed),
            ("paste result correlates to command", PasteResultCorrelatesToCommand),
            ("clipboard event preserves source context", ClipboardEventPreservesSourceContext),
            ("global pointer event preserves clicked window", GlobalPointerEventPreservesClickedWindow),
            ("panel request preserves target and cursor", PanelRequestPreservesTargetAndCursor),
            ("hotkey capability reflects registration", HotKeyCapabilityReflectsRegistration),
            ("bounded reader preserves adjacent frames", ReaderPreservesFrames),
            ("bounded reader rejects oversized frames", ReaderRejectsOversizedFrame),
            ("native file drag does not block later platform commands", FileDragDoesNotBlockLaterCommands),
            ("shell enumeration does not block later platform commands", ShellEnumerationDoesNotBlockLaterCommands),
            ("shell target launch does not block later platform commands", ShellTargetLaunchDoesNotBlockLaterCommands),
            ("directory navigation does not block later platform commands", DirectoryNavigationDoesNotBlockLaterCommands),
            ("browser extension export command parses format and destination", BrowserExtensionExportCommandParses),
            ("browser extension packages produce crx and xpi artifacts", BrowserExtensionPackagesProduceArtifacts),
            ("normal desktop pipe closure is not logged as an error", NormalPipeClosureIsNotAnError),
            ("indexer service command carries no caller-controlled path", IndexerServiceCommandHasNoPath),
            ("browser native messaging command preserves fixed extension identities", BrowserNativeMessagingCommandPreservesIdentities),
            ("indexer service installer only accepts packaged sibling", IndexerServiceInstallerAcceptsOnlyPackagedSibling),
            ("protected indexer service root stays under ProgramData", ProtectedIndexerServiceRootStaysUnderProgramData),
            ("indexer service update stops only exact protected executable paths", IndexerServiceUpdateStopsOnlyExactProtectedPaths),
            ("indexer service registration requires service arguments", IndexerServiceRegistrationRequiresServiceArguments),
            ("error dialog arguments parse the packaged startup report", ErrorDialogArgumentsParseReportPath),
            ("error dialog reads a bounded startup failure report", ErrorDialogReadsBoundedReport)
        };

        var selectedTests = args.Length == 0
            ? tests
            : tests.Where(test => args.Any(filter =>
                test.Name.Contains(filter, StringComparison.OrdinalIgnoreCase))).ToArray();
        foreach (var test in selectedTests)
        {
            try
            {
                // WinForms-installing tests (tray icon, context menus) leave a
                // WindowsFormsSynchronizationContext on this thread; a console
                // runner has no message pump, so the first later async test
                // that suspends posts its continuation into that dead context
                // and the suite freezes. Every test starts ambient-free.
                SynchronizationContext.SetSynchronizationContext(null);
                Console.WriteLine($"RUN {test.Name}");
                await test.Run();
                Console.WriteLine($"PASS {test.Name}");
            }
            catch (Exception error)
            {
                Console.Error.WriteLine($"FAIL {test.Name}: {error.Message}");
                return 1;
            }
        }

        return 0;
    }

    private sealed class FakeCompletionSystemActions : ICompletionSystemActions
    {
        public int SoundRequests { get; private set; }
        public ushort? ShutdownDelaySeconds { get; private set; }

        public WindowsCommandResult PlayNotificationSound()
        {
            SoundRequests++;
            return new WindowsCommandResult(true);
        }

        public WindowsCommandResult RequestShutdown(ushort delaySeconds)
        {
            ShutdownDelaySeconds = delaySeconds;
            return new WindowsCommandResult(true);
        }
    }

    private static Task CompletionActionsUseTypedWindowsAdapter()
    {
        var soundEnvelope = ProtocolEnvelope.Create(
            "platform_command",
            new { Command = "play_system_notification_sound" });
        Assert(PlatformCommandParser.GetCommandName(soundEnvelope) == "play_system_notification_sound");
        var shutdownEnvelope = ProtocolEnvelope.Create(
            "platform_command",
            new { Command = "shutdown_system", Data = new { DelaySeconds = 1 } });
        var shutdown = PlatformCommandParser.ParseShutdownSystem(shutdownEnvelope);
        Assert(shutdown.DelaySeconds == 1);

        var actions = new FakeCompletionSystemActions();
        using var adapter = new WindowsPlatformAdapter(new ForegroundContextProvider(), completionSystemActions: actions);
        Assert(adapter.PlaySystemNotificationSound().Success);
        Assert(adapter.ShutdownSystem(shutdown).Success);
        Assert(actions.SoundRequests == 1);
        Assert(actions.ShutdownDelaySeconds == 1);
        return Task.CompletedTask;
    }

    private static Task EnvelopeUsesWireNames()
    {
        var envelope = ProtocolEnvelope.Create("hello", new
        {
            ClientName = "platform_host",
            ClientVersion = "0.1.0",
            Capabilities = new[] { "foreground_context" }
        });
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);

        Assert(json.Contains("\"protocol_version\":1", StringComparison.Ordinal));
        Assert(json.Contains("\"type\":\"hello\"", StringComparison.Ordinal));
        Assert(json.Contains("\"client_name\":\"platform_host\"", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task CommandResultPreservesNativeErrorCode()
    {
        var request = ProtocolEnvelope.Create("platform_command", new { Command = "pick_program" });
        var response = PlatformPipeClient.CreateCommandResultEnvelope(
            request,
            new WindowsCommandResult(
                false,
                "native picker failure",
                NativeErrorCode: 5));

        Assert(response.CorrelationId == request.Id);
        Assert(response.Payload.GetProperty("success").GetBoolean() == false);
        Assert(response.Payload.GetProperty("native_error_code").GetInt32() == 5);
        return Task.CompletedTask;
    }
    private static Task ParserExceptionsProduceCorrelatedFailureResults()
    {
        var request = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "pick_program"
        });
        Exception? parserError = null;
        try
        {
            _ = PlatformCommandParser.ParsePickProgram(request);
        }
        catch (Exception error)
        {
            parserError = error;
        }
        Assert(parserError is not null);

        var response = PlatformPipeClient.CreateCommandFailureEnvelope(request, parserError!);
        Assert(response.CorrelationId == request.Id);
        Assert(!response.Payload.GetProperty("success").GetBoolean());
        Assert(!string.IsNullOrWhiteSpace(response.Payload.GetProperty("error").GetString()));
        return Task.CompletedTask;
    }

    private static Task PlatformCommandDiagnosticsPreferOperationCorrelation()
    {
        var operationCorrelationId = Guid.NewGuid();
        foreach (var commandName in new[]
        {
            "open_shell_target",
            "export_browser_extension",
            "query_shell_actions",
        })
        {
            var request = ProtocolEnvelope.Create(
                "platform_command",
                new { Command = commandName },
                operationCorrelationId);
            Assert(PlatformPipeClient.ResolveDiagnosticCorrelationId(request) == operationCorrelationId);
            var failure = PlatformPipeClient.CreateCommandFailureEnvelope(
                request,
                new ExternalException("synthetic native failure", 5));
            Assert(failure.CorrelationId == request.Id);
            Assert(!failure.Payload.GetProperty("success").GetBoolean());
            Assert(failure.Payload.GetProperty("native_error_code").GetInt32() == 5);
        }

        var uncorrelated = ProtocolEnvelope.Create(
            "platform_command",
            new { Command = "paste_text" });
        Assert(PlatformPipeClient.ResolveDiagnosticCorrelationId(uncorrelated) == uncorrelated.Id);
        return Task.CompletedTask;
    }

    private static Task TaskbarProgressCommandPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_taskbar_progress",
            Data = new
            {
                WindowHandle = 781L,
                State = "normal",
                Completed = 42UL,
                Total = 100UL
            }
        });
        var command = PlatformCommandParser.ParseSetTaskbarProgress(envelope);
        Assert(command.WindowHandle == 781L);
        Assert(command.State == "normal");
        Assert(command.Completed == 42UL);
        Assert(command.Total == 100UL);
        return Task.CompletedTask;
    }

    private static Task TaskbarOverlayPlacementPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "place_taskbar_progress_overlay",
            Data = new
            {
                WindowHandle = 782L,
                Enabled = true,
                Visible = false,
                Anchor = "right",
                OffsetX = 12,
                OffsetY = 0,
                Width = 132,
                Height = 8,
                Interactive = true
            }
        });
        var command = PlatformCommandParser.ParsePlaceTaskbarProgressOverlay(envelope);
        Assert(command.WindowHandle == 782L);
        Assert(command.Enabled);
        Assert(!command.Visible);
        Assert(command.Anchor == "right");
        Assert(command.OffsetX == 12);
        Assert(command.OffsetY == 0);
        Assert(command.Width == 132);
        Assert(command.Height == 8);
        Assert(command.Interactive);
        return Task.CompletedTask;
    }

    private static Task TaskbarOverlayBoundsScaleAndClamp()
    {
        var taskbar = new TaskbarOverlayBounds(0, 1040, 1920, 1080);
        var right = TaskbarOverlayPlacement.CalculateBounds(
            taskbar,
            dpi: 144,
            anchor: "right",
            offsetX: 12,
            offsetY: 0,
            width: 132,
            height: 8);
        Assert(right == new TaskbarOverlayBounds(1704, 1040, 1902, 1052));

        var exactLogicalOffset = TaskbarOverlayPlacement.CalculateBounds(
            taskbar,
            dpi: 96,
            anchor: "left",
            offsetX: 10,
            offsetY: 10,
            width: 132,
            height: 8);
        Assert(exactLogicalOffset == new TaskbarOverlayBounds(10, 1050, 142, 1058));

        var clamped = TaskbarOverlayPlacement.CalculateBounds(
            taskbar,
            dpi: 96,
            anchor: "left",
            offsetX: -200,
            offsetY: 100,
            width: 132,
            height: 8);
        Assert(clamped == new TaskbarOverlayBounds(0, 1072, 132, 1080));
        return Task.CompletedTask;
    }

    private static Task TaskbarOverlayVisibilityPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_taskbar_progress_overlay_visibility",
            Data = new
            {
                WindowHandle = 782L,
                Visible = true
            }
        });
        var command = PlatformCommandParser.ParseSetTaskbarProgressOverlayVisibility(envelope);
        Assert(command.WindowHandle == 782L);
        Assert(command.Visible);
        return Task.CompletedTask;
    }

    private static Task TaskbarOverlayEditFinishPreservesPayload()
    {
        var envelope = PlatformEventEnvelopeFactory.Create(
            new TaskbarOverlayEditGestureFinishedEvent(782L));
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);
        Assert(json.Contains(
            "\"event\":\"taskbar_overlay_edit_gesture_finished\"",
            StringComparison.Ordinal));
        Assert(json.Contains("\"window_handle\":782", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task TaskbarOverlayUsesPassiveNativeLifecycle()
    {
        Exception? failure = null;
        var thread = new Thread(() =>
        {
            try
            {
                using var overlay = new Form
                {
                    ShowInTaskbar = false,
                    FormBorderStyle = FormBorderStyle.None
                };
                _ = overlay.Handle;
                var placed = TaskbarOverlayPlacement.Place(
                    new PlaceTaskbarProgressOverlayCommand(
                        overlay.Handle.ToInt64(),
                        Enabled: true,
                        Visible: false,
                        Anchor: "right",
                        OffsetX: 12,
                        OffsetY: 0,
                        Width: 132,
                        Height: 8,
                        Interactive: false));
                Assert(placed.Success);
                Assert(TaskbarOverlayPlacement.HasPassiveOverlayStyles(overlay.Handle));
                var editPlaced = TaskbarOverlayPlacement.Place(
                    new PlaceTaskbarProgressOverlayCommand(
                        overlay.Handle.ToInt64(),
                        Enabled: true,
                        Visible: false,
                        Anchor: "right",
                        OffsetX: 12,
                        OffsetY: 0,
                        Width: 180,
                        Height: 24,
                        Interactive: true));
                Assert(editPlaced.Success);
                Assert(TaskbarOverlayPlacement.HasInteractiveOverlayStyles(overlay.Handle));

                using var visibility = new Form
                {
                    ShowInTaskbar = false,
                    FormBorderStyle = FormBorderStyle.None,
                    StartPosition = FormStartPosition.Manual,
                    Bounds = new System.Drawing.Rectangle(-32_000, -32_000, 1, 1)
                };
                _ = visibility.Handle;
                Assert(TaskbarOverlayPlacement.SetVisibility(visibility.Handle, true).Success);
                Assert(TaskbarOverlayPlacement.SetVisibility(visibility.Handle, false).Success);
            }
            catch (Exception error)
            {
                failure = error;
            }
        })
        {
            IsBackground = true,
            Name = "NxTools.TaskbarOverlayContract"
        };
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        Assert(thread.Join(TimeSpan.FromSeconds(3)));
        Assert(failure is null);
        return Task.CompletedTask;
    }

    private static Task TaskbarProgressUsesSupportedShellInterface()
    {
        WindowsCommandResult? result = null;
        Exception? failure = null;
        var thread = new Thread(() =>
        {
            try
            {
                using var window = new Form();
                _ = window.Handle;
                result = TaskbarProgress.Set(
                    window.Handle,
                    "none",
                    0,
                    0);
            }
            catch (Exception error)
            {
                failure = error;
            }
        })
        {
            IsBackground = true,
            Name = "NxTools.TaskbarProgressContract"
        };
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        Assert(thread.Join(TimeSpan.FromSeconds(3)));
        Assert(failure is null);
        Assert(result?.Success == true);
        return Task.CompletedTask;
    }

    private static Task TaskbarProgressDiagnosticsAreBounded()
    {
        var gate = new TaskbarProgressLogGate();
        var success = new WindowsCommandResult(true);
        var failure = new WindowsCommandResult(false, "shell unavailable");
        var initial = gate.Evaluate(781, "normal", 0, 100, success, 0);
        Assert(initial?.EventName == "taskbar_progress.applied");
        Assert(initial?.PercentMilestone == 0);
        Assert(gate.Evaluate(781, "normal", 10, 100, success, 10) is null);
        var quarter = gate.Evaluate(781, "normal", 25, 100, success, 20);
        Assert(quarter?.PercentMilestone == 25);
        Assert(gate.Evaluate(781, "paused", 25, 100, success, 30) is not null);

        Assert(gate.Evaluate(781, "paused", 25, 100, failure, 1_000) is not null);
        Assert(gate.Evaluate(781, "paused", 25, 100, failure, 2_000) is null);
        Assert(gate.Evaluate(781, "paused", 25, 100, failure, 31_001) is not null);
        return Task.CompletedTask;
    }

    private static Task TaskbarProgressSkipsGenericCommandLogs()
    {
        Assert(!PlatformPipeClient.ShouldLogCommandLifecycle("set_taskbar_progress"));
        Assert(PlatformPipeClient.ShouldLogCommandLifecycle("open_shell_target"));
        Assert(PlatformPipeClient.ShouldLogCommandLifecycle("begin_file_drag"));
        return Task.CompletedTask;
    }

    private static Task FileDragCommandPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "begin_file_drag",
            Data = new
            {
                Path = @"C:\Downloads\artifact.zip",
                SourceWindowHandle = 781L
            }
        });
        var command = PlatformCommandParser.ParseBeginFileDrag(envelope);
        Assert(command.Path == @"C:\Downloads\artifact.zip");
        Assert(command.SourceWindowHandle == 781L);
        return Task.CompletedTask;
    }

    private static Task NativeFileDragTracksPointerState()
    {
        Assert(NativeFileDragService.QueryContinueDrag(false, 0x0001, true) == 0);
        Assert(NativeFileDragService.QueryContinueDrag(false, 0, true) == 0);
        Assert(NativeFileDragService.QueryContinueDrag(false, 0x0001, false) == 0x00040100);
        Assert(NativeFileDragService.QueryContinueDrag(true, 0x0001, true) == 0x00040101);
        return Task.CompletedTask;
    }

    private static Task NativeFileDragInputQueuesUseOneLease()
    {
        var calls = new List<(uint Current, uint Source, bool Attach)>();
        var detached = new List<bool>();
        Assert(NativeFileDragService.TryAttachInputQueues(
            14100,
            16900,
            (current, source, attach) =>
            {
                calls.Add((current, source, attach));
                return true;
            },
            out var lease,
            detached.Add));
        Assert(lease is not null);
        Assert(calls.SequenceEqual(
        [
            (14100U, 16900U, true)
        ]));

        lease!.Dispose();
        lease.Dispose();
        Assert(calls.SequenceEqual(
        [
            (14100U, 16900U, true),
            (14100U, 16900U, false)
        ]));
        Assert(detached.SequenceEqual([true]));

        calls.Clear();
        Assert(NativeFileDragService.TryAttachInputQueues(
            14100,
            14100,
            (current, source, attach) =>
            {
                calls.Add((current, source, attach));
                return true;
            },
            out lease));
        Assert(lease is null);
        Assert(calls.Count == 0);
        return Task.CompletedTask;
    }

    private static async Task NativeFileDragRunsOnDedicatedStaWorker()
    {
        var callerThreadId = Environment.CurrentManagedThreadId;
        using var releaseDrag = new ManualResetEventSlim();
        var workerStarted = new TaskCompletionSource<(int ThreadId, ApartmentState Apartment)>(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var coordinator = new NativeFileDragCoordinator(
            NullPlatformLog.Instance,
            (_, _, _) =>
            {
                workerStarted.TrySetResult((
                    Environment.CurrentManagedThreadId,
                    Thread.CurrentThread.GetApartmentState()));
                releaseDrag.Wait();
                return new WindowsCommandResult(true);
            });

        var completion = coordinator.BeginAsync(
            new BeginFileDragCommand(@"C:\Downloads\completed.bin", 782),
            Guid.NewGuid(),
            CancellationToken.None);
        var worker = await workerStarted.Task.WaitAsync(TimeSpan.FromSeconds(1));
        try
        {
            Assert(worker.ThreadId != callerThreadId);
            Assert(worker.Apartment == ApartmentState.STA);
            Assert(!completion.IsCompleted);
        }
        finally
        {
            releaseDrag.Set();
        }
        Assert((await completion.WaitAsync(TimeSpan.FromSeconds(1))).Success);
    }

    private static async Task NativeFileDragDiagnosticsPreserveCorrelation()
    {
        var log = new RecordingPlatformLog();
        using var releaseDrag = new ManualResetEventSlim();
        var diagnosticReported = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var coordinator = new NativeFileDragCoordinator(
            log,
            (_, _, reportDiagnostic) =>
            {
                reportDiagnostic(
                    "query_continue_drag",
                    new Dictionary<string, object?>
                    {
                        ["callback_count"] = 1,
                        ["result_name"] = "continue"
                    });
                diagnosticReported.TrySetResult();
                releaseDrag.Wait();
                return new WindowsCommandResult(true);
            });
        var correlationId = Guid.NewGuid();
        var completion = coordinator.BeginAsync(
            new BeginFileDragCommand(@"C:\Downloads\completed.bin", 783),
            correlationId,
            CancellationToken.None);
        await diagnosticReported.Task.WaitAsync(TimeSpan.FromSeconds(1));
        try
        {
            var worker = log.Entries.Single(entry =>
                Equals(entry.Fields["stage"], "worker_started"));
            var callback = log.Entries.Single(entry =>
                Equals(entry.Fields["stage"], "query_continue_drag"));
            Assert(worker.CorrelationId == correlationId);
            Assert(callback.CorrelationId == correlationId);
            Assert(!callback.Fields.ContainsKey("probe"));
            Assert(!callback.Fields.ContainsKey("file_name"));
            Assert(Equals(callback.Fields["source_window_handle"], 783L));
            Assert(Equals(callback.Fields["callback_count"], 1));
            Assert(Equals(callback.Fields["result_name"], "continue"));
        }
        finally
        {
            releaseDrag.Set();
        }
        Assert((await completion.WaitAsync(TimeSpan.FromSeconds(1))).Success);
    }

    private static Task IndexerServiceCommandHasNoPath()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "ensure_indexer_service",
            Data = new { }
        });
        PlatformCommandParser.ParseEnsureIndexerService(envelope);
        Assert(!JsonSerializer.Serialize(envelope, JsonDefaults.Options)
            .Contains("indexer_path", StringComparison.OrdinalIgnoreCase));
        return Task.CompletedTask;
    }

    private static Task BrowserNativeMessagingCommandPreservesIdentities()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "ensure_browser_native_messaging",
            Data = new
            {
                Enabled = true,
                ChromiumExtensionId = "haohhnpjfggpdadhcbjakjpkmkljkpif",
                FirefoxExtensionId = "nxtools-download-interceptor@nxtools.local"
            }
        });
        var command = PlatformCommandParser.ParseEnsureBrowserNativeMessaging(envelope);
        Assert(command.Enabled);
        Assert(command.ChromiumExtensionId == "haohhnpjfggpdadhcbjakjpkmkljkpif");
        Assert(command.FirefoxExtensionId == "nxtools-download-interceptor@nxtools.local");
        return Task.CompletedTask;
    }

    private static Task IndexerServiceInstallerAcceptsOnlyPackagedSibling()
    {
        var package = Path.Combine(Path.GetTempPath(), $"nxtools-service-test-{Guid.NewGuid():N}");
        Directory.CreateDirectory(package);
        try
        {
            var host = Path.Combine(package, "NxTools.PlatformHost.exe");
            var indexer = Path.Combine(package, "NxTools.Indexer.exe");
            var desktop = Path.Combine(package, "NxTools.exe");
            var browserHost = Path.Combine(package, "NxTools.BrowserHost.exe");
            File.WriteAllBytes(host, []);
            File.WriteAllBytes(indexer, []);
            File.WriteAllBytes(desktop, []);
            File.WriteAllBytes(browserHost, []);
            Assert(IndexerServiceInstaller.ValidatePackagedIndexerPath(host, indexer) == indexer);
            Assert(IndexerServiceInstaller.ValidatePackagedSibling(
                host,
                "NxTools.exe") == desktop);
            Assert(IndexerServiceInstaller.ValidatePackagedSibling(
                host,
                "NxTools.BrowserHost.exe") == browserHost);

            var outside = Path.Combine(Path.GetTempPath(), "NxTools.Indexer.exe");
            try
            {
                File.WriteAllBytes(outside, []);
                _ = IndexerServiceInstaller.ValidatePackagedIndexerPath(host, outside);
                throw new InvalidOperationException("installer accepted an indexer outside the package");
            }
            catch (InvalidDataException)
            {
            }
            finally
            {
                File.Delete(outside);
            }
        }
        finally
        {
            Directory.Delete(package, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task IndexerServiceRegistrationRequiresServiceArguments()
    {
        const string indexer = @"C:\Program Files\NxTools\NxTools.Indexer.exe";
        Assert(IndexerServiceInstaller.RegisteredImagePathMatches(
            $"\"{indexer}\" --service",
            indexer));
        Assert(!IndexerServiceInstaller.RegisteredImagePathMatches($"\"{indexer}\"", indexer));
        Assert(!IndexerServiceInstaller.RegisteredImagePathMatches(
            $"\"{indexer}\" --console",
            indexer));

        var valid = new IndexerServiceInstaller.IndexerServiceRegistration(
            $"\"{indexer}\" --service",
            "LocalSystem",
            2,
            "D:(A;;CCLCSWRPWPLORC;;;AU)");
        Assert(IndexerServiceInstaller.RegistrationMatches(valid, indexer));
        Assert(!IndexerServiceInstaller.RegistrationMatches(
            valid with { ObjectName = @"NT AUTHORITY\LocalService" },
            indexer));
        Assert(!IndexerServiceInstaller.RegistrationMatches(
            valid with { StartMode = 3 },
            indexer));
        Assert(!IndexerServiceInstaller.RegistrationMatches(
            valid with { SecurityDescriptor = "D:(A;;GA;;;SY)" },
            indexer));
        Assert(IndexerServiceInstaller.RegistrationVerb(serviceExists: true) == "config");
        Assert(IndexerServiceInstaller.RegistrationVerb(serviceExists: false) == "create");
        var binary = Path.GetTempFileName();
        try
        {
            File.WriteAllText(binary, "current indexer build");
            var hash = IndexerServiceInstaller.ComputeBinaryHash(binary);
            Assert(IndexerServiceInstaller.BinaryHashMatches(hash, binary));
            Assert(!IndexerServiceInstaller.BinaryHashMatches(new string('0', 64), binary));
        }
        finally
        {
            File.Delete(binary);
        }
        return Task.CompletedTask;
    }

    private static Task ProtectedIndexerServiceRootStaysUnderProgramData()
    {
        var programData = Path.GetFullPath(
            Environment.GetFolderPath(
                Environment.SpecialFolder.CommonApplicationData));
        var productDirectory = Path.GetFullPath(
            IndexerServiceInstaller.ProtectedProductDirectory());
        var vendorDirectory = Path.GetFullPath(
            IndexerServiceInstaller.ProtectedVendorDirectory());
        var serviceDirectory = Path.GetFullPath(
            IndexerServiceInstaller.ProtectedServiceDirectory());

        Assert(vendorDirectory.StartsWith(
            programData + Path.DirectorySeparatorChar,
            StringComparison.OrdinalIgnoreCase));
        Assert(string.Equals(
            productDirectory,
            Path.Combine(vendorDirectory, "NxTools.Next"),
            StringComparison.OrdinalIgnoreCase));
        Assert(string.Equals(
            serviceDirectory,
            Path.Combine(productDirectory, "Service"),
            StringComparison.OrdinalIgnoreCase));
        Assert(IndexerServiceInstaller.DeploymentSourceMatchesDestination(
            Path.Combine(serviceDirectory, "NxTools.exe"),
            Path.Combine(
                serviceDirectory.ToUpperInvariant(),
                ".",
                "NxTools.exe")));
        Assert(!IndexerServiceInstaller.DeploymentSourceMatchesDestination(
            Path.Combine(serviceDirectory, "NxTools.exe"),
            Path.Combine(serviceDirectory, "NxTools.Indexer.exe")));
        return Task.CompletedTask;
    }

    private static Task IndexerServiceUpdateStopsOnlyExactProtectedPaths()
    {
        var root = Path.Combine(
            Path.GetTempPath(),
            $"nxtools-owned-process-test-{Guid.NewGuid():N}");
        var protectedDirectory = Path.Combine(root, "protected");
        var unrelatedDirectory = Path.Combine(root, "unrelated");
        Directory.CreateDirectory(protectedDirectory);
        Directory.CreateDirectory(unrelatedDirectory);
        var systemCommand = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.System),
            "cmd.exe");
        var protectedHost = Path.Combine(
            protectedDirectory,
            "NxTools.PlatformHost.exe");
        var unrelatedHost = Path.Combine(
            unrelatedDirectory,
            "NxTools.PlatformHost.exe");
        File.Copy(systemCommand, protectedHost);
        File.Copy(systemCommand, unrelatedHost);

        Process? protectedProcess = null;
        Process? unrelatedProcess = null;
        try
        {
            protectedProcess = Process.Start(new ProcessStartInfo
            {
                FileName = protectedHost,
                Arguments = "/d /c \"ping 127.0.0.1 -n 30 > nul\"",
                UseShellExecute = false,
                CreateNoWindow = true
            }) ?? throw new InvalidOperationException(
                "could not start the protected process fixture");
            unrelatedProcess = Process.Start(new ProcessStartInfo
            {
                FileName = unrelatedHost,
                Arguments = "/d /c \"ping 127.0.0.1 -n 30 > nul\"",
                UseShellExecute = false,
                CreateNoWindow = true
            }) ?? throw new InvalidOperationException(
                "could not start the unrelated process fixture");

            Assert(!protectedProcess.WaitForExit(100));
            Assert(!unrelatedProcess.WaitForExit(100));

            IndexerServiceInstaller.StopProtectedProcesses(
                [protectedHost],
                Environment.ProcessId,
                NullPlatformLog.Instance);

            Assert(protectedProcess.WaitForExit(5_000));
            Assert(!unrelatedProcess.HasExited);
            File.Copy(systemCommand, protectedHost, overwrite: true);
        }
        finally
        {
            foreach (var process in new[] { protectedProcess, unrelatedProcess })
            {
                if (process is not null && !process.HasExited)
                {
                    process.Kill(entireProcessTree: true);
                    process.WaitForExit(5_000);
                }
                process?.Dispose();
            }
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task PasteCommandPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "paste_text",
            Data = new
            {
                Text = "paste exactly this",
                TargetWindowHandle = 42L,
                TargetFocusWindowHandle = 43L
            }
        });

        var command = PlatformCommandParser.ParsePasteText(envelope);
        Assert(command.Text == "paste exactly this");
        Assert(command.PasteAfterWrite);
        Assert(command.TargetWindowHandle == 42L);
        Assert(command.TargetFocusWindowHandle == 43L);
        return Task.CompletedTask;
    }

    private static Task ClipboardEventPreservesSourceContext()
    {
        var envelope = PlatformEventEnvelopeFactory.Create(
            new ClipboardCapturedEvent(
                "captured text",
                null,
                [],
                "C:/source.png",
                new ForegroundContext(42, 43, 7, "notepad", "notes.txt"),
                1_700_000_000_000));
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);

        Assert(json.Contains("\"event\":\"clipboard_captured\"", StringComparison.Ordinal));
        Assert(json.Contains("\"text\":\"captured text\"", StringComparison.Ordinal));
        Assert(json.Contains("\"window_handle\":42", StringComparison.Ordinal));
        Assert(json.Contains("\"process_name\":\"notepad\"", StringComparison.Ordinal));
        Assert(json.Contains("C:/source.png", StringComparison.Ordinal));
        return Task.CompletedTask;
    }
    private static Task FullClipboardEventQueueCleansStagingAndReportsOnce()
    {
        var channel = Channel.CreateBounded<WindowsPlatformEvent>(
            new BoundedChannelOptions(1)
            {
                FullMode = BoundedChannelFullMode.Wait,
                SingleReader = true,
                SingleWriter = false
            });
        Assert(channel.Writer.TryWrite(
            new ForegroundChangedEvent(
                new ForegroundContext(1, 1, 1, "source", "window"))));
        var root = Path.Combine(
            Path.GetTempPath(), $"nxtools-clipboard-event-drop-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        var imagePath = Path.Combine(root, "image.png");
        var nativePath = Path.Combine(root, "native.bin");
        var iconPath = Path.Combine(root, "source-icon.png");
        var sourcePath = Path.Combine(root, "source.txt");
        File.WriteAllBytes(imagePath, [1]);
        File.WriteAllBytes(nativePath, [2, 3, 4]);
        File.WriteAllBytes(iconPath, [5]);
        File.WriteAllText(sourcePath, "source payload");
        try
        {
            var captured = new ClipboardCapturedEvent(
                null,
                new ClipboardImageData(imagePath, 10, 10),
                [sourcePath],
                iconPath,
                new ForegroundContext(2, 2, 2, "source", "window"),
                1_700_000_000_000,
                [new ClipboardNativeFormatData(49_200, "TestFormat", nativePath, 3, 0)]);
            var log = new RecordingPlatformLog();
            var dropGate = new ClipboardCaptureDropDiagnosticGate();

            Assert(!WindowsPlatformAdapter.TryPublishClipboardCapturedEvent(
                channel.Writer,
                captured,
                log,
                dropGate));
            Assert(!File.Exists(imagePath));
            Assert(!File.Exists(nativePath));
            Assert(!File.Exists(iconPath));
            Assert(File.Exists(sourcePath));
            var dropped = log.Entries.Single(entry =>
                entry.EventName == "clipboard.capture_event_dropped");
            Assert(log.Entries.Count == 1);
            Assert(!WindowsPlatformAdapter.TryPublishClipboardCapturedEvent(
                channel.Writer,
                captured,
                log,
                dropGate));
            Assert(log.Entries.Count == 1);
            Assert(Equals(dropped.Fields["reason"], "event_queue_full"));
            Assert(Equals(dropped.Fields["image_staging_file_count"], 1));
            Assert(Equals(dropped.Fields["native_format_staging_file_count"], 1));
            Assert(Equals(dropped.Fields["source_icon_staging_file_count"], 1));
            Assert(Equals(dropped.Fields["cleanup_deleted_file_count"], 3));
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;

    }
    private static Task EmptyClipboardPayloadsAreIgnored()
    {
        var noPaths = Array.Empty<string>();
        Assert(!WindowsPlatformAdapter.EventWindow.HasUserClipboardContent(null, null, noPaths));
        Assert(!WindowsPlatformAdapter.EventWindow.HasUserClipboardContent(string.Empty, null, noPaths));
        Assert(!WindowsPlatformAdapter.EventWindow.HasUserClipboardContent(" \r\n ", null, noPaths));
        Assert(WindowsPlatformAdapter.EventWindow.HasUserClipboardContent("text", null, noPaths));
        Assert(WindowsPlatformAdapter.EventWindow.HasUserClipboardContent(null, new ClipboardImageData("C:/image.png", 1, 1), noPaths));
        Assert(WindowsPlatformAdapter.EventWindow.HasUserClipboardContent(null, null, ["C:/file.txt"]));
        return Task.CompletedTask;
    }
    private static Task MultiFormatPayloadsRoundTrip()
    {
        var filesEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "paste_files",
            Data = new { Paths = new[] { "C:/one.txt", "C:/two.txt" }, TargetWindowHandle = 4L, TargetFocusWindowHandle = 5L }
        });
        var files = PlatformCommandParser.ParsePasteFiles(filesEnvelope);
        Assert(files.Paths.Count == 2 && files.TargetWindowHandle == 4);

        var imageEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "paste_image",
            Data = new { ImagePath = "C:/image.png", TargetWindowHandle = 6L, TargetFocusWindowHandle = 7L }
        });
        var image = PlatformCommandParser.ParsePasteImage(imageEnvelope);
        Assert(image.ImagePath == "C:/image.png" && image.TargetFocusWindowHandle == 7);

        var captured = PlatformEventEnvelopeFactory.Create(new ClipboardCapturedEvent(
            "alternate representation",
            new ClipboardImageData("C:/staging.png", 640, 480),
            ["C:/one.txt"],
            "C:/source.png",
            new ForegroundContext(1, 2, 3, "capture", "window"), 10));
        var json = JsonSerializer.Serialize(captured, JsonDefaults.Options);
        Assert(json.Contains("clipboard_captured", StringComparison.Ordinal));
        Assert(json.Contains("\"width\":640", StringComparison.Ordinal));
        Assert(json.Contains("C:/one.txt", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task NativeFormatPayloadRoundTrips()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "paste_native_formats",
            Data = new
            {
                Formats = new[]
                {
                    new
                    {
                        FormatId = 13U,
                        FormatName = "",
                        DataPath = "C:/formats/unicode.bin",
                        SizeBytes = 24UL,
                        SortOrder = 0U
                    }
                },
                PasteAfterWrite = false,
                ImagePathFallback = "C:/images/history.png",
                TargetWindowHandle = 0L,
                TargetFocusWindowHandle = 0L
            }
        });

        var command = PlatformCommandParser.ParsePasteNativeFormats(envelope);
        Assert(command.Formats.Count == 1);
        Assert(command.Formats[0].FormatId == 13);
        Assert(command.Formats[0].DataPath.EndsWith("unicode.bin", StringComparison.Ordinal));
        Assert(command.ImagePathFallback == "C:/images/history.png");
        Assert(!command.PasteAfterWrite);
        return Task.CompletedTask;
    }

    private static Task ClipboardConfigurationRoundTrips()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "configure_clipboard",
            Data = new
            {
                HotkeyModifiers = new[] { "control", "shift" },
                HotkeyKey = "V",
                HotkeyDisplay = "Ctrl+Shift+V",
                ExcludedProcesses = new[] { "KeePass.exe", "Bitwarden.exe" },
                MaxTextBytes = 12UL * 1024 * 1024,
                MaxImageBytes = 96UL * 1024 * 1024,
                MaxImagePixels = 120_000_000
            }
        });

        var command = PlatformCommandParser.ParseConfigureClipboard(envelope);
        Assert(command.HotkeyModifiers.SequenceEqual(["control", "shift"]));
        Assert(command.HotkeyKey == "V");
        Assert(command.HotkeyDisplay == "Ctrl+Shift+V");
        Assert(ClipboardHotKeyBinding.TryCreate(command, out var binding, out _));
        Assert(binding.Modifiers == 0x0006);
        Assert(binding.VirtualKey == 0x56);
        Assert(command.ExcludedProcesses.SequenceEqual(["KeePass.exe", "Bitwarden.exe"]));
        Assert(command.MaxTextBytes == 12UL * 1024 * 1024);
        Assert(command.MaxImageBytes == 96UL * 1024 * 1024);
        Assert(command.MaxImagePixels == 120_000_000);

        var recordingEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_clipboard_hot_key_recording",
            Data = new { Recording = true }
        });
        var recording = PlatformCommandParser.ParseSetClipboardHotKeyRecording(recordingEnvelope);
        Assert(recording.Recording);
        return Task.CompletedTask;
    }

    private static Task ClipboardCaptureLimitsClampAndReject()
    {
        var limits = ClipboardCaptureLimits.From(new ConfigureClipboardCommand(
            [], "C", "Alt+C", [], 1, ulong.MaxValue, 0));
        Assert(limits.MaxTextBytes == ClipboardCaptureLimits.MinTextBytes);
        Assert(limits.MaxImageBytes == ClipboardCaptureLimits.MaxAllowedImageBytes);
        Assert(limits.MaxImagePixels == ClipboardCaptureLimits.MinImagePixels);
        Assert(limits.AllowsText(ClipboardCaptureLimits.MinTextBytes));
        Assert(!limits.AllowsText(ClipboardCaptureLimits.MinTextBytes + 1));
        Assert(limits.ImageRejectionReason(1_000, 1_000, 1) is null);
        Assert(limits.ImageRejectionReason(1_001, 1_000, 1) == "max_image_pixels");
        Assert(limits.ImageRejectionReason(1_000, 1_000, limits.MaxImageBytes + 1) == "max_image_bytes");
        return Task.CompletedTask;
    }

    private static Task ClipboardExclusionsMatchProcessNames()
    {
        var exclusions = ClipboardCaptureExclusions.Create(
            [" KeePass.exe ", "BITWARDEN", "", "keepass.exe"]);
        Assert(exclusions.Contains("KeePass"));
        Assert(exclusions.Contains("keepass.exe"));
        Assert(exclusions.Contains("Bitwarden.exe"));
        Assert(!exclusions.Contains("notepad.exe"));
        Assert(exclusions.ProcessNames.Count == 2);
        return Task.CompletedTask;
    }

    private static Task QuickLaunchContractsRoundTrip()
    {
        var configurationEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "configure_quick_launch",
            Data = new
            {
                Enabled = true,
                HotkeyModifiers = Array.Empty<string>(),
                HotkeyKey = "Control",
                HotkeyDisplay = "双击 Ctrl",
                AutoQuickSwitchEnabled = true,
                FileDialogSearchEnabled = true
            }
        });
        var configuration = PlatformCommandParser.ParseConfigureQuickLaunch(configurationEnvelope);
        Assert(configuration.Enabled);
        Assert(configuration.AutoQuickSwitchEnabled);
        Assert(configuration.FileDialogSearchEnabled);
        Assert(QuickLaunchTriggerBinding.TryCreate(configuration, out var binding, out _));
        Assert(binding.Kind == QuickLaunchTriggerKind.DoubleControl);

        var eventEnvelope = PlatformEventEnvelopeFactory.Create(
            new QuickLaunchPanelRequestedEvent(
                new ForegroundContext(88, 89, 9, "editor", "draft.md"),
                1200, 700, 1, 144, 1_700_000_000_500,
                new DirectoryNavigationContext("explorer", 88, @"C:\Work")));
        var json = JsonSerializer.Serialize(eventEnvelope, JsonDefaults.Options);
        Assert(json.Contains("\"event\":\"quick_launch_panel_requested\"", StringComparison.Ordinal));
        Assert(json.Contains("\"window_handle\":88", StringComparison.Ordinal));
        Assert(json.Contains("\"directory_navigation_context\"", StringComparison.Ordinal));
        Assert(json.Contains("\"host\":\"explorer\"", StringComparison.Ordinal));
        Assert(json.Contains("\"current_directory\":\"C:\\\\Work\"", StringComparison.Ordinal));

        var dialogEnvelope = PlatformEventEnvelopeFactory.Create(
            new QuickLaunchPanelRequestedEvent(
                new ForegroundContext(77, 78, 7, "notepad", "Open"),
                1200, 700, 1, 144, 1_700_000_000_600,
                new DirectoryNavigationContext("file_dialog", 77, @"C:\Incoming")));
        var dialogJson = JsonSerializer.Serialize(dialogEnvelope, JsonDefaults.Options);
        Assert(dialogJson.Contains("\"directory_navigation_context\"", StringComparison.Ordinal));
        Assert(dialogJson.Contains("\"host\":\"file_dialog\"", StringComparison.Ordinal));
        Assert(dialogJson.Contains("\"window_handle\":77", StringComparison.Ordinal));
        Assert(dialogJson.Contains(
            "\"current_directory\":\"C:\\\\Incoming\"", StringComparison.Ordinal));

        var desktopEnvelope = PlatformEventEnvelopeFactory.Create(
            new QuickLaunchPanelRequestedEvent(
                new ForegroundContext(66, 67, 6, "explorer", "Program Manager"),
                1200, 700, 1, 144, 1_700_000_000_700,
                new DirectoryNavigationContext(
                    "desktop",
                    66,
                    @"C:\Users\Test\Desktop")));
        var desktopJson = JsonSerializer.Serialize(desktopEnvelope, JsonDefaults.Options);
        Assert(desktopJson.Contains("\"host\":\"desktop\"", StringComparison.Ordinal));
        Assert(desktopJson.Contains("\"window_handle\":66", StringComparison.Ordinal));

        var thirdPartyEnvelope = PlatformEventEnvelopeFactory.Create(
            new QuickLaunchPanelRequestedEvent(
                new ForegroundContext(55, 56, 5, "dopus", "Work"),
                1200, 700, 1, 144, 1_700_000_000_800,
                new DirectoryNavigationContext(
                    "third_party_file_manager",
                    55,
                    @"D:\Work",
                    "pane-context-55")));
        var thirdPartyJson = JsonSerializer.Serialize(
            thirdPartyEnvelope,
            JsonDefaults.Options);
        Assert(thirdPartyJson.Contains(
            "\"host\":\"third_party_file_manager\"",
            StringComparison.Ordinal));
        Assert(thirdPartyJson.Contains("\"window_handle\":55", StringComparison.Ordinal));
        Assert(thirdPartyJson.Contains(
            "\"host_context_id\":\"pane-context-55\"",
            StringComparison.Ordinal));
        var resolvedEnvelope = PlatformEventEnvelopeFactory.Create(
            new QuickLaunchDirectoryContextResolvedEvent(
                55,
                1_700_000_000_800,
                new DirectoryNavigationContext(
                    "third_party_file_manager",
                    55,
                    @"D:\Elevated",
                    "elevated:context-55")));
        var resolvedJson = JsonSerializer.Serialize(
            resolvedEnvelope,
            JsonDefaults.Options);
        Assert(resolvedJson.Contains(
            "\"event\":\"quick_launch_directory_context_resolved\"",
            StringComparison.Ordinal));
        Assert(resolvedJson.Contains(
            "\"host_context_id\":\"elevated:context-55\"",
            StringComparison.Ordinal));
        var autoSwitchEnvelope = PlatformEventEnvelopeFactory.Create(
            new QuickLaunchAutoSwitchRequestedEvent(
                new DirectoryNavigationContext(
                    "file_dialog",
                    77,
                    @"C:\Incoming"),
                @"D:\Work",
                1_700_000_000_900));
        var autoSwitchJson = JsonSerializer.Serialize(
            autoSwitchEnvelope,
            JsonDefaults.Options);
        Assert(autoSwitchJson.Contains(
            "\"event\":\"quick_launch_auto_switch_requested\"",
            StringComparison.Ordinal));
        Assert(autoSwitchJson.Contains(
            "\"source_directory\":\"D:\\\\Work\"",
            StringComparison.Ordinal));
        Assert(autoSwitchJson.Contains("\"window_handle\":77", StringComparison.Ordinal));
        var searchHostEnvelope = PlatformEventEnvelopeFactory.Create(
            new QuickLaunchFileDialogSearchHostChangedEvent(
                77,
                @"C:\Incoming",
                100,
                200,
                900,
                640,
                144,
                true,
                1_700_000_001_000,
                new AutoQuickSwitchDirectorySource(
                    new DirectoryNavigationContext(
                        DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                        88,
                        @"D:\Workspace",
                        "dopus:lister-88"),
                    "Workspace (Directory Opus)")));
        var searchHostJson = JsonSerializer.Serialize(
            searchHostEnvelope,
            JsonDefaults.Options);
        Assert(searchHostJson.Contains(
            "\"event\":\"quick_launch_file_dialog_search_host_changed\"",
            StringComparison.Ordinal));
        Assert(searchHostJson.Contains("\"left\":100", StringComparison.Ordinal));
        Assert(searchHostJson.Contains("\"monitor_dpi\":144", StringComparison.Ordinal));
        Assert(searchHostJson.Contains(
            "\"quick_switch_source\"",
            StringComparison.Ordinal));
        Assert(searchHostJson.Contains(
            "\"host\":\"third_party_file_manager\"",
            StringComparison.Ordinal));

        var recordingEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_quick_launch_hot_key_recording",
            Data = new { Recording = true }
        });
        Assert(PlatformCommandParser.ParseSetQuickLaunchHotKeyRecording(recordingEnvelope).Recording);
        return Task.CompletedTask;
    }

    private static Task AutoQuickSwitchRequiresKnownDialog()
    {
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(enabled: true);
        var dialog = new DirectoryNavigationContext("file_dialog", 77, @"C:\Initial");

        tracker.ObserveExplorer(88);
        Assert(tracker.ObserveFileDialog(dialog) is null);

        tracker.ObserveExplorer(88);
        var candidate = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException("Expected one auto-switch candidate.");
        Assert(candidate.SourceWindowHandle == 88);
        Assert(candidate.Target == dialog);
        Assert(tracker.IsCurrent(candidate));
        Assert(tracker.ObserveFileDialog(dialog) is null);
        return Task.CompletedTask;
    }

    private static Task AutoQuickSwitchInvalidatesStaleCandidates()
    {
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(enabled: true);
        var dialog = new DirectoryNavigationContext("file_dialog", 77, @"C:\Initial");
        Assert(tracker.ObserveFileDialog(dialog) is null);

        tracker.ObserveExplorer(88);
        var stale = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException("Expected a stale auto-switch candidate.");
        tracker.ObserveExplorer(99);
        Assert(!tracker.IsCurrent(stale));

        var current = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException("Expected a current auto-switch candidate.");
        Assert(current.SourceWindowHandle == 99);
        Assert(tracker.IsCurrent(current));

        tracker.ObserveExplorer(111);
        tracker.ObserveOther();
        Assert(tracker.ObserveFileDialog(dialog) is null);

        tracker.Configure(enabled: false);
        Assert(!tracker.IsCurrent(current));
        Assert(tracker.ObserveFileDialog(dialog) is null);
        return Task.CompletedTask;
    }

    private static Task AutoQuickSwitchAcceptsThirdPartySources()
    {
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(enabled: true);
        var dialog = new DirectoryNavigationContext("file_dialog", 77, @"C:\Initial");
        Assert(tracker.ObserveFileDialog(dialog) is null);

        var directoryOpus = new DirectoryNavigationContext(
            DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
            88,
            @"D:\Workspace",
            "dopus:lister-88");
        tracker.ObserveDirectorySource(directoryOpus, "Workspace (Directory Opus)");

        var latest = tracker.LatestSource ??
            throw new InvalidOperationException("Expected a retained Directory Opus source.");
        Assert(latest.Context == directoryOpus);
        Assert(latest.DisplayName == "Workspace (Directory Opus)");

        var candidate = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException("Expected a third-party auto-switch candidate.");
        Assert(candidate.Source == directoryOpus);
        Assert(candidate.Target == dialog);
        return Task.CompletedTask;
    }

    private static Task FileDialogSearchRetainsSourcesWithoutAutoSwitch()
    {
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(autoSwitchEnabled: false, trackSources: true);
        var source = new DirectoryNavigationContext(
            DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
            88,
            @"D:\Workspace",
            "dopus:lister-88");
        tracker.ObserveDirectorySource(source, "Workspace (Directory Opus)");

        Assert(tracker.LatestSource?.Context == source);
        Assert(tracker.ObserveFileDialog(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.FileDialogHost,
                77,
                @"C:\Initial")) is null);
        return Task.CompletedTask;
    }

    private static Task FileDialogInitializationRetriesPreserveExplorerSource()
    {
        var retry = new FileDialogRecognitionRetry(maxAttempts: 3);
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(enabled: true);
        var dialog = new DirectoryNavigationContext("file_dialog", 77, @"C:\Initial");
        Assert(tracker.ObserveFileDialog(dialog) is null);

        tracker.ObserveExplorer(88);
        Assert(retry.Begin(77));
        Assert(retry.TryBeginAttempt(77, out var firstTarget) && firstTarget == 77);
        Assert(retry.HasPendingAttempt(77));
        Assert(retry.TryBeginAttempt(77, out var secondTarget) && secondTarget == 77);
        retry.Complete(77);

        var candidate = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException(
                "A delayed file-dialog recognition must retain the Explorer source.");
        Assert(candidate.SourceWindowHandle == 88);
        Assert(!retry.HasPendingAttempt(77));

        Assert(retry.Begin(99));
        Assert(!retry.TryBeginAttempt(100, out var deferredTarget));
        Assert(deferredTarget == 99);
        Assert(retry.HasPendingAttempt(99));
        return Task.CompletedTask;
    }

    private static Task FileDialogStructureChangeRearmsPendingRecognition()
    {
        var retry = new FileDialogRecognitionRetry(maxAttempts: 3);
        Assert(retry.Begin(77));
        Assert(retry.ShouldRetryForStructureChange(77));
        Assert(!retry.ShouldRetryForStructureChange(78));
        Assert(retry.TryBeginAttempt(77, out var target) && target == 77);
        retry.Complete(77);
        Assert(!retry.ShouldRetryForStructureChange(77));
        return Task.CompletedTask;
    }

    private static Task FileDialogRecognitionSurvivesTransientForegroundHandoff()
    {
        var retry = new FileDialogRecognitionRetry(maxAttempts: 3);
        Assert(retry.Begin(77));

        Assert(!retry.TryBeginAttempt(99, out var deferredTarget));
        Assert(deferredTarget == 77);
        Assert(retry.HasPendingAttempt(77));
        Assert(retry.TryBeginAttempt(77, out var recognizedTarget));
        Assert(recognizedTarget == 77);
        return Task.CompletedTask;
    }

    private static Task FileDialogMovementCoalescesRootGeometry()
    {
        var gate = new FileDialogGeometryUpdateGate();

        Assert(!gate.TrySchedule(
            activeRootWindowHandle: 77,
            eventRootWindowHandle: 77,
            eventWindowHandle: 700,
            objectId: 0,
            eventType: 0x800B));
        Assert(gate.TrySchedule(
            activeRootWindowHandle: 77,
            eventRootWindowHandle: 77,
            eventWindowHandle: 77,
            objectId: 0,
            eventType: 0x800B));
        for (var index = 0; index < 100; index++)
        {
            Assert(!gate.TrySchedule(77, 77, 77, 0, 0x800B));
        }
        Assert(gate.TryTake(out var pendingRoot) && pendingRoot == 77);
        Assert(gate.TrySchedule(77, 77, 77, 0, 0x800B));
        return Task.CompletedTask;
    }

    private static Task VisibleFileDialogForegroundRepeatsReuseRecognizedHost()
    {
        var snapshot = new FileDialogSearchHostSnapshot(
            77,
            @"D:\Workspace",
            100,
            100,
            900,
            600,
            96);

        Assert(FileDialogSearchHost.CanReuseForeground(
            searchEnabled: true,
            hostVisible: true,
            snapshot,
            foregroundWindowHandle: 77));
        Assert(!FileDialogSearchHost.CanReuseForeground(true, false, snapshot, 77));
        Assert(!FileDialogSearchHost.CanReuseForeground(true, true, snapshot, 88));
        Assert(!FileDialogSearchHost.CanReuseForeground(false, true, snapshot, 77));
        return Task.CompletedTask;
    }

    private static Task FileDialogSearchHostPreservesPlacement()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_window_owner",
            Data = new
            {
                WindowHandle = 501,
                OwnerWindowHandle = 77
            }
        });
        var command = PlatformCommandParser.ParseSetWindowOwner(envelope);
        Assert(command.WindowHandle == 501);
        Assert(command.OwnerWindowHandle == 77);

        var placementEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "place_file_dialog_search_window",
            Data = new
            {
                WindowHandle = 501,
                OwnerWindowHandle = 77,
                VisibleWidth = 500,
                VisibleHeight = 72,
                BottomOverlap = 0
            }
        });
        var placement =
            PlatformCommandParser.ParsePlaceFileDialogSearchWindow(placementEnvelope);
        Assert(placement.WindowHandle == 501);
        Assert(placement.OwnerWindowHandle == 77);
        Assert(placement.VisibleWidth == 500);
        Assert(placement.VisibleHeight == 72);
        Assert(placement.BottomOverlap == 0);
        return Task.CompletedTask;
    }

    private static Task ExplorerNavigationKeepsCapturedWindow()
    {
        var directory = Directory.CreateTempSubdirectory("nxtools-explorer-nav-");
        try
        {
            var operations = new FakeExplorerWindowOperations
            {
                ExplorerWindowHandle = 88
            };
            var captured = ExplorerWindowAdapter.TryCapture(
                new ForegroundContext(88, 89, 9, "explorer", "Work"),
                operations);

            Assert(captured == new ExplorerContext(88, null));

            var result = ExplorerWindowAdapter.Navigate(
                new NavigateExplorerCommand(88, directory.FullName),
                operations,
                Directory.Exists);

            Assert(result.Success);
            Assert(operations.LastNavigatedWindowHandle == 88);
            Assert(operations.LastNavigatedDirectory == directory.FullName);
            Assert(operations.LastActivatedWindowHandle == 88);

            var wrongWindow = ExplorerWindowAdapter.Navigate(
                new NavigateExplorerCommand(99, directory.FullName),
                operations,
                Directory.Exists);
            Assert(!wrongWindow.Success);
            Assert(operations.LastNavigatedWindowHandle == 88);
        }
        finally
        {
            directory.Delete();
        }
        return Task.CompletedTask;
    }

    private static Task ElevatedDirectoryHelperRequestRoundTrips()
    {
        var envelope = PlatformEventEnvelopeFactory.Create(
            new ElevatedDirectoryHelperRequestedEvent(
                "nxtools-next-elevated-nav-0123456789abcdef",
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"));
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);
        Assert(json.Contains(
            "\"event\":\"elevated_directory_helper_requested\"",
            StringComparison.Ordinal));
        Assert(json.Contains(
            "\"pipe_name\":\"nxtools-next-elevated-nav-0123456789abcdef\"",
            StringComparison.Ordinal));
        Assert(json.Contains(
            "\"authentication_token\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"",
            StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task DeferredDirectoryCaptureExpires()
    {
        Assert(WindowsPlatformAdapter.IsQuickLaunchContextRequestCurrent(
            1_700_000_000_000,
            1_700_000_000_999));
        Assert(!WindowsPlatformAdapter.IsQuickLaunchContextRequestCurrent(
            1_700_000_000_000,
            1_700_000_001_001));
        Assert(!WindowsPlatformAdapter.IsQuickLaunchContextRequestCurrent(
            1_700_000_000_001,
            1_700_000_000_000));
        return Task.CompletedTask;
    }

    private static Task ElevatedCapturePreservesSummonTimeFocus()
    {
        var requested = new ForegroundContext(
            88,
            89,
            9,
            "OneCommander",
            "Files");
        var delayed = requested with { FocusWindowHandle = 99 };
        var preserved =
            ElevatedDirectoryNavigationHelper.PreserveRequestedFocus(
                requested,
                delayed,
                _ => true);
        Assert(preserved?.FocusWindowHandle == 89);
        Assert(
            ElevatedDirectoryNavigationHelper.PreserveRequestedFocus(
                requested,
                delayed,
                _ => false) is null);
        return Task.CompletedTask;
    }

    private static Task ThirdPartyFileManagerNavigationKeepsCapturedHost()
    {
        var directory = Directory.CreateTempSubdirectory("nxtools-file-manager-nav-");
        try
        {
            var ignored = new FakeThirdPartyFileManagerOperations(
                "ignored",
                windowHandle: 99,
                currentDirectory: @"C:\Ignored");
            var directoryOpus = new FakeThirdPartyFileManagerOperations(
                "directory_opus",
                windowHandle: 88,
                currentDirectory: @"C:\Work");
            IThirdPartyFileManagerOperations[] adapters = [ignored, directoryOpus];
            var registry = new ThirdPartyFileManagerSessionRegistry(adapters);
            var foreground = new ForegroundContext(
                88,
                89,
                9,
                "dopus",
                "Work");

            var firstCapture = registry.TryCapture(foreground).Context;
            var secondCapture = registry.TryCapture(foreground).Context;

            Assert(firstCapture is not null);
            Assert(secondCapture is not null);
            Assert(firstCapture!.WindowHandle == 88);
            Assert(firstCapture.CurrentDirectory == @"C:\Work");
            Assert(firstCapture.HostContextId != secondCapture!.HostContextId);
            Assert(directoryOpus.LastCapturedFocusWindowHandle == 89);

            var result = registry.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    88,
                    directory.FullName,
                    firstCapture.HostContextId),
                Directory.Exists);

            Assert(result.Success);
            Assert(directoryOpus.LastPaneState == 1);
            Assert(directoryOpus.LastNavigatedWindowHandle == 88);
            Assert(directoryOpus.LastNavigatedDirectory == directory.FullName);
            Assert(directoryOpus.LastActivatedWindowHandle == 88);
            Assert(registry.SessionCount == 1);

            var alreadyConsumed = registry.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    88,
                    directory.FullName,
                    firstCapture.HostContextId),
                Directory.Exists);
            Assert(!alreadyConsumed.Success);

            var wrongWindow = registry.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    77,
                    directory.FullName,
                    secondCapture.HostContextId),
                Directory.Exists);
            Assert(!wrongWindow.Success);
            Assert(directoryOpus.LastNavigatedWindowHandle == 88);
            Assert(registry.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    88,
                    directory.FullName,
                    secondCapture.HostContextId),
                Directory.Exists).Success);
            for (var index = 0; index < 2_000; index++)
            {
                var captured = registry.TryCapture(foreground).Context ??
                    throw new InvalidOperationException("capture failed");
                Assert(registry.Navigate(
                    new NavigateThirdPartyFileManagerCommand(
                        88,
                        directory.FullName,
                        captured.HostContextId),
                    Directory.Exists).Success);
            }
            Assert(registry.SessionCount == 0);
            Assert(registry.SessionOrderCount == 0);

            var fatalAdapter = new FakeThirdPartyFileManagerOperations(
                "one_commander",
                windowHandle: 88,
                currentDirectory: @"C:\Work",
                captureFailure: new WindowsCommandResult(
                    false,
                    "capture timed out",
                    Fatal: true));
            var failedCapture =
                new ThirdPartyFileManagerSessionRegistry([fatalAdapter])
                    .TryCapture(foreground);
            Assert(failedCapture.Context is null);
            Assert(failedCapture.Failure?.Fatal == true);

            var fatalNavigationAdapter =
                new FakeThirdPartyFileManagerOperations(
                    "files",
                    windowHandle: 88,
                    currentDirectory: @"C:\Work",
                    navigationFailure: new WindowsCommandResult(
                        false,
                        "navigation timed out",
                        Fatal: true));
            var fatalNavigationRegistry =
                new ThirdPartyFileManagerSessionRegistry(
                    [fatalNavigationAdapter]);
            var fatalNavigationContext =
                fatalNavigationRegistry.TryCapture(foreground).Context ??
                throw new InvalidOperationException("capture failed");
            var fatalNavigation = fatalNavigationRegistry.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    88,
                    directory.FullName,
                    fatalNavigationContext.HostContextId),
                Directory.Exists);
            Assert(!fatalNavigation.Success);
            Assert(fatalNavigation.Fatal);
            Assert(fatalNavigationRegistry.SessionCount == 0);
            var fatalNavigationRetry = fatalNavigationRegistry.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    88,
                    directory.FullName,
                    fatalNavigationContext.HostContextId),
                Directory.Exists);
            Assert(!fatalNavigationRetry.Success);
            Assert(fatalNavigationAdapter.NavigateCount == 1);
        }
        finally
        {
            directory.Delete();
        }
        return Task.CompletedTask;
    }

    private static Task DirectoryOpusAdapterPreservesActivePane()
    {
        var native = new FakeDirectoryOpusWindowOperations
        {
            ListerWindowHandle = 88,
            ActiveDirectory = @"D:\Work"
        };
        var adapter = new DirectoryOpusFileManagerOperations(native);
        var foreground = new ForegroundContext(
            88,
            89,
            9,
            "dopus",
            "Work");

        Assert(adapter.TryCapture(foreground, out var capture, out _));
        Assert(capture!.CurrentDirectory == @"D:\Work");
        Assert(native.LastFocusWindowHandle == 89);
        Assert(!adapter.TryCapture(
            foreground with { ProcessName = "other" },
            out _,
            out _));
        Assert(adapter.CanNavigate(88, capture.PaneState));
        Assert(!adapter.CanNavigate(99, capture.PaneState));
        var navigation = adapter.Navigate(
            88,
            @"D:\Target",
            capture.PaneState);
        Assert(navigation.Success);
        Assert(native.LastNavigatedWindowHandle == 88);
        Assert(native.LastNavigatedDirectory == @"D:\Target");
        Assert(adapter.TryActivate(
            88,
            capture.PaneState,
            out var activationError));
        Assert(activationError is null);
        Assert(native.LastActivatedWindowHandle == 88);
        return Task.CompletedTask;
    }
    private static Task DirectoryOpusFrameFocusKeepsCapturedPaneNavigable()
    {
        var native = new FakeDirectoryOpusWindowOperations
        {
            ListerWindowHandle = 88,
            ActiveDirectory = @"D:\\Work",
            RequireRestorablePaneFocus = true
        };
        var adapter = new DirectoryOpusFileManagerOperations(native);
        var foreground = new ForegroundContext(
            88,
            88,
            9,
            "dopus",
            "Work");

        Assert(adapter.TryCapture(foreground, out var capture, out _));
        Assert(capture is not null);
        var pane = (DirectoryOpusPaneState)capture!.PaneState;
        Assert(pane.FocusWindowHandle == 0);
        Assert(adapter.CanNavigate(88, capture.PaneState));
        return Task.CompletedTask;
    }


    private static Task DirectoryOpusPaneSelectionUsesReferenceEvidence()
    {
        var titleSelected = DirectoryOpusPaneSelector.Select(
            "D:\\Active",
            [
                new DirectoryOpusPaneCandidate(90, "D:\\Other", 0, 0, false),
                new DirectoryOpusPaneCandidate(91, "D:\\Active", 400, 0, false)
            ],
            null,
            rememberedWindowHandle: 90);
        Assert(titleSelected == 91);

        var geometrySelected = DirectoryOpusPaneSelector.Select(
            "Directory Opus",
            [
                new DirectoryOpusPaneCandidate(90, "D:\\Other", 0, 0, false),
                new DirectoryOpusPaneCandidate(91, "D:\\Active", 400, 0, false)
            ],
            new DirectoryOpusPanePoint(450, 100),
            rememberedWindowHandle: 90);
        Assert(geometrySelected == 91);
        return Task.CompletedTask;
    }

    private static Task DirectorySourceRecognitionFailureClearsStaleSource()
    {
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(autoSwitchEnabled: true, trackSources: true);
        tracker.ObserveDirectorySource(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                88,
                @"D:\\Work",
                "dopus-session"),
            "Work (Directory Opus)");

        tracker.DiscardPendingSource();

        var candidate = tracker.ObserveFileDialog(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.FileDialogHost,
                99,
                @"C:\\Dialog",
                null));
        Assert(candidate is null);
        return Task.CompletedTask;
    }

    private static Task DirectorySourceRecognitionSurvivesReturnToKnownFileDialog()
    {
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(enabled: true);
        var dialog = new DirectoryNavigationContext(
            DirectoryNavigationAdapter.FileDialogHost,
            77,
            @"C:\Dialog",
            null);
        Assert(tracker.ObserveFileDialog(dialog) is null);
        var retry = new FileDialogRecognitionRetry(maxAttempts: 3);
        Assert(retry.Begin(88));
        Assert(!retry.TryBeginAttempt(77, out var deferredTarget));
        Assert(deferredTarget == 88);
        Assert(tracker.ObserveFileDialog(dialog) is null);
        Assert(retry.TryBeginPendingAttempt(out var backgroundTarget));
        Assert(backgroundTarget == 88);
        tracker.ObserveDirectorySource(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                backgroundTarget,
                @"D:\Workspace",
                "dopus:lister-88"),
            "Workspace (Directory Opus)");
        var candidate = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException(
                "A source recognized after the dialog returned should be consumed immediately.");
        Assert(candidate.SourceWindowHandle == 88);
        Assert(tracker.IsCurrent(candidate));
        return Task.CompletedTask;
    }

    private static Task PendingDirectorySourceRecognitionIsBoundedAndSuperseded()
    {
        var retry = new FileDialogRecognitionRetry(maxAttempts: 2);
        Assert(!retry.HasPendingAttempt(0));
        Assert(retry.Begin(88));
        Assert(retry.TryBeginPendingAttempt(out var firstTarget));
        Assert(firstTarget == 88);
        Assert(retry.Begin(99));
        Assert(retry.TryBeginPendingAttempt(out var supersededTarget));
        Assert(supersededTarget == 99);
        retry.Complete(99);
        Assert(!retry.TryBeginPendingAttempt(out _));
        Assert(retry.Begin(77));
        Assert(retry.TryBeginPendingAttempt(out _));
        Assert(retry.TryBeginPendingAttempt(out _));
        Assert(!retry.TryBeginPendingAttempt(out var exhaustedTarget));
        Assert(exhaustedTarget == 0);
        return Task.CompletedTask;
    }

    private static Task TotalCommanderAdapterPreservesSourcePanel()
    {
        var native = new FakeTotalCommanderWindowOperations
        {
            MainWindowHandle = 71,
            SourceDirectory = @"C:\Source"
        };
        var adapter = new TotalCommanderFileManagerOperations(native);
        var foreground = new ForegroundContext(
            71,
            72,
            7,
            "TOTALCMD64",
            "Total Commander");

        Assert(adapter.TryCapture(foreground, out var capture, out _));
        Assert(capture!.CurrentDirectory == @"C:\Source");
        Assert(!adapter.TryCapture(
            foreground with { ProcessName = "other" },
            out _,
            out _));
        Assert(adapter.CanNavigate(71, capture.PaneState));
        Assert(!adapter.CanNavigate(72, capture.PaneState));
        var navigation = adapter.Navigate(
            71,
            @"D:\Target",
            capture.PaneState);
        Assert(navigation.Success);
        Assert(native.LastNavigatedWindowHandle == 71);
        Assert(native.LastNavigatedDirectory == @"D:\Target");
        Assert(adapter.TryActivate(
            71,
            capture.PaneState,
            out var activationError));
        Assert(activationError is null);
        Assert(native.LastActivatedWindowHandle == 71);
        return Task.CompletedTask;
    }

    private static Task OneCommanderAdapterPreservesAutomationPane()
    {
        var native = new FakeOneCommanderWindowOperations
        {
            MainWindowHandle = 61,
            CurrentDirectory = @"C:\Pane"
        };
        var adapter = new OneCommanderFileManagerOperations(native);
        var foreground = new ForegroundContext(
            61,
            62,
            6,
            "OneCommander",
            "OneCommander");

        Assert(adapter.TryCapture(foreground, out var capture, out _));
        Assert(capture!.CurrentDirectory == @"C:\Pane");
        Assert(native.LastAnchoredWindowHandle == 61);
        Assert(native.LastFocusWindowHandle == 62);
        Assert(!adapter.TryCapture(
            foreground with { ProcessName = "other" },
            out _,
            out _));
        Assert(adapter.CanNavigate(61, capture.PaneState));
        Assert(!adapter.CanNavigate(62, capture.PaneState));
        var navigation = adapter.Navigate(
            61,
            @"D:\Target",
            capture.PaneState);
        Assert(navigation.Success);
        Assert(native.LastNavigatedWindowHandle == 61);
        Assert(native.LastNavigatedDirectory == @"D:\Target");
        Assert(adapter.TryActivate(
            61,
            capture.PaneState,
            out var activationError));
        Assert(activationError is null);
        Assert(native.LastActivatedWindowHandle == 61);
        return Task.CompletedTask;
    }

    private static Task FilesAdapterPreservesCapturedWindow()
    {
        var native = new FakeFilesWindowOperations
        {
            MainWindowHandle = 51,
            CurrentDirectory = @"C:\Files"
        };
        var adapter = new FilesFileManagerOperations(native);
        var foreground = new ForegroundContext(
            51,
            52,
            5,
            "Files",
            "Files");

        Assert(adapter.TryCapture(foreground, out var capture, out _));
        Assert(capture!.CurrentDirectory == @"C:\Files");
        Assert(native.LastCapturedWindowHandle == 51);
        Assert(!adapter.TryCapture(
            foreground with { ProcessName = "FilesPreview" },
            out _,
            out _));
        Assert(adapter.CanNavigate(51, capture.PaneState));
        Assert(!adapter.CanNavigate(52, capture.PaneState));
        var navigation = adapter.Navigate(
            51,
            @"D:\Target",
            capture.PaneState);
        Assert(navigation.Success);
        Assert(native.LastNavigatedWindowHandle == 51);
        Assert(native.LastNavigatedDirectory == @"D:\Target");
        Assert(adapter.TryActivate(
            51,
            capture.PaneState,
            out var activationError));
        Assert(activationError is null);
        Assert(native.LastActivatedWindowHandle == 51);
        Assert(ThirdPartyFileManagerAdapter.IsPotentialHost(foreground));
        Assert(DirectoryNavigationAdapter.IsPotentialElevatedHost(foreground));
        return Task.CompletedTask;
    }

    private static Task FilesIdentityMatchesListaryContract()
    {
        Assert(FilesFileManagerOperations.IsFilesProcessName("Files"));
        Assert(FilesFileManagerOperations.IsFilesProcessName("Files.exe"));
        Assert(!FilesFileManagerOperations.IsFilesProcessName("FilesPreview"));
        Assert(!FilesFileManagerOperations.IsFilesProcessName(null));
        Assert(UiaFilesWindowOperations.IsFilesRootName("Files"));
        Assert(UiaFilesWindowOperations.IsFilesRootName(
            "Downloads - Files"));
        Assert(!UiaFilesWindowOperations.IsFilesRootName("File Explorer"));
        Assert(!UiaFilesWindowOperations.IsFilesRootName(null));
        return Task.CompletedTask;
    }

    private static Task FilesNavigationRequiresReadback()
    {
        var controls = new FakeFilesPathControls(
            @"C:\Files",
            confirmWrites: true);
        var confirmed = FilesPathNavigation.Navigate(
            controls,
            @"D:\Target",
            TimeSpan.FromMilliseconds(100),
            CancellationToken.None);

        Assert(confirmed.Success);
        Assert(controls.LastWrittenDirectory == @"D:\Target");
        Assert(controls.ReadCount > 0);

        var staleControls = new FakeFilesPathControls(
            @"C:\Files",
            confirmWrites: false);
        var rejected = FilesPathNavigation.Navigate(
            staleControls,
            @"D:\Target",
            TimeSpan.Zero,
            CancellationToken.None);

        Assert(!rejected.Success);
        Assert(staleControls.LastWrittenDirectory == @"D:\Target");
        Assert(staleControls.ReadCount == 1);
        return Task.CompletedTask;
    }

    private static Task RealDirectoryOpusListerExposesNavigationContext()
    {
        using var process = Process.GetProcessesByName("dopus")
            .FirstOrDefault(candidate => candidate.MainWindowHandle != IntPtr.Zero);
        if (process is null)
        {
            Console.WriteLine("SKIP no interactive Directory Opus lister is available");
            return Task.CompletedTask;
        }
        if (!NativeFocus.IsForegroundWindow(process.MainWindowHandle))
        {
            Console.WriteLine(
                "SKIP Directory Opus is running but is not the summon-time foreground window");
            return Task.CompletedTask;
        }
        var foreground = new ForegroundContext(
            process.MainWindowHandle.ToInt64(),
            NativeFocus.TryGetFocusedWindow(process.MainWindowHandle),
            (uint)process.Id,
            process.ProcessName,
            process.MainWindowTitle);
        var captured = DirectoryNavigationAdapter.TryCapture(foreground);
        Assert(captured is not null);
        Assert(captured!.Host == "third_party_file_manager");
        Assert(captured.WindowHandle == process.MainWindowHandle.ToInt64());
        Assert(ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
            captured.CurrentDirectory));
        return Task.CompletedTask;
    }

    private static Task RealFilesWindowExposesNavigationContext()
    {
        using var process = Process.GetProcessesByName("Files")
            .FirstOrDefault(candidate =>
                candidate.MainWindowHandle != IntPtr.Zero);
        if (process is null)
        {
            Console.WriteLine(
                "SKIP no interactive Files window is available");
            return Task.CompletedTask;
        }
        if (!NativeFocus.IsForegroundWindow(process.MainWindowHandle))
        {
            Console.WriteLine(
                "SKIP Files is running but is not the summon-time foreground window");
            return Task.CompletedTask;
        }
        var foreground = new ForegroundContext(
            process.MainWindowHandle.ToInt64(),
            NativeFocus.TryGetFocusedWindow(process.MainWindowHandle),
            (uint)process.Id,
            process.ProcessName,
            process.MainWindowTitle);
        var captured = DirectoryNavigationAdapter.TryCapture(foreground);
        Assert(captured is not null);
        Assert(captured!.Host ==
            DirectoryNavigationAdapter.ThirdPartyFileManagerHost);
        Assert(captured.WindowHandle ==
            process.MainWindowHandle.ToInt64());
        Assert(ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
            captured.CurrentDirectory));
        return Task.CompletedTask;
    }

    private static async Task ShellIconExtractionAllowsBoundedParallelWorkers()
    {
        var operationName = $"shell-icon-parallel-{Guid.NewGuid():N}";
        using var release = new ManualResetEventSlim();
        using var started = new CountdownEvent(2);
        var runs = Enumerable.Range(0, 2)
            .Select(_ => Task.Run(() => BoundedStaOperationRunner.TryRun(
                operationName,
                TimeSpan.FromSeconds(2),
                maxConcurrency: 2,
                cancellationToken =>
                {
                    started.Signal();
                    release.Wait(cancellationToken);
                    return true;
                },
                out bool? result,
                out string? parallelError,
                CancellationToken.None)
                && result == true))
            .ToArray();

        Assert(started.Wait(TimeSpan.FromSeconds(1)));
        release.Set();
        Assert((await Task.WhenAll(runs)).All(value => value));
    }

    private static async Task BoundedStaOperationsCancelLateWork()
    {
        var lateSideEffects = 0;
        var stopwatch = Stopwatch.StartNew();
        var completed = BoundedStaOperationRunner.TryRun(
            "stalled-provider-test",
            TimeSpan.FromMilliseconds(30),
            cancellationToken =>
            {
                cancellationToken.WaitHandle.WaitOne();
                cancellationToken.ThrowIfCancellationRequested();
                Interlocked.Increment(ref lateSideEffects);
                return true;
            },
            out bool? result,
            out var error);
        stopwatch.Stop();

        Assert(!completed);
        Assert(result is null);
        Assert(error?.Contains("timed out", StringComparison.OrdinalIgnoreCase) == true);
        Assert(ExplorerWindowAdapter.TimedOut(error).Fatal);
        Assert(UiaOneCommanderWindowOperations.TimedOut(error).Fatal);
        Assert(UiaFilesWindowOperations.TimedOut(error).Fatal);
        Assert(stopwatch.Elapsed < TimeSpan.FromMilliseconds(500));
        await Task.Delay(20);
        Assert(Volatile.Read(ref lateSideEffects) == 0);

        using var releaseStalledOperation = new ManualResetEventSlim();
        var stubbornCalls = 0;
        Assert(!BoundedStaOperationRunner.TryRun(
            "stubborn-provider-test",
            TimeSpan.FromMilliseconds(20),
            _ =>
            {
                Interlocked.Increment(ref stubbornCalls);
                releaseStalledOperation.Wait();
                return true;
            },
            out bool? _,
            out _));
        Assert(!BoundedStaOperationRunner.TryRun(
            "stubborn-provider-test",
            TimeSpan.FromMilliseconds(20),
            _ =>
            {
                Interlocked.Increment(ref stubbornCalls);
                return true;
            },
            out bool? _,
            out var busyError));
        Assert(busyError?.Contains(
            "previous bounded operation",
            StringComparison.OrdinalIgnoreCase) == true);
        Assert(Volatile.Read(ref stubbornCalls) == 1);
        releaseStalledOperation.Set();
        Assert(SpinWait.SpinUntil(
            () => BoundedStaOperationRunner.TryRun(
                "stubborn-provider-test",
                TimeSpan.FromMilliseconds(50),
                _ => true,
                out bool? _,
                out _),
            TimeSpan.FromSeconds(1)));
    }

    private static async Task CanceledStaDialogDropsLateResult()
    {
        using var cancellation = new CancellationTokenSource();
        var started = new TaskCompletionSource<ApartmentState>(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var lateResultProduced = 0;
        var run = Task.Run(() => StaDialogRunner.Run(
            "NxTools.CanceledDialogRegression",
            () =>
            {
                started.TrySetResult(Thread.CurrentThread.GetApartmentState());
                cancellation.Token.WaitHandle.WaitOne();
                Interlocked.Increment(ref lateResultProduced);
                return new WindowsCommandResult(true, Data: "late-result");
            },
            cancellation.Token));

        Assert(await started.Task.WaitAsync(TimeSpan.FromSeconds(1)) == ApartmentState.STA);
        cancellation.Cancel();
        var stopwatch = Stopwatch.StartNew();
        var result = await run.WaitAsync(TimeSpan.FromSeconds(1));
        stopwatch.Stop();

        Assert(stopwatch.Elapsed < TimeSpan.FromSeconds(1));
        Assert(!result.Success);
        Assert(result.Data is null);
        Assert(Volatile.Read(ref lateResultProduced) == 1);
    }

    private static async Task CanceledDetachedDrainDoesNotBlockReconnect()
    {
        using var cancellation = new CancellationTokenSource();
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var started = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var processing = dispatcher.DispatchAsync(
            "pick_program",
            async _ =>
            {
                started.TrySetResult();
                await release.Task;
            },
            cancellation.Token);
        await started.Task.WaitAsync(TimeSpan.FromSeconds(1));

        try
        {
            cancellation.Cancel();
            var stopwatch = Stopwatch.StartNew();
            dispatcher.BeginDrain(TimeSpan.FromSeconds(1));
            stopwatch.Stop();
            Assert(stopwatch.Elapsed < TimeSpan.FromMilliseconds(100));

            var reconnected = new PlatformCommandSequenceDispatcher();
            var followUpRan = new TaskCompletionSource(
                TaskCreationOptions.RunContinuationsAsynchronously);
            await reconnected.DispatchAsync(
                "set_window_visibility",
                () =>
                {
                    followUpRan.TrySetResult();
                    return Task.CompletedTask;
                });
            Assert(await Task.WhenAny(
                followUpRan.Task,
                Task.Delay(TimeSpan.FromMilliseconds(250))) == followUpRan.Task);
        }
        finally
        {
            release.TrySetResult();
            await processing.WaitAsync(TimeSpan.FromSeconds(1));
            await dispatcher.DrainAsync(TimeSpan.FromSeconds(1));
        }
    }

    private static async Task DetachedCancellationObservesLateFaults()
    {
        using var cancellation = new CancellationTokenSource();
        var failures = 0;
        var dispatcher = new PlatformCommandSequenceDispatcher(
            _ => Interlocked.Increment(ref failures));
        var started = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        await dispatcher.DispatchAsync(
            "pick_program",
            async _ =>
            {
                started.TrySetResult();
                await release.Task;
                throw new InvalidOperationException("late detached fault");
            },
            cancellation.Token);
        await started.Task.WaitAsync(TimeSpan.FromSeconds(1));

        cancellation.Cancel();
        dispatcher.BeginDrain(TimeSpan.FromSeconds(1));
        release.TrySetResult();
        await dispatcher.DrainAsync(TimeSpan.FromSeconds(1));
        Assert(SpinWait.SpinUntil(() => Volatile.Read(ref failures) == 1, TimeSpan.FromSeconds(1)));
    }

    private static Task CanceledShellWorkerIsNotReusedFromCache()
    {
        var target = Path.Combine(
            Path.GetTempPath(),
            $"nxtools-canceled-shell-worker-{Guid.NewGuid():N}");
        var worker = new ShellContextMenuSessionWorker(target);
        worker.Release();
        var stopwatch = Stopwatch.StartNew();
        ShellContextMenuSessionWorker.CancelCached();
        stopwatch.Stop();
        Assert(stopwatch.Elapsed < TimeSpan.FromSeconds(1));

        var replacement = ShellContextMenuSessionWorker.Acquire(target);
        try
        {
            Assert(!ReferenceEquals(worker, replacement));
        }
        finally
        {
            replacement.CancelCurrent();
        }
        return Task.CompletedTask;
    }

    private static Task ElevatedDirectoryContextsUseSameIntegrityChannel()
    {
        var channel = new FakeElevatedDirectoryNavigationChannel
        {
            CapturedContext = new DirectoryNavigationContext(
                "third_party_file_manager",
                999,
                @"D:\Elevated",
                "elevated:context-999")
        };
        var captured = DirectoryNavigationAdapter.TryCapture(
            new ForegroundContext(
                999,
                998,
                9,
                "dopus",
                "Elevated"),
            channel);
        Assert(captured == channel.CapturedContext);

        var command = new NavigateDirectoryCommand(
            "third_party_file_manager",
            999,
            @"D:\Target",
            "elevated:context-999");
        var result = DirectoryNavigationAdapter.Navigate(command, channel);
        Assert(result.Success);
        Assert(channel.LastCommand == command);
        Assert(channel.CaptureCalls == 1);
        Assert(channel.NavigateCalls == 1);
        return Task.CompletedTask;
    }

    private static Task ElevatedHelperSessionsPreserveCapturedContext()
    {
        var navigated = new List<NavigateDirectoryCommand>();
        var registry = new ElevatedDirectoryNavigationSessionRegistry(
            _ => new DirectoryNavigationCaptureAttempt(
                new DirectoryNavigationContext(
                    "third_party_file_manager",
                    88,
                    @"C:\Pane",
                    "inner-pane-88")),
            command =>
            {
                navigated.Add(command);
                return new WindowsCommandResult(true);
            });
        var foreground = new ForegroundContext(
            88,
            89,
            8,
            "dopus",
            "Elevated");
        var first = registry.TryCapture(foreground).Context;
        var second = registry.TryCapture(foreground).Context;

        Assert(first is not null);
        Assert(second is not null);
        Assert(first!.HostContextId != second!.HostContextId);
        Assert(ElevatedDirectoryNavigationClient.IsElevatedContext(
            first.HostContextId));
        var result = registry.Navigate(new NavigateDirectoryCommand(
            first.Host,
            first.WindowHandle,
            @"D:\Target",
            first.HostContextId));
        Assert(result.Success);
        Assert(navigated.Count == 1);
        Assert(navigated[0].HostContextId == "inner-pane-88");
        Assert(registry.SessionCount == 1);
        Assert(!registry.Navigate(new NavigateDirectoryCommand(
            first.Host,
            first.WindowHandle,
            @"D:\Target",
            first.HostContextId)).Success);
        Assert(registry.Navigate(new NavigateDirectoryCommand(
            second!.Host,
            second.WindowHandle,
            @"D:\Target",
            second.HostContextId)).Success);
        for (var index = 0; index < 2_000; index++)
        {
            var captured = registry.TryCapture(foreground).Context
                ?? throw new InvalidOperationException("capture failed");
            Assert(registry.Navigate(new NavigateDirectoryCommand(
                captured.Host,
                captured.WindowHandle,
                @"D:\Target",
                captured.HostContextId)).Success);
        }
        Assert(registry.SessionCount == 0);
        Assert(registry.SessionOrderCount == 0);

        var fatalRegistry = new ElevatedDirectoryNavigationSessionRegistry(
            _ => new DirectoryNavigationCaptureAttempt(
                null,
                new WindowsCommandResult(
                    false,
                    "capture timed out",
                    Fatal: true)),
            _ => new WindowsCommandResult(true));
        var fatalCapture = fatalRegistry.TryCapture(foreground);
        Assert(fatalCapture.Context is null);
        Assert(fatalCapture.Failure?.Fatal == true);
        return Task.CompletedTask;
    }

    private static Task ElevatedHelperDiagnosticsKeepPidsDistinct()
    {
        var connected = ElevatedDirectoryNavigationClient.HelperConnectedDiagnosticFields(12_345);
        Assert(Equals(connected["helper_process_id"], 12_345));
        Assert(!connected.ContainsKey("process_id"));

        var disconnected = ElevatedDirectoryNavigationClient
            .HelperDisconnectedDiagnosticFields(12_345, "pipe_closed");
        Assert(Equals(disconnected["helper_process_id"], 12_345));
        Assert(Equals(disconnected["reason"], "pipe_closed"));
        Assert(!disconnected.ContainsKey("process_id"));
        var terminalFailure = ElevatedDirectoryNavigationClient
            .HelperConnectionFailureDiagnosticFields(null, 4, terminal: true, "timeout");
        Assert(terminalFailure["helper_process_id"] is null);
        Assert(Equals(terminalFailure["attempt"], 4));
        Assert(Equals(terminalFailure["terminal"], true));
        Assert(Equals(terminalFailure["retry_exhausted"], true));
        return Task.CompletedTask;
    }

    private static Task ElevatedHelperRendezvousIsBounded()
    {
        var pipe = "nxtools-next-elevated-nav-0123456789abcdef";
        var token =
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        Assert(ElevatedHelperProcessBroker.IsValidRendezvous(pipe, token));
        Assert(!ElevatedHelperProcessBroker.IsValidRendezvous(
            @"nxtools-next-elevated-nav-..\other",
            token));
        Assert(!ElevatedHelperProcessBroker.IsValidRendezvous(pipe, "short"));
        Assert(ElevatedDirectoryHelperPeerVerifier.IsSameSession(
            (uint)Environment.ProcessId,
            (uint)Environment.ProcessId));
        Assert(!ElevatedDirectoryHelperPeerVerifier.IsSameSession(
            uint.MaxValue,
            (uint)Environment.ProcessId));
        using var oversized = new StringReader(new string('x', 4_097));
        AssertThrows<InvalidDataException>(() =>
            BoundedTextFrameReader.ReadLine(oversized, 4_096));
        var fatalResponse = new ElevatedDirectoryHelperResponse(
            Guid.NewGuid(),
            false,
            null,
            "navigation timed out",
            true);
        var restoredFatal = JsonSerializer.Deserialize<
            ElevatedDirectoryHelperResponse>(
            JsonSerializer.Serialize(
                fatalResponse,
                JsonDefaults.Options),
            JsonDefaults.Options);
        Assert(restoredFatal?.Fatal == true);
        return Task.CompletedTask;
    }

    private static Task DesktopNavigationUsesShellDesktop()
    {
        var directory = Directory.CreateTempSubdirectory("nxtools-desktop-nav-");
        try
        {
            var operations = new FakeDesktopWindowOperations
            {
                DesktopWindowHandle = 66
            };
            var captured = DesktopWindowAdapter.TryCapture(
                new ForegroundContext(66, 67, 6, "explorer", "Program Manager"),
                operations,
                () => @"C:\Users\Test\Desktop");

            Assert(captured == new DesktopContext(66, @"C:\Users\Test\Desktop"));
            Assert(DesktopWindowAdapter.TryCapture(
                new ForegroundContext(67, 67, 6, "explorer", "WorkerW"),
                operations,
                () => @"C:\Users\Test\Desktop") is null);
            Assert(DesktopWindowAdapter.TryCapture(
                new ForegroundContext(66, 67, 6, "other", "Program Manager"),
                operations,
                () => @"C:\Users\Test\Desktop") is null);

            var result = DesktopWindowAdapter.Navigate(
                new NavigateDesktopCommand(66, directory.FullName),
                operations,
                Directory.Exists);

            Assert(result.Success);
            Assert(operations.LastOpenedDirectory == directory.FullName);

            var wrongWindow = DesktopWindowAdapter.Navigate(
                new NavigateDesktopCommand(67, directory.FullName),
                operations,
                Directory.Exists);
            Assert(!wrongWindow.Success);
            Assert(operations.LastOpenedDirectory == directory.FullName);
        }
        finally
        {
            directory.Delete();
        }
        return Task.CompletedTask;
    }

    private static Task DesktopIdentityRequiresShellEvidence()
    {
        Assert(Win32DesktopWindowOperations.IsShellDesktopIdentity(
            true,
            "Progman",
            false,
            true));
        Assert(!Win32DesktopWindowOperations.IsShellDesktopIdentity(
            false,
            "Progman",
            false,
            true));
        Assert(Win32DesktopWindowOperations.IsShellDesktopIdentity(
            false,
            "WorkerW",
            true,
            true));
        Assert(!Win32DesktopWindowOperations.IsShellDesktopIdentity(
            false,
            "WorkerW",
            false,
            true));
        Assert(!Win32DesktopWindowOperations.IsShellDesktopIdentity(
            false,
            "WorkerW",
            true,
            false));
        Assert(!Win32DesktopWindowOperations.IsShellDesktopIdentity(
            true,
            "Progman",
            false,
            false));
        Assert(!Win32DesktopWindowOperations.IsShellDesktopIdentity(
            true,
            "UnrelatedWindow",
            false,
            true));
        return Task.CompletedTask;
    }

    private static Task FileDialogNavigationKeepsCapturedWindow()
    {
        var directory = Directory.CreateTempSubdirectory("nxtools-dialog-nav-");
        try
        {
            var operations = new FakeFileDialogWindowOperations
            {
                FileDialogWindowHandle = 77,
                CurrentDirectory = @"C:\Work"
            };
            var captured = FileDialogWindowAdapter.TryCapture(
                new ForegroundContext(77, 78, 7, "notepad", "Open"),
                operations);

            Assert(captured == new FileDialogContext(77, @"C:\Work"));

            var result = FileDialogWindowAdapter.Navigate(
                new NavigateFileDialogCommand(77, directory.FullName),
                operations,
                Directory.Exists);

            Assert(result.Success);
            Assert(operations.LastNavigatedWindowHandle == 77);
            Assert(operations.LastNavigatedDirectory == directory.FullName);

            var wrongWindow = FileDialogWindowAdapter.Navigate(
                new NavigateFileDialogCommand(99, directory.FullName),
                operations,
                Directory.Exists);
            Assert(!wrongWindow.Success);
            Assert(operations.LastNavigatedWindowHandle == 77);

            operations.IsFileDialog = false;
            Assert(FileDialogWindowAdapter.TryCapture(
                new ForegroundContext(77, 78, 7, "regedit", "Find"),
                operations) is null);
        }
        finally
        {
            directory.Delete();
        }
        return Task.CompletedTask;
    }

    private static Task FileDialogSearchSubmitsSelectedFile()
    {
        var file = Path.GetTempFileName();
        try
        {
            var operations = new FakeFileDialogWindowOperations
            {
                FileDialogWindowHandle = 77,
                CurrentDirectory = Path.GetDirectoryName(file)
            };
            var result = FileDialogWindowAdapter.OpenFile(
                new OpenFileInDialogCommand(77, file),
                operations,
                File.Exists);

            Assert(result.Success);
            Assert(operations.LastOpenedWindowHandle == 77);
            Assert(operations.LastOpenedFile == file);

            var wrongDialog = FileDialogWindowAdapter.OpenFile(
                new OpenFileInDialogCommand(99, file),
                operations,
                File.Exists);
            Assert(!wrongDialog.Success);
        }
        finally
        {
            File.Delete(file);
        }
        return Task.CompletedTask;
    }

    private static Task FileDialogRecognitionUsesStructuralMarkers()
    {
        const uint wsChild = 0x40000000;
        var instance = GetModuleHandle(null);
        var fileDialog = CreateWindowEx(
            0, "#32770", "Open", 0, 0, 0, 400, 300,
            IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        var unrelatedDialog = CreateWindowEx(
            0, "#32770", "Find", 0, 0, 0, 400, 300,
            IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        var fileNameOnlyDialog = CreateWindowEx(
            0, "#32770", "Rename", 0, 0, 0, 400, 300,
            IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        Assert(fileDialog != IntPtr.Zero);
        Assert(unrelatedDialog != IntPtr.Zero);
        Assert(fileNameOnlyDialog != IntPtr.Zero);
        try
        {
            var fileName = CreateWindowEx(
                0, "Edit", string.Empty, wsChild, 0, 0, 200, 24,
                fileDialog, new IntPtr(1152), instance, IntPtr.Zero);
            var fileType = CreateWindowEx(
                0, "ComboBox", string.Empty, wsChild, 0, 30, 200, 24,
                fileDialog, new IntPtr(1136), instance, IntPtr.Zero);
            var unrelatedEdit = CreateWindowEx(
                0, "Edit", string.Empty, wsChild, 0, 0, 200, 24,
                unrelatedDialog, new IntPtr(1152), instance, IntPtr.Zero);
            var unrelatedCombo = CreateWindowEx(
                0, "ComboBox", string.Empty, wsChild, 0, 30, 200, 24,
                unrelatedDialog, new IntPtr(99), instance, IntPtr.Zero);
            var fileNameOnlyCombo = CreateWindowEx(
                0, "ComboBoxEx32", string.Empty, wsChild, 0, 0, 200, 24,
                fileNameOnlyDialog, new IntPtr(1148), instance, IntPtr.Zero);
            var fileNameOnlyEdit = CreateWindowEx(
                0, "Edit", string.Empty, wsChild, 0, 0, 180, 20,
                fileNameOnlyCombo, IntPtr.Zero, instance, IntPtr.Zero);
            Assert(fileName != IntPtr.Zero);
            Assert(fileType != IntPtr.Zero);
            Assert(unrelatedEdit != IntPtr.Zero);
            Assert(unrelatedCombo != IntPtr.Zero);
            Assert(fileNameOnlyCombo != IntPtr.Zero);
            Assert(fileNameOnlyEdit != IntPtr.Zero);

            var operations = new Win32FileDialogWindowOperations();
            Assert(operations.TryInspect(fileDialog.ToInt64(), out _));
            Assert(!operations.TryInspect(unrelatedDialog.ToInt64(), out _));
            Assert(!operations.TryInspect(fileNameOnlyDialog.ToInt64(), out _));
        }
        finally
        {
            _ = DestroyWindow(fileNameOnlyDialog);
            _ = DestroyWindow(unrelatedDialog);
            _ = DestroyWindow(fileDialog);
        }
        return Task.CompletedTask;
    }

    private static Task FileDialogNavigationUsesCapturedControls()
    {
        const uint wsChild = 0x40000000;
        var instance = GetModuleHandle(null);
        var fileDialog = CreateWindowEx(
            0, "#32770", "Open", 0, 0, 0, 400, 300,
            IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        Assert(fileDialog != IntPtr.Zero);
        var directory = Directory.CreateTempSubdirectory("nxtools-dialog-native-");
        try
        {
            var fileName = CreateWindowEx(
                0, "Edit", string.Empty, wsChild, 0, 0, 200, 24,
                fileDialog, new IntPtr(1152), instance, IntPtr.Zero);
            var fileType = CreateWindowEx(
                0, "ComboBox", string.Empty, wsChild, 0, 30, 200, 24,
                fileDialog, new IntPtr(1136), instance, IntPtr.Zero);
            Assert(fileName != IntPtr.Zero);
            Assert(fileType != IntPtr.Zero);

            long activatedWindow = 0;
            long focusedWindow = 0;
            var operations = new Win32FileDialogWindowOperations(
                (windowHandle, focusHandle) =>
                {
                    activatedWindow = windowHandle;
                    focusedWindow = focusHandle;
                    return true;
                },
                (_, targetDirectory) =>
                    targetDirectory == directory.FullName.TrimEnd('\\') + "\\");
            if (!operations.TryNavigate(
                    fileDialog.ToInt64(), directory.FullName, out var error))
            {
                throw new InvalidOperationException(error);
            }

            var text = new StringBuilder(1024);
            _ = GetWindowText(fileName, text, text.Capacity);
            Assert(text.ToString() == directory.FullName.TrimEnd('\\') + "\\");
            Assert(activatedWindow == fileDialog.ToInt64());
            Assert(focusedWindow == fileName.ToInt64());

            var incompleteNavigation = new Win32FileDialogWindowOperations(
                (_, _) => true,
                (_, _) => false);
            Assert(!incompleteNavigation.TryNavigate(
                fileDialog.ToInt64(),
                directory.FullName,
                out var incompleteError));
            Assert(incompleteError?.Contains("did not switch", StringComparison.Ordinal) == true);
        }
        finally
        {
            directory.Delete();
            _ = DestroyWindow(fileDialog);
        }
        return Task.CompletedTask;
    }

    private static Task ModernFileDialogNavigationUsesAddressEdit()
    {
        const uint wsChild = 0x40000000;
        var instance = GetModuleHandle(null);
        var breadcrumbClass = RegisterTestWindowClass("Breadcrumb Parent", instance);
        Assert(breadcrumbClass != 0);
        var fileDialog = CreateWindowEx(
            0, "#32770", "Save As", 0, 0, 0, 400, 300,
            IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        Assert(fileDialog != IntPtr.Zero);
        var directory = Directory.CreateTempSubdirectory("nxtools-dialog-address-");
        try
        {
            var breadcrumb = CreateWindowEx(
                0, "Breadcrumb Parent", string.Empty, wsChild, 0, 0, 300, 28,
                fileDialog, IntPtr.Zero, instance, IntPtr.Zero);
            var fileNameCombo = CreateWindowEx(
                0, "ComboBoxEx32", string.Empty, wsChild, 0, 220, 260, 24,
                fileDialog, new IntPtr(1148), instance, IntPtr.Zero);
            var fileName = CreateWindowEx(
                0, "Edit", "draft.txt", wsChild, 0, 0, 250, 22,
                fileNameCombo, IntPtr.Zero, instance, IntPtr.Zero);
            var addressEdit = CreateWindowEx(
                0, "Edit", string.Empty, wsChild, 0, 0, 280, 22,
                breadcrumb, new IntPtr(41477), instance, IntPtr.Zero);
            Assert(breadcrumb != IntPtr.Zero);
            Assert(fileNameCombo != IntPtr.Zero);
            Assert(fileName != IntPtr.Zero);
            Assert(addressEdit != IntPtr.Zero);

            long focusedWindow = 0;
            var operations = new Win32FileDialogWindowOperations(
                (_, focusHandle) =>
                {
                    focusedWindow = focusHandle;
                    return true;
                },
                (_, targetDirectory) =>
                    targetDirectory == directory.FullName.TrimEnd('\\') + "\\",
                (windowHandle, fileNameHandle, breadcrumbHandle) =>
                {
                    Assert(windowHandle == fileDialog.ToInt64());
                    Assert(fileNameHandle == fileName);
                    Assert(breadcrumbHandle == breadcrumb);
                    return addressEdit;
                });

            if (!operations.TryNavigate(
                    fileDialog.ToInt64(), directory.FullName, out var error))
            {
                throw new InvalidOperationException(error);
            }

            var fileNameText = new StringBuilder(1024);
            var addressText = new StringBuilder(1024);
            _ = GetWindowText(fileName, fileNameText, fileNameText.Capacity);
            _ = GetWindowText(addressEdit, addressText, addressText.Capacity);
            Assert(fileNameText.ToString() == "draft.txt");
            Assert(addressText.ToString() == directory.FullName.TrimEnd('\\') + "\\");
            Assert(focusedWindow == addressEdit.ToInt64());
        }
        finally
        {
            directory.Delete();
            _ = DestroyWindow(fileDialog);
            _ = UnregisterClass("Breadcrumb Parent", instance);
        }
        return Task.CompletedTask;
    }

    private static Task FileDialogPathCaptureAvoidsFilesystemValidation()
    {
        var missing = Path.Combine(
            Path.GetTempPath(),
            "nxtools-missing-dialog-path",
            Guid.NewGuid().ToString("N"));
        Assert(!Directory.Exists(missing));
        Assert(Win32FileDialogWindowOperations.NormalizeBreadcrumbText(missing) == missing);
        Assert(Win32FileDialogWindowOperations.NormalizeBreadcrumbText(
            @"Address: \\offline-server\share\folder") ==
            @"\\offline-server\share\folder");
        return Task.CompletedTask;
    }

    private static Task RealExplorerShellWindowResolvesExactHwnd()
    {
        var explorerWindow = FindExplorerWindowHandle();
        if (explorerWindow == 0)
        {
            Console.WriteLine("SKIP no interactive Explorer window is available");
            return Task.CompletedTask;
        }

        string? error = null;
        var resolved = StaDialogRunner.Run(
            "NxTools.ExplorerDispatchRegression",
            () => new ComExplorerWindowOperations().TryResolveWindow(explorerWindow, out error)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, error));

        if (!resolved.Success)
        {
            throw new InvalidOperationException(resolved.Error);
        }
        var operations = new ComExplorerWindowOperations();
        if (!operations.TryGetCurrentDirectory(
                explorerWindow, out var currentDirectory, out var pathError) ||
            string.IsNullOrWhiteSpace(currentDirectory))
        {
            throw new InvalidOperationException(pathError ?? "Explorer did not expose a filesystem path.");
        }
        var navigate = ExplorerWindowAdapter.Navigate(
            new NavigateExplorerCommand(explorerWindow, currentDirectory),
            operations,
            Directory.Exists);
        if (!navigate.Success)
        {
            throw new InvalidOperationException(navigate.Error);
        }
        return Task.CompletedTask;
    }

    private static long FindExplorerWindowHandle()
    {
        long match = 0;
        _ = EnumWindows((windowHandle, _) =>
        {
            var className = new StringBuilder(256);
            _ = GetClassName(windowHandle, className, className.Capacity);
            if (className.ToString() is "CabinetWClass" or "ExploreWClass")
            {
                match = windowHandle.ToInt64();
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return match;
    }

    private delegate bool EnumWindowsCallback(IntPtr windowHandle, IntPtr parameter);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumWindows(EnumWindowsCallback callback, IntPtr parameter);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr windowHandle,
        StringBuilder className,
        int maxCount);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(
        IntPtr windowHandle,
        StringBuilder text,
        int maxCount);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr GetModuleHandle(string? moduleName);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateWindowEx(
        uint extendedStyle,
        string className,
        string windowName,
        uint style,
        int x,
        int y,
        int width,
        int height,
        IntPtr parent,
        IntPtr menu,
        IntPtr instance,
        IntPtr parameter);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DestroyWindow(IntPtr windowHandle);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr windowHandle);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsIconic(IntPtr windowHandle);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsZoomed(IntPtr windowHandle);

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    private static readonly WindowProcedure TestWindowProcedure = DefWindowProc;

    private static ushort RegisterTestWindowClass(string className, IntPtr instance)
    {
        var windowClass = new WindowClassEx
        {
            Size = (uint)Marshal.SizeOf<WindowClassEx>(),
            WindowProcedure = Marshal.GetFunctionPointerForDelegate(TestWindowProcedure),
            Instance = instance,
            ClassName = className
        };
        return RegisterClassEx(ref windowClass);
    }

    private delegate IntPtr WindowProcedure(
        IntPtr windowHandle,
        uint message,
        IntPtr wParam,
        IntPtr lParam);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct WindowClassEx
    {
        public uint Size;
        public uint Style;
        public IntPtr WindowProcedure;
        public int ClassExtra;
        public int WindowExtra;
        public IntPtr Instance;
        public IntPtr Icon;
        public IntPtr Cursor;
        public IntPtr Background;
        public string? MenuName;
        public string ClassName;
        public IntPtr SmallIcon;
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern ushort RegisterClassEx(ref WindowClassEx windowClass);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool UnregisterClass(string className, IntPtr instance);

    [DllImport("user32.dll")]
    private static extern IntPtr DefWindowProc(
        IntPtr windowHandle,
        uint message,
        IntPtr wParam,
        IntPtr lParam);

    private static Task DoubleControlGestureRequiresTwoReleasedTaps()
    {
        var detector = new ModifierDoubleTapDetector(0xA2, 60, 800);

        Assert(!detector.OnKeyDown(0xA2, 1_000));
        Assert(!detector.OnKeyUp(0xA2));
        Assert(!detector.OnKeyDown(0xA2, 1_240));
        Assert(detector.OnKeyUp(0xA2));

        detector.Reset();
        Assert(!detector.OnKeyDown(0xA2, 2_000));
        Assert(!detector.OnKeyUp(0xA2));
        Assert(!detector.OnKeyDown(0xA2, 3_100));
        return Task.CompletedTask;
    }

    private static Task DoubleControlGestureRejectsRepeatAndInterveningKeys()
    {
        var detector = new ModifierDoubleTapDetector(0xA2, 60, 800);

        Assert(!detector.OnKeyDown(0xA2, 1_000));
        Assert(!detector.OnKeyDown(0xA2, 1_200));
        Assert(!detector.OnKeyUp(0xA2));
        Assert(!detector.OnKeyDown(0xA2, 1_300));
        Assert(!detector.OnKeyDown(0xA2, 1_450));
        Assert(detector.OnKeyUp(0xA2));
        Assert(!detector.OnKeyDown(0xA2, 1_600));

        detector.Reset();
        Assert(!detector.OnKeyDown(0xA2, 2_000));
        Assert(!detector.OnKeyUp(0xA2));
        Assert(!detector.OnKeyDown(0x41, 2_100));
        Assert(!detector.OnKeyDown(0xA2, 2_200));
        return Task.CompletedTask;
    }

    private static Task ClipboardHotKeyUses350MsDebounce()
    {
        var debouncer = new ClipboardHotKeyDebouncer();
        Assert(debouncer.TryAccept(1_000));
        Assert(!debouncer.TryAccept(1_349));
        Assert(debouncer.TryAccept(1_350));
        Assert(debouncer.TryAccept(900));
        Assert(ClipboardHotKeyDebouncer.DebounceMilliseconds == 350);
        return Task.CompletedTask;
    }

    private static Task HotKeyRecordingReleasesAndRestoresShortcut()
    {
        var registrar = new FakeClipboardHotKeyRegistrar();
        var registration = new ClipboardHotKeyRegistration(
            registrar,
            new ClipboardHotKeyBinding(0x0001, 0x43, "Alt+C"));

        Assert(registration.Initialize().Success);
        Assert(registration.IsAvailable);
        Assert(registration.BeginRecording().Success);
        Assert(registration.IsRecording);
        Assert(!registration.IsAvailable);
        Assert(registrar.UnregisterCalls == 1);

        Assert(registration.CancelRecording().Success);
        Assert(!registration.IsRecording);
        Assert(registration.IsAvailable);
        Assert(registrar.RegisteredBindings.SequenceEqual([
            (0x0001U, 0x43U),
            (0x0001U, 0x43U)
        ]));
        return Task.CompletedTask;
    }

    private static Task TopmostCommandPreservesNativeWindow()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_window_topmost",
            Data = new
            {
                WindowHandle = 777L,
                Enabled = true
            }
        });

        var command = PlatformCommandParser.ParseSetWindowTopmost(envelope);
        Assert(command.WindowHandle == 777L);
        Assert(command.Enabled);
        return Task.CompletedTask;
    }

    private static Task VisibilityCommandPreservesNativeWindow()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_window_visibility",
            Data = new
            {
                WindowHandle = 778L,
                Visible = false
            }
        });

        var command = PlatformCommandParser.ParseSetWindowVisibility(envelope);
        Assert(command.WindowHandle == 778L);
        Assert(!command.Visible);
        return Task.CompletedTask;
    }

    private static Task DesktopShellAndTrayContractsRoundTrip()
    {
        var commandEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "configure_desktop_shell",
            Data = new
            {
                StartOnLoginSilently = true,
                TrayIconVisible = true
            }
        });
        var command = PlatformCommandParser.ParseConfigureDesktopShell(commandEnvelope);
        Assert(command.StartOnLoginSilently);
        Assert(command.TrayIconVisible);

        var correlationId = Guid.NewGuid();
        var eventEnvelope = PlatformEventEnvelopeFactory.Create(
            new TrayActionRequestedEvent(correlationId, "open"));
        var json = JsonSerializer.Serialize(eventEnvelope, JsonDefaults.Options);
        Assert(json.Contains("\"event\":\"tray_action_requested\"", StringComparison.Ordinal));
        Assert(json.Contains("\"action\":\"open\"", StringComparison.Ordinal));
        Assert(eventEnvelope.CorrelationId == correlationId);
        return Task.CompletedTask;
    }

    private static Task DesktopShellUsesRealWindowsBoundaries()
    {
        var keyName = $@"Software\NxTools\Tests\DesktopShell\{Guid.NewGuid():N}";
        using var runKey = Registry.CurrentUser.CreateSubKey(keyName, writable: true)
            ?? throw new InvalidOperationException("Could not create isolated startup registry key.");
        try
        {
            var executable = Environment.ProcessPath
                ?? throw new InvalidOperationException("Test executable path is unavailable.");
            runKey.SetValue("NxTools", "\"C:\\Legacy\\NxTools.exe\" --silent-start");
            var enabled = WindowsPlatformAdapter.ApplyStartupRegistration(runKey, executable, true);
            Assert(enabled.Success);
            Assert((string?)runKey.GetValue("NxTools.Next") == $"\"{executable}\" --silent-start");
            Assert(runKey.GetValue("NxTools") is null);
            runKey.SetValue("NxTools", "\"C:\\Legacy\\NxTools.exe\" --silent-start");
            var disabled = WindowsPlatformAdapter.ApplyStartupRegistration(runKey, executable, false);
            Assert(disabled.Success);
            Assert(runKey.GetValue("NxTools.Next") is null);
            Assert(runKey.GetValue("NxTools") is null);

            var opened = 0;
            var exited = 0;
            using var controller = new DesktopTrayController((_, open, exit) =>
                WindowsPlatformAdapter.CreateTrayIconForTest(
                    open,
                    exit,
                    () => new ContextMenuStrip(),
                    () => new NotifyIcon()));
            var configured = controller.Configure(true, executable, () => opened++, () => exited++);
            Assert(configured.Success);
            var tray = controller.TrayIcon ?? throw new InvalidOperationException("tray missing");
            var firstMenu = tray.ContextMenuStrip
                ?? throw new InvalidOperationException("tray menu missing");
            Assert(firstMenu.Items.Count == 3);
            Assert(tray.Visible);
            Assert(tray.Text == DesktopTrayPresentation.Tooltip);
            Assert(firstMenu.Items[0].Text == DesktopTrayPresentation.OpenLabel);
            Assert(firstMenu.Items[1] is ToolStripSeparator);
            Assert(firstMenu.Items[2].Text == DesktopTrayPresentation.ExitLabel);
            Assert(tray.Icon is not null);

            var reconfigured = controller.Configure(true, executable, () => opened++, () => exited++);
            Assert(reconfigured.Success);
            Assert(ReferenceEquals(tray, controller.TrayIcon));

            var mouseClick = typeof(NotifyIcon).GetMethod(
                "OnMouseClick",
                System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)
                ?? throw new InvalidOperationException("NotifyIcon mouse boundary unavailable");
            mouseClick.Invoke(tray, [new MouseEventArgs(MouseButtons.Left, 1, 0, 0, 0)]);
            firstMenu.Items[0].PerformClick();
            firstMenu.Items[2].PerformClick();
            Assert(opened == 2);
            Assert(exited == 1);

            var disabledTray = controller.Configure(false, executable, () => opened++, () => exited++);
            Assert(disabledTray.Success);
            Assert(controller.TrayIcon is null);
            Assert(firstMenu.IsDisposed);
        }
        finally
        {
            Registry.CurrentUser.DeleteSubKeyTree(keyName, throwOnMissingSubKey: false);
        }
        return Task.CompletedTask;
    }

    private static Task StartupRegistrationResolvesCurrentDesktopPath()
    {
        var root = Path.Combine(
            Path.GetTempPath(),
            $"nxtools-startup-path-{Guid.NewGuid():N}");
        var protectedDesktop = Path.Combine(
            root,
            "ProgramData",
            "NxTools",
            "NxTools.Next",
            "Service",
            "NxTools.exe");
        var configuredDesktop = Path.Combine(root, "selected", "NxTools.exe");
        Directory.CreateDirectory(Path.GetDirectoryName(protectedDesktop)!);
        Directory.CreateDirectory(Path.GetDirectoryName(configuredDesktop)!);
        File.WriteAllText(protectedDesktop, "protected desktop");
        File.WriteAllText(configuredDesktop, "configured desktop");
        var keyName = $@"Software\NxTools\Tests\DesktopShell\{Guid.NewGuid():N}";
        try
        {
            Assert(WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                configuredDesktop,
                protectedDesktop,
                candidate => candidate == configuredDesktop) == configuredDesktop);
            Assert(WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                configuredDesktop,
                protectedDesktop,
                candidate => candidate == protectedDesktop) == protectedDesktop);
            Assert(WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                configuredDesktop,
                protectedDesktop,
                _ => false) is null);

            using var runKey = Registry.CurrentUser.CreateSubKey(keyName, writable: true)
                ?? throw new InvalidOperationException("Could not create isolated startup registry key.");
            Assert(runKey.GetValue("NxTools.Next") is null);
            var enabled = WindowsPlatformAdapter.ApplyStartupRegistration(
                runKey,
                protectedDesktop,
                true);
            Assert(enabled.Success);
            Assert((string?)runKey.GetValue("NxTools.Next") ==
                $"\"{protectedDesktop}\" --silent-start");
        }
        finally
        {
            Registry.CurrentUser.DeleteSubKeyTree(keyName, throwOnMissingSubKey: false);
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task DesktopTrayRequiresProductIcon()
    {
        using var controller = new DesktopTrayController();
        var iconlessExecutable = Path.Combine(Environment.SystemDirectory, "where.exe");
        Assert(File.Exists(iconlessExecutable));

        var result = controller.Configure(true, iconlessExecutable, () => { }, () => { });

        Assert(!result.Success);
        Assert(result.NativeErrorCode == DesktopExecutableIcon.ErrorResourceTypeNotFound);
        Assert(controller.TrayIcon is null);
        return Task.CompletedTask;
    }

    private static Task TrayFactoryFailureDisposesPartialResources()
    {
        ContextMenuStrip? menu = null;
        try
        {
            _ = WindowsPlatformAdapter.CreateTrayIconForTest(
                () => { },
                () => { },
                () => menu = new ContextMenuStrip(),
                () => throw new InvalidOperationException("injected tray construction failure"));
            throw new InvalidOperationException("Tray factory unexpectedly accepted an injected failure.");
        }
        catch (InvalidOperationException error) when (
            error.Message == "injected tray construction failure")
        {
            Assert(menu is not null && menu.IsDisposed);
        }
        return Task.CompletedTask;
    }

    private static Task TitlebarMoveUsesRealNativeWindow()
    {
        const uint wsOverlappedWindow = 0x00CF0000;
        var instance = GetModuleHandle(null);
        var window = CreateWindowEx(
            0, "#32770", "NxTools titlebar move test", wsOverlappedWindow,
            0, 0, 320, 200, IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        Assert(window != IntPtr.Zero);
        try
        {
            var result = WindowsPlatformAdapter.BeginNativeWindowMoveForTest(window.ToInt64());
            Assert(result.Success);
            Assert(!WindowsPlatformAdapter.BeginNativeWindowMoveForTest(0).Success);
        }
        finally
        {
            _ = DestroyWindow(window);
        }
        return Task.CompletedTask;
    }

    private static Task TitlebarMaximizeToggleUsesRealNativeWindow()
    {
        const uint wsOverlappedWindow = 0x00CF0000;
        var instance = GetModuleHandle(null);
        var window = CreateWindowEx(
            0, "#32770", "NxTools maximize test", wsOverlappedWindow,
            0, 0, 320, 200, IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        Assert(window != IntPtr.Zero);
        try
        {
            Assert(WindowsPlatformAdapter.ToggleNativeWindowMaximizedForTest(window.ToInt64()).Success);
            Assert(IsZoomed(window));
            Assert(WindowsPlatformAdapter.ToggleNativeWindowMaximizedForTest(window.ToInt64()).Success);
            Assert(!IsZoomed(window));
        }
        finally
        {
            _ = DestroyWindow(window);
        }
        return Task.CompletedTask;
    }

    private static Task MinimizedCommandUsesRealNativeWindow()
    {
        const uint wsOverlappedWindow = 0x00CF0000;
        var instance = GetModuleHandle(null);
        var window = CreateWindowEx(
            0, "#32770", "NxTools startup test", wsOverlappedWindow,
            0, 0, 320, 200, IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        Assert(window != IntPtr.Zero);
        try
        {
            var envelope = ProtocolEnvelope.Create("platform_command", new
            {
                Command = "set_window_minimized",
                Data = new { WindowHandle = window.ToInt64() }
            });
            var command = PlatformCommandParser.ParseSetWindowMinimized(envelope);
            var foregroundBefore = GetForegroundWindow();
            var result = WindowsPlatformAdapter.MinimizeNativeWindowForTest(command.WindowHandle);

            Assert(result.Success);
            Assert(IsWindowVisible(window));
            Assert(IsIconic(window));
            Assert(GetForegroundWindow() == foregroundBefore);
        }
        finally
        {
            _ = DestroyWindow(window);
        }
        return Task.CompletedTask;
    }

    private static Task NoActivateCommandPreservesNativeWindow()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_window_no_activate",
            Data = new
            {
                WindowHandle = 779L,
                Enabled = true
            }
        });

        var command = PlatformCommandParser.ParseSetWindowNoActivate(envelope);
        Assert(command.WindowHandle == 779L);
        Assert(command.Enabled);
        return Task.CompletedTask;
    }

    private static Task WindowBoundsCommandPreservesPlacement()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_window_bounds",
            Data = new
            {
                WindowHandle = 780L,
                X = 120,
                Y = 240,
                Width = 400,
                Height = 410
            }
        });

        var command = PlatformCommandParser.ParseSetWindowBounds(envelope);
        Assert(command.WindowHandle == 780L);
        Assert(command.X == 120 && command.Y == 240);
        Assert(command.Width == 400 && command.Height == 410);
        return Task.CompletedTask;
    }

    private static Task CenteredRectanglePreservesAnchorCenter()
    {
        // The download task window phases resize in place; the new rectangle
        // must keep the anchor's visual center stable across transitions.
        var anchor = new NativeWindowRectangle(100, 200, 560, 676);
        var centered = WindowsPlatformAdapter.CenteredRectangleForTest(anchor, 440, 196);
        Assert(centered.Left == 110 && centered.Top == 340);
        Assert(centered.Width == 440 && centered.Height == 196);
        Assert((centered.Left + centered.Right) / 2 == (anchor.Left + anchor.Right) / 2);
        Assert((centered.Top + centered.Bottom) / 2 == (anchor.Top + anchor.Bottom) / 2);

        var odd = WindowsPlatformAdapter.CenteredRectangleForTest(
            new NativeWindowRectangle(0, 0, 101, 51), 20, 10);
        Assert(odd.Left == 40 && odd.Top == 20);
        return Task.CompletedTask;
    }

    private static Task AnchoredWindowPlacementCentersVisibleContent()
    {
        var placement = AnchoredWindowPlacement.Calculate(
            new NativeWindowRectangle(100, 200, 1_000, 840),
            new NativeWindowRectangle(0, 0, 516, 52),
            new NativeWindowRectangle(8, 8, 508, 44),
            visibleWidth: 500,
            visibleHeight: 36,
            bottomOverlap: 0);

        Assert(placement == new NativeWindowRectangle(292, 832, 808, 884));
        Assert((placement.Left + 8 + placement.Right - 8) / 2 ==
            (100 + 1_000) / 2);
        return Task.CompletedTask;
    }

    private static Task AnchoredWindowFollowReusesCalibratedInsets()
    {
        var placement = AnchoredWindowPlacement.Calculate(
            new NativeWindowRectangle(340, 280, 1_540, 920),
            new NativeWindowInsets(8, 8, 8, 8),
            visibleWidth: 500,
            visibleHeight: 72,
            bottomOverlap: 0);

        Assert(placement == new NativeWindowRectangle(682, 912, 1_198, 1_000));
        return Task.CompletedTask;
    }

    private static Task SingleImagePathIsPromoted()
    {
        var root = Path.Combine(Path.GetTempPath(), "nxtools-image-promotion-tests", Guid.NewGuid().ToString("N"));
        var staging = Path.Combine(root, "staging");
        Directory.CreateDirectory(root);
        try
        {
            var source = Path.Combine(root, "qq-drop-without-trusted-extension.dat");
            using (var bitmap = new System.Drawing.Bitmap(9, 7))
            {
                bitmap.Save(source, System.Drawing.Imaging.ImageFormat.Png);
            }

            var result = ClipboardImagePromotion.TryPromote([source], null, staging);
            var promoted = result ?? throw new InvalidOperationException("image path was not promoted");
            Assert(promoted.RemainingPaths.Length == 0);
            Assert(promoted.Image.Width == 9 && promoted.Image.Height == 7);
            Assert(File.Exists(promoted.Image.StagingPath));

            var notImage = Path.Combine(root, "notes.txt");
            File.WriteAllText(notImage, "not an image");
            Assert(ClipboardImagePromotion.TryPromote([notImage], null, staging) is null);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task PureImageCaptureRequiresCompatibilityFallbacks()
    {
        var pixPinFormats = new CapturedClipboardNativeFormat[]
        {
            new(8, "", [1], 0),
            new(17, "", [2], 1),
            new(49_200, "PixPinData", [3], 2)
        };
        Assert(ClipboardImageCompatibility.RequiresPngFallback(
            hasImage: true, hasText: false, fileCount: 0, pixPinFormats));

        var withPng = pixPinFormats.Append(
            new CapturedClipboardNativeFormat(49_201, "PNG", [4], 3)).ToArray();
        Assert(!ClipboardImageCompatibility.RequiresPngFallback(
            hasImage: true, hasText: false, fileCount: 0, withPng));
        Assert(!ClipboardImageCompatibility.RequiresPngFallback(
            hasImage: true, hasText: true, fileCount: 0, pixPinFormats));
        Assert(!ClipboardImageCompatibility.RequiresPngFallback(
            hasImage: true, hasText: false, fileCount: 1, pixPinFormats));
        Assert(ClipboardImageCompatibility.RequiresFileDropFallback(
            hasImage: true, hasText: false, fileCount: 0, pixPinFormats));

        var withFileDrop = pixPinFormats.Append(
            new CapturedClipboardNativeFormat(15, "", [5], 3)).ToArray();
        Assert(!ClipboardImageCompatibility.RequiresFileDropFallback(
            hasImage: true, hasText: false, fileCount: 0, withFileDrop));

        var storedFormats = pixPinFormats.ToList();
        ClipboardImageCompatibility.AddPngFallback(storedFormats, 49_201, [4, 5]);
        Assert(storedFormats[0].FormatName == "PNG");
        Assert(storedFormats[0].SortOrder == 0);
        Assert(storedFormats[1].SortOrder == 1);
        Assert(storedFormats[3].FormatName == "PixPinData");
        return Task.CompletedTask;
    }

    private static Task FileDropFallbackUsesUnicodeDropFilesPayload()
    {
        const string path = @"C:\图片\capture.png";
        var payload = ClipboardFileDropPayload.Encode([path]);

        Assert(BitConverter.ToUInt32(payload, 0) == 20);
        Assert(BitConverter.ToInt32(payload, 16) == 1);
        var decoded = Encoding.Unicode.GetString(payload, 20, payload.Length - 20);
        Assert(decoded == path + "\0\0");
        return Task.CompletedTask;
    }

    private static Task PureImageFormatsGainBothFallbacks()
    {
        var formats = new List<CapturedClipboardNativeFormat>
        {
            new(8, "", [1], 0),
            new(17, "", [2], 1)
        };
        ClipboardImageCompatibility.AddCompatibilityFallbacks(
            formats, pngFormatId: 49_201, pngData: [3, 4], @"C:\images\capture.png");

        Assert(formats.Any(format => format.FormatName == "PNG"));
        var fileDrop = formats.Single(format => format.FormatId == 15);
        Assert(Encoding.Unicode.GetString(fileDrop.Data, 20, fileDrop.Data.Length - 20)
            == "C:\\images\\capture.png\0\0");
        return Task.CompletedTask;
    }

    private static Task HistoryImageExposesCompatibilityFormats()
    {
        var root = Path.Combine(Path.GetTempPath(), "nxtools-image-data-tests", Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            var path = Path.Combine(root, "history.jpg");
            using (var bitmap = new System.Drawing.Bitmap(11, 7))
            {
                bitmap.Save(path, System.Drawing.Imaging.ImageFormat.Jpeg);
            }

            using var imageData = ClipboardImageDataObject.Load(path);
            Assert(imageData.Data.GetDataPresent("PNG", autoConvert: false));
            Assert(imageData.Data.GetDataPresent(DataFormats.Bitmap, autoConvert: true));
            Assert(imageData.Data.GetDataPresent(DataFormats.FileDrop, autoConvert: true));
            var paths = imageData.Data.GetData(DataFormats.FileDrop, autoConvert: true) as string[]
                ?? throw new InvalidOperationException("file drop fallback was not exported");
            Assert(paths.Length == 1 && paths[0] == path);
            var png = imageData.Data.GetData("PNG", autoConvert: false) as Stream
                ?? throw new InvalidOperationException("PNG stream was not exported");
            Span<byte> signature = stackalloc byte[8];
            Assert(png.Read(signature) == signature.Length);
            Assert(signature.SequenceEqual(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }));
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task RichImageTargetOmitsFileDropFormat()
    {
        var root = Path.Combine(Path.GetTempPath(), "nxtools-image-data-tests", Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            var path = Path.Combine(root, "history.png");
            using (var bitmap = new System.Drawing.Bitmap(11, 7))
            {
                bitmap.Save(path, System.Drawing.Imaging.ImageFormat.Png);
            }

            using var imageData = ClipboardImageDataObject.Load(path, includeFileDrop: false);
            Assert(imageData.Data.GetDataPresent("PNG", autoConvert: false));
            Assert(imageData.Data.GetDataPresent(DataFormats.Bitmap, autoConvert: true));
            Assert(!imageData.Data.GetDataPresent(DataFormats.FileDrop, autoConvert: false));
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task FallbackSequenceRecheckPreventsEmptyingChangedClipboard()
    {
        var openCalls = 0;
        var emptyCalls = 0;
        var closeCalls = 0;
        var sequenceReads = new Queue<uint>([41]);
        var beforeOpenChanged = NativeClipboardFormats.TryOpenAndPrepareClipboardRewrite(
            42,
            () => sequenceReads.Dequeue(),
            () =>
            {
                openCalls++;
                return true;
            },
            () =>
            {
                emptyCalls++;
                return true;
            },
            () => closeCalls++,
            () => (5, "native failure"),
            out var beforeError,
            out _);
        Assert(!beforeOpenChanged);
        Assert(beforeError == "clipboard_changed");
        Assert(openCalls == 0);
        Assert(emptyCalls == 0);
        sequenceReads = new Queue<uint>([42, 43]);
        var afterOpenChanged = NativeClipboardFormats.TryOpenAndPrepareClipboardRewrite(
            42,
            () => sequenceReads.Dequeue(),
            () =>
            {
                openCalls++;
                return true;
            },
            () =>
            {
                emptyCalls++;
                return true;
            },
            () => closeCalls++,
            () => (5, "native failure"),
            out var afterError,
            out _);
        Assert(!afterOpenChanged);
        Assert(afterError == "clipboard_changed");
        Assert(openCalls == 1);
        Assert(emptyCalls == 0);
        Assert(closeCalls == 1);
        return Task.CompletedTask;
    }
    private static Task ImagePathFallbackCleanupIsBounded()
    {
        var root = Path.Combine(
            Path.GetTempPath(), "nxtools-image-fallback-tests", Guid.NewGuid().ToString("N"));
        var sourceDirectory = Path.Combine(root, "source");
        var fallbackDirectory = Path.Combine(root, "fallback");
        Directory.CreateDirectory(sourceDirectory);
        Directory.CreateDirectory(fallbackDirectory);
        try
        {
            var now = new DateTimeOffset(2026, 7, 20, 10, 0, 0, TimeSpan.Zero);
            var source = Path.Combine(sourceDirectory, "capture.png");
            File.WriteAllBytes(source, [137, 80, 78, 71, 13, 10, 26, 10]);
            var stale = Path.Combine(fallbackDirectory, "stale.png");
            var recent = Path.Combine(fallbackDirectory, "recent.png");
            var unrelated = Path.Combine(fallbackDirectory, "keep.txt");
            File.WriteAllText(stale, "stale");
            File.WriteAllText(recent, "recent");
            File.WriteAllText(unrelated, "not managed by the image fallback store");
            File.SetLastWriteTimeUtc(stale, now.AddDays(-8).UtcDateTime);
            File.SetLastWriteTimeUtc(recent, now.AddDays(-1).UtcDateTime);
            File.SetLastWriteTimeUtc(unrelated, now.AddDays(-30).UtcDateTime);

            var current = ClipboardImageFallbackStore.Persist(
                source, fallbackDirectory, now, TimeSpan.FromDays(7));

            Assert(File.Exists(current));
            Assert(!File.Exists(stale));
            Assert(File.Exists(recent));
            Assert(File.Exists(unrelated));
            Assert(File.GetLastWriteTimeUtc(current) == now.UtcDateTime);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task WindowPositionResizeAndShellCommandsPreservePayload()
    {
        var positionEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_window_position",
            Data = new { WindowHandle = 778L, X = 120, Y = 240 }
        });
        var position = PlatformCommandParser.ParseSetWindowPosition(positionEnvelope);
        Assert(position.WindowHandle == 778L && position.X == 120 && position.Y == 240);

        var activationEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "activate_window",
            Data = new { WindowHandle = 778L }
        });
        Assert(PlatformCommandParser.ParseActivateWindow(activationEnvelope).WindowHandle == 778L);
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.WindowActivation));

        var resizeEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "begin_window_resize",
            Data = new { WindowHandle = 779L }
        });
        Assert(PlatformCommandParser.ParseBeginWindowResize(resizeEnvelope).WindowHandle == 779L);

        var moveEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "begin_window_move",
            Data = new { WindowHandle = 780L }
        });
        Assert(PlatformCommandParser.ParseBeginWindowMove(moveEnvelope).WindowHandle == 780L);

        var maximizeEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "toggle_window_maximized",
            Data = new { WindowHandle = 780L }
        });
        Assert(PlatformCommandParser.ParseToggleWindowMaximized(maximizeEnvelope).WindowHandle == 780L);

        var openEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "open_shell_target",
            Data = new { Target = "C:/data/file.txt", Action = "select_in_explorer" }
        });
        var open = PlatformCommandParser.ParseOpenShellTarget(openEnvelope);
        Assert(open.Target == "C:/data/file.txt");
        Assert(open.Action == "select_in_explorer");

        var navigateEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "navigate_directory",
            Data = new
            {
                Host = "explorer",
                WindowHandle = 88L,
                TargetDirectory = @"C:\Work\Reports"
            }
        });
        var navigate = PlatformCommandParser.ParseNavigateDirectory(navigateEnvelope);
        Assert(navigate.Host == "explorer");
        Assert(navigate.WindowHandle == 88L);
        Assert(navigate.TargetDirectory == @"C:\Work\Reports");

        var navigateDialogEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "navigate_directory",
            Data = new
            {
                Host = "file_dialog",
                WindowHandle = 77L,
                TargetDirectory = @"C:\Work\Reports"
            }
        });
        var navigateDialog = PlatformCommandParser.ParseNavigateDirectory(navigateDialogEnvelope);
        Assert(navigateDialog.Host == "file_dialog");
        Assert(navigateDialog.WindowHandle == 77L);
        Assert(navigateDialog.TargetDirectory == @"C:\Work\Reports");
        var navigateDesktopEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "navigate_directory",
            Data = new
            {
                Host = "desktop",
                WindowHandle = 66L,
                TargetDirectory = @"C:\Work\Reports"
            }
        });
        var navigateDesktop =
            PlatformCommandParser.ParseNavigateDirectory(navigateDesktopEnvelope);
        Assert(navigateDesktop.Host == "desktop");
        Assert(navigateDesktop.WindowHandle == 66L);
        var navigateThirdPartyEnvelope = ProtocolEnvelope.Create(
            "platform_command",
            new
            {
                Command = "navigate_directory",
                Data = new
                {
                    Host = "third_party_file_manager",
                    WindowHandle = 55L,
                    TargetDirectory = @"D:\Work",
                    HostContextId = "pane-context-55"
                }
            });
        var navigateThirdParty =
            PlatformCommandParser.ParseNavigateDirectory(navigateThirdPartyEnvelope);
        Assert(navigateThirdParty.Host == "third_party_file_manager");
        Assert(navigateThirdParty.WindowHandle == 55L);
        Assert(navigateThirdParty.HostContextId == "pane-context-55");
        Assert(PlatformCapabilities.Create(false).Contains(
            PlatformCapabilities.DirectoryNavigation));
        return Task.CompletedTask;
    }

    private static Task TypedProcessLaunchPreservesArgumentsAndFlags()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "launch_process",
            Data = new
            {
                Executable = @"C:\Program Files\Tool\tool.exe",
                Arguments = new[] { "--name", "two words", "quoted \"value\"" },
                WorkingDirectory = @"D:\Work",
                RunAsAdmin = true,
                RunSilently = true
            }
        });
        var command = PlatformCommandParser.ParseLaunchProcess(envelope);
        Assert(command.Executable == @"C:\Program Files\Tool\tool.exe");
        Assert(command.Arguments.SequenceEqual(new[] { "--name", "two words", "quoted \"value\"" }));

        var startInfo = WindowsPlatformAdapter.CreateProcessStartInfo(command);
        Assert(startInfo.UseShellExecute);
        Assert(startInfo.Verb == "runas");
        Assert(startInfo.WindowStyle == ProcessWindowStyle.Hidden);
        Assert(startInfo.WorkingDirectory == @"D:\Work");
        Assert(startInfo.ArgumentList.SequenceEqual(command.Arguments));
        return Task.CompletedTask;
    }

    private static Task ClipboardArchivePickerPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "pick_clipboard_archive_path",
            Data = new
            {
                Mode = "export",
                SuggestedFileName = "NxTools-clipboard.nxtclip"
            }
        });

        var command = PlatformCommandParser.ParsePickClipboardArchivePath(envelope);
        Assert(command.Mode == "export");
        Assert(command.SuggestedFileName == "NxTools-clipboard.nxtclip");
        return Task.CompletedTask;
    }

    private static Task ClipboardStoragePickerPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "pick_clipboard_storage_directory",
            Data = new { CurrentDirectory = @"D:\NxToolsData\Clipboard" }
        });

        var command = PlatformCommandParser.ParsePickClipboardStorageDirectory(envelope);
        Assert(command.CurrentDirectory == @"D:\NxToolsData\Clipboard");
        return Task.CompletedTask;
    }

    private static Task DirectoryPickerPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "pick_directory",
            Data = new { CurrentDirectory = @"D:\Cache", Title = "选择要排除的目录" }
        });
        var command = PlatformCommandParser.ParsePickDirectory(envelope);
        Assert(command.CurrentDirectory == @"D:\Cache");
        Assert(command.Title == "选择要排除的目录");
        return Task.CompletedTask;
    }

    private static Task ProgramPickerPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "pick_program",
            Data = new { CurrentPath = @"C:\Tools\runner.exe" }
        });
        var command = PlatformCommandParser.ParsePickProgram(envelope);
        Assert(command.CurrentPath == @"C:\Tools\runner.exe");
        return Task.CompletedTask;
    }

    private static Task ShellActionSessionCommandsPreservePayload()
    {
        var queryEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "query_shell_actions",
            Data = new { Target = @"C:\Data\one.txt" }
        });
        var query = PlatformCommandParser.ParseQueryShellActions(queryEnvelope);
        Assert(query.Target == @"C:\Data\one.txt");

        var submenuEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "query_shell_submenu",
            Data = new { SessionId = "session-1", SubmenuId = 7U }
        });
        var submenu = PlatformCommandParser.ParseQueryShellSubmenu(submenuEnvelope);
        Assert(submenu.SessionId == "session-1");
        Assert(submenu.SubmenuId == 7U);

        var invokeEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "invoke_shell_action",
            Data = new { SessionId = "session-1", CommandId = 42U, OwnerWindowHandle = 778L }
        });
        var invoke = PlatformCommandParser.ParseInvokeShellAction(invokeEnvelope);
        Assert(invoke.SessionId == "session-1");
        Assert(invoke.CommandId == 42U);
        Assert(invoke.OwnerWindowHandle == 778L);
        return Task.CompletedTask;
    }

    private static Task ShellActionSessionEnumeratesFilesystemTarget()
    {
        var fixture = Path.Combine(Path.GetTempPath(), $"nxtools-shell-actions-{Guid.NewGuid():N}.txt");
        File.WriteAllText(fixture, "NxTools shell action fixture");
        try
        {
            using var sessions = new ShellContextMenuSessionManager();
            var result = sessions.QueryActions(new QueryShellActionsCommand(fixture));
            Assert(result.Success);
            Assert(!string.IsNullOrWhiteSpace(result.Data));
            using var document = JsonDocument.Parse(result.Data!);
            var root = document.RootElement;
            Assert(Guid.TryParse(root.GetProperty("session_id").GetString(), out _));
            Assert(string.Equals(
                root.GetProperty("navigation_directory").GetString(),
                Path.GetDirectoryName(fixture),
                StringComparison.OrdinalIgnoreCase));
            var items = root.GetProperty("items");
            Assert(items.GetArrayLength() > 4);
            Assert(items.EnumerateArray().Any(item =>
                item.GetProperty("separator").GetBoolean() ||
                !string.IsNullOrWhiteSpace(item.GetProperty("title").GetString())));
            Assert(items.EnumerateArray().Where(item =>
                !item.GetProperty("separator").GetBoolean()).All(item =>
                    item.TryGetProperty("icon_path", out _)));
            var submenu = items.EnumerateArray().FirstOrDefault(item =>
                item.TryGetProperty("submenu_id", out var id) && id.ValueKind == JsonValueKind.Number);
            if (submenu.ValueKind != JsonValueKind.Undefined)
            {
                var submenuResult = sessions.QuerySubmenu(new QueryShellSubmenuCommand(
                    root.GetProperty("session_id").GetString()!,
                    submenu.GetProperty("submenu_id").GetUInt32()));
                Assert(submenuResult.Success);
                using var submenuDocument = JsonDocument.Parse(submenuResult.Data!);
                Assert(submenuDocument.RootElement.GetProperty("session_id").GetString() ==
                       root.GetProperty("session_id").GetString());
            }
        }
        finally
        {
            File.Delete(fixture);
        }
        return Task.CompletedTask;
    }

    private static Task ShellActionBitmapIsStaged()
    {
        using var bitmap = new System.Drawing.Bitmap(
            16, 16, System.Drawing.Imaging.PixelFormat.Format32bppArgb);
        using (var graphics = System.Drawing.Graphics.FromImage(bitmap))
        {
            graphics.Clear(System.Drawing.Color.Transparent);
            graphics.FillRectangle(System.Drawing.Brushes.DeepSkyBlue, 2, 2, 12, 12);
        }
        var handle = bitmap.GetHbitmap(System.Drawing.Color.Transparent);
        try
        {
            var path = ShellMenuIconStager.Stage(handle);
            Assert(!string.IsNullOrWhiteSpace(path));
            Assert(File.Exists(path));
            using var staged = new System.Drawing.Bitmap(path!);
            Assert(staged.Width == 16 && staged.Height == 16);
            Assert(staged.GetPixel(0, 0).A == 0);
            Assert(staged.GetPixel(8, 8).A == 255);
        }
        finally
        {
            _ = ShellContextMenuNativeMethods.DeleteObject(handle);
        }
        return Task.CompletedTask;
    }

    private static Task ShellTargetPolicyAcceptsSafeQuickTargets()
    {
        Assert(ShellTarget.TryCreate(
            "HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Services\\ProfSvc",
            "registry_key", out var registry, out _));
        Assert(registry.Kind == ShellTargetKind.RegistryKey);
        Assert(ShellTarget.TryCreate(
            "msconfig.exe", "system_tool", out var tool, out _));
        Assert(tool.Kind == ShellTargetKind.SystemTool);
        Assert(tool.Target == "msconfig.exe");
        foreach (var launcherTool in new[] { "cmd.exe", "powershell.exe", "regedit.exe", "control.exe" })
        {
            Assert(ShellTarget.TryCreate(
                launcherTool, "system_tool", out var launcherTarget, out _));
            Assert(launcherTarget.Target == launcherTool);
        }
        Assert(!ShellTarget.TryCreate(
            "cmd /c whoami", "system_tool", out _, out _));
        Assert(ShellTarget.TryCreate(
            "https://example.com/docs", "open", out var web, out _));
        Assert(web.Kind == ShellTargetKind.Open);
        Assert(!ShellTarget.TryCreate(
            "powershell.exe", "open", out _, out _));
        var folderToLocate = Path.Combine(Path.GetTempPath(), $"nxtools-locate-{Guid.NewGuid():N}");
        Directory.CreateDirectory(folderToLocate);
        try
        {
            Assert(ShellTarget.TryCreate(
                folderToLocate, "select_in_explorer", out var locatedFolder, out _));
            Assert(locatedFolder.Kind == ShellTargetKind.SelectInExplorer);
            Assert(locatedFolder.Target == folderToLocate);
        }
        finally
        {
            Directory.Delete(folderToLocate);
        }
        var elevatedFixture = Path.Combine(Path.GetTempPath(), $"{Guid.NewGuid():N}.exe");
        File.WriteAllBytes(elevatedFixture, []);
        try
        {
            Assert(ShellTarget.TryCreate(
                elevatedFixture, "run_elevated", out var elevated, out _));
            Assert(elevated.Kind == ShellTargetKind.RunElevated);
            Assert(ShellTarget.TryCreate(
                "powershell.exe", "run_elevated", out var elevatedTool, out _));
            Assert(elevatedTool.Kind == ShellTargetKind.RunElevated);
            Assert(!ShellTarget.TryCreate(
                "https://example.com", "run_elevated", out _, out _));
            var document = Path.ChangeExtension(elevatedFixture, ".txt");
            File.WriteAllText(document, "fixture");
            try
            {
                Assert(!ShellTarget.TryCreate(
                    document, "run_elevated", out _, out _));
            }
            finally
            {
                File.Delete(document);
            }
        }
        finally
        {
            File.Delete(elevatedFixture);
        }
        Assert(ShellTarget.BuildRegistryLastKey(
            "HKEY_LOCAL_MACHINE\\SYSTEM",
            "计算机\\HKEY_CURRENT_USER\\Software") ==
            "计算机\\HKEY_LOCAL_MACHINE\\SYSTEM");
        return Task.CompletedTask;
    }

    private static Task ShellIconCommandPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "extract_shell_icon",
            Data = new { Target = @"C:\Apps\Editor.lnk", Size = 32U }
        });
        var command = PlatformCommandParser.ParseExtractShellIcon(envelope);
        Assert(command.Target == @"C:\Apps\Editor.lnk");
        Assert(command.Size == 32);
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.ShellIconExtraction));
        return Task.CompletedTask;
    }

    private static Task ShellIconExtractionRunsOnDedicatedStaWorker()
    {
        var callerThreadId = Environment.CurrentManagedThreadId;
        var observedThreadId = 0;
        var observedApartment = ApartmentState.Unknown;
        var result = ShellIconExtractor.ExtractOnSta(
            new ExtractShellIconCommand("probe", 32),
            _ =>
            {
                observedThreadId = Environment.CurrentManagedThreadId;
                observedApartment = Thread.CurrentThread.GetApartmentState();
                return new WindowsCommandResult(true, Data: "probe-result");
            });

        Assert(result.Success);
        Assert(result.Data == "probe-result");
        Assert(observedThreadId != 0);
        Assert(observedThreadId != callerThreadId);
        Assert(observedApartment == ApartmentState.STA);
        return Task.CompletedTask;
    }

    private static async Task ShellIconExtractionDoesNotBlockLaterCommands()
    {
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var extractionStarted = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var releaseExtraction = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var followUpRan = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);

        var processing = Task.Run(async () =>
        {
            await dispatcher.DispatchAsync(
                "extract_shell_icon",
                async () =>
                {
                    extractionStarted.TrySetResult();
                    await releaseExtraction.Task;
                });
            await dispatcher.DispatchAsync(
                "set_window_visibility",
                () =>
                {
                    followUpRan.TrySetResult();
                    return Task.CompletedTask;
                });
        });

        await extractionStarted.Task.WaitAsync(TimeSpan.FromSeconds(1));
        try
        {
            var completed = await Task.WhenAny(
                followUpRan.Task,
                Task.Delay(TimeSpan.FromMilliseconds(250)));
            Assert(completed == followUpRan.Task);
        }
        finally
        {
            releaseExtraction.TrySetResult();
            await processing.WaitAsync(TimeSpan.FromSeconds(1));
            await dispatcher.DrainAsync().WaitAsync(TimeSpan.FromSeconds(1));
        }
    }

    private static Task IconCacheReusesIdenticalKeys()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-icon-cache-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        try
        {
            var cache = new IconCache(root);
            var key = IconCacheKey.Create(Path.Combine(root, "App.exe"), null, 32, 12, "file");
            var writes = 0;
            Assert(cache.TryGetOrCreate(key, path =>
            {
                writes++;
                File.WriteAllBytes(path, [1, 2, 3]);
                return true;
            }, out var first));
            Assert(cache.TryGetOrCreate(key, _ =>
            {
                writes++;
                return false;
            }, out var second));
            Assert(first == second);
            Assert(writes == 1);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task IconCacheSeparatesSizeAndSelectorKeys()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-icon-cache-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        try
        {
            var cache = new IconCache(root);
            var target = Path.Combine(root, "App.exe");
            var keys = new[]
            {
                IconCacheKey.Create(target, null, 16, 12, "file"),
                IconCacheKey.Create(target, null, 32, 12, "file"),
                IconCacheKey.Create(target, 1, 32, 12, "file")
            };
            var paths = keys.Select(key =>
            {
                Assert(cache.TryGetOrCreate(key, path =>
                {
                    File.WriteAllBytes(path, [1]);
                    return true;
                }, out var path));
                return path;
            }).ToArray();
            Assert(paths.Distinct(StringComparer.OrdinalIgnoreCase).Count() == keys.Length);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task IconCacheBoundsMemoryLru()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-icon-cache-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        try
        {
            var cache = new IconCache(root);
            for (var index = 0; index < IconCache.MaxMemoryEntries + 1; index++)
            {
                var key = IconCacheKey.Create(
                    Path.Combine(root, $"App-{index}.exe"), null, 32, index, "file");
                Assert(cache.TryGetOrCreate(key, path =>
                {
                    File.WriteAllBytes(path, [1]);
                    return true;
                }, out _));
            }
            Assert(cache.MemoryEntryCount == IconCache.MaxMemoryEntries);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task IconCacheRegeneratesMissingDiskFile()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-icon-cache-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        try
        {
            var cache = new IconCache(root);
            var key = IconCacheKey.Create(Path.Combine(root, "App.exe"), null, 32, 12, "file");
            var writes = 0;
            Assert(cache.TryGetOrCreate(key, path =>
            {
                writes++;
                File.WriteAllBytes(path, [1]);
                return true;
            }, out var path));
            File.Delete(path);
            Assert(cache.TryGetOrCreate(key, path =>
            {
                writes++;
                File.WriteAllBytes(path, [2]);
                return true;
            }, out var regenerated));
            Assert(regenerated == path);
            Assert(writes == 2);
            Assert(File.ReadAllBytes(regenerated).SequenceEqual(new byte[] { 2 }));
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static async Task IconCacheDeduplicatesConcurrentGeneration()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-icon-cache-concurrent-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        using var generationEntered = new ManualResetEventSlim();
        using var releaseGeneration = new ManualResetEventSlim();
        try
        {
            var cache = new IconCache(root);
            var key = IconCacheKey.Create(Path.Combine(root, "App.exe"), null, 32, 12, "file");
            var generationCount = 0;
            var results = new bool[8];
            var paths = new string[8];
            var tasks = Enumerable.Range(0, results.Length)
                .Select(index => Task.Run(() =>
                {
                    results[index] = cache.TryGetOrCreate(key, path =>
                    {
                        Interlocked.Increment(ref generationCount);
                        generationEntered.Set();
                        releaseGeneration.Wait();
                        File.WriteAllBytes(path, [7]);
                        return true;
                    }, out paths[index]);
                }))
                .ToArray();
            Assert(generationEntered.Wait(TimeSpan.FromSeconds(1)));
            releaseGeneration.Set();
            await Task.WhenAll(tasks);
            Assert(results.All(result => result));
            Assert(generationCount == 1);
            Assert(paths.All(path => path == paths[0]));
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
    }

    private static async Task IconCacheGenerationDoesNotHoldMetadataLock()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-icon-cache-lock-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        using var generationEntered = new ManualResetEventSlim();
        using var releaseGeneration = new ManualResetEventSlim();
        try
        {
            var cache = new IconCache(root);
            var slowKey = IconCacheKey.Create(Path.Combine(root, "Slow.exe"), null, 32, 12, "file");
            var fastKey = IconCacheKey.Create(Path.Combine(root, "Fast.exe"), null, 32, 12, "file");
            var slow = Task.Run(() => cache.TryGetOrCreate(slowKey, path =>
            {
                generationEntered.Set();
                releaseGeneration.Wait();
                File.WriteAllBytes(path, [1]);
                return true;
            }, out _));
            Assert(generationEntered.Wait(TimeSpan.FromSeconds(1)));
            var fast = Task.Run(() => cache.TryGetOrCreate(
                fastKey,
                path =>
                {
                    File.WriteAllBytes(path, [2]);
                    return true;
                },
                out _));
            Assert(await Task.WhenAny(fast, Task.Delay(TimeSpan.FromMilliseconds(500))) == fast);
            releaseGeneration.Set();
            Assert(await slow);
            Assert(await fast);
        }
        finally
        {
            releaseGeneration.Set();
            Directory.Delete(root, recursive: true);
        }
    }

    private static Task MissingFileUsesTypeIcon()
    {
        var missingPng = Path.Combine(
            Path.GetTempPath(),
            $"nxtools-missing-icon-{Guid.NewGuid():N}.png");
        var result = ShellIconExtractor.ExtractOnSta(new ExtractShellIconCommand(missingPng, 32));

        Assert(result.Success);
        Assert(result.Data is string path && File.Exists(path));
        Assert(Path.GetExtension((string)result.Data!) == ".png");
        return Task.CompletedTask;
    }

    private static Task ShortcutIconResolvesWithoutOverlay()
    {
        var root = Path.Combine(Path.GetTempPath(), "nxtools-shortcut-icon-tests", Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            var target = Environment.ProcessPath
                ?? throw new InvalidOperationException("Test process path is unavailable.");
            var shortcutPath = Path.Combine(root, "target.lnk");
            CreateShortcut(shortcutPath, target);

            var source = ShellIconExtractor.ResolveIconTarget(shortcutPath)
                ?? throw new InvalidOperationException("Shortcut icon target was not resolved.");
            Assert(string.Equals(source.Path, target, StringComparison.OrdinalIgnoreCase));
            Assert(!source.Path.EndsWith(".lnk", StringComparison.OrdinalIgnoreCase));
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task ShortcutExtractionPreservesApplicationIcons()
    {
        var root = Path.Combine(
            Path.GetTempPath(),
            "nxtools-shortcut-icon-identity-tests",
            Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            var command = Path.Combine(Environment.SystemDirectory, "cmd.exe");
            var explorer = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.Windows),
                "explorer.exe");
            var commandShortcut = Path.Combine(root, "Command Prompt.lnk");
            var explorerShortcut = Path.Combine(root, "File Explorer.lnk");
            CreateShortcut(commandShortcut, command);
            CreateShortcut(explorerShortcut, explorer);

            var commandIcon = ShellIconExtractor.ExtractOnSta(
                new ExtractShellIconCommand(commandShortcut, 32));
            var explorerIcon = ShellIconExtractor.ExtractOnSta(
                new ExtractShellIconCommand(explorerShortcut, 32));
            if (!commandIcon.Success || commandIcon.Data is not string)
                throw new InvalidOperationException($"Command shortcut extraction failed: {commandIcon.Error}");
            if (!explorerIcon.Success || explorerIcon.Data is not string)
                throw new InvalidOperationException($"Explorer shortcut extraction failed: {explorerIcon.Error}");
            var commandPath = (string)commandIcon.Data!;
            var explorerPath = (string)explorerIcon.Data!;
            Assert(File.Exists(commandPath));
            Assert(File.Exists(explorerPath));
            if (SHA256.HashData(File.ReadAllBytes(commandPath))
                .SequenceEqual(SHA256.HashData(File.ReadAllBytes(explorerPath))))
            {
                throw new InvalidOperationException(
                    "Distinct application shortcuts collapsed to one generic Shell icon.");
            }
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static void CreateShortcut(string shortcutPath, string targetPath)
    {
        var shellType = Type.GetTypeFromProgID("WScript.Shell")
            ?? throw new InvalidOperationException("Windows Script Host is unavailable.");
        var shell = Activator.CreateInstance(shellType)
            ?? throw new InvalidOperationException("Windows Script Host could not be created.");
        try
        {
            dynamic shortcut = shellType.InvokeMember(
                "CreateShortcut",
                System.Reflection.BindingFlags.InvokeMethod,
                null,
                shell,
                [shortcutPath])!;
            shortcut.TargetPath = targetPath;
            shortcut.Save();
            Marshal.FinalReleaseComObject(shortcut);
        }
        finally
        {
            Marshal.FinalReleaseComObject(shell);
        }
    }

    private static Task ShortcutNavigationResolvesRealTargetDirectory()
    {
        var root = Path.Combine(
            Path.GetTempPath(),
            "nxtools-shortcut-navigation-tests",
            Guid.NewGuid().ToString("N"));
        var targetDirectory = Path.Combine(root, "installed-app");
        Directory.CreateDirectory(targetDirectory);
        try
        {
            var target = Path.Combine(targetDirectory, "Editor.exe");
            File.WriteAllBytes(target, []);
            var shortcutPath = Path.Combine(root, "Editor.lnk");
            var directoryShortcutPath = Path.Combine(root, "Installed App.lnk");
            CreateShortcut(shortcutPath, target);
            CreateShortcut(directoryShortcutPath, targetDirectory);

            var resolved = ShellIconExtractor.ResolveNavigationDirectory(shortcutPath);
            Assert(string.Equals(resolved, targetDirectory, StringComparison.OrdinalIgnoreCase));
            Assert(!string.Equals(resolved, root, StringComparison.OrdinalIgnoreCase));
            Assert(string.Equals(
                ShellIconExtractor.ResolveNavigationDirectory(directoryShortcutPath),
                targetDirectory,
                StringComparison.OrdinalIgnoreCase));
            Assert(string.Equals(
                ShellIconExtractor.ResolveNavigationDirectory("cmd.exe"),
                Environment.SystemDirectory,
                StringComparison.OrdinalIgnoreCase));
            Assert(ShellIconExtractor.ResolveNavigationDirectory("website.url") is null);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static Task WindowChromeUsesNativeContracts()
    {
        Assert(WindowsPlatformAdapter.BottomRightResizeSystemCommand == 0xF008);
        Assert(WindowsPlatformAdapter.MoveCaptionSystemCommand == 0xF012);
        var root = FindRepositoryRoot();
        var adapter = File.ReadAllText(Path.Combine(
            root, "csharp", "NxTools.PlatformHost", "Windows", "WindowsPlatformAdapter.cs"));
        Assert(!adapter.Contains("WindowsWindowChrome", StringComparison.Ordinal));
        Assert(!File.Exists(Path.Combine(
            root, "csharp", "NxTools.PlatformHost", "Windows", "WindowsWindowChrome.cs")));
        return Task.CompletedTask;
    }

    private static Task PasteRetriesTransientTargetActivation()
    {
        var operations = new FakePasteOperations
        {
            ActivationResults = new Queue<bool>([false, true])
        };
        var coordinator = new WindowsPasteCoordinator(operations);

        var result = coordinator.Execute(new PasteTextCommand("paste", 42, 43));

        Assert(result.Success);
        Assert(operations.ActivationAttempts == 2);
        Assert(operations.SendPasteCalls == 1);
        Assert(operations.PostPasteCalls == 0);
        return Task.CompletedTask;
    }

    private static Task CopyOnlyWritesWithoutActivatingTarget()
    {
        var operations = new FakePasteOperations();
        var coordinator = new WindowsPasteCoordinator(operations);

        var result = coordinator.Execute(new PasteTextCommand("copy", 0, 0, false));

        Assert(result.Success);
        Assert(operations.ActivationAttempts == 0);
        Assert(operations.SendPasteCalls == 0);
        Assert(operations.PostPasteCalls == 0);
        return Task.CompletedTask;
    }

    private static Task FourActionsReachExpectedWindowsOutcome()
    {
        var operations = new FakePasteOperations
        {
            ActivationResults = new Queue<bool>([true, true, true])
        };
        var coordinator = new WindowsPasteCoordinator(operations);
        var fixturePath = Path.Combine(
            FindRepositoryRoot(),
            "tests", "fixtures", "clipboard_action_commands.jsonl");
        foreach (var line in File.ReadLines(fixturePath))
        {
            var envelope = JsonSerializer.Deserialize<ProtocolEnvelope>(line, JsonDefaults.Options)
                ?? throw new InvalidDataException("Shared clipboard action fixture was empty.");
            var result = PlatformCommandParser.GetCommandName(envelope) switch
            {
                "paste_native_formats" => coordinator.Execute(
                    PlatformCommandParser.ParsePasteNativeFormats(envelope)),
                "paste_text" => coordinator.Execute(
                    PlatformCommandParser.ParsePasteText(envelope)),
                var command => throw new InvalidDataException(
                    $"Unexpected shared clipboard action fixture '{command}'.")
            };
            Assert(result.Success);
        }

        Assert(operations.WrittenPayloads.Count == 4);
        Assert(operations.WrittenPayloads[0] is NativeFormatsClipboardPastePayload
        {
            PasteAfterWrite: true
        });
        Assert(operations.WrittenPayloads[1] is NativeFormatsClipboardPastePayload
        {
            PasteAfterWrite: false
        });
        Assert(operations.WrittenPayloads[2] is TextClipboardPastePayload { Text: "first\r\nsecond" });
        Assert(operations.WrittenPayloads[3] is TextClipboardPastePayload { Text: "firstsecond" });
        Assert(operations.ActivationAttempts == 3);
        Assert(operations.SendPasteCalls == 3);
        return Task.CompletedTask;
    }

    private static string FindRepositoryRoot()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory);
             directory is not null;
             directory = directory.Parent)
        {
            if (File.Exists(Path.Combine(directory.FullName, "Cargo.toml")))
            {
                return directory.FullName;
            }
        }
        throw new DirectoryNotFoundException("Could not locate the NxTools.Next repository root.");
    }

    private static Task NativeInputLayoutMatchesWindowsAbi()
    {
        var expectedSize = IntPtr.Size == 8 ? 40 : 28;
        Assert(WindowsPlatformAdapter.NativeInputSize == expectedSize);
        Assert(Win32FileDialogWindowOperations.NativeInputSize == expectedSize);
        return Task.CompletedTask;
    }

    private static Task PasteDiagnosticsPreserveCorrelationAndWin32Error()
    {
        var correlationId = Guid.NewGuid();
        var log = new RecordingPlatformLog();
        var coordinator = new WindowsPasteCoordinator(
            new FakePasteOperations { SentInputCount = 0, PostPasteResult = true },
            log);

        var result = coordinator.Execute(new PasteTextCommand("paste", 42, 43), correlationId);

        Assert(!result.Success);
        var inputFailure = log.Entries.Single(entry =>
            entry.EventName == "paste.send_input_step" &&
            Equals(entry.Fields["step"], "ctrl_down"));
        Assert(inputFailure.CorrelationId == correlationId);
        Assert(Equals(inputFailure.Fields["win32_error"], 87));
        Assert(Equals(inputFailure.Fields["input_struct_size"], IntPtr.Size == 8 ? 40 : 28));
        return Task.CompletedTask;
    }

    private static Task PlatformLogWritesStructuredJsonLines()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"nxtools-log-test-{Guid.NewGuid():N}");
        var correlationId = Guid.NewGuid();
        string path;
        using (var log = PlatformLog.CreateForDirectory(directory))
        {
            path = log.CurrentLogPath;
            log.Write("info", "test.event", correlationId, new Dictionary<string, object?>
            {
                ["win32_error"] = 87,
                ["text"] = "must-not-appear",
                ["nested"] = new { Text = "nested-must-not-appear", Safe = 1 },
                ["message"] = "message-secret",
                ["path"] = "C:/secret/path.bin",
                ["window_title"] = "window-secret"
            });
        }
        var json = File.ReadAllText(path);
        using var document = JsonDocument.Parse(json);
        var root = document.RootElement;
        Assert(root.GetProperty("event").GetString() == "test.event");
        Assert(root.GetProperty("correlation_id").GetGuid() == correlationId);
        Assert(root.GetProperty("platform_host_pid").GetInt32() == Environment.ProcessId);
        Assert(root.GetProperty("win32_error").GetInt32() == 87);
        Assert(root.GetProperty("text").GetString() == "[REDACTED]");
        Assert(root.GetProperty("nested").GetProperty("Text").GetString() == "[REDACTED]");
        Assert(root.GetProperty("message").GetString() == "[REDACTED]");
        Assert(root.GetProperty("path").GetString() == "[REDACTED]");
        Assert(root.GetProperty("window_title").GetString() == "[REDACTED]");
        Assert(!json.Contains("must-not-appear", StringComparison.Ordinal));
        Assert(!json.Contains("nested-must-not-appear", StringComparison.Ordinal));
        Directory.Delete(directory, recursive: true);
        return Task.CompletedTask;
    }

    private static Task ProcessExceptionDiagnosticsPreserveEvidence()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"nxtools-exception-diagnostics-{Guid.NewGuid():N}");
        string path;
        Exception exception;
        try
        {
            throw new InvalidOperationException("clipboard secret must not be logged");
        }
        catch (Exception error)
        {
            exception = error;
        }
        using (var log = PlatformLog.CreateForDirectory(directory))
        {
            path = log.CurrentLogPath;
            PlatformExceptionDiagnostics.Record(log, "test.process_exception", exception);
        }
        var json = File.ReadAllText(path);
        using var document = JsonDocument.Parse(json);
        var root = document.RootElement;
        Assert(root.GetProperty("event").GetString() == "test.process_exception");
        Assert(root.GetProperty("exception_type").GetString() == typeof(InvalidOperationException).FullName);
        Assert(root.GetProperty("hresult").GetString()?.StartsWith("0x", StringComparison.Ordinal) == true);
        Assert(root.TryGetProperty("stack_trace", out var stackTrace));
        Assert(stackTrace.ValueKind == JsonValueKind.String);
        Assert(!json.Contains("clipboard secret must not be logged", StringComparison.Ordinal));
        Directory.Delete(directory, recursive: true);
        return Task.CompletedTask;
    }

    private static Task UnverifiedFallbackIsNotReportedAsPasteSuccess()
    {
        var operations = new FakePasteOperations
        {
            ActivationResults = new Queue<bool>([false, false, false, false]),
            PostPasteResult = true
        };
        var coordinator = new WindowsPasteCoordinator(operations);

        var result = coordinator.Execute(new PasteTextCommand("paste", 42, 43));

        Assert(!result.Success);
        Assert(result.Error?.Contains("could not be confirmed", StringComparison.OrdinalIgnoreCase) == true);
        Assert(operations.ActivationAttempts == 4);
        Assert(operations.SendPasteCalls == 0);
        Assert(operations.PostPasteCalls == 1);
        Assert(operations.LastPostTarget == 43);
        return Task.CompletedTask;
    }

    private static Task PressedModifiersKeepPasteResultUnconfirmed()
    {
        var operations = new FakePasteOperations
        {
            ModifiersReleased = false,
            PostPasteResult = true
        };
        var coordinator = new WindowsPasteCoordinator(operations);

        var result = coordinator.Execute(new PasteTextCommand("paste", 42, 43));

        Assert(!result.Success);
        Assert(result.Error?.Contains("could not be confirmed", StringComparison.OrdinalIgnoreCase) == true);
        Assert(operations.SendPasteCalls == 0);
        Assert(operations.PostPasteCalls == 1);
        return Task.CompletedTask;
    }

    private static Task IncompleteInputKeepsPasteResultUnconfirmed()
    {
        var operations = new FakePasteOperations
        {
            SentInputCount = 2,
            PostPasteResult = true
        };
        var coordinator = new WindowsPasteCoordinator(operations);

        var result = coordinator.Execute(new PasteTextCommand("paste", 42, 43));

        Assert(!result.Success);
        Assert(result.Error?.Contains("could not be confirmed", StringComparison.OrdinalIgnoreCase) == true);
        Assert(operations.SendPasteCalls == 1);
        Assert(operations.PostPasteCalls == 1);
        return Task.CompletedTask;
    }

    private static Task PasteResultCorrelatesToCommand()
    {
        var commandId = Guid.NewGuid();
        var envelope = ProtocolEnvelope.Create(
            "platform_command_result",
            new
            {
                Success = false,
                Error = "target closed"
            },
            commandId);
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);

        Assert(envelope.CorrelationId == commandId);
        Assert(json.Contains("\"type\":\"platform_command_result\"", StringComparison.Ordinal));
        Assert(json.Contains("\"success\":false", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task PanelRequestPreservesTargetAndCursor()
    {
        var envelope = PlatformEventEnvelopeFactory.Create(
            new ClipboardPanelRequestedEvent(
                new ForegroundContext(88, 89, 9, "editor", "draft.md"),
                1200,
                700,
                1,
                144,
                1_700_000_000_500,
                41));
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);

        Assert(json.Contains("\"event\":\"clipboard_panel_requested\"", StringComparison.Ordinal));
        Assert(json.Contains("\"window_handle\":88", StringComparison.Ordinal));
        Assert(json.Contains("\"cursor_x\":1200", StringComparison.Ordinal));
        Assert(json.Contains("\"cursor_y\":700", StringComparison.Ordinal));
        Assert(json.Contains("\"cursor_display_index\":1", StringComparison.Ordinal));
        Assert(json.Contains("\"cursor_monitor_dpi\":144", StringComparison.Ordinal));
        Assert(json.Contains("\"event_sequence\":41", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task GlobalPointerEventPreservesClickedWindow()
    {
        var envelope = PlatformEventEnvelopeFactory.Create(
            new GlobalPointerPressedEvent(91, 80, 7, 1_700_000_000_400, 42));
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);

        Assert(json.Contains("\"event\":\"global_pointer_pressed\"", StringComparison.Ordinal));
        Assert(json.Contains("\"window_handle\":91", StringComparison.Ordinal));
        Assert(json.Contains("\"root_window_handle\":80", StringComparison.Ordinal));
        Assert(json.Contains("\"process_id\":7", StringComparison.Ordinal));
        Assert(json.Contains("\"sampled_at_unix_ms\":1700000000400", StringComparison.Ordinal));
        Assert(json.Contains("\"event_sequence\":42", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task HotKeyCapabilityReflectsRegistration()
    {
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.PasteText));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.ShellContextActions));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.DirectoryNavigation));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.FileDialogSearch));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.TaskbarProgressOverlay));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.ClipboardNativeFormats));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.WindowTopmost));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.WindowNoActivate));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.WindowBounds));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.WindowPosition));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.ClipboardConfiguration));
        Assert(!PlatformCapabilities.Create(false).Contains(PlatformCapabilities.ClipboardPanelHotKey));
        Assert(PlatformCapabilities.Create(true).Contains(PlatformCapabilities.ClipboardPanelHotKey));
        Assert(!PlatformCapabilities.Create(true).Contains(PlatformCapabilities.QuickLaunchPanelHotKey));
        Assert(PlatformCapabilities.Create(true, true).Contains(PlatformCapabilities.QuickLaunchPanelHotKey));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.QuickLaunchConfiguration));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.IndexerServiceManagement));
        return Task.CompletedTask;
    }

    private static async Task ReaderPreservesFrames()
    {
        await using var stream = new MemoryStream(Encoding.UTF8.GetBytes("first\nsecond\n"));
        var reader = new BoundedFrameReader(stream);
        Assert(await reader.ReadAsync(CancellationToken.None) == "first");
        Assert(await reader.ReadAsync(CancellationToken.None) == "second");
    }

    private static async Task ReaderRejectsOversizedFrame()
    {
        await using var stream = new MemoryStream(
            Enumerable.Repeat((byte)'x', ProtocolConstants.MaxFrameBytes + 1).ToArray());
        var reader = new BoundedFrameReader(stream);
        try
        {
            _ = await reader.ReadAsync(CancellationToken.None);
            throw new InvalidOperationException("oversized frame was accepted");
        }
        catch (InvalidDataException)
        {
        }
    }

    private static async Task DetachedSyncCommandsScheduleBeforeInvocation()
    {
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var started = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var dispatch = Task.Run(() => dispatcher.DispatchAsync(
            "pick_program",
            () =>
            {
                started.TrySetResult();
                release.Task.GetAwaiter().GetResult();
                return Task.CompletedTask;
            }));
        try
        {
            Assert(await Task.WhenAny(dispatch, Task.Delay(250)) == dispatch);
            Assert(await Task.WhenAny(started.Task, Task.Delay(1_000)) == started.Task);
        }
        finally
        {
            release.TrySetResult();
            await dispatch.WaitAsync(TimeSpan.FromSeconds(1));
            await dispatcher.DrainAsync().WaitAsync(TimeSpan.FromSeconds(1));
        }
    }
    private static async Task FileDragDoesNotBlockLaterCommands()
    {
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var dragStarted = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var releaseDrag = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var shellCommandRan = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);

        var processing = Task.Run(async () =>
        {
            await dispatcher.DispatchAsync(
                "begin_file_drag",
                async () =>
                {
                    dragStarted.TrySetResult();
                    await releaseDrag.Task;
                });
            await dispatcher.DispatchAsync(
                "open_shell_target",
                () =>
                {
                    shellCommandRan.TrySetResult();
                    return Task.CompletedTask;
                });
        });

        await dragStarted.Task.WaitAsync(TimeSpan.FromSeconds(1));
        try
        {
            var completed = await Task.WhenAny(
                shellCommandRan.Task,
                Task.Delay(TimeSpan.FromMilliseconds(250)));
            Assert(completed == shellCommandRan.Task);
        }
        finally
        {
            releaseDrag.TrySetResult();
            await processing.WaitAsync(TimeSpan.FromSeconds(1));
            await dispatcher.DrainAsync().WaitAsync(TimeSpan.FromSeconds(1));
        }
    }

    private static async Task ShellEnumerationDoesNotBlockLaterCommands()
    {
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var shellStarted = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var releaseShell = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var followUpRan = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);

        var processing = Task.Run(async () =>
        {
            await dispatcher.DispatchAsync(
                "query_shell_actions",
                async () =>
                {
                    shellStarted.TrySetResult();
                    await releaseShell.Task;
                });
            await dispatcher.DispatchAsync(
                "open_shell_target",
                () =>
                {
                    followUpRan.TrySetResult();
                    return Task.CompletedTask;
                });
        });

        await shellStarted.Task.WaitAsync(TimeSpan.FromSeconds(1));
        try
        {
            // While the Shell enumeration is still running, the next command
            // must already have executed — this is the pipe-read-loop liveness
            // contract that keeps the quick-launch action mode responsive.
            var completed = await Task.WhenAny(
                followUpRan.Task,
                Task.Delay(TimeSpan.FromMilliseconds(250)));
            Assert(completed == followUpRan.Task);
        }
        finally
        {
            releaseShell.TrySetResult();
            await processing.WaitAsync(TimeSpan.FromSeconds(1));
            await dispatcher.DrainAsync().WaitAsync(TimeSpan.FromSeconds(1));
        }
    }

    private static async Task ShellTargetLaunchDoesNotBlockLaterCommands()
    {
        // Measured incident: launching a program via the quick launcher
        // (open_shell_target) stalled the pipe read loop for seconds, and the
        // panel-hide visibility command queued behind it never ran — even the
        // self-healing retry queued behind the same stall. The launch must be
        // a detached command so follow-ups execute immediately.
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var launchStarted = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var releaseLaunch = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var hideRan = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);

        var processing = Task.Run(async () =>
        {
            await dispatcher.DispatchAsync(
                "open_shell_target",
                async () =>
                {
                    launchStarted.TrySetResult();
                    await releaseLaunch.Task;
                });
            await dispatcher.DispatchAsync(
                "set_window_visibility",
                () =>
                {
                    hideRan.TrySetResult();
                    return Task.CompletedTask;
                });
        });

        await launchStarted.Task.WaitAsync(TimeSpan.FromSeconds(1));
        try
        {
            var completed = await Task.WhenAny(
                hideRan.Task,
                Task.Delay(TimeSpan.FromMilliseconds(250)));
            Assert(completed == hideRan.Task);
        }
        finally
        {
            releaseLaunch.TrySetResult();
            await processing.WaitAsync(TimeSpan.FromSeconds(1));
            await dispatcher.DrainAsync().WaitAsync(TimeSpan.FromSeconds(1));
        }
    }

    private static async Task DirectoryNavigationDoesNotBlockLaterCommands()
    {
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var navigationStarted = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var releaseNavigation = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var followUpRan = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);

        var processing = Task.Run(async () =>
        {
            await dispatcher.DispatchAsync(
                "navigate_directory",
                async () =>
                {
                    navigationStarted.TrySetResult();
                    await releaseNavigation.Task;
                });
            await dispatcher.DispatchAsync(
                "set_window_visibility",
                () =>
                {
                    followUpRan.TrySetResult();
                    return Task.CompletedTask;
                });
        });

        await navigationStarted.Task.WaitAsync(TimeSpan.FromSeconds(1));
        try
        {
            var completed = await Task.WhenAny(
                followUpRan.Task,
                Task.Delay(TimeSpan.FromMilliseconds(250)));
            Assert(completed == followUpRan.Task);
        }
        finally
        {
            releaseNavigation.TrySetResult();
            await processing.WaitAsync(TimeSpan.FromSeconds(1));
            await dispatcher.DrainAsync().WaitAsync(TimeSpan.FromSeconds(1));
        }
    }

    private static Task NormalPipeClosureIsNotAnError()
    {
        Assert(PlatformPipeClient.DisconnectLogLevel(
            new EndOfStreamException("Desktop closed the platform pipe.")) == "info");
        Assert(PlatformPipeClient.DisconnectLogLevel(
            new InvalidDataException("bad frame")) == "error");
        return Task.CompletedTask;
    }

    private static Task ReconnectFailureLoggingIsRateLimited()
    {
        // A dead desktop turns the reconnect loop into a retry storm; the
        // diagnostic sink must keep one full record plus bounded repeats.
        Assert(PlatformPipeClient.ShouldLogDisconnect(1, TimeSpan.FromDays(1)));
        Assert(!PlatformPipeClient.ShouldLogDisconnect(2, TimeSpan.FromSeconds(1)));
        Assert(!PlatformPipeClient.ShouldLogDisconnect(745, TimeSpan.FromSeconds(29)));
        Assert(PlatformPipeClient.ShouldLogDisconnect(746, TimeSpan.FromSeconds(30)));
        return Task.CompletedTask;
    }

    private static Task ErrorDialogArgumentsParseReportPath()
    {
        var report = Path.Combine(
            Path.GetTempPath(),
            $"nxtools-startup-failure-{Guid.NewGuid():N}.log");
        File.WriteAllText(report, "NxTools could not start:\nschema guard");
        try
        {
            Assert(ErrorDialog.TryParseArguments(
                new[] { "--error-dialog", report },
                out var parsed));
            Assert(parsed == report);
            // A missing report is recognized but silent: the mode must never
            // fall through to a full PlatformHost startup.
            Assert(ErrorDialog.TryParseArguments(
                new[] { "--error-dialog", Path.Combine(Path.GetTempPath(), "missing-report.log") },
                out var missing));
            Assert(missing is null);
            Assert(!ErrorDialog.TryParseArguments(Array.Empty<string>(), out _));
        }
        finally
        {
            File.Delete(report);
        }
        return Task.CompletedTask;
    }

    private static Task ErrorDialogReadsBoundedReport()
    {
        var report = Path.Combine(
            Path.GetTempPath(),
            $"nxtools-startup-failure-{Guid.NewGuid():N}.log");
        File.WriteAllText(report, new string('x', 5000));
        try
        {
            var message = ErrorDialog.ReadMessage(report);
            Assert(message.Length <= 4001);
            Assert(message.EndsWith("\n…", StringComparison.Ordinal));
        }
        finally
        {
            File.Delete(report);
        }
        return Task.CompletedTask;
    }

    private static Task BrowserExtensionExportCommandParses()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "export_browser_extension",
            Data = new
            {
                Format = "firefox",
                Destination = @"C:\Users\u\Desktop\NxTools_Firefox_Extension.xpi",
            },
        });
        var command = PlatformCommandParser.ParseExportBrowserExtension(envelope);
        Assert(command.Format == "firefox");
        Assert(command.Destination.EndsWith(".xpi"));
        return Task.CompletedTask;
    }

    private static Task BrowserExtensionPackagesProduceArtifacts()
    {
        var source = System.IO.Path.Combine(
            System.IO.Path.GetTempPath(),
            $"nxtools-extension-source-{Guid.NewGuid():N}");
        System.IO.Directory.CreateDirectory(System.IO.Path.Combine(source, "icons"));
        System.IO.File.WriteAllText(
            System.IO.Path.Combine(source, "manifest.json"),
            "{\"manifest_version\":3,\"name\":\"t\",\"version\":\"0.0.1\",\"permissions\":[\"declarativeNetRequest\",\"storage\"]}");
        System.IO.File.WriteAllText(
            System.IO.Path.Combine(source, "icons", "icon-16.png"),
            "png");
        try
        {
            var work = System.IO.Path.Combine(
                System.IO.Path.GetTempPath(),
                $"nxtools-extension-out-{Guid.NewGuid():N}");
            System.IO.Directory.CreateDirectory(work);
            var crx = BrowserExtensionPackageService.Export(
                BrowserExtensionPackageService.ChromeFormat,
                System.IO.Path.Combine(work, "ext.crx"),
                source);
            var crxBytes = System.IO.File.ReadAllBytes(crx);
            Assert(crxBytes.Length > 4);
            Assert(crxBytes[0] == (byte)'C' && crxBytes[1] == (byte)'r');
            Assert(crxBytes[2] == (byte)'2' && crxBytes[3] == (byte)'4');
            Assert(BitConverter.ToUInt32(crxBytes, 4) == 3);

            var xpi = BrowserExtensionPackageService.Export(
                BrowserExtensionPackageService.FirefoxFormat,
                System.IO.Path.Combine(work, "ext.xpi"),
                source);
            string manifest;
            using (var archive = new System.IO.Compression.ZipArchive(
                System.IO.File.OpenRead(xpi),
                System.IO.Compression.ZipArchiveMode.Read))
            {
                var manifestEntry = archive.GetEntry("manifest.json");
                Assert(manifestEntry is not null);
                using var reader = new System.IO.StreamReader(manifestEntry!.Open());
                manifest = reader.ReadToEnd();
            }
            Assert(manifest.Contains("nxtools-download-interceptor@nxtools.local"));
            Assert(!manifest.Contains("declarativeNetRequest"));
            System.IO.File.Delete(crx);
            System.IO.File.Delete(xpi);
            System.IO.Directory.Delete(work, true);
        }
        finally
        {
            System.IO.Directory.Delete(source, true);
        }
        return Task.CompletedTask;
    }

    private static void Assert(bool condition)
    {
        if (!condition)
        {
            throw new InvalidOperationException("assertion failed");
        }
    }

    private static Task DirectoryOpusDisplayPathsCanonicalize()
    {
        var existing = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "C:\\",
            "C:\\Users",
            "C:\\Users\\Administrator",
            "C:\\Users\\Administrator\\Desktop",
            "C:\\Users\\Administrator\\Desktop\\奕心五子棋",
            "C:\\Users\\Administrator\\Desktop\\奕心五子棋\\piskvork85",
        };
        bool DirectoryExists(string path) => existing.Contains(path);
        IReadOnlyDictionary<string, string> aliases =
            new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
            {
                ["用户"] = "C:\\Users",
                ["桌面"] = "C:\\Users\\Administrator\\Desktop",
            };
        Assert(
            ThirdPartyFileManagerDisplayPath.Canonicalize(
                "C:\\用户\\Administrator\\桌面\\奕心五子棋\\piskvork85",
                DirectoryExists,
                aliases)
            == "C:\\Users\\Administrator\\Desktop\\奕心五子棋\\piskvork85");
        Assert(
            ThirdPartyFileManagerDisplayPath.Canonicalize(
                "C:\\Users\\Administrator\\Desktop",
                DirectoryExists,
                aliases)
            == "C:\\Users\\Administrator\\Desktop");
        Assert(
            ThirdPartyFileManagerDisplayPath.Canonicalize(
                "C:\\不存在\\子目录",
                DirectoryExists,
                aliases)
            is null);
        return Task.CompletedTask;
    }

    private static Task KnownFolderDisplayAliasesCoverDesktop()
    {
        var aliases = ThirdPartyFileManagerDisplayPath.KnownFolderDisplayAliases();
        var desktop = Environment.GetFolderPath(Environment.SpecialFolder.Desktop);
        Assert(aliases.Values.Any(value =>
            string.Equals(value, desktop, StringComparison.OrdinalIgnoreCase)));
        // The alias map must resolve the folder's own shell display name back
        // to the real parsing path on every locale.
        var display = ThirdPartyFileManagerDisplayPath.GetShellItemDisplayName(desktop);
        Assert(!string.IsNullOrWhiteSpace(display));
        Assert(aliases.TryGetValue(display!, out var resolved));
        Assert(string.Equals(resolved, desktop, StringComparison.OrdinalIgnoreCase));
        return Task.CompletedTask;
    }

    private static void AssertThrows<TException>(Action action)
        where TException : Exception
    {
        try
        {
            action();
        }
        catch (TException)
        {
            return;
        }
        throw new InvalidOperationException(
            $"expected {typeof(TException).Name}");
    }

    private sealed class FakePasteOperations : IWindowsPasteOperations
    {
        public Queue<bool> ActivationResults { get; init; } = new([true]);
        public bool PostPasteResult { get; init; }
        public bool ModifiersReleased { get; init; } = true;
        public uint SentInputCount { get; init; } = 4;
        public int ActivationAttempts { get; private set; }
        public int SendPasteCalls { get; private set; }
        public int PostPasteCalls { get; private set; }
        public long LastPostTarget { get; private set; }
        public List<ClipboardPastePayload> WrittenPayloads { get; } = [];

        public bool IsWindow(long windowHandle) => windowHandle != 0;

        public uint GetClipboardSequenceNumber() => 7;

        public ClipboardWriteResult WriteClipboard(ClipboardPastePayload payload)
        {
            WrittenPayloads.Add(payload);
            return new(true, null, [new ClipboardWriteAttempt(1, true, 0, null)]);
        }

        public bool TryActivateWindow(long targetWindowHandle, long focusWindowHandle)
        {
            ActivationAttempts++;
            return ActivationResults.Count > 0 && ActivationResults.Dequeue();
        }

        public bool IsForegroundWindowForTarget(long targetWindowHandle) =>
            ActivationAttempts > 0 && ActivationResults.Count < 3;

        public PasteWindowSnapshot CaptureWindowState(long targetWindowHandle) =>
            new(targetWindowHandle, targetWindowHandle, targetWindowHandle, 1, 2, 2, 3, 3);

        public ModifierReleaseResult ReleaseActivationModifiers() =>
            new(ModifiersReleased, [], ModifiersReleased ? [] : [0x12], 1, []);

        public PasteShortcutResult SendPasteShortcut()
        {
            SendPasteCalls++;
            var steps = new[] { "ctrl_down", "v_down", "v_up", "ctrl_up" }
                .Select((name, index) => new KeyboardInputStep(
                    name,
                    index is 0 or 3 ? (ushort)0x11 : (ushort)0x56,
                    index >= 2 ? 2U : 0U,
                    index < SentInputCount ? 1U : 0U,
                    index < SentInputCount ? 0 : 87))
                .ToArray();
            return new PasteShortcutResult(SentInputCount, WindowsPlatformAdapter.NativeInputSize, steps);
        }

        public PostPasteResult PostPaste(long targetWindowHandle)
        {
            PostPasteCalls++;
            LastPostTarget = targetWindowHandle;
            return new PostPasteResult(
                PostPasteResult,
                PostPasteResult ? 0 : 5,
                PostPasteResult ? null : "post failed");
        }

        public void Delay(int milliseconds)
        {
        }
    }

    private sealed class FakeExplorerWindowOperations : IExplorerWindowOperations
    {
        public long ExplorerWindowHandle { get; init; }
        public long LastNavigatedWindowHandle { get; private set; }
        public string? LastNavigatedDirectory { get; private set; }
        public long LastActivatedWindowHandle { get; private set; }

        public bool IsExplorerWindow(long windowHandle) =>
            windowHandle == ExplorerWindowHandle;

        public bool TryNavigate(
            long windowHandle,
            string targetDirectory,
            CancellationToken cancellationToken,
            out string? error)
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (windowHandle != 88)
            {
                error = "Explorer window was not found.";
                return false;
            }
            LastNavigatedWindowHandle = windowHandle;
            LastNavigatedDirectory = targetDirectory;
            error = null;
            return true;
        }

        public bool TryActivate(long windowHandle, out string? error)
        {
            LastActivatedWindowHandle = windowHandle;
            error = null;
            return true;
        }
    }

    private sealed class FakeFileDialogWindowOperations : IFileDialogWindowOperations
    {
        public long FileDialogWindowHandle { get; init; }
        public bool IsFileDialog { get; set; } = true;
        public string? CurrentDirectory { get; init; }
        public long LastNavigatedWindowHandle { get; private set; }
        public string? LastNavigatedDirectory { get; private set; }
        public long LastOpenedWindowHandle { get; private set; }
        public string? LastOpenedFile { get; private set; }

        public bool TryInspect(long windowHandle, out string? currentDirectory)
        {
            currentDirectory = IsFileDialog && windowHandle == FileDialogWindowHandle
                ? CurrentDirectory
                : null;
            return IsFileDialog && windowHandle == FileDialogWindowHandle;
        }

        public bool TryNavigate(long windowHandle, string targetDirectory, out string? error)
        {
            if (!IsFileDialog || windowHandle != FileDialogWindowHandle)
            {
                error = "File dialog was not found.";
                return false;
            }
            LastNavigatedWindowHandle = windowHandle;
            LastNavigatedDirectory = targetDirectory;
            error = null;
            return true;
        }

        public bool TryOpenFile(long windowHandle, string filePath, out string? error)
        {
            if (!IsFileDialog || windowHandle != FileDialogWindowHandle)
            {
                error = "File dialog was not found.";
                return false;
            }
            LastOpenedWindowHandle = windowHandle;
            LastOpenedFile = filePath;
            error = null;
            return true;
        }
    }

    private sealed class FakeDesktopWindowOperations : IDesktopWindowOperations
    {
        public long DesktopWindowHandle { get; init; }
        public string? LastOpenedDirectory { get; private set; }

        public bool IsDesktopWindow(long windowHandle) =>
            windowHandle == DesktopWindowHandle;

        public bool TryOpenDirectory(string targetDirectory, out string? error)
        {
            LastOpenedDirectory = targetDirectory;
            error = null;
            return true;
        }
    }

    private sealed class FakeThirdPartyFileManagerOperations(
        string id,
        long windowHandle,
        string currentDirectory,
        WindowsCommandResult? captureFailure = null,
        WindowsCommandResult? navigationFailure = null) :
        IThirdPartyFileManagerOperations
    {
        public string Id => id;
        public long LastCapturedFocusWindowHandle { get; private set; }
        public long LastNavigatedWindowHandle { get; private set; }
        public string? LastNavigatedDirectory { get; private set; }
        public long LastActivatedWindowHandle { get; private set; }
        public int LastPaneState { get; private set; }
        public int NavigateCount { get; private set; }
        private int _nextPaneState;

        public bool TryCapture(
            ForegroundContext foreground,
            out ThirdPartyFileManagerCapture? capture,
            out WindowsCommandResult? failure)
        {
            LastCapturedFocusWindowHandle = foreground.FocusWindowHandle;
            failure = captureFailure;
            if (failure is not null)
            {
                capture = null;
                return false;
            }
            capture = foreground.WindowHandle == windowHandle
                ? new ThirdPartyFileManagerCapture(
                    currentDirectory,
                    ++_nextPaneState)
                : null;
            return foreground.WindowHandle == windowHandle;
        }

        public bool CanNavigate(
            long candidateWindowHandle,
            object paneState) =>
            candidateWindowHandle == windowHandle &&
            paneState is int;

        public WindowsCommandResult Navigate(
            long candidateWindowHandle,
            string targetDirectory,
            object paneState)
        {
            NavigateCount++;
            if (candidateWindowHandle != windowHandle)
            {
                return new WindowsCommandResult(
                    false,
                    "File manager window was not found.");
            }
            LastPaneState = (int)paneState;
            LastNavigatedWindowHandle = candidateWindowHandle;
            LastNavigatedDirectory = targetDirectory;
            return navigationFailure ??
                new WindowsCommandResult(true);
        }

        public bool TryActivate(
            long candidateWindowHandle,
            object paneState,
            out string? error)
        {
            LastPaneState = (int)paneState;
            LastActivatedWindowHandle = candidateWindowHandle;
            error = null;
            return true;
        }
    }

    private sealed class FakeDirectoryOpusWindowOperations : IDirectoryOpusWindowOperations
    {
        public long ListerWindowHandle { get; init; }
        public string? ActiveDirectory { get; init; }
        public bool RequireRestorablePaneFocus { get; init; }
        public long LastFocusWindowHandle { get; private set; }
        public long LastNavigatedWindowHandle { get; private set; }
        public string? LastNavigatedDirectory { get; private set; }
        public long LastActivatedWindowHandle { get; private set; }

        public bool IsListerWindow(long windowHandle) =>
            windowHandle == ListerWindowHandle;

        public bool TryCaptureActivePane(
            long windowHandle,
            long focusWindowHandle,
            out string? currentDirectory,
            out DirectoryOpusPaneState? paneState)
        {
            LastFocusWindowHandle = focusWindowHandle;
            currentDirectory = windowHandle == ListerWindowHandle
                ? ActiveDirectory
                : null;
            paneState = currentDirectory is null
                ? null
                : new DirectoryOpusPaneState(90, focusWindowHandle);
            return currentDirectory is not null;
        }

        public bool IsPaneAvailable(
            long windowHandle,
            DirectoryOpusPaneState paneState) =>
            windowHandle == ListerWindowHandle &&
            paneState.ContainerWindowHandle == 90 &&
            (!RequireRestorablePaneFocus ||
             paneState.FocusWindowHandle is 0 or 90);

        public bool TryNavigate(
            long windowHandle,
            string targetDirectory,
            DirectoryOpusPaneState paneState,
            out string? error)
        {
            LastNavigatedWindowHandle = windowHandle;
            LastNavigatedDirectory = targetDirectory;
            error = null;
            return windowHandle == ListerWindowHandle;
        }

        public bool TryActivate(
            long windowHandle,
            DirectoryOpusPaneState paneState,
            out string? error)
        {
            LastActivatedWindowHandle = windowHandle;
            error = null;
            return windowHandle == ListerWindowHandle;
        }
    }

    private sealed class FakeTotalCommanderWindowOperations :
        ITotalCommanderWindowOperations
    {
        public long MainWindowHandle { get; init; }
        public string? SourceDirectory { get; init; }
        public long LastNavigatedWindowHandle { get; private set; }
        public string? LastNavigatedDirectory { get; private set; }
        public long LastActivatedWindowHandle { get; private set; }

        public bool IsMainWindow(long windowHandle) =>
            windowHandle == MainWindowHandle;

        public bool TryGetSourceDirectory(
            long windowHandle,
            out string? currentDirectory)
        {
            currentDirectory = windowHandle == MainWindowHandle
                ? SourceDirectory
                : null;
            return currentDirectory is not null;
        }

        public bool TryNavigate(
            long windowHandle,
            string targetDirectory,
            out string? error)
        {
            LastNavigatedWindowHandle = windowHandle;
            LastNavigatedDirectory = targetDirectory;
            error = null;
            return windowHandle == MainWindowHandle;
        }

        public bool TryActivate(long windowHandle, out string? error)
        {
            LastActivatedWindowHandle = windowHandle;
            error = null;
            return windowHandle == MainWindowHandle;
        }
    }

    private sealed class FakeOneCommanderWindowOperations :
        IOneCommanderWindowOperations
    {
        public long MainWindowHandle { get; init; }
        public string? CurrentDirectory { get; init; }
        public long LastAnchoredWindowHandle { get; private set; }
        public long LastFocusWindowHandle { get; private set; }
        public long LastNavigatedWindowHandle { get; private set; }
        public string? LastNavigatedDirectory { get; private set; }
        public long LastActivatedWindowHandle { get; private set; }

        public bool IsMainWindow(long windowHandle) =>
            windowHandle == MainWindowHandle;

        public bool TryCaptureActivePane(
            long windowHandle,
            long focusWindowHandle,
            out string? currentDirectory,
            out object? paneState,
            out WindowsCommandResult? failure)
        {
            LastAnchoredWindowHandle = windowHandle;
            LastFocusWindowHandle = focusWindowHandle;
            failure = null;
            currentDirectory = windowHandle == MainWindowHandle
                ? CurrentDirectory
                : null;
            paneState = currentDirectory is null ? null : "captured-pane";
            return currentDirectory is not null;
        }

        public WindowsCommandResult Navigate(
            long windowHandle,
            string targetDirectory,
            object paneState)
        {
            LastNavigatedWindowHandle = windowHandle;
            LastNavigatedDirectory = targetDirectory;
            return new WindowsCommandResult(
                windowHandle == MainWindowHandle);
        }

        public bool TryActivate(long windowHandle, out string? error)
        {
            LastActivatedWindowHandle = windowHandle;
            error = null;
            return windowHandle == MainWindowHandle;
        }
    }

    private sealed class FakeFilesWindowOperations : IFilesWindowOperations
    {
        public long MainWindowHandle { get; init; }
        public string? CurrentDirectory { get; init; }
        public long LastCapturedWindowHandle { get; private set; }
        public long LastNavigatedWindowHandle { get; private set; }
        public string? LastNavigatedDirectory { get; private set; }
        public long LastActivatedWindowHandle { get; private set; }

        public bool IsMainWindow(long windowHandle) =>
            windowHandle == MainWindowHandle;

        public bool TryCapture(
            long windowHandle,
            out string? currentDirectory,
            out object? windowState,
            out WindowsCommandResult? failure)
        {
            LastCapturedWindowHandle = windowHandle;
            failure = null;
            currentDirectory = windowHandle == MainWindowHandle
                ? CurrentDirectory
                : null;
            windowState = currentDirectory is null
                ? null
                : "captured-files-window";
            return currentDirectory is not null;
        }

        public WindowsCommandResult Navigate(
            long windowHandle,
            string targetDirectory,
            object windowState)
        {
            LastNavigatedWindowHandle = windowHandle;
            LastNavigatedDirectory = targetDirectory;
            return new WindowsCommandResult(
                windowHandle == MainWindowHandle);
        }

        public bool TryActivate(long windowHandle, out string? error)
        {
            LastActivatedWindowHandle = windowHandle;
            error = null;
            return windowHandle == MainWindowHandle;
        }
    }

    private sealed class FakeFilesPathControls(
        string currentDirectory,
        bool confirmWrites) : IFilesPathControls
    {
        private string _currentDirectory = currentDirectory;

        public string? LastWrittenDirectory { get; private set; }
        public int ReadCount { get; private set; }

        public bool TryWrite(
            string targetDirectory,
            CancellationToken cancellationToken)
        {
            cancellationToken.ThrowIfCancellationRequested();
            LastWrittenDirectory = targetDirectory;
            if (confirmWrites)
            {
                _currentDirectory = targetDirectory;
            }
            return true;
        }

        public string? Read(CancellationToken cancellationToken)
        {
            cancellationToken.ThrowIfCancellationRequested();
            ReadCount++;
            return _currentDirectory;
        }
    }

    private static class NativeFocus
    {
        public static bool IsForegroundWindow(IntPtr window) =>
            GetForegroundWindow() == window;

        public static long TryGetFocusedWindow(IntPtr window)
        {
            var threadId = GetWindowThreadProcessId(window, out _);
            if (threadId == 0)
            {
                return 0;
            }
            var info = new GuiThreadInfo
            {
                Size = Marshal.SizeOf<GuiThreadInfo>()
            };
            return GetGUIThreadInfo(threadId, ref info)
                ? info.Focus.ToInt64()
                : 0;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct GuiThreadInfo
        {
            public int Size;
            public uint Flags;
            public IntPtr Active;
            public IntPtr Focus;
            public IntPtr Capture;
            public IntPtr MenuOwner;
            public IntPtr MoveSize;
            public IntPtr Caret;
            public NativeRectangle CaretRectangle;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeRectangle
        {
            public int Left;
            public int Top;
            public int Right;
            public int Bottom;
        }

        [DllImport("user32.dll")]
        private static extern uint GetWindowThreadProcessId(
            IntPtr window,
            out uint processId);

        [DllImport("user32.dll")]
        private static extern IntPtr GetForegroundWindow();

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetGUIThreadInfo(
            uint threadId,
            ref GuiThreadInfo info);
    }

    private sealed class FakeElevatedDirectoryNavigationChannel :
        IElevatedDirectoryNavigationChannel
    {
        public DirectoryNavigationContext? CapturedContext { get; init; }
        public NavigateDirectoryCommand? LastCommand { get; private set; }
        public int CaptureCalls { get; private set; }
        public int NavigateCalls { get; private set; }

        public DirectoryNavigationContext? TryCapture(
            ForegroundContext foreground)
        {
            CaptureCalls++;
            return CapturedContext;
        }

        public WindowsCommandResult Navigate(
            NavigateDirectoryCommand command)
        {
            NavigateCalls++;
            LastCommand = command;
            return new WindowsCommandResult(true);
        }

        public void Dispose()
        {
        }
    }

    private sealed class RecordingPlatformLog : IPlatformLog
    {
        public List<RecordedLogEntry> Entries { get; } = [];

        public void Write(
            string level,
            string eventName,
            Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null) =>
            Entries.Add(new RecordedLogEntry(
                level,
                eventName,
                correlationId,
                fields is null
                    ? new Dictionary<string, object?>()
                    : new Dictionary<string, object?>(fields)));
    }

    private sealed class FakeClipboardHotKeyRegistrar : IClipboardHotKeyRegistrar
    {
        public List<(uint Modifiers, uint VirtualKey)> RegisteredBindings { get; } = [];
        public int UnregisterCalls { get; private set; }
        public int LastError => 0;

        public bool Register(uint modifiers, uint virtualKey)
        {
            RegisteredBindings.Add((modifiers, virtualKey));
            return true;
        }

        public bool Unregister()
        {
            UnregisterCalls++;
            return true;
        }
    }

    private sealed record RecordedLogEntry(
        string Level,
        string EventName,
        Guid? CorrelationId,
        IReadOnlyDictionary<string, object?> Fields);
}
