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
    internal static TestCase[] Catalog()
    {
        var tests = new (string Name, Func<Task> Run)[]
        {
            ("process window pick fixture strict wire", ProcessWindowPickTests.StrictWire),
            ("process window pick fixture ownership startup failure", ProcessWindowPickTests.OwnershipAndStartupFailure),
            ("process window pick fixture viewport hover names", ProcessWindowPickTests.ViewportHoverAndNames),
            ("process window pick fixture click release suppression", ProcessWindowPickTests.ClickAndReleaseSuppression),
            ("process window pick fixture escape cancellation", ProcessWindowPickTests.EscapeAndCancellation),
            ("process window pick fixture stale cancel bounded admission", ProcessWindowPickTests.StaleCancelAndBoundedAdmission),
            ("process window pick fixture owner disconnect dispose deadline", ProcessWindowPickTests.OwnerDisconnectDisposeAndDeadline),
            ("process window pick fixture diagnostics event envelope", ProcessWindowPickTests.DiagnosticsAndEventEnvelope),
            ("process window pick fixture actual pipe reader route", ProcessWindowPickTests.ActualPipeRouteAndReader),
            ("process window pick fixture native policy hook boundary", ProcessWindowPickTests.NativePolicyAndHookBoundary),
            ("process window pick fixture partial hooks lost release", ProcessWindowPickTests.NativePartialInstallAndLostRelease),
            ("process window pick fixture deferred cancel failure facts", ProcessWindowPickTests.CancelBeforeProcessingAndFailureFacts),
            ("installed applications fixture localized complete JSON", InstalledApplicationTests.LocalizedApplicationsAndJson),
            ("installed applications fixture invalid facts and empty", InstalledApplicationTests.InvalidFactsAndEmptyResult),
            ("installed applications fixture failure discards partial", InstalledApplicationTests.FailureNeverPublishesPartial),
            ("installed applications fixture cancellation retained admission", InstalledApplicationTests.CancellationAndRetainedAdmission),
            ("installed applications fixture exact pipe command result", InstalledApplicationTests.ExactPipeCommandAndResult),
            ("installed applications fixture ordinary shell dispatch", InstalledApplicationTests.OrdinaryShellDispatch),
            ("installed applications fixture rejects unsafe shell actions", InstalledApplicationTests.RejectUnsafeShellActions),
            ("installed applications fixture detached routing", InstalledApplicationTests.DetachedRouting),
            ("archive HZ4 process fixture capability release gate", Hz4ProcessTests.CapabilityAndReleaseGate),
            ("archive HZ4 process fixture no durable restart authority", Hz4ProcessTests.OriginalSessionHasNoRestartAuthority),
            ("archive extraction dual progress fixture wire", ArchiveJobTerminalTests.ExtractionProgressWire),
            ("archive Windows path and descriptor fixture bounds", ArchiveVirtualDragTests.PathAndDescriptorBoundaries),
            ("archive HZ4 TERM01 native cause before worker cancellation", ArchiveJobTerminalTests.NativeFailureBeforeWorkerCancelled),
            ("archive HZ4 TERM01 terminal authority controls", ArchiveJobTerminalTests.TerminalAuthorityControls),
            ("archive association settings fixture strict wire", ArchiveAssociationSettingsTests.StrictWireRejectsInvalidConfiguration),
            ("archive association settings fixture plan consumption", ArchiveAssociationSettingsTests.PlansConsumeActionsFilteringAndLayout),
            ("archive association settings fixture grouped compensation gate", ArchiveAssociationSettingsTests.GroupedServiceCompensationAndGate),
            ("archive association settings fixture create whole group", ArchiveAssociationSettingsTests.CreateForwardsWholeGroupWithoutComOrLaunch),
            ("archive association settings fixture fixed settings detached", ArchiveAssociationSettingsTests.FixedDefaultAppsWireResultsAndDetachedReader),
            ("markdown association fixture strict wire and detached reader", MarkdownAssociationTests.WireAndDetachedReader),
            ("markdown association fixture registration and effective default", MarkdownAssociationTests.RegistrationAndEffectiveDefault),
            ("markdown association fixture conflicts cancellation and failures", MarkdownAssociationTests.ConflictsCancellationAndFailures),
            ("markdown association fixture registered app settings", MarkdownAssociationTests.OpensRegisteredAppSettings),
            ("main startup fixture hidden first show client size", MainWindowStartupTests.OwnedHiddenFirstShowRetainsClientSize),
            ("main startup fixture DPI readback failure and cancellation", MainWindowStartupTests.DpiReadbackFailureAndCancellation),
            ("main startup fixture detached preparation responsiveness", MainWindowStartupTests.DetachedPreparationDoesNotBlockLaterCommands),
            ("system proxy snapshot fixture parsing", SystemProxySnapshotTests.StaticAutomaticAndInvalidConfigurations),
            ("floating picker fixture user wait", FloatingAssistantCoordinatorTests.PickerWaitingForUserIsNotBackgroundTimeout),
            ("floating picker fixture uses source modal owner", FloatingAssistantCoordinatorTests.PickerUsesSourceWindowAsModalOwner),
            ("floating coordinator fixture strict wire and event", FloatingAssistantCoordinatorTests.StrictWireAndEvents),
            ("floating coordinator fixture environment signature retries", FloatingAssistantCoordinatorTests.EnvironmentSignatureRetriesAndDeduplicates),
            ("floating coordinator fixture fullscreen policy", FloatingAssistantCoordinatorTests.FullscreenPolicyRejectsDesktopAndFramedWindows),
            ("floating coordinator fixture invalid picker peer", FloatingAssistantCoordinatorTests.InvalidPickerPeerFailsBeforeDialog),
            ("floating coordinator fixture source destroyed retires", FloatingAssistantCoordinatorTests.SourceDestroyedRetiresInput),
            ("floating coordinator fixture full event queue cleans stage", FloatingAssistantCoordinatorTests.FullEventQueueCleansExactStaging),
            ("floating coordinator fixture owned hidden show retire", FloatingAssistantCoordinatorTests.OwnedHiddenShowRetire),
            ("floating coordinator fixture owned drop lifetime and correlation", FloatingAssistantCoordinatorTests.OwnedDropLifetimeAndCorrelation),
            ("floating coordinator fixture full native queue cleanup", FloatingAssistantCoordinatorTests.FullNativeQueueRetainsCleanup),
            ("floating coordinator fixture release deletion owner", FloatingAssistantCoordinatorTests.ReleaseDeletionFailureRetainsOwner),
            ("floating coordinator fixture retire deletion owner", FloatingAssistantCoordinatorTests.RetireDeletionFailureRetainsOwner),
            ("floating coordinator fixture rejected deletion owner", FloatingAssistantCoordinatorTests.RejectedDeletionFailureRetainsOwner),
            ("floating coordinator fixture stop deletion owner", FloatingAssistantCoordinatorTests.StopDeletionFailureRetainsOwner),
            ("floating input fixture transparent hit and pass through", FloatingAssistantInputTests.TransparentHitAndPassThrough),
            ("floating input fixture bounded OLE capture once", FloatingAssistantDropTests.CapturesOnceAndRejectsInactive),
            ("floating input fixture throwing OLE gate stays passive", FloatingAssistantDropTests.ThrowingOfferGateReturnsOleFailure),
            ("floating input fixture foreign renderer OLE lifecycle", FloatingAssistantInputTests.ForeignRendererOleLifecycle),
            ("floating input fixture native single gestures and lock", FloatingAssistantInputTests.GesturesStaySingleAndLockStillClicks),
            ("floating input fixture native COM ABI and bounded failures", FloatingAssistantDropTests.NativeComAbiAndBoundedFailures),
            ("floating input native actual owned OLE routing", FloatingAssistantOleAcceptance.ActualOleRoutesAcrossOwnedProcesses),
            ("trusted recovery cleanup preserves foreign archive files", TrustedUpdateRecoveryTests.SupersededCleanupPreservesForeignFiles),
            ("missing bootstrap fixture missing root once", TrustedMissingBootstrapRepairTests.MissingRootEnrollsExactlyOnce),
            ("missing bootstrap fixture preserves existing root and rejects registration", TrustedMissingBootstrapRepairTests.ExistingRootAndBadRegistrationNeverStop),
            ("missing bootstrap fixture refuses native failures", TrustedMissingBootstrapRepairTests.FailureMatrixPreservesOriginalBoundary),
            ("missing bootstrap fixture actual missing vendor product parents", TrustedMissingBootstrapRepairTests.MissingProductParentsUseActualDirectoryValidation),
            ("protected uninstall fixture owned services and deployment", TrustedUninstallCleanupTests.OwnedServicesAndProtectedDeploymentsAreRemoved),
            ("protected uninstall fixture foreign and recovery state", TrustedUninstallCleanupTests.ForeignOrRecoveryStateNeverMutates),
            ("protected uninstall fixture native failure", TrustedUninstallCleanupTests.NativeFailureIsNotSuccessfulCleanup),
            ("protected uninstall fixture absent no-op", TrustedUninstallCleanupTests.AbsentComponentsAreANoOp),
            ("bootstrap maintenance rejects wrong trust downgrade tamper", TrustedBootstrapMaintenanceTests.RejectsWrongTrustDowngradeAndTamperBeforeStop),
            ("bootstrap maintenance interrupted rollback retry", TrustedBootstrapMaintenanceTests.InterruptedRollbackRecoversOnNextMaintenance),
            ("bootstrap maintenance start failure restores original", TrustedBootstrapMaintenanceTests.StartFailureRestoresOriginalBootstrap),
            ("trusted recovery legacy rollback strict retry", TrustedUpdateRecoveryTests.LegacySnapshotRollsBackFailedUpdateAndCommitsStrictRetry),
            ("trusted recovery legacy restart snapshot", TrustedUpdateRecoveryTests.RestartRestoresPreciselyBoundLegacySnapshot),
            ("trusted recovery exact identity artifact rejection", TrustedUpdateRecoveryTests.RecoverySnapshotRejectsWrongIdentityAndInvalidArtifacts),
            ("trusted recovery never admits new legacy package", TrustedUpdateRecoveryTests.RecoverySnapshotCannotAdmitLegacyAsNewPackage),
            ("bootstrap maintenance same trust preserves release state", TrustedBootstrapMaintenanceTests.SameTrustMaintenancePreservesState),
            ("archive create overwrite fixture actual confirmation cancel changed target", ArchiveCreateOverwriteTests.ActualConfirmationCancelOverwriteChangedTarget),
            ("archive create overwrite fixture detached preflight cancellation slot", ArchiveCreateOverwriteTests.ActualDetachedPreflightCancellationRetainsSlot),
            ("archive cli fixture exact arguments early mode", ArchiveCliHostTests.ExactArgumentsAndEarlyMode),
            ("archive cli fixture dispatch allowlist receipts", ArchiveCliHostTests.DispatchAllowlistAndReceipts),
            ("archive cli fixture denied commands no effects", ArchiveCliHostTests.DeniedCommandsHaveNoEffects),
            ("archive cli fixture stale controls original sessions", ArchiveCliHostTests.StaleControlsUseOriginalSessions),
            ("archive cli lifecycle fixture actual cleanup timeout retains ownership", ArchiveCliCleanupLifecycleTests.ActualCleanupTimeoutRetainsOwnership),
            ("OPT03 create actual native result locations", Opt03NativeTests.ActualCreateResultLocations),
            ("OPT03 clipboard owned HGLOBAL intake", Opt03NativeTests.ClipboardOwnedMemory),
            ("OPT03 create destination held namespace", Opt03NativeTests.CreateDestinationObservation),
            ("archive job fixture strict wire picker cancellation", ArchiveJobTests.StrictWireAndPickerCancellation),
            ("archive job fixture actual create extract test journal", ArchiveJobTests.ActualCreateExtractTestJournal),
            ("archive job fixture actual conflict global admission cancel", ArchiveJobTests.ActualConflictGlobalAdmissionCancel),
            ("archive job fixture actual selected ZIP 7z RAR source binding", ArchiveJobTests.ActualSelectedZipSevenzRarAndChangedSource),
            ("archive job fixture actual readmission shortfall no output", ArchiveJobTests.ActualReadmissionShortfallNoOutput),
            ("archive job fixture actual password diagnostics native failure", ArchiveJobTests.ActualPasswordDiagnosticsAndNativeFailure),
            ("archive job fixture actual cancel after publication native facts", ArchiveJobTests.ActualCancelAfterPublicationRetainsNativeFacts),
            ("archive job fixture actual create formats link warning empty", ArchiveJobTests.ActualAllCreateFormatsAndLinkWarnings),
            ("archive job fixture actual risk pause prompt timeout", ArchiveJobTests.ActualRiskAndPausePromptTimeout),
            ("archive job fixture actual unique destination folders", ArchiveJobTests.ActualUniqueDestinationFolder),
            ("archive job fixture empty directory result location worker terminal", ArchiveJobTests.ActualEmptyDirectoryResultLocation),
            ("archive job fixture empty directory no output terminal guards", ArchiveJobTests.ActualEmptyDirectoryNoOutputAndTerminalGuards),
            ("archive destination fixture missing low space confirmation", ArchiveDestinationPreflightTests.MissingAndLowSpaceRequireOriginalConfirmation),
            ("archive recovery inspection fixture bounded detached diagnostics", ArchiveRecoveryInspectionTests.BoundedDetachedAndPrivateDiagnostics),
            ("archive recovery inspection fixture readonly descriptor target", ArchiveRecoveryInspectionTests.ReadonlyDescriptorAndTarget),
            ("archive recovery inspection fixture completed torn invalid records", ArchiveRecoveryInspectionTests.CompletedTornAndInvalidRecords),
            ("archive recovery inspection fixture replaced authority strict wire", ArchiveRecoveryInspectionTests.ReplacedAuthorityAndStrictWire),
            ("archive space fixture held read-only observation", ArchiveSpacePreflightTests.HeldObservationDoesNotCreateOrMutate),
            ("archive space fixture native allocation identity", ArchiveSpacePreflightTests.NativeAllocationAndIdentity),
            ("archive space fixture missing suffix target binding", ArchiveSpacePreflightTests.MissingSuffixBindsExpectedTarget),
            ("archive space fixture expected source target changes", ArchiveSpacePreflightTests.ExpectedSourceAndTargetChanges),
            ("archive space fixture capacity byte override", ArchiveSpacePreflightTests.CapacityOverrideIsObservationOnly),
            ("archive space fixture typed native open failures", ArchiveSpacePreflightTests.NativeOpenFailuresAreTyped),
            ("archive space fixture native reparse refusal", ArchiveSpacePreflightTests.NativeReparseIsRejected),
            ("archive space fixture cancellation publication", ArchiveSpacePreflightTests.CancellationCannotPublishObservation),
            ("archive space fixture strict wire private diagnostics", ArchiveSpacePreflightTests.StrictWireAndPrivateDiagnostics),
            ("archive space fixture original dispatcher retained bounded slots", ArchiveSpacePreflightTests.OriginalDispatcherRetainsBlockedSlots),
            ("archive space fixture detached cancellation no late receipt", ArchiveSpacePreflightTests.DetachedCancellationHasNoLateReceipt),
            ("archive temp fixture strict policy wire", ArchiveTempSettingsTests.StrictPolicyWire),
            ("archive temp fixture strict lease open wire", ArchiveTempSettingsTests.StrictLeaseOpenWire),
            ("archive temp fixture placement retention restart", ArchiveTempSettingsTests.PlacementRetentionAndRestart),
            ("archive temp fixture bounded inventory discovery", ArchiveTempSettingsTests.BoundedRootInventoryAndDiscovery),
            ("archive temp fixture reparse inventory preservation", ArchiveTempSettingsTests.ReparseAndInventoryTamperPreserved),
            ("archive temp fixture live child browser release prompt return", ArchiveTempSettingsTests.LiveChildBrowserReleaseAndPromptReturn),
            ("archive temp fixture child exit on off already exited", ArchiveTempSettingsTests.ExitOptionOnOffAndAlreadyExited),
            ("archive temp fixture no handle launch failure native capture", ArchiveTempSettingsTests.NoHandleLaunchFailureAndNativeCapture),
            ("archive temp fixture identity edited locked exit", ArchiveTempSettingsTests.OpenIdentityAndEditedLockedExit),
            ("archive temp fixture disconnect cancellation drain", ArchiveTempSettingsTests.DisconnectCancelsObservationWithoutDrain),
            ("archive temp fixture launch release race detached drain", ArchiveTempSettingsTests.LaunchReleaseRaceAndDetachedDrain),
            ("archive cache fixture obstructed retirement restart", ArchiveCacheRetirementTests.ObstructedRetirementAndRestart),
            ("archive cache fixture expiry startup retained owners", ArchiveCacheRetirementTests.ExpiryStartupAndRetainedOwners),
            ("archive cache fixture ancestry malformed recovery", ArchiveCacheRetirementTests.AncestryAndMalformedRecovery),
            ("archive cache fixture fair local retirement rotation", ArchiveCacheRetirementTests.LocallyRetiredLeasesRotateWithoutStarvation),
            ("archive browser encrypted selected ZIP private lifecycle", ArchiveEncryptedBrowserTests.EncryptedSelectedZipUsesOriginalPrivatePasswordWait),
            ("archive browser encrypted RAR header cancel selected lifecycle", ArchiveEncryptedBrowserTests.HeaderEncryptedRarReadCancelAndSelected),
            ("archive browser fixture session stale missing helper", ArchiveBrowserTests.SessionRejectsStaleAndMissingHelper),
            ("archive browser fixture empty context actual budget", ArchiveBrowserTests.ReadBrowserBudgetAndEmpty),
            ("archive browser worker selected revision lifecycle", ArchiveBrowserTests.ActualSelectedEntryAndRevision),
            ("archive browser fixture strict wire backward defaults", ArchiveBrowserTests.WireAndBackwardDefaults),
            ("archive browser fixture owned lease edited retention", ArchiveBrowserTests.OwnedLeaseRetention),
            ("archive 7z safety fixture packed metadata", SevenzRiskReadmissionTests.ActualSevenzPackedMetadata),
            ("archive 7z safety fixture risk and solid semantics", SevenzRiskReadmissionTests.ActualSevenzRiskAndSolidSemantics),
            ("archive 7z safety fixture publication cancellation skip", SevenzRiskReadmissionTests.ActualSevenzPublicationCancellationAndSkip),
            ("archive 7z safety fixture low total preflight", SevenzRiskReadmissionTests.ActualSevenzLowTotalPreflight),
            ("archive 7z safety fixture TEST no risk interaction", SevenzRiskReadmissionTests.ActualSevenzTestHasNoRiskInteraction),
            ("archive RAR safety fixture ordinary open TEST metadata", RarRiskReadmissionTests.ActualRarOrdinaryOpenAndTest),
            ("archive RAR safety fixture low total preflight", RarRiskReadmissionTests.ActualRarLowTotalPreflight),
            ("archive RAR safety fixture conflicts skip overwrite", RarRiskReadmissionTests.ActualRarConflictSkipAndOverwrite),
            ("archive RAR safety fixture password preparation", RarRiskReadmissionTests.ActualRarPasswordPreparation),
            ("archive RAR safety fixture postpublication cancellation", RarRiskReadmissionTests.ActualRarPostpublicationCancellation),
            ("archive RAR safety fixture current budget risk tickets", RarRiskInteractionTests.ActualRiskTickets),
            ("archive RAR safety fixture solid skip integrity", RarRiskInteractionTests.ActualSolidSkipAndIntegrity),
            ("archive RAR safety fixture volume packed unknown", RarRiskInteractionTests.ActualVolumePackedUnknown),
            ("archive TAR stream safety fixture low total preflight", TarStreamSafetyTests.ActualLowTotalPreflight),
            ("archive TAR stream safety fixture metadata ordinary no risk", TarStreamSafetyTests.ActualMetadataAndNoRisk),
            ("archive TAR stream safety fixture unknown corruption finite spool", TarStreamSafetyTests.ActualUnknownAndCorrupt),
            ("archive TAR stream safety fixture postcommit cancellation", TarStreamSafetyTests.ActualPostcommitCancellation),
            ("archive TAR stream safety fixture known stream risk tickets", TarStreamRiskTests.KnownDeclarationsUseOriginalRiskTickets),
            ("archive readmission fixture preflight no output", ArchiveReadmissionTests.PreflightHasNoOutputEffects),
            ("archive risk fixture actual ZIP confirmation", ArchiveRiskTests.ActualZipRiskConfirmation),
            ("archive conflict fixture actual ZIP decisions", ArchiveConflictTests.ActualZipConflictDecisions),
            ("archive conflict fixture actual 7z RAR TAR decisions", ArchiveFormatConflictTests.ActualFormatConflicts),
            ("archive conflict fixture actual standalone stream decisions", ArchiveStreamConflictTests.ActualStandaloneConflicts),
            ("archive password fixture actual wait retry cancel", ArchivePasswordTests.ActualPasswordWaitRetryAndCancel),
            ("archive password fixture actual encrypted extraction", ArchiveExtractPasswordTests.ActualEncryptedExtraction),
            ("archive encoding fixture Auto manual native output", ArchiveEncodingTests.AutoAndManualNativeOutput),
            ("archive cancel boundary fixture actual streaming receive observe cleanup", ArchiveCancellationBoundaryTests.StreamingReceiveObserveCleanup),
            ("archive long path fixture extraction layout", ArchiveLongPathTests.ExtractionLayout),
            ("archive long path fixture extraction publication", ArchiveLongPathTests.ExtractionPublication),
            ("archive long path fixture creation publication", ArchiveLongPathTests.CreationPublication),
            ("archive long path fixture unsafe members", ArchiveLongPathTests.UnsafeMembers),
            ("archive product fixture rejection transport bound", ArchiveProductSessionTests.RejectionsAndTransportBound),
            ("archive product fixture actual open test privacy", ArchiveProductSessionTests.ActualOpenTestAndPrivacy),
            ("archive product fixture slow credit budget cancellation", ArchiveProductSessionTests.SlowCreditBudgetsAndCancellation),
            ("archive product fixture negotiation crash admission unique terminal", ArchiveProductSessionTests.NegotiationCrashAdmissionAndUniqueTerminal),
            ("archive product fixture strict wire validation", ArchiveProductReadTests.WireValidation),
            ("archive product fixture helper protection", ArchiveProductReadTests.HelperProtection),
            ("archive product fixture metadata sequence and budgets", ArchiveProductReadTests.MetadataValidation),
            ("archive product fixture captured cancellation actual cleanup", ArchiveProductSessionTests.CapturedCancellationFollowsActualCleanup),
            ("archive product fixture timeout is not user cancellation", ArchiveProductSessionTests.TimeoutIsNotUserCancellation),
            ("archive product fixture v2 finite negotiation", ArchiveProductSessionTests.LargeReadNegotiationAndFiniteBudgets),
            ("archive product fixture v2 actual open test privacy", ArchiveProductSessionTests.ActualLargeReadOpenTestAndPrivacy),
            ("archive source delete fixture original session evidence binding", ArchiveSourceDeletionTests.OriginalSessionRejectsChangedEvidence),
            ("archive source delete fixture retired replacement replay", ArchiveSourceDeletionTests.RetiredEvidenceCannotBindReplacement),
            ("archive source delete fixture partial cancellation privacy", ArchiveSourceDeletionTests.PartialResultsCancellationAndPrivacy),
            ("archive source delete fixture codec full extraction", ArchiveSourceDeletionTests.CodecFullExtraction),
            ("archive source delete fixture lease close resolve race", ArchiveSourceDeletionTests.LeaseCloseResolveRace),
            ("archive source delete fixture failure cancellation matrix", ArchiveSourceDeletionTests.FailureAndCancellationMatrix),
            ("archive source delete fixture creation coverage native refusal", ArchiveSourceDeletionTests.CreationCoverageAndNativeRefusal),
            ("archive source delete fixture once only unknown binding", ArchiveSourceDeletionTests.OnceOnlyUnknownAndBinding),
            ("archive source delete fixture consent failure projection", ArchiveSourceDeletionTests.ConsentAndFailureProjection),
            ("archive TAR time fixture local scope", ArchiveTarTimeTests.PaxLocalScope),
            ("archive TAR time fixture edges retention", ArchiveTarTimeTests.TimeEdgesAndRetention),
            ("archive TAR time fixture PAX global clear", ArchiveTarTimeTests.PaxGlobalClear),
            ("archive TAR time fixture GNU negative", ArchiveTarTimeTests.GnuNegative),
            ("archive metadata fixture RAR metadata", ArchiveMetadataTests.RarMetadata),
            ("archive metadata fixture ZIP time edges", ArchiveMetadataTests.ZipTimeEdges),
            ("archive metadata fixture PAX time edges", ArchiveMetadataTests.PaxTimeEdges),
            ("archive metadata fixture stream metadata absence", ArchiveMetadataTests.StreamsWithoutMetadata),
            ("archive metadata fixture ADS capability source binding", ArchiveMetadataTests.AdsCapabilityAndSourceBinding),
            ("archive metadata fixture guarded metadata cleanup", ArchiveMetadataTests.GuardedMetadataCleanup),
            ("archive metadata fixture native lifecycle", ArchiveMetadataTests.NativeLifecycle),
            ("archive metadata fixture 7z metadata", ArchiveMetadataTests.SevenzMetadata),
            ("archive metadata fixture TAR PAX modified", ArchiveMetadataTests.TarModified),
            ("archive metadata fixture Gzip modified", ArchiveMetadataTests.GzipModified),
            ("archive metadata fixture directory finalization", ArchiveMetadataTests.DirectoryFinalization),
            ("archive metadata fixture MOTW options and budget", ArchiveMetadataTests.MotwOptionsAndBudget),
            ("archive metadata fixture capabilities", ArchiveMetadataTests.Capabilities),
            ("archive metadata fixture MOTW default", ArchiveMetadataTests.MotwDefault),
            ("archive metadata fixture ZIP attributes", ArchiveMetadataTests.ZipAttributes),
            ("archive metadata fixture ZIP UTC modified", ArchiveMetadataTests.ZipUtcModified),
            ("archive metadata fixture ZIP DOS modified", ArchiveMetadataTests.ZipDosModified),
            ("archive output fixture native failure diagnostics privacy", ArchiveOutputTests.DiagnosticAndNativeFailure),
            ("archive output fixture nested unicode and unsafe names", ArchiveOutputTests.NestedAndUnsafe),
            ("archive output fixture existing target and owned cleanup", ArchiveOutputTests.PreserveExistingAndCancel),
            ("archive output fixture concurrent target and owned junction", ArchiveOutputTests.ConcurrentTargetAndReparse),
            ("archive output fixture rechecked atomic path overwrite", ArchiveOutputTests.RecheckedAtomicPathOverwrite),
            ("archive direct extraction fixture visible closed and aborted output", ArchiveDirectExtractionTests.DirectFilesStayVisibleAndClosedFilesSurvive),
            ("archive direct extraction fixture metadata MOTW preferred location", ArchiveDirectExtractionTests.MetadataAndPreferredLocation),
            ("archive direct extraction fixture conflict overwrite and failed output", ArchiveDirectExtractionTests.ConflictOverwriteAndFailure),
            ("archive direct extraction fixture unsafe and junction refusal", ArchiveDirectExtractionTests.RefusesUnsafeAndReparse),
            ("archive extraction pipeline fixture ordered writes close and first failure", ArchiveExtractionPipelineTests.OrderedWritesAndClose),
            ("archive parallel ordinary native begin overlap fixture", ArchiveParallelExtractionTests.NativeBeginsOverlap),
            ("archive parallel ordinary native begin cap fixture", ArchiveParallelExtractionTests.NativeBeginCap),
            ("archive parallel ordinary incomplete cancellation and first native error fixture", ArchiveParallelExtractionTests.IncompleteCancellationAndFirstNativeError),
            ("archive parallel missing capability before effects fixture", ArchiveParallelExtractionTests.MissingParallelCapabilityHasNoOutput),
            ("archive parallel native close overlap after payload fixture", ArchiveParallelExtractionTests.NativeClosesOverlapAfterPayloadDrain),
            ("archive parallel later payload after close fixture", ArchiveParallelExtractionTests.LaterPayloadAfterCloseIsRejected),
            ("archive parallel actual Rust native overlap and bytes fixture", ArchiveParallelExtractionTests.ActualZipMemberOverlapAndBytes),
            ("archive parallel actual Rust native multiple incomplete cancellation fixture", ArchiveParallelExtractionTests.ActualCancelMultipleIncomplete),
            ("archive parallel actual Rust native CRC completed retention fixture", ArchiveParallelExtractionTests.ActualCrcFailureRetainsCompleted),
            ("archive parallel pending observation excludes completed calls fixture", ArchiveParallelExtractionTests.PendingObservationExcludesCompletedCalls),
            ("archive overwrite fixture target changes before recheck", ArchiveOverwriteTests.ChangesBeforeRecheck),
            ("archive overwrite fixture failures native occupation capability", ArchiveOverwriteTests.FailuresAndNativeOccupation),
            ("archive overwrite fixture postcommit and cancellation", ArchiveOverwriteTests.PostCommitAndCancellation),
            ("archive overwrite fixture exclusive partial ownership", ArchiveOverwriteTests.ExclusivePartialOwnership),
            ("archive overwrite fixture actual worker integrity and commit lifecycle", ArchiveOverwriteTests.ActualWorkerOverwriteLifecycle),
            ("archive overwrite fixture Win32 counted name ABI", ArchiveRenameAbiTests.CountedNameMustStillBeTerminated),
            ("archive outcome fixture actual worker target failures", ArchiveWorkerOutcomeTests.ActualTargetFailures),
            ("archive outcome fixture actual worker terminal diagnostics", ArchiveWorkerOutcomeTests.ActualTerminalDiagnostics),
            ("archive outcome fixture actual worker operation isolation", ArchiveWorkerOutcomeTests.ActualOperationIsolation),
            ("archive tree creation fixture options diagnostics", ArchiveTreeCreationTests.OptionsAndDiagnostics),
            ("archive tree creation fixture TAR safety", ArchiveTreeCreationTests.TarSafety),
            ("archive tree creation fixture layout immutable admission", ArchiveTreeCreationTests.LayoutAndAdmission),
            ("archive tree creation fixture TXZ levels", ArchiveTreeCreationTests.TarXz),
            ("archive tree creation fixture TGZ levels", ArchiveTreeCreationTests.TarGzip),
            ("archive tree creation fixture TAR plain", ArchiveTreeCreationTests.TarPlain),
            ("archive tree creation fixture 7z options", ArchiveTreeCreationTests.SevenzOptions),
            ("archive tree creation fixture 7z safety", ArchiveTreeCreationTests.SevenzSafety),
            ("archive tree creation fixture 7z plain", ArchiveTreeCreationTests.SevenzPlain),
            ("archive ZIP creation fixture plain single file", ArchiveZipCreationTests.PlainSingleFile),
            ("archive ZIP creation fixture directory levels roots", ArchiveZipCreationTests.DirectoryLevelsAndRoots),
            ("archive ZIP creation fixture multiple sources collisions", ArchiveZipCreationTests.MultipleSourcesAndCollisions),
            ("archive ZIP creation fixture source changes budgets publication", ArchiveZipCreationTests.SourceChangesBudgetsAndPublication),
            ("archive ZIP creation fixture AES independent interoperability", ArchiveZipCreationTests.AesCreationInterop),
            ("archive ZIP creation fixture native source limits reparse", ArchiveZipCreationTests.NativeSourceLimitsAndReparse),
            ("archive ZIP creation fixture diagnostics options ownership", ArchiveZipCreationTests.DiagnosticsAndOptions),
            ("archive ZIP creation fixture large ZIP64 streaming", ArchiveZipCreationTests.LargeZip64Streaming),
            ("archive admission fixture source elements", ArchiveAdmissionTests.SourceElements),
            ("archive admission fixture source count", ArchiveAdmissionTests.SourceCount),
            ("archive admission fixture source clear", ArchiveAdmissionTests.SourceClear),
            ("archive admission fixture exclusion snapshot", ArchiveAdmissionTests.Exclusions),
            ("archive admission fixture budget snapshot", ArchiveAdmissionTests.Budgets),
            ("archive admission fixture overwrite authorization", ArchiveAdmissionTests.OverwriteAuthorization),
            ("archive admission fixture legacy authorization", ArchiveAdmissionTests.LegacySharedAuthorization),
            ("archive admission fixture bounds and unstable capture", ArchiveAdmissionTests.AdmissionBounds),
            ("archive source binding fixture safe parent output", ArchiveSourceBindingTests.ParentOutput),
            ("archive source binding fixture self inclusion guards", ArchiveSourceBindingTests.SelfInclusion),
            ("archive source binding fixture namespace replacement", ArchiveSourceBindingTests.NamespaceReplacement),
            ("archive source binding fixture held output namespace", ArchiveSourceBindingTests.HeldOutputNamespace),
            ("archive source binding fixture missing internal output", ArchiveSourceBindingTests.MissingInternalOutput),
            ("archive source binding fixture missing internal multi identity", ArchiveSourceBindingTests.MissingInternalMultiAndIdentity),
            ("archive source binding fixture existing internal unchanged", ArchiveSourceBindingTests.ExistingInternalUnchanged),
            ("archive source binding fixture missing external output", ArchiveSourceBindingTests.MissingExternalOutput),
            ("archive source binding fixture preparation failures", ArchiveSourceBindingTests.PreparationFailures),
            ("archive admission fixture error compatibility", ArchiveAdmissionTests.ErrorCompatibility),
            ("archive stream fixture actual worker gzip extraction", ArchiveStreamWorkerTests.ActualGzipExtraction),
            ("archive stream fixture actual worker gzip creation", ArchiveStreamWorkerTests.ActualGzipCreation),
            ("archive stream fixture empty Gzip XZ creation interop", ArchiveStreamWorkerTests.EmptyGzipXzCreationInterop),
            ("archive stream fixture creation safety and partial read", ArchiveStreamWorkerTests.CreationSafetyAndPartialRead),
            ("archive stream fixture actual worker bzip2 ZIP extraction", ArchiveStreamWorkerTests.ActualBzip2ZipExtraction),
            ("archive stream fixture actual worker TAR extraction safety", ArchiveStreamWorkerTests.ActualTarExtraction),
            ("archive stream fixture actual worker TAR Gzip integrity", ArchiveStreamWorkerTests.ActualTarGzipExtraction),
            ("archive stream fixture actual worker BZip2 streams integrity", ArchiveStreamWorkerTests.ActualBzip2Streams),
            ("archive stream fixture actual worker XZ creation", ArchiveStreamWorkerTests.ActualXzCreation),
            ("archive XZ fixture actual worker open test extract", ArchiveXzWorkerTests.OpenTestExtract),
            ("archive XZ fixture actual worker damage and budgets", ArchiveXzWorkerTests.DamageAndBudgets),
            ("archive XZ fixture actual worker cancellation source protection", ArchiveXzWorkerTests.CancellationAndSourceProtection),
            ("archive stream fixture actual worker Zstd reads safety", ArchiveStreamWorkerTests.ActualZstdReads),
            ("archive ZIP method fixture Zstd actual worker native reads", ArchiveZipMethodWorkerTests.ZstdRead),
            ("archive ZIP method fixture XZ actual worker native reads", ArchiveZipMethodWorkerTests.XzRead),
            ("archive ZIP method fixture LZMA actual worker native reads", ArchiveZipMethodWorkerTests.LzmaRead),
            ("archive ZIP method fixture Deflate64 actual worker native reads", ArchiveZipMethodWorkerTests.Deflate64Read),
            ("archive ZIP crypto fixture ZipCrypto actual worker native reads", ArchiveZipMethodWorkerTests.ZipCryptoRead),
            ("archive ZIP crypto fixture ZipCrypto descriptor actual worker native reads", ArchiveZipMethodWorkerTests.ZipCryptoDescriptorRead),
            ("archive ZIP remediation fixture directory and comment boundaries", ArchiveZipReadRemediationTests.DirectoryAndCommentBoundaries),
            ("archive ZIP remediation fixture AES authentication and safety", ArchiveZipReadRemediationTests.AesReadsAndAuthentication),
            ("archive ZIP remediation fixture encrypted combinations and ZIP64", ArchiveZipReadRemediationTests.EncryptedCombinations),
            ("archive ZIP remediation fixture noEOS and working budget", ArchiveZipReadRemediationTests.NoEosAndWorkingBudget),
            ("archive worker fixture suspended admission and job limits", ArchiveWorkerTests.SuspendedAdmissionAndJobLimits),
            ("archive worker fixture framed output and privacy", ArchiveWorkerTests.FramesAndPrivacy),
            ("archive worker fixture cancellation and bounded admission", ArchiveWorkerTests.CancellationAndAdmission),
            ("archive worker fixture stuck admission retains slots", ArchiveWorkerTests.StuckAdmissionRetainsSlots),
            ("archive worker actual Rust private protocol fixture", ArchiveWorkerTests.ActualRustWorkerPrivateProtocol),
            ("archive codec actual Rust RAR4 RAR5 and 7z fixture", ArchiveCodecWorkerTests.IndependentRarAndSevenzReadTest),
            ("archive codec fixture x86 BCJ LZMA2 interop", ArchiveCodecWorkerTests.X86BcjLzma2Interop),
            ("archive codec fixture solid packed optional metadata", ArchiveCodecWorkerTests.SolidPackedMetadata),
            ("archive codec actual Rust password budget corruption cancel fixture", ArchiveCodecWorkerTests.PasswordBudgetCorruptionAndCancel),
            ("archive extraction actual Rust handle-session fixture", ArchiveExtractionWorkerTests.ActualZipToHandleSession),
            ("archive extraction actual ZIP cancel during second file write", ArchiveExtractionWorkerTests.CancelWhileSecondFileWriting),
            ("archive extraction actual Rust 7z handle-session fixture", ArchiveExtractionWorkerTests.ActualSevenzToHandleSession),
            ("archive extraction actual Rust RAR handle-session fixture", ArchiveExtractionWorkerTests.ActualRarToHandleSession),
            ("archive remediation actual source lease race barriers fixture", ArchiveRemediationTests.SourceRaceBarriers),
            ("archive remediation budget admission fixture", ArchiveRemediationTests.BudgetAdmission),
            ("archive remediation actual 7z working budget fixture", ArchiveRemediationTests.SevenzWorkingBudget),
            ("archive remediation actual 7z header classification fixture", ArchiveRemediationTests.SevenzHeaderClassification),
            ("archive remediation actual mapped source race fixture", ArchiveRemediationTests.MappedSourceRace),
            ("archive remediation native Job limit and cleanup fixture", ArchiveRemediationTests.NativeJobLimitAndCleanup),
            ("notification window fixture typed wire and peer validation", NotificationWindowTests.WireAndPeerValidation),
            ("notification window fixture six anchors DPI and readback", NotificationWindowTests.SixAnchorsDpiAndReadback),
            ("notification window fixture bounded detached and diagnostics", NotificationWindowTests.BoundedDetachedAndDiagnostics),
            ("notification window fixture owned HWND hidden policy and client readback", NotificationWindowTests.OwnedHwndHiddenPolicyAndClientReadback),
            ("native context menu fixture semantic validation and wire", NativeContextMenuTests.SemanticValidationAndWire),
            ("native context menu fixture admission cancellation and nonblocking", NativeContextMenuTests.AdmissionCancellationAndNonblocking),
            ("native context menu owned HWND overflow and exact cancel", NativeContextMenuTests.OwnedHwndOverflowAndExactCancel),
            ("extension setting path fixture wire owner cancel and selection", ExtensionSettingPathTests.WireOwnerCancellationAndSelection),
            ("extension runtime fixture typed wire picker and STA failures", ExtensionRuntimeTests.TypedWirePickerAndStaFailures),
            ("extension runtime fixture owned versions hash and budgets", ExtensionRuntimeTests.OwnedVersionsHashAndBudgets),
            ("extension runtime fixture owned PATH discovery and precedence", ExtensionRuntimeTests.OwnedPathDiscoveryAndPrecedence),
            ("extension runtime fixture missing timeout cancel and output privacy", ExtensionRuntimeTests.MissingTimeoutCancelAndOutputPrivacy),
            ("extension runtime fixture detached reader responsiveness", ExtensionRuntimeTests.DetachedCommandsKeepReaderResponsive),
            ("extension process fixture owned echo and suspended admission", ExtensionProcessTests.OwnedEchoAndSuspendedAdmission),
            ("extension process fixture strict UTF8 and output limits", ExtensionProcessTests.OutputBoundsAndStrictUtf8),
            ("extension process fixture timeout cancel and pipe holding tree", ExtensionProcessTests.TimeoutCancellationAndPipeHoldingTree),
            ("extension process fixture concurrency memory and duplex", ExtensionProcessTests.AdmissionConcurrencyMemoryAndDuplex),
            ("extension process fixture admission failure and ordinary token", ExtensionProcessTests.FailureBeforeResumeAndOrdinaryToken),
            ("extension process fixture input and native process limits", ExtensionProcessTests.InputBoundsAndNativeProcessLimit),
            ("extension process fixture stuck admission slots and diagnostics privacy", ExtensionProcessTests.StuckAdmissionRetainsBoundedSlotAndPrivacy),
            ("extension process fixture broker unlimited control stream", ExtensionProcessBrokerTests.ControlReaderStreamsBeyondInt32WithoutLifetimeBudget),
            ("extension process fixture finite reader boundaries", ExtensionProcessBrokerTests.FiniteReaderBudgetsAndFrameErrors),
            ("extension process fixture broker controls and parent EOF", ExtensionProcessBrokerTests.ControlResponsivenessAndParentEof),
            ("extension process fixture broker crash and actual parent death", ExtensionProcessBrokerTests.BrokerCrashAndActualParentDeath),
            ("extension process fixture parent identity and malformed controls", ExtensionProcessBrokerTests.ParentIdentityAndMalformedControl),
            ("credential fixture typed routing and value privacy", CredentialAdapterTests.TypedRoutingAndValuePrivacy),
            ("credential fixture bounds malformed and native metadata", CredentialAdapterTests.BoundsMalformedAndNativeMetadata),
            ("credential fixture cancel and attempted write compensation seam", CredentialAdapterTests.CancelAndAttemptedWriteCompensationSeam),
            ("credential fixture bounded worker control responsiveness", CredentialAdapterTests.BoundedWorkerKeepsControlResponsive),
            ("credential fixture synthetic invoke and host call protocol", CredentialAdapterTests.SyntheticInvokeAndHostCallProtocol),
            ("credential fixture authenticated broker pipe fake store round trip", CredentialAdapterTests.BrokerPipeFakeStoreRoundTrip),
            ("test selection default excludes native acceptance", TestSelectionTests.DefaultDoesNotSelectNativeAcceptance),
            ("test selection zero match is an error", TestSelectionTests.ZeroMatchIsAnError),
            ("test selection native authorization and explicit filter", TestSelectionTests.NativeNeedsAuthorizationAndExplicitFilter),
            ("test selection legacy union and catalog order", TestSelectionTests.LegacyFiltersKeepUnionAndCatalogOrder),
            ("test selection catalog retains acceptance and scopes", TestSelectionTests.CatalogRetainsAcceptanceAndExplicitScopes),
            ("test selection runner reconciles skip and failure counts", TestSelectionTests.RunnerCountsSkipWithoutPassAndFailsFast),
            ("browser signing fixture explicit isolated entry", BrowserExtensionSigningTests.ExplicitSigningPathIsRequiredForIsolatedEntry),
            ("browser signing fixture fixed SPKI identity vector", BrowserExtensionSigningTests.FixedSpkiVectorPreservesChromiumIdentity),
            ("browser signing fixture read-only existing PEM", BrowserExtensionSigningTests.RegistrationReadsExistingPemWithoutCreatingOrRewriting),
            ("browser signing fixture allowed origin union", BrowserExtensionSigningTests.OriginsMergeCommandPinnedAndExportedWithoutRegistration),
            ("browser signing fixture CRX signature and reuse", BrowserExtensionSigningTests.IsolatedCrxSignatureAndKeyReuseMatchRegistrationIdentity),
            ("browser signing fixture missing corrupt Firefox cancellation", BrowserExtensionSigningTests.MissingCorruptAndFirefoxKeyBehaviorRemainCompatible),
            ("browser signing fixture distinct directory fallbacks", BrowserExtensionSigningTests.SourceDirectoryFallbacksStayDistinct),
            ("envelope uses protocol v1 wire names", EnvelopeUsesWireNames),
            ("extension manifest picker fixture typed parser and invalid payloads", ExtensionManifestPickerTests.ParserPreservesTypedWireAndRejectsInvalidPayloads),
            ("extension manifest picker fixture ZIP settings and initial path", ExtensionManifestPickerTests.SettingsRequireOneExistingZipAndPreserveInitialPath),
            ("extension manifest picker fixture cancellation drops late selection", ExtensionManifestPickerTests.CancellationRejectsLateSelectionWithoutNativeWork),
            ("extension manifest picker fixture invalid selection and safe native failure", ExtensionManifestPickerTests.InvalidSelectionAndNativeFailuresArePayloadSafe),
            ("extension manifest picker fixture detached reader responsiveness", ExtensionManifestPickerTests.DetachedPickerDoesNotBlockLaterCommands),
            ("elevated input fixture automatic recovery is bounded", ElevatedInputChannelTests.AutomaticRecoveryIsBounded),
            ("elevated input fixture failure before configuration ACK", ElevatedInputChannelTests.FailureBeforeConfigurationAck),
            ("elevated input fixture message routing and owned cleanup", ElevatedInputHostTests.MessageRoutingAndOwnedCleanup),
            ("elevated input fixture paste target and destination validation", ElevatedInputHostTests.PasteTargetAndDestinationValidation),
            ("elevated input fixture administrator outside pointer message window", ElevatedInputHostTests.AdministratorPointerUsesOriginalMessageWindow),
            ("elevated input fixture activation uses configured Desktop ownership", ElevatedInputHostTests.ActivationRequiresConfiguredDesktopOwnership),
            ("elevated input fixture activation and pointer original channel", ElevatedInputChannelTests.ActivationAndPointerUseOriginalConnection),
            ("elevated input fixture pointer survives long idle and tick wrap", ElevatedInputChannelTests.PointerObservationSurvivesLongIdle),
            ("elevated input fixture configuration replay and exit", ElevatedInputChannelTests.ConfigurationReplayAndExit),
            ("elevated input fixture paste and directory share channel", ElevatedInputChannelTests.PasteAndDirectoryShareChannel),
            ("elevated input fixture unavailable keeps ordinary path", ElevatedInputChannelTests.UnavailableKeepsOrdinaryPath),
            ("elevated input fixture clipboard write and input ownership", ElevatedInputTests.ClipboardWriteAndInputOwnership),
            ("elevated input fixture gesture sources never duplicate", ElevatedInputTests.GestureSourcesNeverDuplicate),
            ("elevated input fixture foreground change never injects", ElevatedInputTests.ForegroundChangeNeverInjects),
            ("keyboard monitor fixture dedicated ownership and disposal", KeyboardMonitorTests.DedicatedOwnershipAndDisposal),
            ("keyboard monitor fixture reset generation and injected input", KeyboardMonitorTests.ResetGenerationAndInjectedInput),
            ("keyboard monitor fixture observable failure retry and thread exit", KeyboardMonitorTests.ObservableFailureRetryAndThreadExit),
            ("keyboard monitor fixture health wire and capability negotiation", KeyboardMonitorTests.HealthWireAndCapabilityNegotiation),
            ("keyboard monitor fixture health snapshot survives full queue", KeyboardMonitorTests.HealthSnapshotSurvivesFullQueue),
            ("system motion synthetic reader and wire round trip", SystemMotionPreferenceTests.SyntheticReaderChangeDedupAndWireRoundTrip),
            ("DOpus reveal observation fixture exact active selection shared Reveal", DirectoryOpusTargetObservationTests.ExactActiveSelectionFeedsSharedReveal),
            ("DOpus reveal observation fixture wrong inactive stale and foreign evidence", DirectoryOpusTargetObservationTests.WrongInactiveStaleAndForeignEvidenceRejects),
            ("DOpus reveal observation fixture malformed budgets and diagnostics", DirectoryOpusTargetObservationTests.MalformedBudgetsAndDiagnosticsFailClosed),
            ("DOpus export fixture writer after helper exit cleanup", DirectoryOpusExportTests.WriterAfterHelperExitIsEventuallyCleaned),
            ("DOpus export fixture writer after helper timeout cleanup", DirectoryOpusExportTests.WriterAfterHelperTimeoutIsEventuallyCleaned),
            ("DOpus export fixture directory delete failure cleanup", DirectoryOpusExportTests.DirectoryDeleteFailureRetainsCleanupOwner),
            ("DOpus export fixture pending cleanup bounds admission and recovers", DirectoryOpusExportTests.PendingCleanupBoundsAdmissionAndRecovers),
            ("DOpus export fixture success and native read limit", DirectoryOpusExportTests.SuccessfulOutputAndNativeReadLimitArePreserved),
            ("DOpus export fixture unconfirmed helper exit cleanup", DirectoryOpusExportTests.UnconfirmedHelperExitRetainsCleanupOwner),
            ("Explorer reveal feedback fixture confirmed target survives denied foreground", ExplorerRevealFeedbackTests.ConfirmedTargetSurvivesDeniedForeground),
            ("Explorer reveal feedback fixture folder and acceptance never confirm file", ExplorerRevealFeedbackTests.FolderAndAcceptanceNeverConfirmFile),
            ("Explorer reveal feedback fixture delayed selection and failure retry", ExplorerRevealFeedbackTests.DelayedSelectionAndNativeFailureRemainRetryable),
            ("Explorer reveal feedback fixture destroyed or removed target", ExplorerRevealFeedbackTests.DestroyedOrRemovedTargetCannotSucceed),
            ("Explorer reveal feedback fixture active selection rejects tab root ambiguity", ExplorerRevealFeedbackTests.ActiveViewSelectionRejectsTabRootAmbiguity),
            ("Explorer reveal feedback fixture native COM exact vtable contracts", ExplorerRevealFeedbackTests.NativeComContractsHaveExactVtableSlots),
            ("installed native fixture Explorer native first and fallback", InstalledNativeActionTests.ExplorerNativeFirstAndFallback),
            ("installed native fixture Explorer readback retry and pending", InstalledNativeActionTests.ExplorerReadbackRetryAndPending),
            ("installed native fixture Open With cancellation and failure", InstalledNativeActionTests.OpenWithCancellationAndFailure),
            ("installed native fixture Explorer relative PIDL lifetime", InstalledNativeActionTests.ExplorerRelativePidlLifetime),
            ("installed native fixture protected tuple upgrade decisions", IndexerUpgradeDecisionTests.ProtectedTupleUpgradeDecisions),
            ("installed native fixture protected tuple hash change detection", IndexerUpgradeDecisionTests.ProtectedTupleHashChangeDetection),
            ("installed native fixture protected tuple authorization lifecycle", IndexerUpgradeDecisionTests.ProtectedTupleAuthorizationLifecycle),
            ("trusted update peer fixture ordinary caller service identity", TrustedUpdatePeerTests.OwnedPipeBindsRunningOwnServiceWithoutProcessAccess),
            ("trusted update broker one-shot handshake lifecycle", TrustedUpdateBrokerLifecycleTests.OneShotHandshakesAndTimeouts),
            ("trusted update broker demand start policy", TrustedUpdateBrokerLifecycleTests.StartOnlyStoppedApprovedService),
            ("bootstrap maintenance one-shot readiness", TrustedUpdateBrokerLifecycleTests.MaintenanceReadinessStopsAfterProbe),
            ("trusted update broker transaction survives stop", TrustedUpdateBrokerLifecycleTests.PreparedWorkIgnoresClientStop),
            ("trusted update routing enrolled upgrade avoids UAC", TrustedUpdateRoutingTests.EnrolledUpgradeNeverCallsElevatedInstaller),
            ("trusted update routing rejection never falls back to UAC", TrustedUpdateRoutingTests.RejectedTrustedUpdateCannotFallBackToUac),
            ("trusted update transaction signed commit idempotence downgrade", TrustedUpdateTransactionTests.SignedUpdateCommitsOnceAndRejectsDowngrade),
            ("trusted update transaction initial deployment pending repair", TrustedUpdateTransactionTests.InitialDeploymentPendingRepair),
            ("trusted update transaction partial failure restores and retries", TrustedUpdateTransactionTests.PartialDeploymentRestoresLastGoodAndCanRetry),
            ("trusted update transaction restart and committed cleanup", TrustedUpdateTransactionTests.RestartRecoversInterruptedPublishAndCommittedCleanup),
            ("trusted update transaction bounded frames", TrustedUpdateTransactionTests.FramesRejectOversizeTruncationAndCancellation),
            ("trusted update routing same release enrollment once", TrustedUpdateRoutingTests.SameReleaseMigratesTrustExactlyOnce),
            ("archive release fixture required runtime members", TrustedReleaseManifestTests.ArchiveMembersRequiredBeforeSigning),
            ("archive release fixture mutation restores complete set", TrustedUpdateTransactionTests.ArchiveMemberMutationRestoresCompleteOwnedSet),
            ("archive release fixture stable approval rejects partial mixed set", ArchiveReleaseTests.StableApprovalRejectsPartialMixedSet),
            ("archive release staged actual identities", ArchiveReleaseTests.StagedActualIdentities),
            ("trusted release fixture signature locked copy", TrustedReleaseManifestTests.ValidSignatureAndLockedCopy),
            ("trusted release legacy snapshot recovery-only admission", TrustedReleaseManifestTests.LegacySnapshotIsRecoveryOnly),
            ("trusted release fixture tamper wrong key before parse", TrustedReleaseManifestTests.TamperAndWrongKeyRejectedBeforeParse),
            ("trusted release fixture duplicate traversal schema", TrustedReleaseManifestTests.SignedDuplicateTraversalAndSchemaRejected),
            ("trusted release fixture missing tampered downgrade", TrustedReleaseManifestTests.MissingTamperedAndDowngradeRejected),
            ("trusted release fixture reparse components", TrustedReleaseManifestTests.ReparseComponentsRejected),
            ("trusted release fixture bounds secret signing", TrustedReleaseManifestTests.BoundedInputsAndSecretSigningRejected),
            ("trusted update enrollment partial retry retains key", TrustedUpdateEnrollmentTests.PartialEnrollmentRetainsTrustAndRetries),
            ("installed integration fixture theme reader fallback and dedup", SystemThemePreferenceTests.ReaderFallbackAndDedup),
            ("installed integration fixture theme full queue latest and stop", SystemThemePreferenceTests.FullQueueRetainsLatestAndStops),
            ("installed integration fixture theme event and handshake serialization", SystemThemePreferenceTests.EventAndHandshakeWireRoundTrip),
            ("installed integration fixture Open With target validation and cancelled wire", SystemThemePreferenceTests.OpenWithTargetValidationAndCancelledWire),
            ("owned GPUI fixed compact to confirmation transition", GpuiFixedTransitionTests.OwnedGpuiFixedTransition),
            ("owned GPUI new fixed confirmation placement", GpuiFixedTransitionTests.OwnedGpuiFixedPresentation),
            ("fixed presentation synthetic cancel and final client", FixedPresentationTests.FixedOneShotPreservesAnchorAndCancel),
            ("owned GPUI production probe fixed presentation", GpuiFixedTransitionTests.OwnedProductionProbePresentation),
            ("owned GPUI task completion preserves native center", GpuiFixedTransitionTests.OwnedTaskCompletionPreservesCenter),
            ("owned GPUI passive preview rejects mouse activation", PassivePreviewAcceptanceTests.OwnedPassivePreviewRejectsMouseActivation),
            ("owned GPUI native client and UIA acceptance", PassivePreviewAcceptanceTests.OwnedClientAndUiaAcceptance),
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
            ("clipboard text fixture read-only STA and native results", ClipboardTextTests.ReadOnlyStaAndResults),
            ("clipboard text fixture cancellation frame bound and detached dispatch", ClipboardTextTests.CancellationAndFrameBound),
            ("clipboard pause fixture independent native read gate", ClipboardPauseTests.PauseCommandAndReadGate),
            ("clipboard pause fixture clears retries and resume never backfills", ClipboardPauseTests.RetryResumeNeverBackfills),
            ("clipboard pause fixture bounded admission and failed post cleanup", ClipboardPauseTests.BoundedAdmissionAndPostFailure),
            ("clipboard pause fixture cancellation and disposal retire work", ClipboardPauseTests.CancellationAndStop),
            ("clipboard pause fixture application failures and safe diagnostics", ClipboardPauseTests.ApplicationFailureAndDiagnostics),
            ("clipboard pause fixture application owns late cancellation", ClipboardPauseTests.CancellationAtApplicationBoundary),
            ("clipboard capture limits clamp sizes without payload data", ClipboardCaptureLimitsClampAndReject),
            ("quick-launch configuration and panel request preserve Windows context", QuickLaunchContractsRoundTrip),
            ("Quick Launch diagnostics keep published correlations and rate-limit failures", QuickLaunchSummonDiagnosticsKeepPublishedCorrelationAndRateLimitFailures),
            ("Fullscreen summon classifies borderless and ordinary foreground windows", FullscreenSummonClassifiesForegroundWindows),
            ("Fullscreen summon publish boundary keeps clipboard and quick-launch policies independent", FullscreenSummonPublishBoundaryKeepsPoliciesIndependent),
            ("Quick Launch suppresses both triggers only during exclusive D3D fullscreen", QuickLaunchFullscreenSuppressionIsPrecise),
            ("Quick Launch and clipboard WndProc routes use the fullscreen gate", QuickLaunchWndProcRoutesBothTriggersThroughFullscreenGate),
            ("auto quick switch only consumes an Explorer source after a dialog is known", AutoQuickSwitchRequiresKnownDialog),
            ("Explorer source caller fixture in-flight known return completes once", ExplorerSourceReturnTests.InFlightKnownReturnCompletesOnce),
            ("Explorer source caller fixture ready snapshot refreshes at return", ExplorerSourceReturnTests.ReadySourceStillRefreshesAtReturn),
            ("Explorer source caller fixture first observation never navigates history", ExplorerSourceReturnTests.FirstObservationNeverNavigatesHistory),
            ("Explorer source caller fixture retired returns drop late completion", ExplorerSourceReturnTests.RetiredReturnsDropLateCompletion),
            ("Explorer source caller fixture supersession rejects same HWND ABA", ExplorerSourceReturnTests.SourceSupersessionSerializesAndRejectsSameHwndAba),
            ("Explorer source caller fixture failures and full queue retire without payload", ExplorerSourceReturnTests.FailuresAndFullQueueRetireWithoutPayload),
            ("auto quick switch refreshes a changed Directory Opus source path for the same dialog", AutoQuickSwitchRefreshesChangedDirectoryOpusSource),
            ("Directory Opus dialog return refreshes the current path of an existing tab", DirectoryOpusDialogReturnRefreshesExistingTabPath),
            ("Directory Opus existing-tab selection refreshes the tracked source after navigation", DirectoryOpusExistingTabSelectionRefreshesTrackedSource),
            ("auto quick switch invalidates stale asynchronous candidates", AutoQuickSwitchInvalidatesStaleCandidates),
            ("auto quick switch accepts supported third-party directory sources", AutoQuickSwitchAcceptsThirdPartySources),
            ("fixed file-dialog search retains directory sources without automatic switching", FileDialogSearchRetainsSourcesWithoutAutoSwitch),
            ("file-dialog initialization retries preserve the pending Explorer source", FileDialogInitializationRetriesPreserveExplorerSource),
            ("file-dialog recognition survives a transient foreground handoff", FileDialogRecognitionSurvivesTransientForegroundHandoff),
            ("file-dialog child creation immediately re-arms pending recognition", FileDialogStructureChangeRearmsPendingRecognition),
            ("file-dialog movement ignores children and coalesces root geometry", FileDialogMovementCoalescesRootGeometry),
            ("visible file-dialog foreground repeats reuse the recognized host", VisibleFileDialogForegroundRepeatsReuseRecognizedHost),
            ("visible dialog reuse still observes a returning DOpus source", VisibleDialogReuseObservesReturningDirectoryOpusSource),
            ("elevated directory helper request preserves its opaque rendezvous", ElevatedDirectoryHelperRequestRoundTrips),
            ("deferred directory capture expires with its summon budget", DeferredDirectoryCaptureExpires),
            ("directory navigation resolves library URIs and preserves nested path metadata", DirectoryNavigationTargetResolvesLibraryAndNestedPaths),
            ("desktop navigation recognizes only the Shell desktop and opens the target", DesktopNavigationUsesShellDesktop),
            ("desktop identity requires Shell structure and Explorer ownership", DesktopIdentityRequiresShellEvidence),
            ("file-dialog navigation keeps the captured host window", FileDialogNavigationKeepsCapturedWindow),
            ("third-party file manager navigation keeps the captured host and active pane", ThirdPartyFileManagerNavigationKeepsCapturedHost),
            ("Directory Opus adapter requires the real lister and preserves the active pane", DirectoryOpusAdapterPreservesActivePane),
            ("Directory Opus source refresh accepts tracked lister name and focus events", DirectoryOpusSourceRefreshRequiresTrackedListerNameChange),
            ("Directory Opus frame focus keeps captured pane navigable", DirectoryOpusFrameFocusKeepsCapturedPaneNavigable),
            ("Directory Opus pane selection uses title and geometry before cached pane", DirectoryOpusPaneSelectionUsesReferenceEvidence),
            ("Directory source recognition failure clears stale source", DirectorySourceRecognitionFailureClearsStaleSource),
            ("tracked Directory Opus reactivation keeps one pending dialog sync", TrackedDirectoryOpusReactivationKeepsPendingSync),
            ("pending directory source recognition is bounded and superseded safely", PendingDirectorySourceRecognitionIsBoundedAndSuperseded),
            ("Directory Opus localized display paths canonicalize to parsing paths", DirectoryOpusDisplayPathsCanonicalize),
            ("known folder display aliases cover the desktop folder", KnownFolderDisplayAliasesCoverDesktop),
            ("localized known folder aliases cover the Users root", KnownFolderLocalizedAliasesCoverUsersRoot),
            ("Total Commander adapter uses the captured source panel", TotalCommanderAdapterPreservesSourcePanel),
            ("OneCommander adapter freezes its UI Automation pane before the popup opens", OneCommanderAdapterPreservesAutomationPane),
            ("OneCommander focus filter fixture rejects unrelated UIA before tree walk", StaDispatcherFocusTests.OneCommanderProcessGatePrecedesTreeWalk),
            ("Files identity matches the Listary process and UIA root contract", FilesIdentityMatchesListaryContract),
            ("Files adapter requires its real window and confirms navigation on the captured host", FilesAdapterPreservesCapturedWindow),
            ("Files navigation succeeds only after its path control confirms the target", FilesNavigationRequiresReadback),
            ("bounded STA completion signal contains callback failures", BoundedStaCompletionSignalContainsCallbackFailures),
            ("bounded STA operations release completion wait handles", BoundedStaOperationsReleaseCompletionWaitHandles),
            ("bounded STA operations cancel before a stalled provider can perform late work", BoundedStaOperationsCancelLateWork),
            ("STA dispatcher cleanup fixture owns runner and Shell session release", StaDispatcherFocusTests.DispatcherOwnersShutdownBeforeRelease),
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
            ("real Directory Opus navigation reaches C drive targets", RealDirectoryOpusNavigationReachesCDrive),
            ("real Files window exposes a third-party navigation context", RealFilesWindowExposesNavigationContext),
            ("file-dialog recognition rejects unrelated #32770 windows", FileDialogRecognitionUsesStructuralMarkers),
            ("file-dialog navigation writes a directory and restores its edit focus", FileDialogNavigationUsesCapturedControls),
            ("modern file-dialog navigation uses its address edit without replacing the file name", ModernFileDialogNavigationUsesAddressEdit),
            ("file-dialog hotkey capture normalizes paths without filesystem access", FileDialogPathCaptureAvoidsFilesystemValidation),
            ("real Explorer Shell window resolves its exact HWND and self-navigates", RealExplorerShellWindowResolvesExactHwnd),
            ("Explorer activation selects files despite an existing matching folder window", ExplorerActivationSelectsFileInExistingFolder),
            ("Explorer activation opens existing directories directly", ExplorerActivationOpensExistingDirectoryDirectly),
            ("Explorer activation rejects a window destroyed before foreground readback", ExplorerActivationRejectsDestroyedWindow),
            ("Explorer activation rejects unrelated foreground and denied activation", ExplorerActivationRejectsUnrelatedForeground),
            ("double-control gesture requires two released taps inside the activation window", DoubleControlGestureRequiresTwoReleasedTaps),
            ("double-control gesture rejects key repeat and intervening keys", DoubleControlGestureRejectsRepeatAndInterveningKeys),
            ("hotkey recording releases and restores the registered shortcut", HotKeyRecordingReleasesAndRestoresShortcut),
            ("clipboard hotkey uses 350ms debounce", ClipboardHotKeyUses350MsDebounce),
            ("clipboard exclusions match process names safely", ClipboardExclusionsMatchProcessNames),
            ("topmost command preserves native window", TopmostCommandPreservesNativeWindow),
            ("minimized command shows a hidden native window minimized without focus", MinimizedCommandUsesRealNativeWindow),
            ("no-activate command preserves native window", NoActivateCommandPreservesNativeWindow),
            ("window-bounds command preserves physical placement", WindowBoundsCommandPreservesPlacement),
            ("window geometry reports actual device-pixel rectangles", WindowGeometryReportsActualDevicePixels),
            ("centered rectangle preserves the anchor visual center", CenteredRectanglePreservesAnchorCenter),
            ("centered native resize follows the HWND after a user move", CenteredNativeResizeUsesMovedWindowBounds),
            ("centered resize reapplies after move end using the latest HWND center", CenteredResizeReappliesAfterNativeMoveEnd),
            ("centered resize parks client correction during a native move", CenteredResizeParksClientCorrectionDuringMove),
            ("centered resize checks client after position correction", CenteredResizeChecksClientAfterPositionCorrection),
            ("centered replay rechecks client size at retirement", CenteredResizeRechecksAtRetirement),
            ("client resize synchronization never publishes stale queued size", ClientResizeSynchronizationUsesLiveClient),
            ("client resize synchronization retains parameters until delayed receiver returns", ClientResizeSynchronizationRetainsParameters),
            ("old centered replay cannot retire or overwrite a newer request", CenteredResizeGenerationSurvivesReplay),
            ("centered worker admission remains bounded and preserves HWND order", CenteredResizeWorkerIsBounded),
            ("recovery ignores stale generation completion", CenteredResizeRecoveryIgnoresStaleCompletion),
            ("browser registration compensates every partial store failure", BrowserRegistrationCompensatesPartialFailure),
            ("native drag cancel has a non-error terminal result", NativeDragCancellationHasTerminalStatus),
            ("delayed native HWND accepts newer resize without blocking", CenteredResizeDelayedHwndRemainsResponsive),
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
            ("desktop shell accepts verified current package after an older protected upgrade", DesktopShellAcceptsVerifiedCurrentPackageAfterUpgrade),
            ("desktop shell failures return bounded stage codes", DesktopShellFailuresReturnBoundedStageCodes),
            ("desktop tray rejects a missing product executable icon", DesktopTrayRequiresProductIcon),
            ("tray factory failure disposes partially created resources", TrayFactoryFailureDisposesPartialResources),
            ("titlebar move reaches the real native window boundary", TitlebarMoveUsesRealNativeWindow),
            ("titlebar maximize toggle reaches the real native window boundary", TitlebarMaximizeToggleUsesRealNativeWindow),
            ("window position, resize and shell commands preserve payload", WindowPositionResizeAndShellCommandsPreservePayload),
            ("completion sound and shutdown use the typed Windows adapter", CompletionActionsUseTypedWindowsAdapter),
            ("B5 completion suspend system wire routes through production dispatch", B5CompletionSuspendSystemWireRoutesThroughProductionDispatch),
            ("B5 completion held suspend keeps dispatcher responsive and drains", B5CompletionHeldSuspendKeepsDispatcherResponsiveAndDrains),
            ("typed process launch preserves arguments and Windows flags", TypedProcessLaunchPreservesArgumentsAndFlags),
            ("shell action session commands preserve payload", ShellActionSessionCommandsPreservePayload),
            ("shell action Invoke waits beyond machine deadline", ShellActionWaitTests.InvokeWaitsBeyondMachineDeadline),
            ("shell action machine wait retains five second deadline", ShellActionWaitTests.MachineWaitRetainsFiveSecondDeadline),
            ("shell action machine timeout retires queued Invoke", ShellActionWaitTests.MachineTimeoutRetiresQueuedInvoke),
            ("shell action cancellation retains old worker until actual exit", ShellActionWaitTests.CancellationReturnsPromptlyWithoutWorkerReuse),
            ("shell action session enumerates a real filesystem target", ShellActionSessionEnumeratesFilesystemTarget),
            ("shell action bitmap is staged as a GPUI image", ShellActionBitmapIsStaged),
            ("shell icon extraction command preserves target and size", ShellIconCommandPreservesPayload),
            ("shell icon extraction runs on a dedicated STA worker", ShellIconExtractionRunsOnDedicatedStaWorker),
            ("shell icon extraction allows bounded parallel STA workers", ShellIconExtractionAllowsBoundedParallelWorkers),
            ("packaged application icon fixture namespace and cache identity", PackagedApplicationIconTests.NamespaceResolutionAndCacheIdentity),
            ("packaged application icon fixture real extraction PNG and STA cache", PackagedApplicationIconTests.RealExtractUsesFactoryPngAndOriginalStaCache),
            ("packaged application icon fixture HRESULT retry cancellation cleanup", PackagedApplicationIconTests.HResultRetryCancellationAndCleanup),
            ("archive folder type icon fixture resolves safe directory attributes", ArchiveTypeIconTests.FixedFolderTargetHasDirectoryAttributes),
            ("archive folder type icon fixture caches PNG on the original STA", ArchiveTypeIconTests.FolderPngUsesOwnedCacheAndOriginalSta),
            ("archive polish quick-create title selection", ArchiveShellSelectionTests.QuickCreateTitlesAndCompleteSelectionUseExplorerCommand),
            ("archive polish quick-create recommendation plans", ArchiveShellSelectionTests.QuickCreateRecommendationsAndPlans),
            ("download icon recovery fixture preserves parallel image-list ownership", ArchiveTypeIconTests.ParallelPngExtractionKeepsNativeImageListReferences),
            ("download icon recovery fixture rebuilds a corrupt PNG", ArchiveTypeIconTests.CorruptPngRegeneratesThroughOriginalSta),
            ("download icon recovery fixture retains safe native failure", ArchiveTypeIconTests.NativeFailureRetainsSafeCategoryAndCode),
            ("download icon recovery fixture extracts a completed executable", ArchiveTypeIconTests.CompletedExecutableUsesOwnedCache),
            ("download icon resolution fixture selects 48px for tiny jumbo artwork", ArchiveTypeIconTests.SmallArtworkOnJumboCanvasUsesExtraLarge),
            ("download icon cache fixture refreshes disk recency on hit", ArchiveTypeIconTests.CacheHitRefreshesDiskRecency),
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
            ("private image formats are canonicalized without dropping custom formats", ActiveImageRewriteKeepsOnlyCanonicalImageFormats),
            ("browser image metadata formats force canonical rewrite", BrowserImageMetadataFormatsForceCanonicalRewrite),
            ("canonical image formats use standard top-down bitmap headers", CanonicalImageFormatsUseStandardHeaders),
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

        return tests.Select(test => TestEffects.Create(test.Name, test.Run)).Concat(new[] {
            new TestCase("native allocation fixture archive second allocation failure", NativeAllocationTests.ArchiveSecondAllocationFailure,
                TestEffect.Fixture, "Reviewed: actual OutputNative.Open with a tiny first HGlobal and injected second allocation exception; exact-pointer cleanup, no files/process/UI/registry/service effects"),
            new TestCase("native allocation fixture spawn second allocation failure", NativeAllocationTests.SpawnSecondAllocationFailure,
                TestEffect.Fixture, "Reviewed: actual Spawn attribute-size query and tiny first HGlobal with injected second allocation exception; no process creation, valid stdio/native initialization or user state"),
            new TestCase("native allocation fixture archive first allocation failure", NativeAllocationTests.ArchiveFirstAllocationFailure,
                TestEffect.Fixture, "Reviewed: actual OutputNative.Open with first-allocation exception before native file calls; no successful allocation or files/process/UI/registry/service effects"),
            new TestCase("native allocation fixture spawn first allocation failure", NativeAllocationTests.SpawnFirstAllocationFailure,
                TestEffect.Fixture, "Reviewed: actual Spawn size query and first-allocation exception before native initialization or process creation; no user state"),
            new TestCase("native allocation fixture archive native outcomes", NativeAllocationTests.ArchiveNativeOutcomes,
                TestEffect.Fixture, "Reviewed: tiny HGlobals and actual relative native file opens in an owned temporary directory; success/readback, missing-file and post-create failure cleanup; no user files/process/UI/registry/service"),
            new TestCase("native allocation fixture spawn native failure", NativeAllocationTests.SpawnNativeFailure,
                TestEffect.Fixture, "Reviewed: actual attribute initialization/allowlist with owned anonymous pipes and missing executable under an owned temporary directory; no process creation or user state"),
            new TestCase("OneCommander focus tracker fixture sample remember replace", OneCommanderFocusTrackerTests.SampleRememberAndReplace,
                TestEffect.Fixture, "Reviewed: real tracker owner/thread with injected native/UIA operations and synthetic elements only; no HWND, UIA subscription, desktop apps, hooks, files, registry or service"),
            new TestCase("OneCommander focus tracker fixture closed without lookup", OneCommanderFocusTrackerTests.ClosedWindowRetiresWithoutLookup,
                TestEffect.Fixture, "Reviewed: same tracker background lifecycle with synthetic native window closure; no HWND, UIA subscription, desktop apps, hooks, files, registry or service"),
            new TestCase("OneCommander focus tracker fixture bounded recent windows", OneCommanderFocusTrackerTests.CapacityKeepsMostRecentFocus,
                TestEffect.Fixture, "Reviewed: same tracker with 65 synthetic HWND identities and focus replacement; no HWND, UIA subscription, desktop apps, hooks, files, registry or service"),
            new TestCase("OneCommander focus tracker fixture invalid lookup", OneCommanderFocusTrackerTests.InvalidLookupRetiresRecord,
                TestEffect.Fixture, "Reviewed: injected native/UIA availability at original tracker query boundary only; no HWND, UIA subscription, desktop apps, hooks, files, registry or service"),
            new TestCase("OneCommander focus tracker fixture stale validation replacement", OneCommanderFocusTrackerTests.StaleValidationPreservesReplacement,
                TestEffect.Fixture, "Reviewed: controlled concurrent tracker query/focus callback using synthetic elements and barriers only; no HWND, UIA subscription, desktop apps, hooks, registry or service"),
            new TestCase("OneCommander focus tracker fixture window reuse and stop", OneCommanderFocusTrackerTests.WindowReuseAndStop,
                TestEffect.Fixture, "Reviewed: original owner reuse/retirement/unsubscribe with synthetic operations only; no HWND, UIA subscription, desktop apps, hooks, files, registry or service"),
            new TestCase("OneCommander focus tracker fixture in-flight stop", OneCommanderFocusTrackerTests.InFlightFocusCannotReviveStoppedOwner,
                TestEffect.Fixture, "Reviewed: controlled original focus resolution during stop, bounded owner join and synthetic callbacks only; no HWND, UIA subscription, desktop apps, hooks, registry or service"),
            new TestCase("clipboard publication budget fixture blocked consumer rejects bytes", ClipboardPublicationBudgetTests.BlockedConsumerRejectsBytes,
                TestEffect.Fixture, "Reviewed: original serial publication tail blocked at icon consumer, two synthetic 2MiB format payloads and own PNG staging; GUID scratch only, no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard publication budget fixture accepted order and reuse", ClipboardPublicationBudgetTests.AcceptedOrderAndReuse,
                TestEffect.Fixture, "Reviewed: two controlled original-tail consumer gates, byte rejection/order/reuse and owned synthetic files; no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard publication budget fixture oversized capture is exclusive", ClipboardPublicationBudgetTests.OversizedCaptureIsExclusive,
                TestEffect.Fixture, "Reviewed: one synthetic 4MiB capture in a scaled 3MiB empty-tail budget, exclusive admission and retirement; GUID scratch only, no user state"),
            new TestCase("clipboard publication budget fixture UTF16 text charge", ClipboardPublicationBudgetTests.Utf16TextIsCharged,
                TestEffect.Fixture, "Reviewed: two distinct synthetic text payloads and controlled original-tail consumer, byte admission and owned staging; no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard publication budget fixture cancellation waits for retirement", ClipboardPublicationBudgetTests.CancellationWaitsForActualRetirement,
                TestEffect.Fixture, "Reviewed: blocked original-tail work, actual pause/resume/stop and byte/count retirement with owned files; no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard publication budget fixture handoff and failures release", ClipboardPublicationBudgetTests.HandoffAndFailuresRelease,
                TestEffect.Fixture, "Reviewed: real owned file-copy lock/partial write, full/closed event channel, metadata envelope and original downstream cleanup; no clipboard, user files/processes, HWND, registry or service"),
            new TestCase("clipboard image budget fixture native pixels before encoding", ClipboardImageBudgetTests.NativePixelsBeforeEncoding,
                TestEffect.Fixture, "Reviewed: original capture with a 1001x1000 owned bitmap, configured 1MP limit and encoder observation; session scratch only, no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard image budget fixture file pixels before encoding", ClipboardImageBudgetTests.FilePixelsBeforeEncoding,
                TestEffect.Fixture, "Reviewed: original image-file promotion with valid 1001x1000 PNG and configured 1MP limit; owned scratch only, no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard image budget fixture encoded output stops at budget", ClipboardImageBudgetTests.EncodedOutputStopsAtBudget,
                TestEffect.Fixture, "Reviewed: actual PNG encoder on 640x640 owned native/file inputs with configured 1MiB limit and output-length observation; scratch only, no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard image budget fixture pixel boundary roundtrip", ClipboardImageBudgetTests.PixelBoundaryRoundTrip,
                TestEffect.Fixture, "Reviewed: actual capture/PNG decode for 1000x1000 native/file images exactly at configured 1MP limit; owned scratch only, no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard image budget fixture encoded boundary roundtrip", ClipboardImageBudgetTests.EncodedBoundaryRoundTrip,
                TestEffect.Fixture, "Reviewed: actual bounded PNG encoder compared with native encoder oracle and exact/one-byte-short supported budgets; 640x640 owned image and scratch only, no user state"),
            new TestCase("clipboard image budget fixture conversion failures and canonical formats", ClipboardImageBudgetTests.ConversionFailuresAndCanonicalFormats,
                TestEffect.Fixture, "Reviewed: valid/corrupt 2x2 PNG, original promotion/codec failure cleanup and canonical DIB conversion; owned scratch only, no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard staging fixture partial format write", ClipboardStagingTests.PartialFormatWrite,
                TestEffect.Fixture, "Reviewed: original format staging with partial file-write failure in GUID-owned session scratch; no clipboard, HWND, hooks, registry, service or user files"),
            new TestCase("clipboard staging fixture partial image write", ClipboardStagingTests.PartialImageWrite,
                TestEffect.Fixture, "Reviewed: tiny owned bitmap and original image promotion with partial PNG write failure in GUID-owned session scratch; no clipboard, HWND, hooks, registry, service or user files"),
            new TestCase("clipboard staging fixture capture failure before publication", ClipboardStagingTests.CaptureFailureBeforePublication,
                TestEffect.Fixture, "Reviewed: original capture with injected Windows reads, tiny owned image and post-image failure; GUID-owned session scratch only, no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard staging fixture stopped publication", ClipboardStagingTests.StoppedPublication,
                TestEffect.Fixture, "Reviewed: original event-window stop and publication tail with owned image and isolated channel; no clipboard, HWND, hooks, registry, service or user files"),
            new TestCase("clipboard staging fixture publication ownership", ClipboardStagingTests.PublicationOwnership,
                TestEffect.Fixture, "Reviewed: original capture/publication/full channel and exact consumer cleanup; owned tiny files and borrowed synthetic icon cache only, no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard staging fixture pending tail retirement", ClipboardStagingTests.TailRetirement,
                TestEffect.Fixture, "Reviewed: original bounded publication tail and pause/stop with controlled writer; tiny owned files and isolated channel only, no clipboard, HWND, hooks, registry or service"),
            new TestCase("clipboard staging fixture inactive run recovery", ClipboardStagingTests.RecoveryOwnership,
                TestEffect.Fixture, "Reviewed: GUID-owned scratch, synthetic run metadata and own-process identity/exclusive file leases; no clipboard, user processes, registry or service"),
            new TestCase("clipboard staging fixture recovery bounds and retry", ClipboardStagingTests.RecoveryBoundsAndRetry,
                TestEffect.Fixture, "Reviewed: GUID-owned one-byte files, finite sweep budgets and owned file lock; no clipboard, user files/processes, registry or service"),
            new TestCase("archive OPT07 actual worker batch password reuse retirement", Opt07NativeWorkflowTests.ActualBatchPasswordReuseAndRetirement,
                TestEffect.Fixture,"Reviewed: GUID-owned PI_SCRATCH_DIR AES ZIP copies, current worker copy and five original Test/Extract Runs; actual protected memory/password input/output/native cleanup and exclusive reopen, byte-for-byte source retention; no GUI/OS picker/Shell/clipboard/recycle/registry/service/user archives"),
            new TestCase("archive OPT07 batch password initial prompt reuse", Opt07BatchPasswordTests.InitialPromptReuse,
                TestEffect.Fixture,"Reviewed: GUID-owned empty session scratch, original Session/Run/password frames, serialized Job events/controls and input pump; Windows SAME_PROCESS CryptProtectMemory only, no worker process/codec/UI/Shell/clipboard/user archives/registry/service"),
            new TestCase("archive OPT07 batch password retired pending reply", Opt07BatchPasswordTests.RetiredPendingReply,
                TestEffect.Fixture,"Reviewed: GUID-owned empty session scratch, original Session/Run and serialized retirement/reply controls; protected-memory zeroing only, no worker process/codec/UI/Shell/clipboard/user archives/registry/service"),
            new TestCase("archive OPT07 batch password retry generation isolation", Opt07BatchPasswordTests.RetryGenerationIsolation,
                TestEffect.Fixture,"Reviewed: GUID-owned empty session scratch, original Session/Run/tickets/serialized password reply and input pump; no worker process/codec/UI/Shell/clipboard/user archives/registry/service"),
            new TestCase("archive OPT07 batch password ticket and batch isolation", Opt07BatchPasswordTests.TicketAndBatchIsolation,
                TestEffect.Fixture,"Reviewed: two original in-memory Runs, numeric-array IPC/Job tickets and independent protected candidates in GUID session scratch; no worker process/codec/UI/Shell/clipboard/user archives/registry/service"),
            new TestCase("archive OPT07 batch password wrong and ambiguous errors", Opt07BatchPasswordTests.WrongPasswordAndAmbiguity,
                TestEffect.Fixture,"Reviewed: original Session/Run/password failure frames and actual terminal mapper/serialized Job events, protected-memory invalidation; GUID scratch only, no worker process/codec/UI/Shell/clipboard/user archives/registry/service"),
            new TestCase("archive OPT07 batch password cancel disconnect drain", Opt07BatchPasswordTests.CancelDisconnectAndDrain,
                TestEffect.Fixture,"Reviewed: original Session/Run cancel/Stop and input pump, protected-memory zeroing and independent pipe-session owner; GUID empty scratch only, no worker process/codec/UI/Shell/clipboard/user archives/registry/service"),
            new TestCase("markdown document picker authenticates peer in reader", MarkdownDocumentPickerTests.RejectsInvalidPeerInReader,
                TestEffect.Fixture, "Reviewed: GUID-named owned pipe and original serial reader; rejected metadata does not enter a dialog or native adapter"),
            new TestCase("markdown document picker uses detached STA and retires HWND", MarkdownDocumentPickerTests.OwnedMultiselectIsDetachedAndRetired,
                TestEffect.Fixture, "Reviewed: GUID-named pipe, hidden self-owned HWND, original reader/STA and controlled modal return; no actual system picker or user data"),
            new TestCase("markdown document picker bounds wire result and diagnostics", MarkdownDocumentPickerTests.ResultBoundsAndDiagnostics,
                TestEffect.Fixture, "Reviewed: GUID-named pipe, hidden self-owned HWND and controlled native outcomes through the original reader/STA; no actual system picker or user data"),
            new TestCase("markdown export save picker owns cancel and retirement", MarkdownDocumentPickerTests.ExportPickerOwnsCancelAndRetirement,
                TestEffect.Fixture, "Reviewed: owned pipe, hidden self-owned HWND, controlled Save dialog outcomes on real STA; no visible dialog or user files"),
            new TestCase("markdown export PDF transfers only a successful staged render", MarkdownDocumentPickerTests.PdfTransfersOnlySuccessfulRender,
                TestEffect.Fixture, "Reviewed: owned pipe, dedicated STA and owned synthetic staging files; injected renderer, no WebView runtime or network"),
            new TestCase("markdown export PDF discards a retired pipe result", MarkdownDocumentPickerTests.PdfDisconnectRetiresInFlightStaging,
                TestEffect.Fixture, "Reviewed: owned pipe and synthetic scratch files; controlled in-flight STA cancellation and stale completion, no WebView runtime"),
            new TestCase("markdown export PDF actual offline WebView2", MarkdownDocumentPickerTests.PdfUsesActualOfflineWebView,
                TestEffect.RestrictedFixture, "Authorized only: real installed WebView2 runtime in hidden owned window, owned synthetic HTML/PDF and loopback connection sentinel; no runtime install, PDF viewer or user files"),
            new TestCase("archive OPT07 picker multiple folders", Opt07PickerTests.MultipleFolders,
                TestEffect.Fixture,"Reviewed: hidden self-owned HWND, original parser/Pick/bounded STA, configured IFileOpenDialog options and instance-scoped controlled return; 2/16/32 vectors and typed envelope, no actual system dialog/Shell/clipboard/files/registry/service/user data"),
            new TestCase("archive OPT07 picker atomic limits", Opt07PickerTests.LimitsAreAtomic,
                TestEffect.Fixture,"Reviewed: hidden owned HWND/original Pick/STA, controlled vectors at exact UTF8/count bounds and overflow/invalid paths; no actual system dialog/Shell/clipboard/files/registry/service/user data"),
            new TestCase("archive OPT07 picker cancel native failure", Opt07PickerTests.CancelAndNativeFailure,
                TestEffect.Fixture,"Reviewed: hidden owned HWND/original Pick/STA, controlled native Cancel/pre-cancel/Win32/COM failures; no actual system dialog/Shell/clipboard/files/registry/service/user data"),
            new TestCase("archive OPT07 picker retired owner", Opt07PickerTests.RetiredOwner,
                TestEffect.Fixture,"Reviewed: destroy only own hidden HWND on creating thread during controlled original modal boundary; late vector refused; no actual system dialog/Shell/clipboard/files/registry/service/user data"),
            new TestCase("archive OPT07 picker actual cancellation reservation", Opt07PickerTests.CancelRetainsActualReservation,
                TestEffect.Fixture,"Reviewed: own hidden HWND/original bounded STA, controlled blocked modal return/cancel/no overtaking/actual thread join; no actual system dialog/Shell/clipboard/files/registry/service/user data"),
            new TestCase("archive OPT07 picker destination owner admission", Opt07PickerTests.DestinationAndOwnerAdmission,
                TestEffect.Fixture,"Reviewed: own hidden HWND/original Pick/parser/STA, single destination/initial fallback and native invalid-owner/peer rejection; no actual system dialog/Shell/clipboard/files/registry/service/user data"),
            new TestCase("archive OPT07 picker capability diagnostics", Opt07PickerTests.CapabilityAndDiagnostics,
                TestEffect.Fixture,"Reviewed: production capability list/original Pick/STA, self-owned hidden HWND and recording/throwing log with controlled return; no actual system dialog/Shell/clipboard/files/registry/service/user data"),
            new TestCase("archive HZ5 wire and admission", Hz5ReclamationTests.WireAndAdmission,
                TestEffect.Fixture,"Pure DTO and admission validation; no native effects"),
            new TestCase("archive HZ5 native pending punch recovery output protection", Hz5ReclamationTests.PendingPunchRecoveryAndOutputProtection,
                TestEffect.RestrictedFixture,"Authorized session scratch hz5-GUID only: actual NTFS sparse punch, durable journal and retained native output; Debug-only fault seam, no Shell/service/registry"),
            new TestCase("archive HZ5 native source activity edit handoff", Hz5ReclamationTests.SourceActivityAndEditHandoff,
                TestEffect.RestrictedFixture,"Session scratch hz5-GUID native source handles/activity only; no source mutation or Shell/service/registry"),
            new TestCase("archive HZ5 native ready recovery incomplete refusal", Hz5ReclamationTests.ReadyRecoveryAndIncompleteRefusal,
                TestEffect.RestrictedFixture,"Session scratch hz5-GUID source/output/ADS/receipt recovery through original CommitCore; no punch; Debug interruption seam"),
            new TestCase("archive HZ5 native framing bounds preserving refusals", Hz5ReclamationTests.FramingBoundsAndRefusals,
                TestEffect.RestrictedFixture,"Session scratch hz5-GUID real native journal framing/read limits/corruption only; no punch or Shell/service/registry"),
            new TestCase("archive Phase F virtual async actual slow native consumer UI", ArchiveVirtualAsyncTests.ActualSlowConsumerReturnsUiBeforeContent,
                TestEffect.Fixture,"Reviewed: GUID-owned4MiBZIP/originalworkerJob/cache; native COM QI/vtable + standard apartment marshal; controlled writer gate, no real OLE drop/Explorer/GUI/Registry/ACL/service"),
            new TestCase("archive Phase F virtual async native ABI state refs", ArchiveVirtualAsyncLifecycleTests.NativeAsyncAbiTransitionsAndReferences,
                TestEffect.Fixture,"Reviewed: owned COM vtable/BOOL/HRESULT/native memory streams only, no Shell/GUI/Registry/ACL/service"),
            new TestCase("archive Phase F virtual async actual receiver lease cancel abort disconnect", ArchiveVirtualAsyncLifecycleTests.ActualReceiverLeaseCancelAbortDisconnect,
                TestEffect.Fixture,"Reviewed: tiny GUID ZIP/original worker/cache/native stream; controlled COM End and native STA pump/serial dispatcher, no Explorer/GUI/Registry/ACL/service"),
            new TestCase("archive Phase F virtual async actual multi directory unknown progress ordering", ArchiveVirtualAsyncLifecycleTests.ActualMultiDirectoryUnknownProgressAndEndOrdering,
                TestEffect.Fixture,"Reviewed: GUID-owned 2 MiB ZIP/original worker/write-probe delay/cache/native streams/COM; stale callback injection, real member bytes and both End/loop orders; no Explorer/GUI/Registry/ACL/service"),
            new TestCase("archive Phase F virtual password wire no output admission", ArchiveVirtualPasswordTests.AuthorizeWireAndNoOutputAdmission,
                TestEffect.Fixture,"Strict JSON and original runner missing-helper admission in GUID-owned scratch; no worker/GUI/OLE/Registry"),
            new TestCase("archive Phase F virtual password actual ZIP lazy mixed retained stream", ArchiveVirtualPasswordTests.ActualZipGrantLazyMixedTreeAndRetainedStream,
                TestEffect.Fixture,"Owned tiny AES ZIP plus plain member; original TEST/worker/Job/source/cache and controlled native IStream; no real OLE/GUI/Registry"),
            new TestCase("archive Phase F virtual password actual RAR tickets revoke expiry source", ArchiveVirtualPasswordTests.ActualRarTicketsRevokeExpiryReplacementAndSource,
                TestEffect.Fixture,"Owned tiny RAR5; original metadata/password TEST tickets; exact revoke/expiry/replacement/source cleanup; no real OLE/GUI/Registry"),
            new TestCase("archive Phase F virtual password actual ZIP ambiguity cleanup revoke", ArchiveVirtualPasswordTests.ActualZipWrongIsTerminalAndPendingCleanupRevoke,
                TestEffect.Fixture,"Owned tiny AES ZIP; original wrong attempt terminal and cleanup-bound revoke race; no real OLE/GUI/Registry"),
            new TestCase("archive Phase F virtual tree wire shapes conflicts bounds", ArchiveVirtualTreeTests.WireShapesConflictsAndBounds,
                TestEffect.Fixture,"Reviewed: strict JSON tree/unknown/default/authority and finite bounds only; no native work"),
            new TestCase("archive Phase F virtual tree SDK descriptors directory refusal", ArchiveVirtualTreeTests.SdkDescriptorsAndDirectoryRefusal,
                TestEffect.Fixture,"Reviewed: controlled descriptor HGLOBAL/native memory streams only; no GUI/clipboard/OLE loop/Registry"),
            new TestCase("archive output readmission fixture native failure authority", ArchiveOutputReadmissionHost.NativeFailureAuthority,
                TestEffect.Fixture,"Reviewed: original fixture result projection with fixed native/worker category facts only; no process, files, GUI, Shell or system state"),
            new TestCase("archive OPT01 CSharp production editing progress wire", ArchiveJobTerminalTests.EditingProgressProductionWire,
                TestEffect.Fixture,"Reviewed: original in-memory Job Operation/OnFrame/event channel, compiled CSharp JSON and optional session-scratch export only; no worker/native API/archive mutation/GUI/Registry/service"),
            new TestCase("archive OPT01 AC repair production progress wire", ArchiveJobTerminalTests.RepairProgressProductionWire,
                TestEffect.Fixture,"Reviewed: original in-memory Operation/OnFrame/two channels/serializer; four formats and held coalesced frames, optional scratch export; no worker/native API/GUI/Registry/service"),
            new TestCase("archive OPT05 native identity version and cancellation", Opt05NativeObservationTests.IdentityAndCancellation,
                TestEffect.Fixture, "Reviewed: GUID-owned PI_SCRATCH_DIR ZIPs; actual SourceLease/OutputSession/native handles, identity/version refusals and deterministic chunk cancellation; owned file timestamps/Hidden attribute only; no processes/GUI/Shell/clipboard/registry/service/user archives"),
            new TestCase("archive OPT05 native equivalent observations", Opt05NativeObservationTests.EquivalentObservations,
                TestEffect.Fixture, "Reviewed: marked GUID-owned PI_SCRATCH_DIR 8MiB ZIP; real SourceLease/OutputSession/native replacement, held-partial readback, FILE-only handoff and exclusive-reopen cleanup; no processes/GUI/Shell/clipboard/registry/service/user archives"),
            new TestCase("archive OPT05 actual worker native terminal observations", Opt05NativeObservationTests.ActualTerminalObservations,
                TestEffect.Fixture, "Reviewed: GUID-owned scratch ZIPs/current explicit matching worker; original browser/Run/Job/native replacement, established-lease chunk cancellation, production terminal sink and throwing-sink isolation, actual cleanup/exclusive reopen; no GUI/Shell/clipboard/registry/service/user sources"),
            new TestCase("archive OPT06 wire raw original runner", Opt06BulkWireTests.RawRunner,
                TestEffect.Fixture, "Reviewed: GUID-owned session scratch, controlled current-apphost peer in original Runner/Job/pipes and actual native output/cleanup; no GUI/Shell/clipboard/picker/user data/registry/service"),
            new TestCase("archive OPT06 wire capabilities before effects", Opt06BulkWireTests.Capabilities,
                TestEffect.Fixture, "Reviewed: owned synthetic sources/current-apphost peer/original native output preparation; no GUI/Shell/clipboard/picker/user data/registry/service"),
            new TestCase("archive OPT06 wire literal binary header", Opt06BulkWireTests.LiteralContract,
                TestEffect.Fixture, "Reviewed: independent byte literals and original binary parser/encoder, finite IPC accounting only; no native effects"),
            new TestCase("archive OPT06 wire malformed original runner", Opt06BulkWireTests.MalformedRunner,
                TestEffect.Fixture, "Reviewed: GUID-owned scratch/current-apphost peer/original Job/native tokens, malformed and stale binary refusals/owned cleanup; no GUI/Shell/clipboard/user data/registry/service"),
            new TestCase("archive OPT06 wire native source partial short EOF", Opt06BulkWireTests.NativeReads,
                TestEffect.Fixture, "Reviewed: owned native source/partial reads and output writes through original Run/Job, exact independent frames/short reads/EOF/native errors and cleanup; no GUI/Shell/clipboard/user data/registry/service"),
            new TestCase("archive OPT06 wire first failure publication sink", Opt06BulkWireTests.FailureAndPublication,
                TestEffect.Fixture, "Reviewed: owned original Run/Job/native write and commit failures, single publication/late cancellation/throwing log sink; no GUI/Shell/clipboard/user data/registry/service"),
            new TestCase("archive OPT06 wire backpressure control cancel drain", Opt06BulkWireTests.CancelBackpressure,
                TestEffect.Fixture, "Reviewed: owned current-apphost peer stalls original native-source reply; bounded input/control/cancellation/Job termination and actual buffer/handle cleanup; no GUI/Shell/clipboard/user data/registry/service"),
            new TestCase("archive OPT06 wire native read request refusals", Opt06BulkWireTests.ReadRequestRefusals,
                TestEffect.Fixture, "Reviewed: GUID-owned synthetic sources/original Run/Job/native ID-source authority; exact typed sequence/length/token refusal before payload reads, real cleanup; no GUI/Shell/clipboard/user data/registry/service"),
            new TestCase("archive OPT06 wire data acknowledgement orders commit", Opt06BulkWireTests.OrderedAcknowledgement,
                TestEffect.Fixture, "Reviewed: owned apphost/Run/Job/native output and original input-pump gate; peer commit cannot cross one unconfirmed data binding; no GUI/Shell/clipboard/user data/registry/service"),
            new TestCase("archive OPT06 wire full channel cancellation zero drain", Opt06BulkWireTests.QueueCancellation,
                TestEffect.Fixture, "Reviewed: original Run capacity-four channel/SendBulkResult/SendInput over memory stream, cancellation/zeroing and maximum two buffers; no process/native file/GUI/Shell/clipboard/registry/service effects"),
            new TestCase("archive OPT06 wire binary budget separation", Opt06BulkWireTests.BinaryBudgetSeparation,
                TestEffect.Fixture, "Reviewed: GUID-owned 129x64KiB synthetic output/current-apphost peer/original Job/native publication, bounded independent byte verification and actual cleanup; no measurements/GUI/Shell/clipboard/user data/registry/service"),
            new TestCase("archive OPT06 binary transfer measurements", Opt06BulkTransferTests.Measurements,
                TestEffect.Fixture, "Reviewed: explicit matching worker and GUID-owned scratch inputs/stable worker path; original browser/Job/Runner/native ZIP create/extract/Add, three completed trials per route, independent decode and actual cleanup; owned per-trial outputs only, no GUI/Shell/clipboard/picker/user sources/registry/service"),
            new TestCase("archive OPT05 completed compound format baseline", ArchiveOptimizationTests.Opt05CompletedFormatBaseline,
                TestEffect.Fixture, "Reviewed: marked owned TGZ/TXZ inputs and independently decoded expectations/current explicit worker; original Read/Job/Entry/native output/cache, three completed trials, exact bytes/cleanup/source exclusive reopen; no GUI/Shell/clipboard/registry/service/user archives"),
            new TestCase("archive OPT05 compound format plan reuse", ArchiveOptimizationTests.Opt05FormatPlanReuse,
                TestEffect.Fixture, "Reviewed: same marked owned TGZ/TXZ/original native sessions and outputs; one-operation outer/catalog work assertions with exact bytes and real cleanup; no GUI/Shell/clipboard/registry/service/user archives"),
            new TestCase("archive OPT04 retained split ZIP materialization", Opt04DragTests.RetainedSplitZipMaterialization,
                TestEffect.Fixture, "Reviewed: original Job creates owned two-member split ZIP, actual source-set browse and retained controlled COM/native stream demands, forward/backward order and exact cleanup; no GUI/Shell/clipboard/service/registry/user source"),
            new TestCase("archive OPT04 solid RAR4 encrypted preview grant lifecycle", Opt04DragTests.SolidRar4EncryptedPreviewGrant,
                TestEffect.Fixture, "Reviewed: fresh owned copies of independently decoded pinned solid encrypted RAR4/current explicit worker; actual preview and separate whole-TEST prompts/grant, controlled COM/native cursors, nonmonotonic retained TEST and revoke/cleanup; no GUI/Shell/clipboard/service/registry/user source"),
            new TestCase("archive OPT04 retained full block integrity before ready", Opt04DragTests.RetainedBlockIntegrityBeforeReady,
                TestEffect.Fixture, "Reviewed: single-byte packed-tail corruption in a new owned copy of prepared solid7z; original catalog/retained worker/native output/cache and controlled COM refusal/cleanup; no GUI/Shell/clipboard/service/registry/user source"),
            new TestCase("archive OPT04 drag retained format materialization", Opt04DragTests.RetainedFormatMaterialization,
                TestEffect.Fixture, "Reviewed: copies of explicitly prepared owned solid7z/RAR5/TGZ/TXZ and independent expectations/current explicit worker; original read/drag/runner/output/cache and controlled COM streams, actual work/source/process/cleanup facts; no GUI/Shell/clipboard/service/registry/user source"),
            new TestCase("archive OPT04 retained idle work failure lifetime", Opt04DragTests.RetainedIdleWorkAndFailureLifetime,
                TestEffect.Fixture, "Reviewed: GUID-owned scratch ZIP/current explicit worker, original native COM/runner/output/cache; 31-second receiver idle and original work timeout, Seal cancellation, handle-verified owned worker exit and owned stdin EOF; no GUI/Shell/clipboard/service/registry/user source"),
            new TestCase("archive OPT04 preview drag lease adoption", Opt04DragTests.PreviewDragLeaseAdoption,
                TestEffect.Fixture, "Reviewed: GUID-owned ZIP/cache/current explicit worker, original preview and controlled native COM drag; exact match/mismatch, independent native cursors and owned consumer release only; no GUI/Shell/clipboard/service/registry/user source"),
            new TestCase("archive OPT04 drag completed baseline", Opt04DragTests.CompletedBaseline,
                TestEffect.Fixture,"Reviewed: GUID-owned PI_SCRATCH_DIR ZIP/current explicit worker; original native read/drag/EntrySession/runner/output/cache and COM GetData/IStream, three completed trials per demand, preserved input/evidence; no physical gesture/Explorer/GUI/clipboard/registry/service/user archives"),
            new TestCase("archive OPT04 drag retained context lifecycle", Opt04DragTests.RetainedContextLifecycle,
                TestEffect.Fixture,"Reviewed: same owned retained ZIP/worker/native COM chain and actual worker/source/catalog and receiver cleanup assertions; no physical gesture/Explorer/GUI/clipboard/registry/service/user archives"),
            TestEffects.Create("archive R45 same-source extract and Root drag measurements", Opt04DragTests.Release45SameSourceMeasurements),
            new TestCase("archive OPT00 completed read baseline", ArchiveOptimizationTests.CompletedReadBaseline,
                TestEffect.Fixture,"Reviewed: explicit owned OPT temporary root/current worker; original Read/Job/Entry source-output-cache; three completed open/Test/materialize/all-selected trials, exact bytes and native cleanup; no GUI/Registry/service/user mutation"),
            new TestCase("archive OPT00 completed creation baseline", ArchiveOptimizationTests.CompletedCreationBaseline,
                TestEffect.Fixture,"Reviewed: explicit owned OPT temporary root/current worker; original create/each-input/split/post-Test and Add on newly created fixture only, native publication and direct payload checks; no GUI/Registry/service/user mutation"),
            new TestCase("archive OPT00 completed format baseline", ArchiveOptimizationTests.CompletedFormatBaseline,
                TestEffect.Fixture,"Reviewed: explicitly marked owned solid7z/solidRAR5/TAR.GZ/TAR.XZ copies and independent expected bytes; original native Read/Job/Entry, three completed trials with sampled memory/transport and output cleanup; no GUI/Shell/Registry/service/user mutation"),
            new TestCase("archive OPT01 native shared read revision work", ArchiveOptimizationTests.SharedReadWork,
                TestEffect.Fixture,"Reviewed: same owned Read/Job/Entry native routes, 1/2/16 selected ZIP work assertions and exact output cleanup; no GUI/Registry/service/user mutation"),
            new TestCase("archive OPT01 drag sealed size accounting no extra cursor", ArchiveVirtualTreeTests.ActualSealedSizeAccountingWithoutExtraCursor,
                TestEffect.Fixture,"Reviewed: GUID-owned tiny ZIP/current explicit worker/original read-Job-output-cache; native IDataObject slot3/IStream, three trials and aggregate hash/cursor observations; no real OLE/GUI/Registry/service/user data"),
            new TestCase("archive OPT01 drag actual first repeated cursor tamper", ArchiveVirtualTreeTests.ActualFirstAndRepeatedCursorTamper,
                TestEffect.Fixture,"Reviewed: GUID-owned tiny ZIP/current explicit worker/original output-Seal-consumer; same-size same-time synthetic changes before first/repeated cursor, changed-output retention; no real OLE/GUI/Registry/service"),
            new TestCase("archive OPT01 cache cursor integrity lifetime", ArchiveVirtualTreeTests.CacheCursorIntegrityAndLifetime,
                TestEffect.Fixture,"Reviewed: GUID-owned temporary cache/original native publication-Seal/native IStream; first/repeated same-size same-time tamper and independent transferred cursor cleanup; no worker/real OLE/GUI/Registry/service"),
            new TestCase("archive Phase F virtual tree actual ZIP unknown streams identity", ArchiveVirtualTreeTests.ActualZipTreeUnknownStreamsAndIdentity,
                TestEffect.Fixture,"Reviewed: tiny GUID-owned ZIP and two-block same-name/empty-file 7z; original read/worker Job/lease/native IStream with nullable estimates and reordered ordinals; controlled consumers, no actual OLE loop/GUI/Registry"),
            new TestCase("archive Phase F virtual tree actual remaining budget cleanup", ArchiveVirtualTreeTests.ActualRemainingBudgetAndCleanup,
                TestEffect.Fixture,"Reviewed: tiny owned ZIP/original worker/output with internal lowered total ceiling, concurrent/repeat/no-medium/actual cleanup; no actual OLE loop/GUI/Registry"),
            new TestCase("archive Phase F virtual tree actual unknown Zstd native stream", ArchiveVirtualTreeTests.ActualUnknownZstdReadToNativeStream,
                TestEffect.Fixture,"Reviewed:12byte owned unknown-content-size Zstd/original read-worker-source-Job-cache/native stream; controlled consumer, no external OLE/GUI/Registry"),
            new TestCase("archive Phase F virtual tree source versus output budget", ArchiveVirtualTreeTests.SourceAdmissionIsIndependentOfRemainingOutputBudget,
                TestEffect.Fixture,"Reviewed: tiny owned ZIP/original worker exact6byte narrowed output independent of existing source cap; no GUI/Registry/external OLE"),
            new TestCase("archive Phase F virtual formal wire alias source peer rejection", ArchiveVirtualDragTests.FormalWireAliasAndSourcePeerRejections,
                TestEffect.Fixture,"Reviewed: formal/legacy strict parser and injected rejected HWND/peer admission only; GUID-owned empty package, no worker/STA/OLE/GUI/Registry"),
            new TestCase("archive Phase F virtual descriptor indexed streams lifetime", ArchiveVirtualDragTests.DescriptorIndexedStreamsAndRetirement,
                TestEffect.Fixture,"Reviewed: controlled COM IDataObject consumer/native memory streams on owned STA; no GUI/clipboard/registry/Shell drag"),
            new TestCase("archive Phase F virtual STA gesture cancellation responsiveness", ArchiveVirtualDragTests.StaGestureOrderingCancelAndReaderResponsiveness,
                TestEffect.Fixture,"Reviewed: injected source/capture/button seams, real dedicated OLE STA and original dispatcher; no physical gesture/GUI/Shell"),
            new TestCase("archive Phase F virtual actual worker delayed indexed contents", ArchiveVirtualDragTests.ActualWorkerDelayedIndexedContentsAndChangedSource,
                TestEffect.Fixture,"Reviewed: tiny GUID-owned ZIP/original worker Job/native lease to native IStream under controlled COM consumer; no actual OLE target/GUI/registry"),
            new TestCase("archive Phase F virtual slow worker cancel source close disconnect", ArchiveVirtualDragTests.ActualSlowMaterializationCancelSourceCloseAndDisconnect,
                TestEffect.Fixture,"Reviewed: tiny GUID-owned worker/Job with controlled source checkpoint; exact cancel/source-close/disconnect cleanup, no real OLE target/GUI/Shell"),
            new TestCase("archive Phase F virtual native COM ABI final cancelled boundary", ArchiveVirtualDragTests.NativeComAbiAndFinalCancelledOleBoundary,
                TestEffect.Fixture,"Reviewed: synthetic COM vtable consumer and final pre-cancel OLE admission check on owned STA; no actual DoDragDrop/input/external target/GUI/registry"),
            new TestCase("archive output fixture new file streamed fingerprint", ArchiveOutputStreamedFingerprintTests.NewFilePublicationUsesWrittenDigest,
                TestEffect.Fixture,"Reviewed: GUID-owned scratch output session, new/empty/replacement files and verified owned cleanup; no worker/GUI/Registry"),
            new TestCase("archive Root drop locate wire shape", ArchiveDropLocateTests.LocateWireAdmitsOnlyOneWrapper,
                TestEffect.Fixture,"Reviewed: strict locate parser only; no STA/OLE/files/GUI/Registry"),
            new TestCase("archive Root drop locate receiver copy", ArchiveDropLocateTests.ReceiverCopyLocatesCreatedFolder,
                TestEffect.Fixture,"Reviewed: GUID-owned scratch staging/destination, controlled CF_HDROP copy receiver on owned STA, scoped FileSystemWatcher; no physical gesture/Explorer/GUI/Registry"),
            new TestCase("archive Root drop locate direct marker parent", ArchiveDropLocateTests.DirectMarkerLocatesParentWithoutWrapper,
                TestEffect.Fixture,"Reviewed: GUID-owned scratch marker and controlled CF_HDROP receiver on owned STA; direct parent location leaves no wrapper folder or marker; no physical gesture/Explorer/GUI/Registry"),
            new TestCase("archive Root drop locate uncopied cancelled busy", ArchiveDropLocateTests.UncopiedOrCancelledDropReportsNoDestination,
                TestEffect.Fixture,"Reviewed: GUID-owned scratch staging, injected loop/cancel on owned STA, scoped FileSystemWatcher; no physical gesture/Explorer/GUI/Registry"),
            new TestCase("archive Phase F integration default disabled pure HKCU", ArchiveIntegrationPlanTests.DefaultsAreDisabledAndPlanIsPureHkcu,
                TestEffect.Fixture,"Reviewed: memory-only pure HKCU plan; no Registry/files/process/COM/Explorer/system"),
            new TestCase("archive Phase F integration classic cascade OpenWithProgids plan", ArchiveIntegrationPlanTests.ClassicCascadeAndOpenWithProgidsHaveExactOwnedValues,
                TestEffect.Fixture,"Reviewed: memory-only command/value plan; no Registry/files/process/COM/Explorer/system"),
            new TestCase("archive Phase F integration precapture write throw compensation", ArchiveIntegrationPlanTests.ApplyCapturesAllBeforeWriteAndCompensatesWriteThenThrow,
                TestEffect.Fixture,"Reviewed: fake-store dictionaries only; no Registry/files/process/COM/Explorer/system"),
            new TestCase("archive Phase F integration exact owned uninstall mixed ownership", ArchiveIntegrationPlanTests.UninstallRestoresOwnedValuesAndPreservesForeignOrModifiedEntries,
                TestEffect.Fixture,"Reviewed: fake-store dictionaries only; no Registry/files/process/COM/Explorer/system"),
            new TestCase("archive Phase F integration saved intent separate applied facts", ArchiveIntegrationPlanTests.SavedIntentAndAppliedFactsRemainSeparate,
                TestEffect.Fixture,"Reviewed: fake-store dictionaries only; no Registry/files/process/COM/Explorer/system"),
            new TestCase("archive large count legacy and metadata admission", ArchiveLargeCountTests.LegacyAndMetadataAdmission,
                TestEffect.Fixture,"Reviewed: GUID-owned 5000-file ZIP/current worker/original Job; legacy and deficient metadata rejection preserve foreign sentinel with zero commits/partials; no system/user-source/Shell effects"),
            new TestCase("archive large count supplied 70000 files", ArchiveLargeCountTests.SuppliedSeventyThousand,
                TestEffect.Fixture,"Reviewed: explicit supplied ZIP copied into GUID scratch/current isolated worker/original Job/native output; full 70000-name/byte comparison, bounded terminal/log counts and cleanup; no GUI/Shell/user-source mutation"),
            new TestCase("archive large count cancellation beyond legacy boundary", ArchiveLargeCountTests.CancellationBeyondLegacyBoundary,
                TestEffect.Fixture,"Reviewed: GUID-owned 5000-file ZIP/current worker/original Job/native publication checkpoint; cancel after 4200 native commits, truthful surviving files/terminal/cleanup; no GUI/Shell/user data or system settings"),
            new TestCase("archive large count corruption beyond legacy boundary", ArchiveLargeCountTests.CorruptionBeyondLegacyBoundary,
                TestEffect.Fixture,"Reviewed: GUID-owned 5000-file ZIP with final stored payload damage/current worker/original Job/native output; prior publication, CRC failure, summary and partial cleanup; no user-source/system/Shell effects"),
            new TestCase("archive compatibility additional formats native corpus", ArchiveCompatibilityTests.AdditionalFormatsCorpus,
                TestEffect.Fixture,"Reviewed: fourteen supplied formats copied into GUID scratch/current worker/original Read, Job and native output; system makecab prepares an owned 80B/128KiB CAB only; exact bytes, budgets, commits and cleanup; no GUI/Shell/system configuration/user-source mutation"),
            new TestCase("archive compatibility 7z methods corpus", ArchiveCompatibilityTests.SevenzMethodsCorpus,
                TestEffect.Fixture,"Reviewed: explicit independent PPMd/BZip2/Deflate/BCJ2 corpus copied into GUID scratch/current worker/original Read and Job; full member counts, CRC/bytes and actual cleanup; no GUI/Shell/system/user source mutation"),
            new TestCase("archive compatibility split ZIP preview independent source budget", ArchiveCompatibilityTests.SplitZipPreviewSourceBudget,
                TestEffect.Fixture,"Reviewed: GUID scratch generated ZIP volumes/current worker/original Job, Read and Entry/native output; tiny original ordinal preview and cleanup; no GUI/Shell/system/user source mutation"),
            new TestCase("archive compatibility py7zr corpus", ArchiveCompatibilityTests.PySevenzCorpus,
                TestEffect.Fixture,"Reviewed: explicit independently produced py7zr source copied into GUID scratch/current worker/original Job, Read and preview/native output; exact bytes and cleanup; no GUI/Shell/system/user source mutation"),
            new TestCase("archive compatibility 7z preview independent source budget", ArchiveCompatibilityTests.SevenzPreviewSourceBudget,
                TestEffect.Fixture,"Reviewed: GUID session-scratch generated 7z/current explicit worker/original Job, Read and Entry/native cache; exact selected bytes, refusal and cleanup; no GUI/Shell/system/user source mutation"),
            new TestCase("archive compatibility numbered ZIP corpus", ArchiveCompatibilityTests.NumberedZipCorpus,
                TestEffect.Fixture,"Reviewed: explicit corpus copied to GUID scratch/current worker/original Read, Job, native all-source leases and outputs; independent payload bytes/missing-volume refusal; no GUI/Shell/system/user source mutation"),
            new TestCase("archive preview fixture strict readonly purpose", ArchivePreviewTests.PurposeWire,
                TestEffect.Fixture,"Reviewed strict DTO only, no native state effects"),
            new TestCase("archive preview fixture actual output budgets", ArchivePreviewTests.ActualOutputBudgets,
                TestEffect.Fixture,"Reviewed GUID-owned ZIP/cache and current isolated Rust worker in original Job/output/source owners; no GUI/Shell/Registry/service/user data"),
            new TestCase("archive Phase F P206 product composition explicit apply", ArchiveIntegrationServiceTests.ProductCompositionAppliesOnlyExplicitRequests,
                TestEffect.Fixture,"Reviewed: actual product composition with injected memory store/ledger/identity/notify; no native Registry/COM/Shell/process effects"),
            new TestCase("archive Phase F P206 product dispatch responsive", ArchiveIntegrationServiceTests.ProductDispatchDoesNotBlockReader,
                TestEffect.Fixture,"Reviewed: original command dispatcher with blocking memory-store fixture only; no Registry/COM/Shell/process effects"),
            new TestCase("archive Phase F P206 product identity retained release", ArchiveIntegrationServiceTests.ProductIdentityLeaseAcceptsExactReleaseAndRejectsUnknown,
                TestEffect.Fixture,"Reviewed: GUID-owned synthetic release bytes/original read-only identity leases plus memory Registry store; no real Registry/COM/Shell/service/process effects"),
            new TestCase("archive Phase F P206 product cancel timeout bounded", ArchiveIntegrationServiceTests.ProductCancelTimeoutRetainsBoundedSlotAndNoAppliedReply,
                TestEffect.Fixture,"Reviewed: blocking memory store/original bounded STA/cancel/log observations; no Registry/COM/Shell/service/process effects"),
            new TestCase("archive Phase F P206 service enabled apply exact revert", ArchiveIntegrationServiceTests.EnabledApplyRecordsFactsAndRevertsExactly,
                TestEffect.Fixture,"Reviewed: explicitly enabled service over fake store/in-memory ledger only; no Registry/files/process/COM/Explorer/system"),
            new TestCase("archive Phase F P206 service set default claims class", ArchiveIntegrationServiceTests.SetDefaultClaimsClassAndExplorerChoice,
                TestEffect.Fixture,"Reviewed: explicitly enabled service over fake store/in-memory ledger and fake Explorer choice owner only; no Registry/files/process/COM/Explorer/system"),
            new TestCase("archive Phase F P206 incomplete default restoration", ArchiveIntegrationServiceTests.IncompleteDefaultRestoreRetainsRetryEvidence,
                TestEffect.Fixture,"Reviewed: memory store and ledger, injected partial Explorer choice restoration; no Registry, process or Shell effects"),
            new TestCase("archive Phase F P206 user choice proof", ArchiveIntegrationServiceTests.UserChoiceProofIsStableAndMinuteScoped,
                TestEffect.Fixture,"Reviewed: pure proof computation over fixed inputs; no Registry/files/process/COM/Explorer/system"),
            new TestCase("archive Phase F P206 service failures compensate", ArchiveIntegrationServiceTests.ApplyFailuresCompensateAndNeverReportApplied,
                TestEffect.Fixture,"Reviewed: fake store write-then-throw and failing in-memory ledger; no Registry/files/process/COM/Explorer/system"),
            new TestCase("archive Phase F P206 ledger wire registry target guards", ArchiveIntegrationServiceTests.LedgerRoundTripWireAndRegistryTargetGuards,
                TestEffect.Fixture,"Reviewed: GUID-owned temporary ledger file, parser/serializer and pure target validation; no Registry API/process/COM/Explorer/system"),
            new TestCase("archive Phase F P207 selection forwards once in order", ArchiveShellSelectionTests.OneSelectionForwardsOnceInOrderThroughComVtable,
                TestEffect.Fixture,"Reviewed: in-process class factory/COM vtable on owned STA with managed fake IShellItemArray and recording forwarder; no CoRegisterClassObject/Registry/Explorer/process"),
            new TestCase("archive Phase F P207 partial failures reject whole selection", ArchiveShellSelectionTests.PartialFailuresRejectWholeSelectionWithoutDrops,
                TestEffect.Fixture,"Reviewed: managed fake Shell items on owned STA and recording forwarder; no COM registration/Registry/Explorer/process"),
            new TestCase("archive Phase F P207 identity queue cancel lifetime", ArchiveShellSelectionTests.IdentityQueueCancellationAndLifetimeFailures,
                TestEffect.Fixture,"Reviewed: GUID-owned temporary fake desktop bytes, fake launcher, in-process class factory lifetime on owned STA; no process launch/COM registration/Registry/Explorer"),
            new TestCase("archive Phase F P207 future registration plan unapplied", ArchiveShellSelectionTests.FutureRegistrationPlanMatchesHelperAndStaysUnapplied,
                TestEffect.Fixture,"Reviewed: pure future CLSID/DelegateExecute plan and fake-store transaction only; no Registry/COM registration/Explorer/system"),
            new TestCase("archive Phase F P206 ledger failure compensates all values", ArchiveIntegrationServiceTests.LedgerFailureCompensatesEveryValueDespiteRestoreFailure,
                TestEffect.Fixture,"Reviewed: fake store with one failing Restore and failing in-memory ledger; no Registry/files/process/COM/Explorer/system"),
            new TestCase("archive Phase F P207 final release without execute readmits", ArchiveShellSelectionTests.FinalReleaseWithoutExecuteRetiresSlotAndReadmits,
                TestEffect.Fixture,"Reviewed: in-process class factory/CCW final Release on owned STA plus forced GC; no CoRegisterClassObject/Registry/Explorer/process"),
            new TestCase("archive Phase F P207 lock server keeps pump alive", ArchiveShellSelectionTests.LockServerKeepsPumpAliveUntilUnlocked,
                TestEffect.Fixture,"Reviewed: actual pump decision boundary with short explicit bounds on owned STA, no COM init/registration; no Registry/Explorer/process"),
            new TestCase("archive Phase E delivery real RAR four five families", ArchivePhaseEDeliveryTests.ActualRarFourFiveRealFamilies,
                TestEffect.Fixture,"Reviewed: fixed-provenance tiny genuine RAR4/5 volumes copied to GUID-owned scratch, original Job/worker/native leases; independent payload hash oracle; no GUI/Shell/system mutation"),
            new TestCase("archive Phase E source all-volume protection missing", ArchiveVolumeSourceTests.ProtectedFamilyAndMissingMembers,
                TestEffect.Fixture,"Reviewed: GUID-owned scratch encoded segments and original native source leases; protection/discovery only,not codec interoperability; no GUI/Shell/system mutation"),
            new TestCase("archive Phase E source single ZIP exact disk count", ArchiveVolumeSourceTests.OrdinarySingleSourceAndExactZipDiskCount,
                TestEffect.Fixture,"Reviewed: GUID-owned scratch single/ZIP disk discovery,all-member lease protection; no GUI/Shell/system mutation"),
            new TestCase("archive REF19 source ALZ genuine protected released", ArchiveAlzegVolumeSourceTests.AlzGenuineFamiliesProtectedAndReleased,
                TestEffect.Fixture,"Reviewed: CC0 ordinary/real ALZ families copied to GUID-owned session scratch, original native source leases, writer/rename protection and exact handle release; no worker/GUI/Shell/Registry/service/member execution"),
            new TestCase("archive REF19 source EGG genuine protected released", ArchiveAlzegVolumeSourceTests.EggGenuineFamiliesProtectedAndReleased,
                TestEffect.Fixture,"Reviewed: CC0 ordinary/real EGG families copied to GUID-owned session scratch, first/middle/final entry, original native leases and exact handle release; no worker/GUI/Shell/Registry/service/member execution"),
            new TestCase("archive REF19 source ALZ missing corrupt release", ArchiveAlzegVolumeSourceTests.AlzMissingCorruptAndFailureRelease,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 ALZ copies only, missing/corrupt headers/tails, typed acquisition failure and released handles; no worker/GUI/Shell/Registry/service/member execution"),
            new TestCase("archive REF19 source EGG missing corrupt release", ArchiveAlzegVolumeSourceTests.EggMissingCorruptAndFailureRelease,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 EGG copies only, missing/corrupt/duplicate/cyclic Header IDs, typed acquisition failure and released handles; no worker/GUI/Shell/Registry/service/member execution"),
            new TestCase("archive REF19 source EGG IDs not filename order", ArchiveAlzegVolumeSourceTests.EggHeaderIdsNotFilenameOrder,
                TestEffect.Fixture,"Reviewed: renamed GUID-owned CC0 EGG continuations, unrelated owned archive, original ordered source leases and release; no worker/GUI/Shell/Registry/service/member execution"),
            new TestCase("archive REF19 source finite admission content dispatch", ArchiveAlzegVolumeSourceTests.FiniteAdmissionAndContentDispatch,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 and synthetic source copies only, finite encoded/metadata quotas and content-bound discovery; no worker/GUI/Shell/Registry/service/member execution"),
            new TestCase("archive REF20 SFX source ordinary protected release", ArchiveSfxVolumeSourceTests.OrdinaryProtectedAndReleased,
                TestEffect.Fixture, "Reviewed: GUID-owned data-only ordinary SFX coordinate copies, original source leases/writer and parent-rename denial/exclusive reopen; no executable or member launch"),
            new TestCase("archive REF20 SFX source split entries protected release", ArchiveSfxVolumeSourceTests.SplitEntriesProtectedAndReleased,
                TestEffect.Fixture, "Reviewed: GUID-owned data-only SFX split families from EXE and final ZIP entries, complete ordered native source leases and release; no executable or member launch"),
            new TestCase("archive REF20 SFX source missing surplus corrupt release", ArchiveSfxVolumeSourceTests.MissingSurplusAndCorruptRelease,
                TestEffect.Fixture, "Reviewed: owned missing/surplus parts and descriptor mutations, typed native acquisition failure and exclusive reopen; no executable or member launch"),
            new TestCase("archive REF20 SFX source finite admission unrelated", ArchiveSfxVolumeSourceTests.FiniteAdmissionAndUnrelatedSources,
                TestEffect.Fixture, "Reviewed: owned coordinate family and unrelated data file, bounded original source admission and actual lease release; no executable or member launch"),
            new TestCase("archive REF20 SFX missing corrupt typed cleanup", ArchiveSfxProductTests.MissingCorruptFamilyTypedCleanup,
                TestEffect.Fixture, "Reviewed: missing own generated continuation and malformed own prefix descriptor, original Test/lease failure and actual release; generated EXE/member never executed"),
            new TestCase("archive REF20 SFX complete prefix split limit", ArchiveSfxProductTests.CompletePrefixMustFitVolume,
                TestEffect.Fixture, "Reviewed: GUID-owned source and original creation quota at full stub plus span marker boundary, zero publication and cleanup; no generated EXE execution"),
            new TestCase("archive REF20 SFX explicit intent wire", ArchiveSfxProductTests.ExplicitIntentWireContract,
                TestEffect.Fixture, "Reviewed: synthetic immutable Job JSON only, explicit SFX intent, legacy omission and incompatible option refusal; no native execution"),
            new TestCase("archive REF20 SFX ordinary Store Deflate", ArchiveSfxProductTests.OrdinaryStoredDeflate,
                TestEffect.Fixture, "Reviewed: GUID-owned source trees and matching worker, original Create/Read/Test/extract/native publication, exact bytes and handle release; generated EXE/member never executed"),
            new TestCase("archive REF20 SFX split Store Deflate", ArchiveSfxProductTests.SplitStoredDeflate,
                TestEffect.Fixture, "Reviewed: GUID-owned small forced physical SFX families, original Job/native candidate journal and full-family readback; generated EXE/member never executed"),
            new TestCase("archive REF20 SFX fresh family no overwrite", ArchiveSfxProductTests.FreshFamilyNoOverwrite,
                TestEffect.Fixture, "Reviewed: GUID-owned preexisting family sentinels, original bounded held-directory refusal and zero publication; no deployed files or EXE execution"),
            new TestCase("archive REF20 SFX cancel publication facts", ArchiveSfxProductTests.CancellationBeforeAndAfterPublication,
                TestEffect.Fixture, "Reviewed: GUID-owned split creation, original pre/late publication cancellation and actual worker/output cleanup; no generated EXE/member execution"),
            new TestCase("archive REF20 SFX native first error protection", ArchiveSfxProductTests.NativeWriteFirstErrorAndProtection,
                TestEffect.Fixture, "Reviewed: owned native write112 injection, original source leases/first error/actual cleanup and exclusive reopen; no generated EXE/member execution"),
            new TestCase("archive REF20 SFX partial publication journal", ArchiveSfxProductTests.LaterPublicationFailureKeepsJournal,
                TestEffect.Fixture, "Reviewed: GUID-owned later-target conflict after first native publication, no-replace journal and cleanup facts; no generated EXE/member execution"),
            new TestCase("archive REF20 SFX selected small output source credit", ArchiveSfxProductTests.SelectedSmallOutputKeepsSourceAdmission,
                TestEffect.Fixture, "Reviewed: GUID-owned copies of independently qualified actual product SFX, original Browse-bound selected Job, exact low output/metadata quotas and exclusive source release; no EXE/member execution"),
            new TestCase("archive REF20 SFX selected output quota remains exact", ArchiveSfxProductTests.SelectedOutputQuotaRemainsExact,
                TestEffect.Fixture, "Reviewed: GUID-owned qualified actual SFX, original Browser selection with one-byte output shortfall, typed no-publication and actual source/helper release; no EXE/member execution"),
            new TestCase("archive REF19 product raw names Browse Test reject extract", ArchiveAlzegProductTests.RawUnsafeNamesBrowseTestRejectExtract,
                TestEffect.Fixture, "Reviewed: owned specification ALZ/EGG raw names, original read-only integrity and unsafe-path refusal before output, protected sources and actual cleanup; no member execution"),
            new TestCase("archive REF19 product EGG name declarations typed cleanup", ArchiveAlzegProductTests.EggNameDeclarationsTypedCleanup,
                TestEffect.Fixture, "Reviewed: owned specification locale/parent/encrypted-name/reserved-global declarations, typed original Read/Job refusal and no output; no member execution"),
            new TestCase("archive REF19 product EGG POSIX kinds Browse Test extract", ArchiveAlzegProductTests.EggPosixKindsBrowseTestExtract,
                TestEffect.Fixture, "Reviewed: owned CC0 POSIX/specification kinds, original Read/Job/native payload and timestamp restoration, nonmaterializing links/FIFOs and actual cleanup; no member execution"),
            new TestCase("archive REF19 product EGG short POSIX failure lifecycle", ArchiveAlzegProductTests.EggShortAndPosixFailureLifecycle,
                TestEffect.Fixture, "Reviewed: owned specification short AES/LEA headers, typed unsupported/no-output and original POSIX cancel/native-write failure cleanup; no member execution"),
            new TestCase("archive REF19 product EGG AZO Browse Test extract", ArchiveAlzegProductTests.EggAzoBrowseTestExtract,
                TestEffect.Fixture, "owned CC0/specification AZO inputs, original Read/Browser/Job/native output and actual cleanup; no member execution"),
            new TestCase("archive REF19 product EGG AZO failure lifecycle", ArchiveAlzegProductTests.EggAzoFailureLifecycle,
                TestEffect.Fixture, "owned AZO corruption/limit/cancel/write fixtures, protected sources and actual cleanup; no member execution"),
            new TestCase("archive REF19 product EGG solid Browse Test extract", ArchiveAlzegProductTests.EggSolidBrowseTestExtract,
                TestEffect.Fixture, "owned CC0/specification solid samples, original Read/Browser/Job/native output and cleanup"),
            new TestCase("archive REF19 product EGG solid selected tail limit", ArchiveAlzegProductTests.EggSolidSelectedTailKeepsPredecessorsAndLimit,
                TestEffect.Fixture, "owned solid predecessors, original Browser selection/output credit and actual native cleanup"),
            new TestCase("archive REF19 product EGG solid failure lifecycle", ArchiveAlzegProductTests.EggSolidFailureLifecycle,
                TestEffect.Fixture, "owned solid corruption/limit/cancel/write fixtures with protected sources and actual cleanup"),
            new TestCase("archive REF19 product ordinary ALZ Browse Test extract", ArchiveAlzegProductTests.OrdinaryAlzBrowseTestExtract,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 ALZ copies, explicit actual Rust worker under original Read/Job/native output, independent exact bytes and complete cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product ordinary EGG Browse Test extract", ArchiveAlzegProductTests.OrdinaryEggBrowseTestExtract,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 EGG method copies, explicit actual Rust worker under original Read/Job/native output, independent exact bytes and complete cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product split ALZ Browse Test extract", ArchiveAlzegProductTests.SplitAlzBrowseTestExtract,
                TestEffect.Fixture,"Reviewed: GUID-owned genuine CC0 ALZ volumes, explicit actual Rust worker, original complete-source Read/Browser-bound Job/native output, independent bytes/CP949 names and cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product split EGG Browse Test extract", ArchiveAlzegProductTests.SplitEggBrowseTestExtract,
                TestEffect.Fixture,"Reviewed: GUID-owned genuine CC0 EGG volumes including continuation entry, explicit actual Rust worker, original complete-source Read/Browser-bound Job/native output, independent exact bytes/cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product missing corrupt typed cleanup", ArchiveAlzegProductTests.MissingCorruptOutcomesAndCleanup,
                TestEffect.Fixture,"Reviewed: missing/corrupt GUID-owned CC0 volumes only, explicit actual Rust worker, original Read/Test/extract Job failures and owned zero-output cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product changed continuation old Browser", ArchiveAlzegProductTests.ChangedContinuationRejectsOldBrowser,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 continuations replaced only after original Read cleanup, explicit actual Rust worker and stale Browser-bound Job rejection before output; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product cancel all-source cleanup", ArchiveAlzegProductTests.CancelRetainsAllSourcesThroughCleanup,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 volumes/output, explicit actual Rust worker, original Job cancel at existing native precommit probe, all-source protection through real cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product native write first-error cleanup", ArchiveAlzegProductTests.NativeWriteFailureRetainsAllSourcesThroughCleanup,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 volumes/output, explicit actual Rust worker, existing native write probe returns owned error112, all-source protection/first-error/actual cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product source-bound output readmission", ArchiveAlzegProductTests.LowOutputReadmissionUsesCompleteSourceBinding,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 volume families, explicit original worker/Job shortfall and output-only retry, exact independent bytes and cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product selected-member output limit", ArchiveAlzegProductTests.SelectedMemberKeepsOriginalOutputLimit,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 families, original Read/Browser entry materialization narrowed to one byte, typed selected quota and worker/cache/source cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product encrypted ALZ Browse Test extract", ArchiveAlzegProductTests.EncryptedAlzBrowseTestExtract,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 encrypted samples, original Read/Browser/Job/password ticket and explicit actual worker/native output, independent exact bytes and cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product encrypted ALZ password failure lifecycle", ArchiveAlzegProductTests.EncryptedAlzPasswordFailureLifecycle,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 copies/ciphertext mutations, original Job password wrong/cancel and stale tickets, source/helper protection, zero preparation writes and actual cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product encrypted EGG Browse Test extract", ArchiveAlzegProductTests.EncryptedEggBrowseTestExtract,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 EGG samples, original Read/Browser/Job/password/native output and independent byte comparison; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product encrypted EGG password failure lifecycle", ArchiveAlzegProductTests.EncryptedEggPasswordFailureLifecycle,
                TestEffect.Fixture,"Reviewed: GUID-owned EGG ciphertext/tag mutations, original password tickets, source/helper protection, zero preparation writes and actual cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product ALZ DLZ Browse Test extract", ArchiveAlzegProductTests.AlzDlzBrowseTestExtract,
                TestEffect.Fixture,"Reviewed: GUID-owned CC0 and labelled specification samples, actual Read/Browser/Job/worker/native output, independent bytes and cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive REF19 product ALZ DLZ failure lifecycle", ArchiveAlzegProductTests.AlzDlzFailureLifecycle,
                TestEffect.Fixture,"Reviewed: GUID-owned DLZ mutations, finite budgets, current Job cancel/native write112, source protection and actual cleanup; no GUI/Shell/member execution/Registry/service"),
            new TestCase("archive incoming Add backslash conflict decisions", EditInteractionTests.IncomingBackslashDecisions,
                TestEffect.Fixture,"Reviewed: GUID-owned backslash ZIP and source, matching worker, original conflict tickets and native publication probes; independent content/source/no-partial/cleanup checks; no GUI/Shell/system/user archive"),
            new TestCase("archive incoming Add Desktop to native boundary", EditInteractionTests.IncomingDesktopNativeAdd,
                TestEffect.Fixture,"Reviewed: GUID-owned source/ZIP, explicit actual Desktop TestApp apphost renders and clicks original Add, emitted typed request enters original Job/worker/native publication with independent bytes; no physical GUI or user/system mutation"),
            new TestCase("archive incoming Add isolated sample copy", EditInteractionTests.IncomingSampleCopy,
                TestEffect.Fixture,"Reviewed: explicit read-only sample copied into GUID-owned temporary archive-product root, only copy modified through original worker/native Job; no member execution or user archive writes"),
            new TestCase("archive incoming Add normal Unicode paths", EditInteractionTests.IncomingNormalPaths,
                TestEffect.Fixture,"Reviewed: matching worker/hash, GUID-owned ZIP/files/folders, original Browse/Job/native single-publication probes and independent .NET bytes; no GUI/Shell/Registry/service/user archive"),
            new TestCase("archive edit interaction native ZIP Ask decisions single publish", EditInteractionTests.ActualZipAskDecisionsSinglePublication,
                TestEffect.Fixture,"Reviewed: explicit matching frozen worker/hash, GUID-owned ZIP/source and original Job/private conflict/native single-publication chain; independent .NET decode; no GUI/Shell/Registry/service/user archive"),
            new TestCase("archive EIC AC01 native directory merge zero publish", EditInteractionTests.EicAc01DirectoryMergeZeroPublication,
                TestEffect.Fixture,"Reviewed: matching frozen worker/hash, GUID-owned ZIP/directories, original Browse/Job/native publication probes, .NET exact bytes and zero-publication assertion; no GUI/Shell/system/user archive"),
            new TestCase("archive EIC AC02 native cancelled reply only ZIP TAR 7z", EditInteractionTests.EicAc02NativeCancelledReplyOnly,
                TestEffect.Fixture,"Reviewed: matching worker/hash and explicit Python/stock7Zip seed oracles; GUID-owned originals and existing held-source commit probe returns cancelled without control frame; no publication/GUI/Shell/system/user archive"),
            new TestCase("archive OPT02 native Add namespace ZIP", EditInteractionTests.Opt02AddNamespaceCorrectnessZip,
                TestEffect.Fixture,"Reviewed: GUID-owned PI_SCRATCH_DIR sources and ZIP, hash-pinned copied worker, original Job/browser/native single publication with independent readers; no GUI/Explorer/Registry/service/user archive"),
            new TestCase("archive OPT02 native Add namespace SevenZ", EditInteractionTests.Opt02AddNamespaceCorrectnessSevenz,
                TestEffect.Fixture,"Reviewed: GUID-owned PI_SCRATCH_DIR sources and encrypted/header 7z, hash-pinned copied worker, original Job/browser/native publish and independent Python/stock7Zip; no GUI/Explorer/Registry/service/user archive"),
            new TestCase("archive OPT02 native Add namespace TAR family", EditInteractionTests.Opt02AddNamespaceCorrectnessTarFamily,
                TestEffect.Fixture,"Reviewed: GUID-owned PI_SCRATCH_DIR TAR/TGZ/TXZ sources, copied frozen worker, original Job/browser/native publish and independent Python/bsdtar oracle; no GUI/Explorer/Registry/service/user archive"),
            new TestCase("archive OPT03 native actual Add result terminal", EditInteractionTests.Opt03NativeAddResults,
                TestEffect.Fixture,"Reviewed: matching frozen worker/oracles, GUID-owned PI_SCRATCH_DIR ZIP/7z/TAR originals and inputs, original Job and native publication, independent content verification; no GUI/Shell/system/user archives"),
            new TestCase("archive OPT03 Add result wire validation", EditInteractionTests.Opt03AddWireValidation,
                TestEffect.Fixture,"Reviewed: strict private DTO serialization/validation only; no worker/native/system effects"),
            new TestCase("archive HZ3 AC01 native directory publication zero output", Hz3BatchTests.NativeDirectoryPublicationAndZeroOutput,
                TestEffect.Fixture,"Reviewed: explicit matching frozen worker/hash; GUID-owned ZIP/target original Job/native directory-metadata and before-flush failure seams; exports measured terminal for original Desktop bridge; source unchanged, no GUI/Shell/system/user archive"),
            new TestCase("archive HZ3 AC01 publication wire retained directories", Hz3BatchTests.PublicationWireAndRetainedDirectories,
                TestEffect.Fixture,"Reviewed: symmetric optional bool DTO roundtrip and GUID-owned original native Prepare/partial root/ancestry/unknown commit fixtures; retained directories and evidence are not deleted; no worker/GUI/Shell/system/user archive"),
            new TestCase("archive tar edit wire shapes capabilities", TarEditTests.StrictWireAndCapabilities,
                TestEffect.Fixture,"Reviewed: strict DTO/capability checks only; no worker/native/system effects"),
            new TestCase("archive tar edit native actions metadata empty", TarEditTests.ActualActionsAndEmpty,
                TestEffect.Fixture,"Reviewed: explicit frozen worker/hash, GUID-owned archives/original Job/native one-shot replacement and independent CPython oracle; no GUI/Shell/Registry/service/ACL"),
            new TestCase("archive tar edit native source cancel handoff", TarEditTests.ActualSourceCancellationAndHandoff,
                TestEffect.Fixture,"Reviewed: GUID-owned source-writer/stale/cancel/held-parent checks at original commit boundary; no GUI/Shell/system mutation"),
            new TestCase("archive tar edit native failure unknown late cancel", TarEditTests.ActualNativeFailureUnknownAndLateCancel,
                TestEffect.Fixture,"Reviewed: GUID-owned native5/64/latecancel probes, original publication journal and unknown evidence retained; independent oracle; no GUI/Shell/system mutation"),
            new TestCase("archive tar edit native conflict injected lowspace", TarEditTests.ActualConflictAndInjectedLowSpace,
                TestEffect.Fixture,"Reviewed: original conflict tickets plus injected disk-full112 at existing before_flush boundary on GUID-owned TAR only; no disk-fill/GUI/Shell/system mutation"),
            new TestCase("archive sevenz edit wire ordinary replace accepted", SevenzEditTests.OrdinarySevenzReplaceAccepted,
                TestEffect.Fixture,"Reviewed: strict DTO roundtrip only; no worker/native/files/system effects"),
            new TestCase("archive sevenz edit wire strict shapes ZIP preserved", SevenzEditTests.StrictShapesPreserveZipAndBounds,
                TestEffect.Fixture,"Reviewed: strict DTO shape/unknown-authority bounds only; no worker/native/files/system effects"),
            new TestCase("archive sevenz edit wire admission snapshot capabilities", SevenzEditTests.AdmissionSnapshotAndCapabilities,
                TestEffect.Fixture,"Reviewed: immutable capture and capability validation only; no worker/native/files/system effects"),
            new TestCase("archive sevenz edit native actions subtree all delete", SevenzEditTests.ActualActionsAndEmptyResult,
                TestEffect.Fixture,"Reviewed: hash-pinned matching worker/original Job/source/native same-path commit; GUID-owned session scratch and existing independent7Zip oracle only; no GUI/Shell/system mutation"),
            new TestCase("archive sevenz edit native AES header replace conflict", SevenzEditTests.ActualEncryptedReplacementAndConflict,
                TestEffect.Fixture,"Reviewed: GUID-owned encrypted fixtures/original private password and conflict ports; oracle secrets stdin only, never argv/log/disk; no GUI/Shell/system mutation"),
            new TestCase("archive sevenz edit native source cancel handoff unknown", SevenzEditTests.ActualSourceCancellationAndHandoff,
                TestEffect.Fixture,"Reviewed: GUID-owned scratch/source-writer refusal/stale/cancel/parent protection; original native injected unknown outcome and retained candidate; no GUI/Shell/system mutation"),
            new TestCase("archive sevenz edit native empty subtree implicit unicode", SevenzEditTests.ActualEmptyInputsSubtreeImplicitAndNames,
                TestEffect.Fixture,"Reviewed: GUID-owned worker/Job/native output, zero-length sources, subtree and exact independent 7z oracle; no GUI/Shell/system mutation"),
            new TestCase("archive sevenz edit native independent metadata retention", SevenzEditTests.ActualIndependentMetadataRetention,
                TestEffect.Fixture,"Reviewed: explicit stock 7-Zip creates GUID-owned metadata seed, original worker/native edit and independent 7-Zip listing; no GUI/Shell/system mutation"),
            new TestCase("archive sevenz edit native conflict skip native terminal", SevenzEditTests.ActualConflictWarningsAndNativeCommitTruth,
                TestEffect.Fixture,"Reviewed: GUID-owned original private conflict port/native commit probe/late cancel, independent oracle; no GUI/Shell/system mutation"),
            new TestCase("archive Phase E delivery ZIP edit all actions stale source", ArchivePhaseEDeliveryTests.ActualZipAllEditsAndStaleBinding,
                TestEffect.Fixture,"Reviewed: GUID-owned scratch ZIPs, original product Job/worker/source handoff/native publication; independent .NET read; no GUI/Shell/system mutation"),
            new TestCase("archive Phase E delivery ZIP seven volumes old family", ArchivePhaseEDeliveryTests.ActualZipSevenVolumesAndOldFamilyRefusal,
                TestEffect.Fixture,"Reviewed: GUID-owned small ZIP/7z real volumes, original Job/worker/native leases and no-replace publication, explicit independent oracle; no GUI/Shell/system mutation"),
            new TestCase("archive Phase E delivery partial volume race facts", ArchivePhaseEDeliveryTests.ActualPartialVolumePublicationNoOverwrite,
                TestEffect.Fixture,"Reviewed: GUID-owned small volume output race; retains original native partial publication journal and foreign target; no GUI/Shell/system mutation"),
            new TestCase("archive B3 explicit split sizes count cancellation", ArchivePhaseEDeliveryTests.ActualExplicitSplitSizesLimitsAndCancellation,
                TestEffect.Fixture,"Reviewed: GUID-owned150000-byte and64MiB synthetic sources, original Job/worker/native ZIP/7z output, explicit existing7Zip oracle, real pre/late-cancel and1024-volume limit; only owned cleanup, no GUI/Shell/system mutation"),
            new TestCase("raw numbered join serial reader physical drain", RawNumberedJoinReaderTests.SerialReaderResponsiveAndPhysicalDrain,
                TestEffect.Fixture, "Reviewed: GUID-owned scratch/source/output and GUID named pipe, production serial ReadLoop/dispatcher/session; held actual transfer, Ping/invalid clipboard parsing/exact cancel, diagnostics and awaited handle drain; no product/GUI/clipboard/Shell/system mutation"),
            new TestCase("raw numbered join family source refusals", RawNumberedJoinTests.FamilyAndSourceRefusals,
                TestEffect.Fixture, "Reviewed: GUID-owned scratch ordinary parts/metadata, missing/alias/count/source/output refusal through original Windows handles; no GUI/Shell/system mutation"),
            new TestCase("raw numbered join stream cancel failure handles", RawNumberedJoinTests.StreamCancellationFailureAndHandleCompletion,
                TestEffect.Fixture, "Reviewed: GUID-owned2MiB source, actual64KiB reads/writes, source writer refusal, original output write-error checkpoint and pre/late cancellation/cleanup/handle completion; no execution or system mutation"),
            new TestCase("raw numbered join later part byte exact", RawNumberedJoinTests.LaterPartPreviewAndByteExactJoin,
                TestEffect.Fixture, "Reviewed: GUID-owned scratch ordinary .001 parts, existing Windows source/output handles and byte readback only; no GUI/Shell/system mutation or execution"),
            new TestCase("archive ZIP edit native held source same path commit", ArchiveZipEditCommitTests.HeldSourceSamePathReplacementPublishesValidZip,
                TestEffect.Fixture, "Reviewed: GUID-owned ZIPs in the approved session scratch tree; existing source lease and atomic native output commit only; no GUI/Shell/config/registry/service"),
            new TestCase("archive ZIP edit native same path cancellation truth", ArchiveZipEditCommitTests.CancellationBeforeAndAfterPublicationPreservesTruth,
                TestEffect.Fixture, "Reviewed: GUID-owned ZIPs in the approved session scratch tree; existing source lease and atomic native output commit only; no GUI/Shell/config/registry/service"),
            new TestCase("archive ZIP edit native changed revision refusal", ArchiveZipEditCommitTests.ChangedRevisionIsRefusedAfterLeaseRetires,
                TestEffect.Fixture, "Reviewed: GUID-owned ZIPs in the approved session scratch tree; existing source lease and atomic native output commit only; no GUI/Shell/config/registry/service"),
            new TestCase("archive ZIP edit native handoff gates and ownership", ArchiveZipEditCommitTests.HandoffGatesSourceTargetCancellationAndOwnership,
                TestEffect.Fixture, "Reviewed: GUID-owned ZIPs in approved session scratch; typed edit-only source close evidence and existing atomic output boundary; no GUI/Shell/config/registry/service"),
            new TestCase("archive ZIP edit native handoff failure unknown no retry", ArchiveZipEditCommitTests.HandoffNativeFailureUnknownAndNoRetry,
                TestEffect.Fixture, "Reviewed: GUID-owned ZIPs in approved session scratch; injected existing native commit checkpoints and exact retained facts; no GUI/Shell/config/registry/service"),
            new TestCase("archive resource tool control initialization owns status", ArchiveResourceToolTests.InitializationOwnsStatus,
                TestEffect.Fixture, "Reviewed: session scratch files/process-local environment only; no worker launched"),
            new TestCase("archive resource tool control bounded idempotent", ArchiveResourceToolTests.ControlBoundedAndIdempotent,
                TestEffect.Fixture, "Reviewed: bounded synthetic control files only"),
            new TestCase("archive resource tool counter atomic validity", ArchiveResourceToolTests.CounterReadsCommitTogether,
                TestEffect.Fixture, "Reviewed: fake counters only; no process started"),
            new TestCase("archive resource tool time dispatch excludes setup", ArchiveResourceToolTests.DispatchExcludesSetup,
                TestEffect.Fixture, "Reviewed: virtual clock only"),
            new TestCase("archive resource tool actual small stop convergence", ArchiveResourceToolTests.ActualSmallStopConverges,
                TestEffect.Fixture, "Reviewed: independent 256KiB ZIP in session scratch, exact test child/helper/runner Job; original native cleanup, no user data")
        }).ToArray();
    }

    public static async Task<int> Main(string[] args)
    {
        if (args.Length == 2 && args[0] == "/info" && Environment.GetEnvironmentVariable(DirectoryOpusExportTests.ControlVariable) is not null)
            return DirectoryOpusExportTests.RunHelper(args[1]);
        if (args.Length == 2 && args[0] == "--dopus-export-writer")
            return DirectoryOpusExportTests.RunWriter(args[1]);
        if (args.SequenceEqual(new[] { "--archive-direct-peer" }))
            return await ArchiveExtractionPipelineTests.Peer();
        if (args.SequenceEqual(new[] { "--archive-parallel-begin-peer" }))
            return await ArchiveParallelExtractionTests.Peer();
        if (args.SequenceEqual(new[] { "--opt06-bulk-peer" }))
            return await Opt06BulkWireTests.Peer();
        if (args.Length == 3 && args[0] == "--hz4-owned-process")
            return await Hz4ProcessTests.RunAsync(args[1], args[2]);
        if (args.SequenceEqual(new[] { "--floating-renderer-fixture" }))
            return await FloatingAssistantRendererFixture.Run();
        if (args.Length == 2 && args[0] == "--sevenz-output-readmission-host")
            return await SevenzReadmissionHost.RunAsync(args[1]);
        if (args.Length == 2 && args[0] == "--rar-output-readmission-host")
            return await RarReadmissionHost.RunAsync(args[1]);
        if (args.Length == 2 && args[0] == "--tar-stream-output-readmission-host")
            return await TarStreamReadmissionHost.RunAsync(args[1]);
        if (args.Length == 2 && args[0] == "--archive-output-readmission-host")
            return await ArchiveOutputReadmissionHost.RunAsync(args[1]);
        if (args.Length == 3 && args[0] == "--archive-resource-probe")
            return await ArchiveResourceProbe.RunAsync(args[1], args[2]);
        if (args.SequenceEqual(new[] { "--archive-product-worker-fixture" }))
            return await ArchiveProductWorkerFixture.RunAsync();
        if (args.Length == 3 && args[0] == "--archive-product-host")
            return await ArchiveProductHost.RunAsync(args[1], args[2]);
        if (args.SequenceEqual(new[] { "--archive-worker-fixture" }))
            return await ArchiveWorkerFixture.RunAsync();
        if (args.SequenceEqual(new[] { "--version" }) ||
            args.SequenceEqual(new[] { "-I", "-S", "-c", "import sys;print('Python '+'.'.join(map(str,sys.version_info[:3])))" }) ||
            args.SequenceEqual(new[] { "-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "$PSVersionTable.PSVersion.ToString()" }))
            return await ExtensionRuntimeTests.VersionFixtureAsync(args);
        if (args.Length > 0 && args[0] is "--extension-fixture-child" or "--extension-fixture-broker" or "--extension-fixture-parent")
            return await ExtensionProcessFixture.RunAsync(args);
        try
        {
            var options = TestRunOptions.Parse(args);
            var selectedTests = TestSelection.Select(Catalog(), options);
            if (options.List)
            {
                foreach (var test in selectedTests)
                    Console.WriteLine($"LIST {test.Effect} | {test.Name} | {test.Scope}");
                Console.WriteLine($"SUMMARY selected={selectedTests.Length} run=0 pass=0 skip=0 fail=0 not_run={selectedTests.Length}");
                return 0;
            }
            return await TestRunner.Run(selectedTests, Console.Out, Console.Error);
        }
        catch (ArgumentException error)
        {
            Console.Error.WriteLine($"SELECTION ERROR: {error.Message}");
            Console.WriteLine("SUMMARY selected=0 run=0 pass=0 skip=0 fail=0 not_run=0");
            return 2;
        }
    }

    private sealed class FakeCompletionSystemActions : ICompletionSystemActions
    {
        public int SoundRequests { get; private set; }
        public List<bool> SuspendRequests { get; } = [];
        public Queue<WindowsCommandResult> SuspendResults { get; } = new();
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

        public WindowsCommandResult RequestSuspend(bool hibernate)
        {
            SuspendRequests.Add(hibernate);
            return SuspendResults.Count == 0
                ? new WindowsCommandResult(true)
                : SuspendResults.Dequeue();
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

    private static async Task B5CompletionSuspendSystemWireRoutesThroughProductionDispatch()
    {
        var actions = new FakeCompletionSystemActions();
        var expected = new[]
        {
            (Hibernate: false, Result: new WindowsCommandResult(true)),
            (Hibernate: true, Result: new WindowsCommandResult(true)),
            (Hibernate: false, Result: new WindowsCommandResult(false, "unsupported", NativeErrorCode: 50)),
            (Hibernate: false, Result: new WindowsCommandResult(false, "access denied", NativeErrorCode: 5)),
            (Hibernate: true, Result: new WindowsCommandResult(false, "privilege not held", NativeErrorCode: 1314)),
            (Hibernate: true, Result: new WindowsCommandResult(false, "native failure", NativeErrorCode: 87))
        };
        foreach (var item in expected)
        {
            actions.SuspendResults.Enqueue(item.Result);
        }

        var adapter = CreateCompletionOnlyAdapter(actions);
        var log = new RecordingPlatformLog();
        var client = new PlatformPipeClient("unused-b5-fixture", adapter, log);
        var execute = typeof(PlatformPipeClient).GetMethods(
                System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)
            .Single(method => method.Name == "ExecuteCommandAsync" &&
                method.GetParameters().Length == 5 &&
                method.GetParameters()[4].ParameterType == typeof(uint));

        for (var index = 0; index < expected.Length; index++)
        {
            var item = expected[index];
            var diagnosticCorrelationId = Guid.NewGuid();
            var request = ProtocolEnvelope.Create(
                "platform_command",
                new { Command = "suspend_system", Data = new { item.Hibernate } },
                diagnosticCorrelationId);
            var parsed = PlatformCommandParser.ParseSuspendSystem(request);
            Assert(parsed.Hibernate == item.Hibernate);

            var outgoing = Channel.CreateBounded<ProtocolEnvelope>(1);
            await (Task)execute.Invoke(
                client,
                [request, "suspend_system", outgoing.Writer, CancellationToken.None, 0u])!;
            var response = await outgoing.Reader.ReadAsync();
            Assert(response.CorrelationId == request.Id);
            Assert(response.Payload.GetProperty("success").GetBoolean() == item.Result.Success);
            Assert(ReadOptionalString(response.Payload, "error") == item.Result.Error);
            Assert(ReadOptionalInt32(response.Payload, "native_error_code") == item.Result.NativeErrorCode);
            Assert(response.Payload.GetProperty("data").ValueKind == JsonValueKind.Null);

            var action = item.Hibernate ? "hibernate" : "sleep";
            var accepted = log.Entries.Single(entry =>
                entry.EventName == "command.received" &&
                entry.CorrelationId == diagnosticCorrelationId);
            Assert(Equals(accepted.Fields["command"], "suspend_system"));
            Assert(Equals(accepted.Fields["action"], action));
            var terminal = log.Entries.Single(entry =>
                entry.EventName == "command.completed" &&
                entry.CorrelationId == diagnosticCorrelationId);
            Assert(Equals(terminal.Fields["action"], action));
            Assert(Equals(terminal.Fields["terminal"], item.Result.Success ? "succeeded" : "failed"));
            Assert(Equals(terminal.Fields["native_error_code"], item.Result.NativeErrorCode));
            Assert(terminal.Fields.ContainsKey("elapsed_ms"));
        }

        Assert(actions.SuspendRequests.SequenceEqual(expected.Select(item => item.Hibernate)));
    }

    private static async Task B5CompletionHeldSuspendKeepsDispatcherResponsiveAndDrains()
    {
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        using var release = new ManualResetEventSlim();
        await dispatcher.DispatchAsync("suspend_system", () =>
        {
            entered.TrySetResult();
            Assert(release.Wait(TimeSpan.FromSeconds(10)));
            return Task.CompletedTask;
        });
        await entered.Task.WaitAsync(TimeSpan.FromSeconds(5));

        var laterCommandRan = false;
        await dispatcher.DispatchAsync("set_window_visibility", () =>
        {
            laterCommandRan = true;
            return Task.CompletedTask;
        });
        Assert(laterCommandRan);

        var drain = dispatcher.DrainAsync(TimeSpan.FromSeconds(10));
        await Task.Delay(25);
        Assert(!drain.IsCompleted);
        release.Set();
        await drain.WaitAsync(TimeSpan.FromSeconds(5));
    }

    private static WindowsPlatformAdapter CreateCompletionOnlyAdapter(
        ICompletionSystemActions completionSystemActions)
    {
        var adapter = (WindowsPlatformAdapter)System.Runtime.CompilerServices.RuntimeHelpers
            .GetUninitializedObject(typeof(WindowsPlatformAdapter));
        typeof(WindowsPlatformAdapter).GetField(
            "_completionSystemActions",
            System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
            .SetValue(adapter, completionSystemActions);
        return adapter;
    }

    private static string? ReadOptionalString(JsonElement element, string propertyName)
    {
        var value = element.GetProperty(propertyName);
        return value.ValueKind == JsonValueKind.Null ? null : value.GetString();
    }

    private static int? ReadOptionalInt32(JsonElement element, string propertyName)
    {
        var value = element.GetProperty(propertyName);
        return value.ValueKind == JsonValueKind.Null ? null : value.GetInt32();
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


    private static Task FullscreenSummonClassifiesForegroundWindows()
    {
        var monitor = new FullscreenRect(1920, 0, 3840, 1080);
        var coveredWindow = new FullscreenRect(1920, 0, 3840, 1080);
        var borderless = new FullscreenForegroundSnapshot(
            true, true, false, false, 0, coveredWindow, monitor);
        var exclusiveQueryFailed = new FullscreenSummonObservation(
            FullscreenNotificationStateResult.Failure(unchecked((int)0x80004005)),
            true, borderless);
        var publications = 0;
        var decision = QuickLaunchSummonGate.DispatchObserved(
            true, true, QuickLaunchSummonTrigger.ClipboardHotKey,
            () => exclusiveQueryFailed, _ => publications++);
        Assert(decision.Suppressed);
        Assert(decision.FullscreenKind == FullscreenSuppressionKind.Borderless);
        Assert(decision.QueryHresult == unchecked((int)0x80004005));
        Assert(publications == 0);

        var ordinaryMaximized = borderless with { WindowBounds = new FullscreenRect(0, 0, 1600, 900), Style = 0x00C40000 };
        var desktop = borderless with { Desktop = true };
        var hidden = borderless with { Visible = false };
        var minimized = borderless with { Minimized = true };
        var invalid = borderless with { Valid = false };
        foreach (var foreground in new[] { ordinaryMaximized, desktop, hidden, minimized, invalid })
        {
            decision = QuickLaunchSummonGate.DispatchObserved(
                true, true, QuickLaunchSummonTrigger.HotKey,
                () => new FullscreenSummonObservation(
                    new FullscreenNotificationStateResult(true, 2, 0), true, foreground),
                _ => publications++);
            Assert(!decision.Suppressed);
        }
        Assert(publications == 5);
        return Task.CompletedTask;
    }

    private static Task FullscreenSummonPublishBoundaryKeepsPoliciesIndependent()
    {
        var monitor = new FullscreenRect(0, 0, 2560, 1440);
        var foreground = new FullscreenForegroundSnapshot(
            true, true, false, false, 0,
            new FullscreenRect(0, 0, 2560, 1440), monitor);
        var observation = new FullscreenSummonObservation(
            new FullscreenNotificationStateResult(true, 2, 0), true, foreground);
        var quickPublications = 0;
        var clipboardPublications = 0;
        var quick = QuickLaunchSummonGate.DispatchObserved(
            true, false, QuickLaunchSummonTrigger.DoubleControl,
            () => observation, _ => quickPublications++);
        var clipboard = QuickLaunchSummonGate.DispatchObserved(
            true, true, QuickLaunchSummonTrigger.ClipboardHotKey,
            () => observation, _ => clipboardPublications++);
        Assert(quick.Published && quickPublications == 1);
        Assert(clipboard.Suppressed && clipboardPublications == 0);

        var source = File.ReadAllText(Path.Combine(
            FindRepositoryRoot(), "csharp", "NxTools.PlatformHost", "Windows", "WindowsPlatformAdapter.EventWindow.cs"));
        Assert(source.Contains("_clipboardSuppressInFullscreen = command.SuppressInFullscreen", StringComparison.Ordinal));
        Assert(source.Contains("_quickLaunchSuppressInExclusiveFullscreen = request.Command.SuppressInExclusiveFullscreen", StringComparison.Ordinal));
        Assert(source.Contains("PublishPanelRequest()", StringComparison.Ordinal));
        Assert(source.Contains("QuickLaunchSummonTrigger.ClipboardHotKey", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task QuickLaunchFullscreenSuppressionIsPrecise()
    {
        foreach (var trigger in new[]
        {
            QuickLaunchSummonTrigger.HotKey,
            QuickLaunchSummonTrigger.DoubleControl
        })
        {
            var queries = 0;
            var publications = 0;
            var decision = QuickLaunchSummonGate.Dispatch(
                triggerEnabled: true,
                suppressInExclusiveFullscreen: true,
                trigger: trigger,
                queryFullscreenState: () =>
                {
                    queries++;
                    return new FullscreenNotificationStateResult(true, 3, 0);
                },
                publish: _ => publications++);

            Assert(decision.Suppressed);
            Assert(decision.Trigger == trigger);
            Assert(queries == 1);
            Assert(publications == 0);

            decision = QuickLaunchSummonGate.Dispatch(
                triggerEnabled: true,
                suppressInExclusiveFullscreen: true,
                trigger: trigger,
                queryFullscreenState: () => new FullscreenNotificationStateResult(true, 2, 0),
                publish: _ => publications++);
            Assert(!decision.Suppressed);
            var borderlessOrWindowed = QuickLaunchSummonGate.Dispatch(
                triggerEnabled: true,
                suppressInExclusiveFullscreen: true,
                trigger: trigger,
                queryFullscreenState: () => new FullscreenNotificationStateResult(true, 1, 0),
                publish: _ => publications++);
            Assert(!borderlessOrWindowed.Suppressed);
            Assert(publications == 2);

            decision = QuickLaunchSummonGate.Dispatch(
                triggerEnabled: true,
                suppressInExclusiveFullscreen: true,
                trigger: trigger,
                queryFullscreenState: () => FullscreenNotificationStateResult.Failure(unchecked((int)0x80004001)),
                publish: _ => publications++);
            Assert(!decision.Suppressed);
            Assert(decision.QueryHresult == unchecked((int)0x80004001));
            Assert(publications == 3);

            queries = 0;
            decision = QuickLaunchSummonGate.Dispatch(
                triggerEnabled: true,
                suppressInExclusiveFullscreen: false,
                trigger: trigger,
                queryFullscreenState: () =>
                {
                    queries++;
                    return new FullscreenNotificationStateResult(true, 3, 0);
                },
                publish: _ => publications++);
            Assert(!decision.Suppressed);
            Assert(queries == 0);
            Assert(publications == 4);
        }
        return Task.CompletedTask;
    }

    private static Task QuickLaunchSummonDiagnosticsKeepPublishedCorrelationAndRateLimitFailures()
    {
        var diagnosticGate = new QuickLaunchSummonDiagnosticGate();
        var success = new QuickLaunchSummonDecision(
            Guid.NewGuid(),
            QuickLaunchSummonTrigger.HotKey,
            true,
            false,
            true,
            true,
            2,
            null,
            0.1);
        Assert(diagnosticGate.TryAcquire(success, out _));
        Assert(diagnosticGate.TryAcquire(success with { CorrelationId = Guid.NewGuid() }, out _));
        var suppressed = success with { CorrelationId = Guid.NewGuid(), Suppressed = true, Published = false };
        Assert(diagnosticGate.TryAcquire(suppressed, out _));
        Assert(diagnosticGate.TryAcquire(success with { CorrelationId = Guid.NewGuid() }, out _));
        Assert(diagnosticGate.TryAcquire(suppressed with { CorrelationId = Guid.NewGuid() }, out _));

        var failure = success with
        {
            CorrelationId = Guid.NewGuid(),
            Published = true,
            QueryHresult = unchecked((int)0x80004001)
        };
        Assert(diagnosticGate.TryAcquire(failure, out _));
        Assert(!diagnosticGate.TryAcquire(failure with { CorrelationId = Guid.NewGuid() }, out _));
        return Task.CompletedTask;
    }

    private static Task QuickLaunchWndProcRoutesBothTriggersThroughFullscreenGate()
    {
        var repositoryRoot = FindRepositoryRoot();
        var source = File.ReadAllText(Path.Combine(
            repositoryRoot,
            "csharp",
            "NxTools.PlatformHost",
            "Windows",
            "WindowsPlatformAdapter.EventWindow.cs"));
        Assert(source.Contains(
            "case WmHotKey when message.WParam.ToInt32() == QuickLaunchHotKeyId:\r\n                    PublishQuickLaunchPanelRequest(QuickLaunchSummonTrigger.HotKey);",
            StringComparison.Ordinal));
        Assert(source.Contains(
            "case WmQuickLaunchGesture:\r\n                    HandleQuickLaunchGesture(",
            StringComparison.Ordinal));
        var routing = File.ReadAllText(Path.Combine(repositoryRoot, "csharp", "NxTools.PlatformHost",
            "Windows", "WindowsPlatformAdapter.ElevatedInput.cs"));
        Assert(routing.Contains("PublishQuickLaunchPanelRequest(QuickLaunchSummonTrigger.DoubleControl);", StringComparison.Ordinal));
        Assert(source.Contains(
            "var decision = QuickLaunchSummonGate.DispatchObserved(",
            StringComparison.Ordinal));
        Assert(source.Contains("QuickLaunchSummonTrigger.ClipboardHotKey", StringComparison.Ordinal));
        Assert(source.Contains("PublishPanelRequestCore", StringComparison.Ordinal));
        Assert(source.Contains(
            "correlationId => PublishQuickLaunchPanelRequestCore(correlationId)",
            StringComparison.Ordinal));
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
        Assert(IndexerServiceInstaller.RegistrationMatches(
            valid with { StartMode = 3 },
            indexer));
        Assert(!IndexerServiceInstaller.RegistrationMatches(valid with { StartMode = 4 }, indexer));
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
                MaxImagePixels = 120_000_000,
                SuppressInFullscreen = false,
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
        Assert(!command.SuppressInFullscreen);

        var recordingEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_clipboard_hot_key_recording",
            Data = new { Recording = true }
        });
        var recording = PlatformCommandParser.ParseSetClipboardHotKeyRecording(recordingEnvelope);
        Assert(recording.Recording);
        var summonId = Guid.NewGuid();
        var summon = PlatformEventEnvelopeFactory.Create(new ClipboardPanelRequestedEvent(
            new ForegroundContext(42, 42, 7, "fixture", "fixture"),
            100, 200, 0, 96, 1, 1, summonId));
        Assert(summon.CorrelationId == summonId);
        Assert(summon.Payload.GetProperty("event").GetString() == "clipboard_panel_requested");
        return Task.CompletedTask;
    }

    private static Task ClipboardCaptureLimitsClampAndReject()
    {
        Assert(new ConfigureClipboardCommand([], "C", "Alt+C", []).SuppressInFullscreen);
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
                FileDialogSearchEnabled = true,
                SuppressInExclusiveFullscreen = true,
            }
        });
        var configuration = PlatformCommandParser.ParseConfigureQuickLaunch(configurationEnvelope);
        Assert(configuration.Enabled);
        Assert(configuration.AutoQuickSwitchEnabled);
        Assert(configuration.FileDialogSearchEnabled);
        Assert(configuration.SuppressInExclusiveFullscreen);
        Assert(QuickLaunchTriggerBinding.TryCreate(configuration, out var binding, out _));
        Assert(binding.Kind == QuickLaunchTriggerKind.DoubleControl);

        var legacyConfigurationEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "configure_quick_launch",
            Data = new
            {
                Enabled = true,
                HotkeyModifiers = Array.Empty<string>(),
                HotkeyKey = "Control",
                HotkeyDisplay = "双击 Ctrl"
            }
        });
        var legacyConfiguration = PlatformCommandParser.ParseConfigureQuickLaunch(legacyConfigurationEnvelope);
        Assert(legacyConfiguration.SuppressInExclusiveFullscreen);
        Assert(legacyConfiguration.AutoQuickSwitchEnabled);
        Assert(legacyConfiguration.FileDialogSearchEnabled);

        var summonCorrelationId = Guid.NewGuid();
        var eventEnvelope = PlatformEventEnvelopeFactory.Create(
            new QuickLaunchPanelRequestedEvent(
                new ForegroundContext(88, 89, 9, "editor", "draft.md"),
                1200, 700, 1, 144, 1_700_000_000_500,
                new DirectoryNavigationContext("explorer", 88, @"C:\Work"),
                summonCorrelationId));
        Assert(eventEnvelope.CorrelationId == summonCorrelationId);
        var eventData = eventEnvelope.Payload.GetProperty("data");
        Assert(eventData.GetProperty("correlation_id").GetGuid() == summonCorrelationId);
        Assert(eventData.GetProperty("requested_at_unix_ms").GetInt64() == 1_700_000_000_500);
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

        // An unrelated foreground between source and dialog (taskbar, Alt+Tab) is not observed;
        // the newest source still synchronizes exactly once.
        tracker.ObserveExplorer(111);
        var afterUnrelated = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException("Expected the newest source to survive an unrelated foreground.");
        Assert(afterUnrelated.SourceWindowHandle == 111);
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

    private static Task VisibleDialogReuseObservesReturningDirectoryOpusSource()
    {
        var snapshot = new FileDialogSearchHostSnapshot(
            77,
            @"D:\Workspace",
            100,
            100,
            900,
            600,
            96);

        Assert(FileDialogSearchHost.ShouldRefreshDirectorySourceOnReuse(
            autoQuickSwitchEnabled: true,
            searchEnabled: true,
            hostVisible: true,
            snapshot,
            foregroundWindowHandle: 77,
            trackedSourceIsDirectoryOpus: true));
        Assert(!FileDialogSearchHost.ShouldRefreshDirectorySourceOnReuse(
            autoQuickSwitchEnabled: false,
            searchEnabled: true,
            hostVisible: true,
            snapshot,
            foregroundWindowHandle: 77,
            trackedSourceIsDirectoryOpus: true));
        Assert(!FileDialogSearchHost.ShouldRefreshDirectorySourceOnReuse(
            autoQuickSwitchEnabled: true,
            searchEnabled: true,
            hostVisible: true,
            snapshot,
            foregroundWindowHandle: 88,
            trackedSourceIsDirectoryOpus: true));
        Assert(!FileDialogSearchHost.ShouldRefreshDirectorySourceOnReuse(
            autoQuickSwitchEnabled: true,
            searchEnabled: true,
            hostVisible: true,
            snapshot,
            foregroundWindowHandle: 77,
            trackedSourceIsDirectoryOpus: false));
        Assert(!FileDialogSearchHost.ShouldRefreshDirectorySourceOnReuse(
            autoQuickSwitchEnabled: true,
            searchEnabled: false,
            hostVisible: false,
            snapshot,
            foregroundWindowHandle: 77,
            trackedSourceIsDirectoryOpus: true));
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

    private static Task DirectoryNavigationTargetResolvesLibraryAndNestedPaths()
    {
        var pictures = Environment.GetFolderPath(
            Environment.SpecialFolder.MyPictures);
        Assert(Directory.Exists(pictures));
        Assert(DirectoryNavigationTarget.TryResolve(
            "lib://Pictures",
            out var resolvedPictures));
        Assert(string.Equals(
            resolvedPictures,
            Path.GetFullPath(pictures),
            StringComparison.OrdinalIgnoreCase));
        Assert(string.Equals(
            ShellIconExtractor.ResolveNavigationDirectory("lib://Pictures"),
            Path.GetFullPath(pictures),
            StringComparison.OrdinalIgnoreCase));

        var localizedPictures =
            ThirdPartyFileManagerDisplayPath.GetLocalizedFolderDisplayName(
                pictures);
        if (!string.IsNullOrWhiteSpace(localizedPictures))
        {
            Assert(DirectoryNavigationTarget.TryResolve(
                $"lib://{localizedPictures}",
                out var resolvedLocalizedPictures));
            Assert(string.Equals(
                resolvedLocalizedPictures,
                Path.GetFullPath(pictures),
                StringComparison.OrdinalIgnoreCase));
        }

        var fields = DirectoryNavigationTarget.DiagnosticFields(
            @"D:\Qingfeng");
        Assert(string.Equals(
            fields["target_kind"] as string,
            "filesystem_absolute",
            StringComparison.Ordinal));
        Assert(string.Equals(
            fields["target_root"] as string,
            @"D:\",
            StringComparison.Ordinal));
        Assert(Convert.ToInt32(fields["target_segment_count"]) == 2);
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
            var navigationOwnedAdapter =
                new FakeThirdPartyFileManagerOperations(
                    "directory_opus",
                    windowHandle: 88,
                    currentDirectory: @"C:\Work")
                {
                    RequiresPostNavigationActivation = false
                };
            var navigationOwnedRegistry =
                new ThirdPartyFileManagerSessionRegistry(
                    [navigationOwnedAdapter]);
            var navigationOwnedContext =
                navigationOwnedRegistry.TryCapture(foreground).Context ??
                throw new InvalidOperationException("capture failed");
            Assert(navigationOwnedRegistry.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    88,
                    directory.FullName,
                    navigationOwnedContext.HostContextId),
                Directory.Exists).Success);
            Assert(navigationOwnedAdapter.LastActivatedWindowHandle == 0);

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
    private static Task DirectoryOpusRefreshUsesCapturedSession()
    {
        var directory = Directory.CreateTempSubdirectory("nxtools-dopus-refresh-");
        try
        {
            var native = new FakeThirdPartyFileManagerOperations(
                "Directory Opus", 88, @"D:\OldTab");
            var registry = new ThirdPartyFileManagerSessionRegistry([native]);
            var foreground = new ForegroundContext(88, 89, 9, "dopus", "Old tab");
            var initial = registry.TryCapture(foreground).Context ??
                throw new InvalidOperationException("Initial DOpus session capture failed.");

            native.CurrentDirectory = @"D:\NewTab";
            var refreshed = registry.TryRefreshDirectoryOpusSource(
                    88,
                    initial.HostContextId,
                    foreground with { WindowTitle = "New tab" }) ??
                throw new InvalidOperationException("The existing DOpus session was not refreshed.");
            Assert(refreshed.WindowHandle == initial.WindowHandle);
            Assert(refreshed.HostContextId == initial.HostContextId);
            Assert(refreshed.CurrentDirectory == @"D:\NewTab");
            Assert(native.NavigateCount == 0);
            Assert(registry.IsDirectoryOpusSource(
                initial.WindowHandle,
                initial.HostContextId));
            Assert(!registry.IsDirectoryOpusSource(99, initial.HostContextId));
            var otherRegistry = new ThirdPartyFileManagerSessionRegistry(
                [new FakeThirdPartyFileManagerOperations("Total Commander", 89, @"E:\Other")]);
            var otherContext = otherRegistry.TryCapture(
                new ForegroundContext(89, 89, 9, "totalcmd", "Other"))
                .Context ?? throw new InvalidOperationException("Other file manager session capture failed.");
            Assert(!otherRegistry.IsDirectoryOpusSource(
                otherContext.WindowHandle,
                otherContext.HostContextId));
            var navigation = registry.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    refreshed.WindowHandle,
                    directory.FullName,
                    refreshed.HostContextId),
                Directory.Exists);
            Assert(navigation.Success);
            Assert(native.LastPaneState == 2);
            Assert(registry.TryRefreshDirectoryOpusSource(
                99,
                refreshed.HostContextId,
                foreground with { WindowHandle = 99 }) is null);
        }
        finally
        {
            directory.Delete(recursive: true);
        }
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

    private static Task DirectoryOpusDialogReturnRefreshesExistingTabPath()
    {
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(autoSwitchEnabled: true, trackSources: true);
        var dialog = new DirectoryNavigationContext(
            DirectoryNavigationAdapter.FileDialogHost,
            77,
            @"C:\Dialog");
        var native = new FakeThirdPartyFileManagerOperations(
            "Directory Opus", 88, @"D:\OldTab");
        var sessions = new ThirdPartyFileManagerSessionRegistry([native]);
        var foreground = new ForegroundContext(88, 89, 9, "dopus", "Directory Opus");
        var capturedSource = sessions.TryCapture(foreground).Context ??
            throw new InvalidOperationException("The initial DOpus session capture failed.");

        Assert(tracker.ObserveFileDialog(dialog) is null);
        tracker.ObserveDirectorySource(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                capturedSource.WindowHandle,
                capturedSource.CurrentDirectory,
                capturedSource.HostContextId),
            "Old tab (Directory Opus)");
        var initial = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException("The initial source should be consumed.");
        Assert(initial.Source.CurrentDirectory == @"D:\OldTab");
        Assert(tracker.ObserveFileDialog(dialog) is null);
        Assert(!tracker.HasPendingDirectorySourceForWindow(88));

        native.CurrentDirectory = @"D:\ExistingTab";
        var switched = DirectoryOpusDialogReturnRefresh.Observe(
            tracker,
            dialog,
            source => sessions.IsDirectoryOpusSource(
                source.Context.WindowHandle,
                source.Context.HostContextId),
            source =>
            {
                var refreshed = sessions.TryRefreshDirectoryOpusSource(
                    source.Context.WindowHandle,
                    source.Context.HostContextId,
                    foreground);
                if (refreshed is null)
                {
                    return false;
                }
                _ = tracker.RefreshDirectorySource(
                    source.Context with
                    {
                        CurrentDirectory = refreshed.CurrentDirectory,
                        HostContextId = refreshed.HostContextId
                    },
                    "Existing tab (Directory Opus)");
                return true;
            }) ?? throw new InvalidOperationException(
                "Returning to a known dialog after an existing-tab switch should sync once.");
        Assert(switched.Source.CurrentDirectory == @"D:\ExistingTab");
        Assert(switched.Source.HostContextId == capturedSource.HostContextId);
        Assert(native.NavigateCount == 0);
        Assert(tracker.ObserveFileDialog(dialog) is null);

        var unchanged = DirectoryOpusDialogReturnRefresh.Observe(
            tracker,
            dialog,
            source => sessions.IsDirectoryOpusSource(
                source.Context.WindowHandle,
                source.Context.HostContextId),
            source =>
            {
                var refreshed = sessions.TryRefreshDirectoryOpusSource(
                    source.Context.WindowHandle,
                    source.Context.HostContextId,
                    foreground);
                if (refreshed is null)
                {
                    return false;
                }
                _ = tracker.RefreshDirectorySource(
                    source.Context with { CurrentDirectory = refreshed.CurrentDirectory },
                    "Existing tab (Directory Opus)");
                return true;
            });
        Assert(unchanged is null);

        _ = tracker.RefreshDirectorySource(
            tracker.LatestSource!.Context with { CurrentDirectory = @"D:\PendingOldPath" },
            "Pending (Directory Opus)");
        var failedRefresh = DirectoryOpusDialogReturnRefresh.Observe(
            tracker,
            dialog,
            _ => true,
            _ => false);
        Assert(failedRefresh is null);
        Assert(!tracker.HasPendingDirectorySourceForWindow(88));
        Assert(DirectoryOpusDialogReturnRefresh.Observe(
            tracker,
            dialog with { WindowHandle = 78 },
            _ => throw new InvalidOperationException("A new dialog must not refresh the source."),
            _ => throw new InvalidOperationException("A new dialog must not refresh the source.")) is null);
        return Task.CompletedTask;
    }

    private static Task AutoQuickSwitchRefreshesChangedDirectoryOpusSource()
    {
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(enabled: true);
        var dialog = new DirectoryNavigationContext("file_dialog", 77, @"C:\Dialog");
        Assert(tracker.ObserveFileDialog(dialog) is null);

        tracker.ObserveDirectorySource(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                88,
                @"D:\OldTab",
                "dopus:old-session"),
            "Old tab (Directory Opus)");
        var initial = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException("The first source should sync once.");
        Assert(initial.Source.CurrentDirectory == @"D:\OldTab");
        Assert(tracker.ObserveFileDialog(dialog) is null);

        Assert(tracker.RefreshDirectorySource(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                88,
                @"D:\NewTab",
                "dopus:new-session"),
            "New tab (Directory Opus)"));
        var refreshed = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException("A changed source path should produce one new sync.");
        Assert(refreshed.SourceWindowHandle == 88);
        Assert(refreshed.Source.CurrentDirectory == @"D:\NewTab");
        Assert(refreshed.Source.HostContextId == "dopus:new-session");
        Assert(tracker.ObserveFileDialog(dialog) is null);

        Assert(!tracker.RefreshDirectorySource(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                88,
                @"D:\NewTab",
                "dopus:latest-session"),
            "New tab (Directory Opus)"));
        Assert(!tracker.RefreshDirectorySource(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                99,
                @"E:\Unrelated",
                "dopus:unrelated-session"),
            "Unrelated (Directory Opus)"));
        Assert(tracker.ObserveFileDialog(dialog) is null);
        return Task.CompletedTask;
    }


    private static Task DirectoryOpusExistingTabSelectionRefreshesTrackedSource()
    {
        const uint eventObjectFocus = 0x8005;
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(autoSwitchEnabled: true, trackSources: true);
        var dialog = new DirectoryNavigationContext(
            DirectoryNavigationAdapter.FileDialogHost,
            77,
            @"C:\Dialog");
        var native = new FakeThirdPartyFileManagerOperations(
            "Directory Opus", 88, @"D:\OldTab")
        {
            RequiresPostNavigationActivation = false
        };
        var sessions = new ThirdPartyFileManagerSessionRegistry([native]);
        var foreground = new ForegroundContext(88, 89, 9, "dopus", "Directory Opus");
        var captured = sessions.TryCapture(foreground).Context ??
            throw new InvalidOperationException("The initial DOpus session capture failed.");
        Assert(tracker.ObserveFileDialog(dialog) is null);
        tracker.ObserveDirectorySource(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                captured.WindowHandle,
                captured.CurrentDirectory,
                captured.HostContextId),
            "Old tab (Directory Opus)");
        var initial = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException("The initial source should be consumed.");
        Assert(initial.Source.CurrentDirectory == @"D:\OldTab");

        var commandTarget = Path.GetTempPath();
        Assert(sessions.Navigate(
            new NavigateThirdPartyFileManagerCommand(
                captured.WindowHandle,
                commandTarget,
                captured.HostContextId),
            Directory.Exists).Success);
        Assert(native.NavigateCount == 1);

        // Selecting an already-open tab keeps the same lister HWND. DOpus
        // raises an object-focus event, after which PlatformHost re-reads the
        // same source identity even though its one-shot navigation was used.
        native.CurrentDirectory = @"D:\ExistingTab";
        Assert(DirectoryOpusSourceRefresh.ShouldRefresh(
            eventType: eventObjectFocus,
            objectId: DirectoryOpusSourceRefresh.ObjectIdWindow,
            rootWindowHandle: captured.WindowHandle,
            trackedListerWindowHandle: captured.WindowHandle));
        var refreshed = sessions.TryRefreshDirectoryOpusSource(
            captured.WindowHandle,
            captured.HostContextId,
            foreground) ?? throw new InvalidOperationException(
                "Selecting an existing tab must refresh the source after its one-shot navigation was consumed.");
        Assert(refreshed.HostContextId != captured.HostContextId);
        Assert(!sessions.IsDirectoryOpusSource(
            captured.WindowHandle,
            captured.HostContextId));
        Assert(sessions.IsDirectoryOpusSource(
            refreshed.WindowHandle,
            refreshed.HostContextId));
        Assert(tracker.RefreshDirectorySource(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                refreshed.WindowHandle,
                refreshed.CurrentDirectory,
                refreshed.HostContextId),
            "Existing tab (Directory Opus)"));
        var candidate = DirectoryOpusDialogReturnRefresh.Observe(
            tracker,
            dialog,
            source => sessions.IsDirectoryOpusSource(
                source.Context.WindowHandle,
                source.Context.HostContextId),
            _ => true) ?? throw new InvalidOperationException(
                "Returning to the known dialog after selecting an existing tab should yield one candidate.");
        Assert(candidate.Source.CurrentDirectory == @"D:\ExistingTab");
        Assert(candidate.Source.HostContextId == refreshed.HostContextId);
        Assert(native.NavigateCount == 1);
        Assert(DirectoryOpusDialogReturnRefresh.Observe(
            tracker,
            dialog,
            _ => true,
            _ => true) is null);

        var refreshedNavigation = sessions.Navigate(
            new NavigateThirdPartyFileManagerCommand(
                refreshed.WindowHandle,
                commandTarget,
                refreshed.HostContextId),
            Directory.Exists);
        Assert(refreshedNavigation.Success);
        Assert(native.NavigateCount == 2);
        Assert(!sessions.Navigate(
            new NavigateThirdPartyFileManagerCommand(
                refreshed.WindowHandle,
                commandTarget,
                refreshed.HostContextId),
            Directory.Exists).Success);
        Assert(native.NavigateCount == 2);
        Assert(sessions.TryRefreshDirectoryOpusSource(
            refreshed.WindowHandle,
            refreshed.HostContextId,
            foreground with { ProcessId = foreground.ProcessId + 1 }) is null);
        return Task.CompletedTask;
    }



    private static Task DirectoryOpusSourceRefreshRequiresTrackedListerNameChange()
    {
        Assert(DirectoryOpusSourceRefresh.ShouldRefresh(
            eventType: DirectoryOpusSourceRefresh.EventObjectNameChange,
            objectId: DirectoryOpusSourceRefresh.ObjectIdWindow,
            rootWindowHandle: 88,
            trackedListerWindowHandle: 88));
        Assert(DirectoryOpusSourceRefresh.ShouldRefresh(
            DirectoryOpusSourceRefresh.EventObjectSelection,
            DirectoryOpusSourceRefresh.ObjectIdWindow,
            88,
            88));
        Assert(DirectoryOpusSourceRefresh.ShouldRefresh(
            DirectoryOpusSourceRefresh.EventObjectSelection,
            DirectoryOpusSourceRefresh.ObjectIdClient,
            88,
            88));
        Assert(DirectoryOpusSourceRefresh.ShouldRefresh(
            DirectoryOpusSourceRefresh.EventObjectFocus,
            DirectoryOpusSourceRefresh.ObjectIdClient,
            88,
            88));
        Assert(!DirectoryOpusSourceRefresh.ShouldRefresh(
            DirectoryOpusSourceRefresh.EventObjectFocus,
            objectId: 1,
            rootWindowHandle: 88,
            trackedListerWindowHandle: 88));
        Assert(!DirectoryOpusSourceRefresh.ShouldRefresh(
            DirectoryOpusSourceRefresh.EventObjectLocationChange,
            DirectoryOpusSourceRefresh.ObjectIdWindow,
            88,
            88));
        Assert(!DirectoryOpusSourceRefresh.ShouldRefresh(
            DirectoryOpusSourceRefresh.EventObjectNameChange,
            DirectoryOpusSourceRefresh.ObjectIdWindow,
            99,
            88));
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

    private static Task TrackedDirectoryOpusReactivationKeepsPendingSync()
    {
        var tracker = new AutoQuickSwitchTracker();
        tracker.Configure(enabled: true);
        var dialog = new DirectoryNavigationContext(
            DirectoryNavigationAdapter.FileDialogHost,
            77,
            @"C:\Dialog",
            null);
        Assert(tracker.ObserveFileDialog(dialog) is null);
        var lister = new DirectoryNavigationContext(
            DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
            88,
            @"D:\Work",
            "dopus:lister-88");
        tracker.ObserveDirectorySource(lister, "Work (Directory Opus)");
        Assert(tracker.ObserveFileDialog(dialog) is not null);

        // The same lister is activated again but its pane is not yet recognizable.
        Assert(!tracker.MarkSourcePending(99));
        Assert(tracker.MarkSourcePending(88));
        var candidate = tracker.ObserveFileDialog(dialog) ??
            throw new InvalidOperationException("Expected the tracked lister to synchronize again.");
        Assert(candidate.Source == lister);
        Assert(tracker.ObserveFileDialog(dialog) is null);
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
            throw new TestSkippedException("no interactive Directory Opus lister is available");
        }
        if (!NativeFocus.IsForegroundWindow(process.MainWindowHandle))
        {
            throw new TestSkippedException("Directory Opus is running but is not the summon-time foreground window");
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

    private static Task RealDirectoryOpusNavigationReachesCDrive()
    {
        using var process = Process.GetProcessesByName("dopus")
            .FirstOrDefault(candidate => candidate.MainWindowHandle != IntPtr.Zero);
        if (process is null)
        {
            throw new TestSkippedException("no interactive Directory Opus lister is available");
        }
        var focusWindowHandle = NativeFocus.IsForegroundWindow(
            process.MainWindowHandle)
            ? NativeFocus.TryGetFocusedWindow(process.MainWindowHandle)
            : 0;

        ThirdPartyFileManagerContext? initial = null;
        for (var attempt = 0; attempt < 20 && initial is null; attempt++)
        {
            process.Refresh();
            var currentForeground = new ForegroundContext(
                process.MainWindowHandle,
                NativeFocus.IsForegroundWindow(process.MainWindowHandle)
                    ? NativeFocus.TryGetFocusedWindow(process.MainWindowHandle)
                    : 0,
                (uint)process.Id,
                process.ProcessName,
                process.MainWindowTitle);
            initial = ThirdPartyFileManagerAdapter.TryCapture(
                    currentForeground)
                .Context;
            if (initial is null)
            {
                Thread.Sleep(50);
            }
        }
        if (initial is null)
        {
            throw new InvalidOperationException(
                "The real Directory Opus lister did not expose a navigation session.");
        }
        var initialDirectory = initial.CurrentDirectory ??
            throw new InvalidOperationException(
                "The real Directory Opus lister did not expose its current directory.");
        var current = initial;
        try
        {
            foreach (var targetDirectory in new[] { @"C:\Windows" })
            {
                var navigation = ThirdPartyFileManagerAdapter.Navigate(
                    new NavigateThirdPartyFileManagerCommand(
                        current.WindowHandle,
                        targetDirectory,
                        current.HostContextId));
                if (!navigation.Success)
                {
                    throw new InvalidOperationException(
                        "Directory Opus navigation command failed before readback: " +
                        (navigation.Error ?? "unknown"));
                }
                ThirdPartyFileManagerContext? recaptured = null;
                for (var attempt = 0; attempt < 20 && recaptured is null; attempt++)
                {
                    process.Refresh();
                    var afterNavigation = new ForegroundContext(
                        process.MainWindowHandle,
                        NativeFocus.TryGetFocusedWindow(process.MainWindowHandle),
                        (uint)process.Id,
                        process.ProcessName,
                        process.MainWindowTitle);
                    recaptured = ThirdPartyFileManagerAdapter.TryCapture(
                            afterNavigation)
                        .Context;
                    if (recaptured is null)
                    {
                        Thread.Sleep(50);
                    }
                }
                current = recaptured ??
                    throw new InvalidOperationException(
                        $"The navigated Directory Opus lister could not be recaptured after {targetDirectory}.");
                if (!ThirdPartyFileManagerPath.SameDirectory(
                        current.CurrentDirectory,
                        targetDirectory))
                {
                    throw new InvalidOperationException(
                        "Directory Opus navigation readback did not reach the target.");
                }
            }
        }
        finally
        {
            ThirdPartyFileManagerContext? restoreContext = null;
            for (var attempt = 0; attempt < 20 && restoreContext is null; attempt++)
            {
                process.Refresh();
                var restoreForeground = new ForegroundContext(
                    process.MainWindowHandle,
                    NativeFocus.IsForegroundWindow(process.MainWindowHandle)
                        ? NativeFocus.TryGetFocusedWindow(process.MainWindowHandle)
                        : 0,
                    (uint)process.Id,
                    process.ProcessName,
                    process.MainWindowTitle);
                restoreContext = ThirdPartyFileManagerAdapter.TryCapture(
                        restoreForeground)
                    .Context;
                if (restoreContext is null)
                {
                    Thread.Sleep(50);
                }
            }
            if (restoreContext is null)
            {
                throw new InvalidOperationException(
                    "The Directory Opus lister could not be recaptured for restoration.");
            }
            var restore = ThirdPartyFileManagerAdapter.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    restoreContext.WindowHandle,
                    initialDirectory,
                    restoreContext.HostContextId));
            if (!restore.Success)
            {
                throw new InvalidOperationException(
                    "Directory Opus navigation test could not restore the original directory: " +
                    (restore.Error ?? "unknown"));
            }
        }
        return Task.CompletedTask;
    }

    private static Task RealFilesWindowExposesNavigationContext()
    {
        using var process = Process.GetProcessesByName("Files")
            .FirstOrDefault(candidate =>
                candidate.MainWindowHandle != IntPtr.Zero);
        if (process is null)
        {
            throw new TestSkippedException("no interactive Files window is available");
        }
        if (!NativeFocus.IsForegroundWindow(process.MainWindowHandle))
        {
            throw new TestSkippedException("Files is running but is not the summon-time foreground window");
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

    private static Task BoundedStaCompletionSignalContainsCallbackFailures()
    {
        var expected = new InvalidOperationException("completion callback failed");
        var completed = BoundedStaOperationRunner.TrySignalCompletion(
            () => throw expected,
            out var error);

        Assert(!completed);
        Assert(error == expected.Message);
        return Task.CompletedTask;
    }
    private static async Task BoundedStaOperationsReleaseCompletionWaitHandles()
    {
        using var process = Process.GetCurrentProcess();
        GC.Collect();
        GC.WaitForPendingFinalizers();
        GC.Collect();
        process.Refresh();
        var baselineHandles = process.HandleCount;

        for (var index = 0; index < 192; index++)
        {
            using var workerStarted = new ManualResetEventSlim();
            using var releaseWorker = new ManualResetEventSlim();
            var releaseTask = Task.Run(async () =>
            {
                if (!workerStarted.Wait(TimeSpan.FromSeconds(2)))
                {
                    throw new InvalidOperationException($"STA worker {index} did not start.");
                }
                await Task.Delay(35);
                releaseWorker.Set();
            });
            var completed = BoundedStaOperationRunner.TryRun(
                $"completion-handle-lifetime-{index}",
                TimeSpan.FromSeconds(2),
                cancellationToken =>
                {
                    workerStarted.Set();
                    releaseWorker.Wait(cancellationToken);
                    if (index % 2 == 1)
                    {
                        throw new InvalidOperationException("injected provider failure");
                    }
                    return true;
                },
                out bool? result,
                out var error);

            await releaseTask;
            var operationSucceeded = index % 2 == 0;
            if (completed != operationSucceeded ||
                result != (operationSucceeded ? true : null) ||
                (operationSucceeded ? error is not null : error != "injected provider failure"))
            {
                throw new InvalidOperationException(
                    $"STA operation {index} did not preserve its expected completion result.");
            }
        }


        await Task.Delay(100);
        GC.Collect();
        GC.WaitForPendingFinalizers();
        GC.Collect();
        process.Refresh();
        var finalHandleCount = process.HandleCount;
        if (finalHandleCount > baselineHandles + 64)
        {
            throw new InvalidOperationException(
                $"Runner leaked process handles across repeated operations: baseline={baselineHandles}, final={finalHandleCount}.");
        }
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
        using var cancellation = new CancellationTokenSource();
        using var cancellationWorkerStarted = new ManualResetEventSlim();
        using var releaseCancelledWorker = new ManualResetEventSlim();
        var cancelledRun = Task.Run(() =>
        {
            var succeeded = BoundedStaOperationRunner.TryRun(
                "cancelled-stubborn-provider-test",
                TimeSpan.FromSeconds(2),
                token =>
                {
                    cancellationWorkerStarted.Set();
                    releaseCancelledWorker.Wait();
                    return token.IsCancellationRequested;
                },
                out bool? result,
                out var error,
                cancellation.Token);
            return (Succeeded: succeeded, Result: result, Error: error);
        });
        Assert(cancellationWorkerStarted.Wait(TimeSpan.FromSeconds(1)));
        cancellation.Cancel();
        var cancelled = await cancelledRun.WaitAsync(TimeSpan.FromSeconds(1));
        Assert(!cancelled.Succeeded);
        Assert(cancelled.Result is null);
        Assert(cancelled.Error?.Contains("cancelled", StringComparison.OrdinalIgnoreCase) == true);
        Assert(!BoundedStaOperationRunner.TryRun(
            "cancelled-stubborn-provider-test",
            TimeSpan.FromMilliseconds(20),
            _ => true,
            out bool? _,
            out var cancelledBusyError));
        Assert(cancelledBusyError?.Contains(
            "previous bounded operation",
            StringComparison.OrdinalIgnoreCase) == true);
        releaseCancelledWorker.Set();
        Assert(SpinWait.SpinUntil(
            () => BoundedStaOperationRunner.TryRun(
                "cancelled-stubborn-provider-test",
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
            throw new TestSkippedException("no interactive Explorer window is available");
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

    private static Task ExplorerActivationSelectsFileInExistingFolder()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"nxtools-explorer-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        var file = Path.Combine(directory, "artifact.bin");
        File.WriteAllBytes(file, []);
        try
        {
            var platform = new FakeExplorerShellPlatform
            {
                CandidateWindow = new IntPtr(10),
                ForegroundWindow = new IntPtr(10),
            };
            var stages = new List<string>();
            var result = ExplorerShellActivation.LaunchAndActivate(file, platform, stages.Add);
            Assert(result.Success && result.Data == "foreground_confirmed");
            Assert(platform.LaunchArguments.SequenceEqual([ $"/select,\"{file}\"" ]));
            Assert(platform.RestoreTargets.SequenceEqual([new IntPtr(10)]));
            Assert(platform.ActivationTargets.SequenceEqual([new IntPtr(10)]));
            Assert(stages.Contains("target_resolved"));
            Assert(stages.Contains("selection_requested"));
            Assert(stages.Contains("foreground_confirmed"));
        }
        finally
        {
            Directory.Delete(directory, true);
        }
        return Task.CompletedTask;
    }

    private static Task ExplorerActivationOpensExistingDirectoryDirectly()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"nxtools-explorer-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        try
        {
            var platform = new FakeExplorerShellPlatform
            {
                CandidateWindow = new IntPtr(10),
                ForegroundWindow = new IntPtr(10),
            };
            var result = ExplorerShellActivation.EnterAndActivate(directory, platform);
            Assert(result.Success);
            Assert(platform.LaunchArguments.Count == 0);
            Assert(platform.ActivationTargets.SequenceEqual([new IntPtr(10)]));
            platform.CandidateWindow = IntPtr.Zero;
            platform.ForegroundWindow = new IntPtr(11);
            _ = ExplorerShellActivation.EnterAndActivate(directory, platform);
            Assert(platform.LaunchArguments.Last() == $"\"{directory}\"");
        }
        finally
        {
            Directory.Delete(directory, true);
        }
        return Task.CompletedTask;
    }

    private static Task ExplorerActivationWaitsForReusedWindowAndAcceptsForegroundChild()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"nxtools-explorer-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        var file = Path.Combine(directory, "artifact.bin");
        File.WriteAllBytes(file, []);
        try
        {
            var platform = new FakeExplorerShellPlatform
            {
                PublishCandidateAfterLookup = 3,
                ForegroundWindow = new IntPtr(21),
                RootByWindow = { [new IntPtr(21)] = new IntPtr(20), [new IntPtr(20)] = new IntPtr(20) },
            };
            var result = ExplorerShellActivation.LaunchAndActivate(file, platform);
            Assert(result.Success);
            Assert(platform.LaunchArguments.Count == 1);
            Assert(platform.WaitCount >= 1);
            Assert(platform.ActivationTargets.Last() == new IntPtr(20));
        }
        finally
        {
            Directory.Delete(directory, true);
        }
        return Task.CompletedTask;
    }

    private static Task ExplorerActivationRejectsDestroyedWindow()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"nxtools-explorer-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        var file = Path.Combine(directory, "artifact.bin");
        File.WriteAllBytes(file, []);
        try
        {
            var platform = new FakeExplorerShellPlatform
            {
                CandidateWindow = new IntPtr(10),
                DestroyAfterActivation = true,
                ForegroundWindow = new IntPtr(10),
            };
            var result = ExplorerShellActivation.LaunchAndActivate(file, platform);
            Assert(!result.Success && result.Data == "foreground_not_confirmed");
            Assert(platform.ActivationTargets.SequenceEqual([new IntPtr(10)]));
        }
        finally
        {
            Directory.Delete(directory, true);
        }
        return Task.CompletedTask;
    }

    private static Task ExplorerActivationRejectsUnrelatedForeground()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"nxtools-explorer-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        var file = Path.Combine(directory, "artifact.bin");
        File.WriteAllBytes(file, []);
        try
        {
            var platform = new FakeExplorerShellPlatform
            {
                CandidateWindow = new IntPtr(10),
                ForegroundWindow = new IntPtr(12),
                LastErrorCode = 5,
            };
            var result = ExplorerShellActivation.LaunchAndActivate(file, platform);
            Assert(!result.Success && result.Data == "foreground_not_confirmed");
            Assert(result.NativeErrorCode == 5);
            Assert(platform.WaitCount == 7);
            Assert(ExplorerShellActivation.IsExpectedForegroundRoot(
                new IntPtr(10), new IntPtr(11),
                window => window == new IntPtr(11) ? new IntPtr(10) : window));
            Assert(!ExplorerShellActivation.IsExpectedForegroundRoot(
                new IntPtr(10), new IntPtr(12), window => window));
        }
        finally
        {
            Directory.Delete(directory, true);
        }
        return Task.CompletedTask;
    }

    private sealed class FakeExplorerShellPlatform : IExplorerShellPlatform
    {
        public IntPtr CandidateWindow { get; set; }
        public IntPtr ForegroundWindow { get; set; }
        public bool DestroyAfterActivation { get; set; }
        public int PublishCandidateAfterLookup { get; set; }
        public int LookupCount { get; private set; }
        public int WaitCount { get; private set; }
        public int LastErrorCode { get; set; }
        public List<string> LaunchArguments { get; } = [];
        public List<IntPtr> ActivationTargets { get; } = [];
        public List<IntPtr> RestoreTargets { get; } = [];
        public Dictionary<IntPtr, IntPtr> RootByWindow { get; } = [];

        public int OpenNative(string target, bool isDirectory) =>
            isDirectory ? 0 : unchecked((int)0x80004001);

        public IntPtr FindExplorerWindow(string directory)
        {
            LookupCount++;
            if (LookupCount < PublishCandidateAfterLookup)
            {
                return IntPtr.Zero;
            }
            if (CandidateWindow == IntPtr.Zero && PublishCandidateAfterLookup > 0)
            {
                CandidateWindow = new IntPtr(20);
            }
            return CandidateWindow;
        }

        public void LaunchExplorer(string arguments) => LaunchArguments.Add(arguments);
        public bool IsWindow(IntPtr window) => window != IntPtr.Zero &&
            !(DestroyAfterActivation && window == new IntPtr(10) && ActivationTargets.Count > 0);
        public bool IsIconic(IntPtr window) => window == new IntPtr(10);
        public void RestoreWindow(IntPtr window) => RestoreTargets.Add(window);
        public void TryActivateWindow(IntPtr window)
        {
            ActivationTargets.Add(window);
        }
        public IntPtr GetForegroundWindow() => ForegroundWindow;
        public IntPtr GetRootWindow(IntPtr window) => RootByWindow.TryGetValue(window, out var root) ? root : window;
        public void WaitForNextAttempt() => WaitCount++;
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

    private static Task DesktopShellAcceptsVerifiedCurrentPackageAfterUpgrade()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-shell-upgrade-{Guid.NewGuid():N}");
        var package = Path.Combine(root, "current-package");
        var protectedRoot = Path.Combine(root, "old-protected");
        Directory.CreateDirectory(package);
        Directory.CreateDirectory(protectedRoot);
        var desktop = Path.Combine(package, "NxTools.exe");
        var host = Path.Combine(package, "NxTools.PlatformHost.exe");
        var oldDesktop = Path.Combine(protectedRoot, "NxTools.exe");
        File.WriteAllText(desktop, "verified new desktop bytes");
        File.WriteAllText(host, "current host fixture");
        File.WriteAllText(oldDesktop, "old protected desktop bytes");
        var expectedHash = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(desktop)));
        try
        {
            var resolved = WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                desktop, oldDesktop, host, expectedHash, _ => false);
            Assert(resolved == desktop, "A hash-matched current package must apply shell settings even when protected service belongs to an older build.");
            Assert(WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                desktop, oldDesktop, host, "invalid-hash", _ => false) is null,
                "Invalid expected hash must fail closed.");
            var wrongName = Path.Combine(package, "Another.exe");
            File.Copy(desktop, wrongName);
            Assert(WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                wrongName, oldDesktop, host, expectedHash, _ => false) is null,
                "A same-directory file with another name is not the desktop sibling.");
            var outside = Path.Combine(root, "outside", "NxTools.exe");
            Directory.CreateDirectory(Path.GetDirectoryName(outside)!);
            File.Copy(desktop, outside);
            Assert(WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                outside, oldDesktop, host, expectedHash, _ => false) is null,
                "An environment path outside the running PlatformHost package must not be trusted.");
            File.WriteAllText(desktop, "tampered desktop");
            Assert(WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                desktop, oldDesktop, host, expectedHash, _ => false) is null,
                "Current-package fallback must still reject a mismatched desktop hash.");
            Assert(WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                desktop, oldDesktop, host, expectedHash, candidate => candidate == oldDesktop) == oldDesktop,
                "Existing protected approval fallback must be preserved.");
            File.Delete(oldDesktop);
            Assert(WindowsPlatformAdapter.ResolveApprovedDesktopExecutablePath(
                desktop, oldDesktop, host, expectedHash, _ => true) is null,
                "Missing protected fallback and invalid current package must fail closed.");
        }
        finally { Directory.Delete(root, recursive: true); }
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

    private static Task DesktopShellFailuresReturnBoundedStageCodes()
    {
        var keyName = $@"Software\NxTools\Tests\DesktopShell\{Guid.NewGuid():N}";
        using var runKey = Registry.CurrentUser.CreateSubKey(keyName, writable: true)
            ?? throw new InvalidOperationException("Could not create isolated startup registry key.");
        try
        {
            var missing = WindowsPlatformAdapter.ApplyStartupRegistration(
                runKey,
                Path.Combine(Path.GetTempPath(), $"missing-nxtools-{Guid.NewGuid():N}.exe"),
                true);
            Assert(!missing.Success);
            Assert(missing.Data == "desktop_executable_unavailable");

            using var readOnlyRunKey = Registry.CurrentUser.OpenSubKey(keyName, writable: false)
                ?? throw new InvalidOperationException("Could not reopen isolated startup key.");
            var writeFailure = WindowsPlatformAdapter.ApplyStartupRegistration(
                readOnlyRunKey,
                Environment.ProcessPath ?? throw new InvalidOperationException("Test process path is unavailable."),
                true);
            Assert(!writeFailure.Success);
            Assert(writeFailure.Data == "startup_registration_failed");
            Assert(writeFailure.NativeErrorCode is not null);

            var trayException = WindowsPlatformAdapter.CreateDesktopShellApplyFailure(
                new System.ComponentModel.Win32Exception(5, "private tray detail"),
                "tray_icon");
            Assert(!trayException.Success);
            Assert(trayException.Data == "tray_icon_failed");
            Assert(trayException.NativeErrorCode == 5);
            Assert(!trayException.Error!.Contains("private tray detail", StringComparison.Ordinal));

            var genericException = WindowsPlatformAdapter.CreateDesktopShellApplyFailure(
                new InvalidOperationException("private registry path"),
                "unknown");
            Assert(!genericException.Success);
            Assert(genericException.Data == "windows_shell_apply_failed");
            Assert(!genericException.Error!.Contains("private registry path", StringComparison.Ordinal));

            using var controller = new DesktopTrayController((_, _, _) =>
                throw new InvalidOperationException("injected tray failure"));
            var tray = controller.Configure(true, "unused", () => { }, () => { });
            Assert(!tray.Success);
            Assert(tray.Data == "tray_icon_failed");

            var request = ProtocolEnvelope.Create("platform_command", new { Command = "configure_desktop_shell" });
            var response = PlatformPipeClient.CreateCommandResultEnvelope(request, tray);
            Assert(response.Payload.GetProperty("data").GetString() == "tray_icon_failed");
            Assert(!response.Payload.GetRawText().Contains("injected tray failure", StringComparison.Ordinal));
        }
        finally
        {
            Registry.CurrentUser.DeleteSubKeyTree(keyName, throwOnMissingSubKey: false);
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

    private static Task WindowGeometryReportsActualDevicePixels()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "query_window_geometry",
            Data = new { WindowHandle = 780L }
        });
        Assert(PlatformCommandParser.ParseQueryWindowGeometry(envelope).WindowHandle == 780L);
        var outer = new NativeWindowRectangle(-1610, 74, -758, 166);
        var client = new NativeWindowRectangle(-1600, 80, -768, 160);
        var result = WindowsPlatformAdapter.WindowGeometryResult(outer, client, 0xABCDUL, 120, false);
        Assert(result.Success && result.Data is not null);
        using var json = System.Text.Json.JsonDocument.Parse(result.Data!);
        var geometry = json.RootElement;
        Assert(geometry.GetProperty("outer").GetProperty("left").GetInt32() == -1610);
        Assert(geometry.GetProperty("outer").GetProperty("right").GetInt32() == -758);
        Assert(geometry.GetProperty("client").GetProperty("left").GetInt32() == -1600);
        Assert(geometry.GetProperty("client").GetProperty("bottom").GetInt32() == 160);
        Assert(geometry.GetProperty("display_id").GetUInt64() == 0xABCDUL);
        Assert(geometry.GetProperty("dpi").GetUInt32() == 120);
        Assert(!geometry.GetProperty("visible").GetBoolean());
        Assert(!WindowsPlatformAdapter.WindowGeometryResult(default, client, 0, 120, false).Success);
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

    private static bool _restoreCompactClientOnMove;
    private static bool _observeClientSizeMessages;
    private static int _staleClientSizeMessages;
    private static int _observedClientSizeMessages;
    private static readonly WindowProcedure ClientChromeWindowProcedure = HandleClientChromeWindow;

    [StructLayout(LayoutKind.Sequential)]
    private struct ResizeTestWindowPosition
    {
        public IntPtr Window;
        public IntPtr InsertAfter;
        public int X;
        public int Y;
        public int Width;
        public int Height;
        public uint Flags;
    }

    private static IntPtr HandleClientChromeWindow(IntPtr window, uint message, IntPtr wParam, IntPtr lParam)
    {
        // Model a client-chrome window rather than the system dialog used by
        // the old tests. GPUI also calculates its own WM_NCCALCSIZE rectangle.
        if (message == 0x0083 && wParam != IntPtr.Zero)
        {
            var rectangle = Marshal.PtrToStructure<NativeWindowRectangle>(lParam);
            Marshal.StructureToPtr(new NativeWindowRectangle(
                rectangle.Left + 1, rectangle.Top + 2,
                rectangle.Right - 1, rectangle.Bottom), lParam, false);
            return IntPtr.Zero;
        }
        if (message == 0x0005 && _observeClientSizeMessages)
        {
            _observedClientSizeMessages++;
            var current = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64());
            var announcedHeight = (int)((lParam.ToInt64() >> 16) & 0xffff);
            if (current.Exists && announcedHeight != current.ClientBounds.Height)
                _staleClientSizeMessages++;
        }
        if (message == 0x0046 && _restoreCompactClientOnMove)
        {
            var position = Marshal.PtrToStructure<ResizeTestWindowPosition>(lParam);
            if ((position.Flags & 0x0001) != 0 && (position.Flags & 0x0002) == 0)
            {
                _restoreCompactClientOnMove = false;
                position.Flags &= ~0x0001u;
                position.Height = 178; // compact 176px client plus custom chrome
                Marshal.StructureToPtr(position, lParam, false);
            }
        }
        return DefWindowProc(window, message, wParam, lParam);
    }

    private static Task CenteredResizeGenerationSurvivesReplay()
    {
        var window = CreateWindowEx(0, "#32770", "resize ownership", 0x00CF0000,
            100, 100, 500, 360, IntPtr.Zero, IntPtr.Zero, GetModuleHandle(null), IntPtr.Zero);
        Assert(window != IntPtr.Zero);
        try
        {
            Assert(WindowsPlatformAdapter.ParkNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 278).Success);
            using var entered = new ManualResetEventSlim();
            using var resume = new ManualResetEventSlim();
            var oldReplay = Task.Run(() => WindowsPlatformAdapter.ReplayCenteredResizeBarrierForTest(
                window.ToInt64(), () => { entered.Set(); Assert(resume.Wait(TimeSpan.FromSeconds(2))); }));
            Assert(entered.Wait(TimeSpan.FromSeconds(2)));
            Assert(WindowsPlatformAdapter.ParkNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 410).Success);
            resume.Set();
            Assert(oldReplay.Wait(TimeSpan.FromSeconds(2)));
            Assert(WindowsPlatformAdapter.PendingCenteredResizeForTest(window.ToInt64()).Height == 410,
                "old readback must not remove or restore the superseded 278px request");
            _ = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(window.ToInt64());
            _ = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(window.ToInt64());
            Assert(WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64()).ClientBounds.Height == 410);
            foreach (var terminal in new[] { false, true })
            {
                Assert(WindowsPlatformAdapter.ParkNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 278).Success);
                if (terminal) _ = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(window.ToInt64());
                _ = WindowsPlatformAdapter.ReplayCenteredResizeCommitForTest(window.ToInt64(), () =>
                    Assert(WindowsPlatformAdapter.ParkNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 410).Success));
                Assert(WindowsPlatformAdapter.PendingCenteredResizeForTest(window.ToInt64()).Height == 410,
                    "both stale CAS writeback and stale retirement must preserve replacement");
            }
        }
        finally { _ = DestroyWindow(window); }
        return Task.CompletedTask;
    }

    private static readonly WindowProcedure DelayedResizeWindowProcedure = HandleDelayedResizeWindow;
    private static readonly ManualResetEventSlim DelayedResizeEntered = new();
    private static readonly ManualResetEventSlim DelayedResizeRelease = new();
    private static int _delayNextResize;

    private static IntPtr HandleDelayedResizeWindow(IntPtr window, uint message, IntPtr wParam, IntPtr lParam)
    {
        if (message == 0x0046 && Interlocked.Exchange(ref _delayNextResize, 0) == 1)
        {
            DelayedResizeEntered.Set();
            _ = DelayedResizeRelease.Wait(TimeSpan.FromSeconds(5));
        }
        return DefWindowProc(window, message, wParam, lParam);
    }

    private static Task CenteredResizeDelayedHwndRemainsResponsive()
    {
        using var ready = new ManualResetEventSlim();
        using var stop = new ManualResetEventSlim();
        long windowHandle = 0;
        Exception? ownerFailure = null;
        var owner = new Thread(() =>
        {
            var instance = GetModuleHandle(null);
            var className = $"NxToolsDelayedResize-{Guid.NewGuid():N}";
            var windowClass = new WindowClassEx { Size = (uint)Marshal.SizeOf<WindowClassEx>(),
                WindowProcedure = Marshal.GetFunctionPointerForDelegate(DelayedResizeWindowProcedure),
                Instance = instance, ClassName = className };
            IntPtr window = IntPtr.Zero;
            try
            {
                Assert(RegisterClassEx(ref windowClass) != 0);
                window = CreateWindowEx(0, className, "delayed resize", 0x00CF0000,
                    100, 100, 500, 360, IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
                Assert(window != IntPtr.Zero);
                windowHandle = window.ToInt64(); ready.Set();
                while (!stop.IsSet) { Application.DoEvents(); Thread.Sleep(1); }
            }
            catch (Exception failure) { ownerFailure = failure; ready.Set(); }
            finally { if (window != IntPtr.Zero) _ = DestroyWindow(window); _ = UnregisterClass(className, instance); }
        }) { IsBackground = true };
        owner.SetApartmentState(ApartmentState.STA);
        DelayedResizeEntered.Reset(); DelayedResizeRelease.Reset();
        owner.Start();
        try
        {
            Assert(ready.Wait(TimeSpan.FromSeconds(2)) && ownerFailure is null && windowHandle != 0);
            Interlocked.Exchange(ref _delayNextResize, 1);
            var timer = Stopwatch.StartNew();
            Assert(WindowsPlatformAdapter.QueueNativeCenteredResizeForTest(windowHandle, 560, 278).Success);
            Assert(timer.ElapsedMilliseconds < 500, "acceptance must not await native HWND delivery");
            Assert(DelayedResizeEntered.Wait(TimeSpan.FromSeconds(2)));
            timer.Restart();
            Assert(WindowsPlatformAdapter.QueueNativeCenteredResizeForTest(windowHandle, 560, 410).Success);
            Assert(timer.ElapsedMilliseconds < 500, "new request must remain responsive while old SetWindowPos is held");
            Assert(WindowsPlatformAdapter.PendingCenteredResizeForTest(windowHandle).Height == 410);
            DelayedResizeRelease.Set();
            for (var retry = 0; retry < 200; retry++)
            {
                if (WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(windowHandle).ClientBounds.Height == 410) break;
                Thread.Sleep(10);
            }
            Assert(WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(windowHandle).ClientBounds.Height == 410);
        }
        finally { DelayedResizeRelease.Set(); stop.Set(); Assert(owner.Join(TimeSpan.FromSeconds(6))); }
        return Task.CompletedTask;
    }

    private static async Task CenteredResizeWorkerIsBounded()
    {
        var coordinator = new CenteredResizeCoordinator(maximumWorkers: 1, maximumWindows: 2);
        using var entered = new ManualResetEventSlim();
        using var resume = new ManualResetEventSlim();
        var order = new System.Collections.Concurrent.ConcurrentQueue<int>();
        Assert(coordinator.TrySchedule(new IntPtr(1), () => {
            order.Enqueue(1); entered.Set(); Assert(resume.Wait(TimeSpan.FromSeconds(2)));
        }));
        Assert(entered.Wait(TimeSpan.FromSeconds(2)));
        Assert(coordinator.TrySchedule(new IntPtr(1), () => order.Enqueue(2)));
        Assert(coordinator.TrySchedule(new IntPtr(1), () => order.Enqueue(3)));
        Assert(coordinator.TrySchedule(new IntPtr(2), () => order.Enqueue(4)));
        Assert(!coordinator.TrySchedule(new IntPtr(3), () => order.Enqueue(5)));
        Assert(order.ToArray().SequenceEqual(new[] { 1 }), "no parallel native operation may overtake a blocked HWND");
        resume.Set();
        for (var retry = 0; retry < 100 && order.Count < 3; retry++) await Task.Delay(10);
        Assert(order.ToArray().SequenceEqual(new[] { 1, 3, 4 }), "queued HWND updates must coalesce, not spawn hung workers");
    }

    private static Task CenteredResizeRecoveryIgnoresStaleCompletion()
    {
        var root = new IntPtr(567);
        var recovery = new CenteredResizeRecovery();
        var newer = new CenteredResizeRecoverySnapshot(root, Guid.NewGuid(), 567, 1, 0) { Generation = 2, Width = 560, Height = 410 };
        recovery.TrackPending(newer);
        var stale = new CenteredResizeReplayResult(true, Guid.NewGuid(), 567, 1, 8, true, false, null) { Generation = 1 };
        Assert(recovery.Observe(root, stale) == CenteredResizeRecoveryDecision.NoPendingResize);
        Assert(recovery.TryGetSnapshot(root, out var retained) && retained == newer);
        return Task.CompletedTask;
    }

    private static Task BrowserRegistrationCompensatesPartialFailure()
    {
        for (var failureAt = 1; failureAt <= 8; failureAt++)
        {
            var store = new SyntheticRegistrationStore(failureAt);
            var original = store.Values.ToDictionary(pair => pair.Key, pair => pair.Value);
            try
            {
                BrowserRegistrationTransaction.Apply(store, new[] {
                    new BrowserRegistrationChange("manifest:chrome", "new chrome"),
                    new BrowserRegistrationChange("manifest:firefox", "new firefox"),
                    new BrowserRegistrationChange("key:chrome", "new chrome"),
                    new BrowserRegistrationChange("key:edge", "new chrome"),
                    new BrowserRegistrationChange("key:firefox", "new firefox"),
                    new BrowserRegistrationChange("key:legacy-chrome", null),
                    new BrowserRegistrationChange("key:legacy-edge", null),
                    new BrowserRegistrationChange("key:legacy-firefox", null) });
                Assert(false, "injected write must fail");
            }
            catch (IOException) { }
            Assert(store.Values.Count == original.Count && original.All(pair => store.Values.GetValueOrDefault(pair.Key) == pair.Value),
                $"partial failure {failureAt} must restore old manifests/keys and remove newly created ones");
        }
        return Task.CompletedTask;
    }

    private sealed class SyntheticRegistrationStore(int failureAt) : IBrowserRegistrationStore
    {
        internal readonly Dictionary<string, string> Values = new() { ["manifest:chrome"] = "old manifest", ["key:chrome"] = "old key", ["key:firefox"] = "prior firefox", ["key:legacy-chrome"] = "legacy value" };
        private int _writes;
        public object? Capture(string target) => Values.GetValueOrDefault(target);
        public void Write(string target, string? value)
        {
            if (value is null) Values.Remove(target); else Values[target] = value;
            if (++_writes == failureAt) throw new IOException("synthetic write-after-mutation failure");
        }
        public void Restore(string target, object? snapshot)
        {
            if (snapshot is string value) Values[target] = value; else Values.Remove(target);
        }
    }

    private static Task NativeDragCancellationHasTerminalStatus()
    {
        var cancelled = NativeFileDragService.ResultForOleExit(0x00040101, 0);
        Assert(cancelled.Success && cancelled.Error is null && cancelled.Data!.Contains("cancelled", StringComparison.Ordinal));
        var dropped = NativeFileDragService.ResultForOleExit(0x00040100, 1);
        Assert(dropped.Success && dropped.Data!.Contains("dropped", StringComparison.Ordinal));
        Assert(!NativeFileDragService.ResultForOleExit(unchecked((int)0x80004005), 0).Success);
        return Task.CompletedTask;
    }

    private static Task ClientResizeSynchronizationRetainsParameters()
    {
        using var ready = new ManualResetEventSlim();
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var stop = new ManualResetEventSlim();
        long windowHandle = 0;
        Exception? ownerFailure = null;
        var skipDelayedPointer = 0;
        var refreshRead = 0;
        var sizeNotifications = 0;
        var staleNotifications = 0;
        WindowProcedure procedure = (window, message, wParam, lParam) =>
        {
            if (message == 0x0005 && entered.IsSet)
            {
                Interlocked.Increment(ref sizeNotifications);
                var live = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64());
                if (((lParam.ToInt64() >> 16) & 0xffff) != live.ClientBounds.Height)
                    Interlocked.Increment(ref staleNotifications);
            }
            if (message == 0x0047 && lParam != IntPtr.Zero)
            {
                var position = Marshal.PtrToStructure<ResizeTestWindowPosition>(lParam);
                if (position.Width == 0 && position.Height == 0 && position.Flags == 0x0016)
                {
                    entered.Set();
                    if (!release.Wait(TimeSpan.FromSeconds(6)) || Volatile.Read(ref skipDelayedPointer) != 0)
                        return IntPtr.Zero; // Never inspect a parameter after the old timed sender returned.
                    position = Marshal.PtrToStructure<ResizeTestWindowPosition>(lParam);
                    if (position.Window == window && position.Flags == 0x0016)
                        Interlocked.Increment(ref refreshRead);
                }
            }
            return HandleClientChromeWindow(window, message, wParam, lParam);
        };
        var owner = new Thread(() =>
        {
            var instance = GetModuleHandle(null);
            var className = $"NxToolsDelayedClientRefresh-{Guid.NewGuid():N}";
            var windowClass = new WindowClassEx { Size = (uint)Marshal.SizeOf<WindowClassEx>(),
                WindowProcedure = Marshal.GetFunctionPointerForDelegate(procedure),
                Instance = instance, ClassName = className };
            IntPtr window = IntPtr.Zero;
            try
            {
                Assert(RegisterClassEx(ref windowClass) != 0);
                window = CreateWindowEx(0, className, "delayed client refresh", 0x80000000,
                    300, 300, 562, 178, IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
                Assert(window != IntPtr.Zero);
                windowHandle = window.ToInt64(); ready.Set();
                while (!stop.IsSet) { Application.DoEvents(); Thread.Sleep(1); }
            }
            catch (Exception failure) { ownerFailure = failure; ready.Set(); }
            finally
            {
                if (window != IntPtr.Zero) _ = DestroyWindow(window);
                _ = UnregisterClass(className, instance);
                GC.KeepAlive(procedure);
            }
        }) { IsBackground = true };
        owner.SetApartmentState(ApartmentState.STA);
        Task<WindowsCommandResult>? sender = null;
        owner.Start();
        try
        {
            Assert(ready.Wait(TimeSpan.FromSeconds(2)) && ownerFailure is null && windowHandle != 0);
            var before = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(windowHandle);
            var foreground = GetForegroundWindow();
            sender = Task.Run(() => WindowsPlatformAdapter.RefreshNativeClientSizeForTest(windowHandle));
            Assert(entered.Wait(TimeSpan.FromSeconds(2)), "same-size synchronization must reach the delayed HWND");
            var returnedEarly = sender.Wait(TimeSpan.FromMilliseconds(2300));
            if (returnedEarly) Volatile.Write(ref skipDelayedPointer, 1);
            release.Set();
            Assert(sender.Wait(TimeSpan.FromSeconds(3)), "released native refresh must finish");
            Assert(!returnedEarly, "client refresh returned while the receiver still owned its WINDOWPOS parameter");
            Assert(sender.Result.Success && refreshRead > 0, "receiver must read the retained parameter before sender completes");
            var after = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(windowHandle);
            Assert(after.ClientBounds == before.ClientBounds, "refresh must preserve the actual initial client");
            Assert(sizeNotifications > 0 && staleNotifications == 0,
                "released refresh must notify the current client without stale dimensions");
            Assert(after.Bounds == before.Bounds && GetForegroundWindow() == foreground,
                "same-size refresh must preserve placement and foreground");
        }
        finally
        {
            release.Set();
            if (sender is not null) Assert(sender.Wait(TimeSpan.FromSeconds(3)));
            stop.Set();
            Assert(owner.Join(TimeSpan.FromSeconds(3)));
        }
        return Task.CompletedTask;
    }

    private static Task ClientResizeSynchronizationUsesLiveClient()
    {
        var instance = GetModuleHandle(null);
        var className = $"NxToolsQueuedClientSize-{Guid.NewGuid():N}";
        var windowClass = new WindowClassEx
        {
            Size = (uint)Marshal.SizeOf<WindowClassEx>(),
            WindowProcedure = Marshal.GetFunctionPointerForDelegate(ClientChromeWindowProcedure),
            Instance = instance, ClassName = className
        };
        Assert(RegisterClassEx(ref windowClass) != 0);
        var window = CreateWindowEx(0, className, "queued size boundary", 0x80000000,
            300, 300, 562, 178, IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        Assert(window != IntPtr.Zero);
        try
        {
            _observeClientSizeMessages = false;
            Assert(WindowsPlatformAdapter.SetNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 278).Success);
            Assert(WindowsPlatformAdapter.SetNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 176).Success);
            _staleClientSizeMessages = 0;
            _observeClientSizeMessages = true;
            Application.DoEvents();
            Assert(_staleClientSizeMessages == 0,
                $"queued size synchronization announced a stale client: mismatches={_staleClientSizeMessages}");
            Assert(WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64()).ClientBounds.Height == 176);
            _observedClientSizeMessages = 0;
            Assert(WindowsPlatformAdapter.SetNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 176).Success);
            Assert(_observedClientSizeMessages > 0,
                "same-size native correction must refresh the current client-size notification");
            Assert(_staleClientSizeMessages == 0);
        }
        finally
        {
            _observeClientSizeMessages = false;
            _ = DestroyWindow(window);
            _ = UnregisterClass(className, instance);
        }
        return Task.CompletedTask;
    }

    private static Task CenteredResizeRechecksAtRetirement()
    {
        var window = CreateWindowEx(0, "#32770", "resize retirement boundary", 0x80000000,
            300, 300, 560, 176, IntPtr.Zero, IntPtr.Zero, GetModuleHandle(null), IntPtr.Zero);
        Assert(window != IntPtr.Zero);
        try
        {
            Assert(WindowsPlatformAdapter.ParkNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 278).Success);
            var first = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(window.ToInt64());
            Assert(first.Success && first.StillPending);
            var replay = WindowsPlatformAdapter.ReplayCenteredResizeCommitForTest(window.ToInt64(), () =>
            {
                Assert(WindowsPlatformAdapter.SetNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 176).Success);
            });
            var final = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64());
            Assert(final.ClientBounds.Height == 176, "test must restore compact native client after readback");
            Assert(!replay.Success && replay.StillPending,
                $"retired success after native client changed: height={final.ClientBounds.Height}, success={replay.Success}, pending={replay.StillPending}");
            Assert(WindowsPlatformAdapter.PendingCenteredResizeForTest(window.ToInt64()).Height == 278);
            var settled = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(window.ToInt64());
            Assert(settled.Success && !settled.StillPending);
            Assert(WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64()).ClientBounds.Height == 278);
            Assert(settled.RequestedHeight == 278 && settled.ReadbackHeight == 278);
            Assert(settled.MatchesCurrentClient(WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64())));
            Assert(WindowsPlatformAdapter.SetNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 176).Success);
            Assert(!settled.MatchesCurrentClient(WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64())),
                "terminal publication cannot report settled after a later compact restoration");
        }
        finally { _ = DestroyWindow(window); }
        return Task.CompletedTask;
    }

    private static Task CenteredResizeChecksClientAfterPositionCorrection()
    {
        var instance = GetModuleHandle(null);
        var className = $"NxToolsClientResizeBoundary-{Guid.NewGuid():N}";
        var windowClass = new WindowClassEx
        {
            Size = (uint)Marshal.SizeOf<WindowClassEx>(),
            WindowProcedure = Marshal.GetFunctionPointerForDelegate(ClientChromeWindowProcedure),
            Instance = instance,
            ClassName = className
        };
        Assert(RegisterClassEx(ref windowClass) != 0);
        var window = CreateWindowEx(0, className, "client-resize boundary", 0x80000000,
            400, 300, 562, 178, IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        Assert(window != IntPtr.Zero);
        try
        {
            var original = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64());
            _restoreCompactClientOnMove = true;
            var applied = WindowsPlatformAdapter.SetNativeWindowBoundsCenteredForTest(
                window.ToInt64(), 560, 278);
            var final = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64());
            Assert(applied.Success && final.ClientBounds.Width == 560 && final.ClientBounds.Height == 278,
                $"custom chrome must reach the confirmed client after position correction: {final.ClientBounds.Width}x{final.ClientBounds.Height}");
            Assert(Math.Abs(final.Bounds.Left + final.Bounds.Right -
                original.Bounds.Left - original.Bounds.Right) <= 2, "custom client horizontal center moved");
            Assert(Math.Abs(final.Bounds.Top + final.Bounds.Bottom -
                original.Bounds.Top - original.Bounds.Bottom) <= 2, "custom client vertical center moved");
            Assert(WindowsPlatformAdapter.ParkNativeWindowBoundsCenteredForTest(window.ToInt64(), 560, 278).Success);
            _ = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(window.ToInt64());
            _restoreCompactClientOnMove = true;
            var replay = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(window.ToInt64());
            final = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(window.ToInt64());
            Assert(replay.Success && !replay.StillPending &&
                final.ClientBounds.Width == 560 && final.ClientBounds.Height == 278,
                $"recovery must settle the final client before retiring pending: {final.ClientBounds.Width}x{final.ClientBounds.Height}");
        }
        finally
        {
            _restoreCompactClientOnMove = false;
            _ = DestroyWindow(window);
            _ = UnregisterClass(className, instance);
        }
        return Task.CompletedTask;
    }

    private static Task CenteredResizeReappliesAfterNativeMoveEnd()
    {
        const uint wsOverlappedWindow = 0x00CF0000;
        const uint wsVisible = 0x10000000;
        var window = CreateWindowEx(
            0,
            "#32770",
            "NxTools deferred-resize test",
            wsOverlappedWindow | wsVisible,
            120,
            140,
            500,
            360,
            IntPtr.Zero,
            IntPtr.Zero,
            GetModuleHandle(null),
            IntPtr.Zero);
        Assert(window != IntPtr.Zero);
        try
        {
            Assert(WindowsPlatformAdapter.ParkNativeWindowBoundsCenteredForTest(
                window.ToInt64(), 560, 278).Success);
            Assert(WindowsPlatformAdapter.SetNativeWindowPositionForTest(
                window.ToInt64(), 430, 320).Success);
            var latest = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(
                window.ToInt64());
            Assert(latest.Exists && latest.Visible);

            var firstReplay = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(window.ToInt64());
            Assert(firstReplay.HadPendingResize);
            Assert(firstReplay.WindowHandle == window.ToInt64());
            Assert(firstReplay.CorrelationId == Guid.Empty);
            Assert(firstReplay.Success && firstReplay.StillPending);
            Assert(firstReplay.AttemptCount == 1);

            var replay = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(window.ToInt64());
            Assert(replay.HadPendingResize);
            Assert(replay.WindowHandle == window.ToInt64());
            Assert(replay.CorrelationId == Guid.Empty);
            Assert(replay.Success && !replay.StillPending);
            Assert(replay.AttemptCount == 2);
            var resized = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(
                window.ToInt64());
            Assert(
                resized.ClientBounds.Width == 560 && resized.ClientBounds.Height == 278,
                $"client={resized.ClientBounds.Width}x{resized.ClientBounds.Height}, visible={resized.Bounds.Width}x{resized.Bounds.Height}");
            Assert(Math.Abs(
                resized.Bounds.Left + resized.Bounds.Right -
                latest.Bounds.Left - latest.Bounds.Right) <= 2);
            Assert(
                Math.Abs(resized.Bounds.Top + resized.Bounds.Bottom - latest.Bounds.Top - latest.Bounds.Bottom) <= 2,
                $"visible vertical center shifted: before={latest.Bounds.Top + latest.Bounds.Bottom}, after={resized.Bounds.Top + resized.Bounds.Bottom}, client={resized.ClientBounds.Width}x{resized.ClientBounds.Height}");
        }
        finally
        {
            _ = DestroyWindow(window);
        }
        return Task.CompletedTask;
    }
    private static Task CenteredResizeRetriesWhenStillPending()
    {
        var recovery = new CenteredResizeRecovery();
        var root = new IntPtr(123);
        var correlationId = Guid.NewGuid();

        var firstReplay = Replay(correlationId, attempt: 1, stillPending: true);
        Assert(recovery.OnMoveSizeEnd(root, firstReplay) ==
            CenteredResizeRecoveryDecision.Retry);
        Assert(recovery.AttemptsFor(root) == 1);
        Assert(recovery.Observe(root, Replay(correlationId, attempt: 2, stillPending: true)) ==
            CenteredResizeRecoveryDecision.Retry);
        Assert(recovery.AttemptsFor(root) == 2);
        Assert(recovery.Observe(root, Replay(correlationId, attempt: 3, stillPending: false)) ==
            CenteredResizeRecoveryDecision.Completed);
        Assert(!recovery.HasPending);

        Assert(recovery.OnMoveSizeEnd(root, firstReplay) == CenteredResizeRecoveryDecision.Retry);
        var decision = CenteredResizeRecoveryDecision.Retry;
        for (var attempt = 2; attempt <= CenteredResizeRecovery.MaximumAttempts; attempt++)
        {
            decision = recovery.Observe(
                root,
                Replay(correlationId, attempt, stillPending: true));
        }
        Assert(decision == CenteredResizeRecoveryDecision.Exhausted);
        Assert(!recovery.HasPending);

        var replacementCorrelationId = Guid.NewGuid();
        Assert(recovery.OnMoveSizeEnd(root, firstReplay) == CenteredResizeRecoveryDecision.Retry);
        var replacementReplay = Replay(replacementCorrelationId, attempt: 1, stillPending: true);
        Assert(recovery.Observe(root, replacementReplay) == CenteredResizeRecoveryDecision.Retry);
        Assert(recovery.AttemptsFor(root) == 1);
        return Task.CompletedTask;

        static CenteredResizeReplayResult Replay(
            Guid correlationId,
            int attempt,
            bool stillPending) => new(
                HadPendingResize: true,
                CorrelationId: correlationId,
                WindowHandle: 321,
                StartedAtTimestamp: 1,
                AttemptCount: attempt,
                Success: true,
                StillPending: stillPending,
                NativeErrorCode: null);

    }


    private static Task CenteredResizeParksClientCorrectionDuringMove()
    {
        var repositoryRoot = FindRepositoryRoot();
        var clientResize = File.ReadAllText(Path.Combine(
            repositoryRoot,
            "csharp",
            "NxTools.PlatformHost",
            "Windows",
            "WindowsPlatformAdapter.ClientResize.cs"));
        var adapter = File.ReadAllText(Path.Combine(
            repositoryRoot,
            "csharp",
            "NxTools.PlatformHost",
            "Windows",
            "WindowsPlatformAdapter.cs"));
        var moving = clientResize.IndexOf("NativeMethods.IsDraggingOrSizing(window)", StringComparison.Ordinal);
        var park = clientResize.IndexOf("NativeMethods.ParkCenteredResizeForRecovery(", moving, StringComparison.Ordinal);
        Assert(moving >= 0 && park > moving);
        Assert(clientResize.Contains("Guid correlationId", StringComparison.Ordinal));
        Assert(adapter.Contains("pending.CorrelationId", StringComparison.Ordinal));
        Assert(adapter.Contains("pending.Width, pending.Height, pending.CorrelationId", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task CenteredNativeResizeUsesMovedWindowBounds()
    {
        const uint wsOverlappedWindow = 0x00CF0000;
        const uint wsVisible = 0x10000000;
        var window = CreateWindowEx(
            0,
            "#32770",
            "NxTools centered-resize test",
            wsOverlappedWindow | wsVisible,
            120,
            140,
            500,
            360,
            IntPtr.Zero,
            IntPtr.Zero,
            GetModuleHandle(null),
            IntPtr.Zero);
        Assert(window != IntPtr.Zero);
        try
        {
            var moved = WindowsPlatformAdapter.SetNativeWindowPositionForTest(
                window.ToInt64(),
                430,
                320);
            Assert(moved.Success);
            var previous = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(
                window.ToInt64());
            Assert(previous.Exists && previous.Visible);
            Assert(previous.Bounds.Width > 0 && previous.Bounds.Height > 0);

            foreach (var (width, height) in new[]
            {
                (440, 184), // progress
                (440, 196), // failed
                (440, 180)  // completed
            })
            {
                var result = WindowsPlatformAdapter.SetNativeWindowBoundsCenteredForTest(
                    window.ToInt64(),
                    width,
                    height);
                Assert(result.Success);
                var current = WindowsPlatformAdapter.CaptureNativeWindowPresentationForTest(
                    window.ToInt64());
                Assert(current.Exists && current.Visible);
                Assert(
                    current.ClientBounds.Width == width && current.ClientBounds.Height == height,
                    $"target={width}x{height}, client={current.ClientBounds.Width}x{current.ClientBounds.Height}, visible={current.Bounds.Width}x{current.Bounds.Height}");
                Assert(
                    Math.Abs((current.Bounds.Left + current.Bounds.Right) -
                        (previous.Bounds.Left + previous.Bounds.Right)) <= 2,
                    $"visible horizontal center shifted: before={previous.Bounds.Left + previous.Bounds.Right}, after={current.Bounds.Left + current.Bounds.Right}");
                Assert(
                    Math.Abs((current.Bounds.Top + current.Bounds.Bottom) -
                        (previous.Bounds.Top + previous.Bounds.Bottom)) <= 2,
                    $"visible vertical center shifted: before={previous.Bounds.Top + previous.Bounds.Bottom}, after={current.Bounds.Top + current.Bounds.Bottom}, client={current.ClientBounds.Width}x{current.ClientBounds.Height}");
                previous = current;
            }
        }
        finally
        {
            _ = DestroyWindow(window);
        }
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
    private static Task ActiveImageRewriteKeepsOnlyCanonicalImageFormats()
    {
        var formats = new List<CapturedClipboardNativeFormat>
        {
            new(49_201, "PNG", [1], 0),
            new(49_161, "DataObject", [2], 1),
            new(50_080, "PixPinData", [3], 2),
            new(8, "", [4], 3),
            new(17, "", [5], 4),
            new(49_171, "Ole Private Data", [6], 5),
            new(50_123, "CustomImageFormat", [7], 6)
        };

        ClipboardImageCompatibility.ReplaceWithCanonicalImageFormats(
            formats,
            pngFormatId: 49_201,
            pngData: [8, 9],
            imagePath: @"C:\images\capture.png",
            dibV5Data: [10, 11],
            dibData: [12, 13]);

        Assert(formats.Select(format => format.FormatId).SequenceEqual([17u, 8u, 49_201u, 15u, 50_123u]));
        Assert(formats.All(format => format.Data.Length > 0));
        return Task.CompletedTask;
    }
    private static Task BrowserImageMetadataFormatsForceCanonicalRewrite()
    {
        var formats = new List<CapturedClipboardNativeFormat>
        {
            new(49_201, "PNG", [1], 0),
            new(8, "", [2], 1),
            new(17, "", [3], 2),
            new(15, "", [4], 3),
            new(49_202, "HTML Format", [5], 4),
            new(49_203, "text/html", [6], 5),
            new(49_204, "UniformResourceLocatorW", [7], 6),
            new(49_205, "text/uri-list", [8], 7),
            new(49_206, "text/x-moz-url", [9], 8),
            new(49_207, "Chromium internal source URL", [10], 9),
            new(49_208, "Chromium internal source image", [11], 10),
            new(49_209, "Chromium Web Custom MIME Data Format", [12], 11)
        };

        Assert(ClipboardImageCompatibility.RequiresCanonicalRewrite(
            hasImage: true,
            hasText: false,
            fileCount: 0,
            formats));

        ClipboardImageCompatibility.ReplaceWithCanonicalImageFormats(
            formats,
            pngFormatId: 49_201,
            pngData: [8, 9],
            imagePath: @"C:\images\capture.png",
            dibV5Data: [10, 11],
            dibData: [12, 13]);

        Assert(formats.Select(format => format.FormatId).SequenceEqual([17u, 8u, 49_201u, 15u]));
        Assert(!formats.Any(format =>
            format.FormatName is "HTML Format" or "text/html" or
                "UniformResourceLocatorW" or "text/uri-list" or "text/x-moz-url" or
                "Chromium internal source URL" or "Chromium internal source image" or
                "Chromium Web Custom MIME Data Format"));
        return Task.CompletedTask;
    }
    private static Task PrivateImageFormatsForceCanonicalRewrite()
    {
        var formats = new List<CapturedClipboardNativeFormat>
        {
            new(49_201, "PNG", [1], 0),
            new(8, "", [2], 1),
            new(17, "", [3], 2),
            new(15, "", [4], 3),
            new(50_080, "PixPinData", [5], 4)
        };

        Assert(ClipboardImageCompatibility.RequiresCanonicalRewrite(
            hasImage: true,
            hasText: false,
            fileCount: 0,
            formats));
        formats.RemoveAt(formats.Count - 1);
        Assert(!ClipboardImageCompatibility.RequiresCanonicalRewrite(
            hasImage: true,
            hasText: false,
            fileCount: 0,
            formats));
        return Task.CompletedTask;
    }
    private static Task CanonicalImageFormatsUseStandardHeaders()
    {
        using var bitmap = new System.Drawing.Bitmap(3, 2);
        using var stream = new MemoryStream();
        bitmap.Save(stream, System.Drawing.Imaging.ImageFormat.Png);

        Assert(ClipboardImageCompatibility.TryCreateCanonicalBitmapFormats(
            stream.ToArray(),
            out var dibV5,
            out var dib));
        Assert(dibV5 is not null && dib is not null);
        Assert(BitConverter.ToUInt32(dibV5!, 0) == 124);
        Assert(BitConverter.ToInt32(dibV5!, 4) == 3);
        Assert(BitConverter.ToInt32(dibV5!, 8) == -2);
        Assert(BitConverter.ToUInt16(dibV5!, 14) == 32);
        Assert(BitConverter.ToUInt32(dib!, 0) == 40);
        Assert(BitConverter.ToInt32(dib!, 8) == -2);
        Assert(dibV5!.Length == 124 + 3 * 2 * 4);
        Assert(dib!.Length == 40 + 3 * 2 * 4);
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
        foreach (var application in new[] { "notepad.exe", "ms-settings:" })
        {
            Assert(ShellTarget.TryCreate(application, "system_tool", out var target, out _));
            Assert(target.Kind == ShellTargetKind.SystemTool);
            var launched = target.LaunchCaptured(_ => { }, start =>
            {
                Assert(start.FileName == application && start.UseShellExecute && start.Verb.Length == 0);
                return null;
            });
            Assert(launched.Success);
        }
        Assert(!ShellTarget.TryCreate("ms-settings:display", "system_tool", out _, out _));
        Assert(!ShellTarget.TryCreate("ms-settings:", "run_elevated", out _, out _));
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
        var work = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"nxtools-extension-out-{Guid.NewGuid():N}");
        try
        {
            System.IO.Directory.CreateDirectory(work);
            var crx = BrowserExtensionPackageService.ExportWithSigningKey(
                BrowserExtensionPackageService.ChromeFormat,
                System.IO.Path.Combine(work, "ext.crx"),
                source, System.IO.Path.Combine(work, "fixture-signing.pem"));
            var crxBytes = System.IO.File.ReadAllBytes(crx);
            Assert(crxBytes.Length > 4);
            Assert(crxBytes[0] == (byte)'C' && crxBytes[1] == (byte)'r');
            Assert(crxBytes[2] == (byte)'2' && crxBytes[3] == (byte)'4');
            Assert(BitConverter.ToUInt32(crxBytes, 4) == 3);

            var xpi = BrowserExtensionPackageService.ExportWithSigningKey(
                BrowserExtensionPackageService.FirefoxFormat,
                System.IO.Path.Combine(work, "ext.xpi"),
                source, System.IO.Path.Combine(work, "fixture-signing.pem"));
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
            if (System.IO.Directory.Exists(work)) System.IO.Directory.Delete(work, true);
            System.IO.Directory.Delete(source, true);
        }
        return Task.CompletedTask;
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }
    }

    private static void Assert(bool condition)
    {
        Assert(condition, "assertion failed");
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

    private static Task KnownFolderLocalizedAliasesCoverUsersRoot()
    {
        var profileDirectory = Environment.GetFolderPath(
            Environment.SpecialFolder.UserProfile);
        var usersRoot = Directory.GetParent(profileDirectory)?.FullName ??
            throw new InvalidOperationException(
                "The current user profile has no Users parent directory.");
        var localizedName =
            ThirdPartyFileManagerDisplayPath.GetLocalizedFolderDisplayName(
                usersRoot);
        Assert(!string.IsNullOrWhiteSpace(localizedName));

        var aliases = ThirdPartyFileManagerDisplayPath.KnownFolderDisplayAliases();
        Assert(aliases.TryGetValue(localizedName!, out var resolved));
        Assert(string.Equals(
            resolved,
            usersRoot,
            StringComparison.OrdinalIgnoreCase));

        var root = Path.GetPathRoot(usersRoot) ??
            throw new InvalidOperationException("The Users root has no drive root.");
        var canonicalized = ThirdPartyFileManagerDisplayPath.Canonicalize(
            Path.Join(root, localizedName!),
            Directory.Exists,
            aliases);
        Assert(string.Equals(
            canonicalized,
            usersRoot,
            StringComparison.OrdinalIgnoreCase));
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
        public string CurrentDirectory { get; set; } = currentDirectory;
        public bool RequiresPostNavigationActivation { get; init; } = true;
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
                ? new ThirdPartyFileManagerCapture(CurrentDirectory, ++_nextPaneState)
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
