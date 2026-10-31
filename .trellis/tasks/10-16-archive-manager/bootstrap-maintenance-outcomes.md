# Same-key bootstrap maintenance — implementation / package outcomes

Date2026-10-23. User explicitly approved root-cause repair plus a distinct administrator maintenance
installer, not execution on this workstation. Authorization recorded before implementation in
[plan](implement.md#approved-bootstrap-maintenance-repair--2026-10-23). Same Phase F; not whole archive/
PhaseF/first-release acceptance. [Checkpoint](phase-a-checkpoint.md) remains the sole current status.

## Root cause and delivered path

Actual installed bootstrapSHA5BCD8D28...3BEDAE acceptsseq21 and rejectsnewarchivefile set with
`Unapproved release path.`; new verifier with SAME old SPKI acceptsnewpackage. EarlierF-RC01/02
repairs did not fix this compatibility gap. The installed21snapshot/core files restored intact.

New explicit command `--maintain-trusted-bootstrap` is only invoked by the separate
`installer/nxtools-maintenance.iss` (PrivilegesRequired=admin), never the normal installer.
`TrustedBootstrapMaintenanceWindows` validates fixed existingNxToolsUpdateV1 LocalSystem/path/
registeredhash/key/protected directories, verifies complete signed candidate package/self image,
then invokes `TrustedBootstrapMaintenance` real-file transaction. No missing-service installation,
key rotation, ledger reset, ordinary-app elevation or silent bootstrap replacement.

Transaction: exact same enrolledSPKI/currentbootstrap identity, strictly newer release than product
and prior maintenance floor; verify active snapshot bound to durable sequence/ID/manifestSHA;
copy candidate under stable source lease; durable journal before mutation; stopservice then original
deploymentgate; exact oldbootstrap backup/newreplacement/registeredhash+enrollmentupdate; start and
read-only authenticated pipe readiness. Shared maintenance admission file prevents new Broker business
updates during ready/commit/cleanup. Precommit failure restores old bootstrap/enrollment/hash/service;
interruption retains journal for next maintenance; committed version permits cleanup-only retry.
Original Releases/state.json/snapshots remain unchanged by maintenance. Before replacement, a racing
old broker state change safely aborts preparation/restartsoldservice without rewinding business state.

`TrustedReleaseManifest.VerifyRecoverySnapshot` authenticates signature first, then matches all3durable
identity fields before accepting historical requiredmembers. RecoveryOnly verifiedresults cannot be
new-request admission; normal VerifyPackage/SignPackage remain strict. Original transaction passes
verifiedoldsnapshot to originalNativeDeployment; oldsnapshot's signed deploymentchild restoresoldcore.
Then exact-hash/handlecleanup removes failed-new archive-onlymembers absentfromoldset; foreignbytes
refusecleanup. Matching failed-newarchiveapproval is withdrawn only for legacy-oldset. No unchecked
path deletion/substituted source/second deployment service.

## Actual tests and review

Test evidence:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\bootstrap-maintenance-20261023-130003`.

- Initial maintenance stub actualRED:selected1/run1/pass0/fail1 (06-red-run.log); initial unusedparameter
  compileerrors retained separately, not behavioralred.
- Maintenance success, readinessfailure rollback/retry, interruptedrollback recovery, wrongtrust/
  downgrade/tamperedcurrent/candidate rejection:4fixtures realGUIDfiles+ephemeralRSA/fakeservice.
- Legacyrollback/strictretry,restart/exactidentity/artifactreject,recovery-nevernewadmission and
  foreignarchivecleanup:5fixtures plus original transaction/signature/enrollment/routing coverage.
- Final **LIST21=RUN21=PASS21**,skip/fail/not_run0;27-list.log/28-run.log. FreshC#build0warnings/errors.
  Uses real file compensation/deletion boundary, not SCM/Registry/product operations.
- Read-only oldactualsequence21active and currentstrictpackage verifyPASS with Debug verifier;
  FINALRELEASEverifier repeated PASS in37-final-offline-run.log. NoMain/service/deploycalled.
- One optional bounded read-only review found: committedcleanup/state race, archive-onlyrollback
  residues, stop-timeout mismatch. Fixed by shared admission lock + committedcleanupnotbindingold
  businessstate; exactfailedsigned-setcleanup; stopbound4m15s vs originalBroker4m. Archive residue
  regression recordedRED22-cleanup-red-run.log (afterrealarchivewrites), thenGREEN25/28.
- Foreign cleanup refuses exactwronghash; maintenance readiness fixture confirms normalupdatecannot
  takeexclusiveadmissionlock. No further review cycle. No fullDesktop/codec/split/100k/performance run.
- bootstrap-maintenance,PlatformHostselection,browser-signing-isolation,Innolifecycle/spec/diff gatesPASS.
  Prior green attempts with test-only hashcase errors retained (09/12); not rewritten asallgreen.
- Recoveryimplementation delegate did not runredbeforeitschange; testswrittenbeforeimplementation but
  no actualredclaim for those4fixtures. Parent ranfinaltests and re-used actualoldbootstrapoffline red.

## Final signed release24 and separate installers

Evidence:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\release-maintenance24-20261023-134040`.
OriginalkeySPKISHA03e979d5...083b69,sequence24,reservedunderoriginalledger (noreset/reuse).
ReleaseID`cbccee5e-f1ef-49df-95b5-69ca9541132d`.
Originalpackagewrapper Release-SkipVerification then separateInnoMaintenancecompile; no runtimeinstall.
Headroom18.88GiB≥6GiB; serialjobs1/incremental0/codegen256. Source929filesunchangedduringbuild.

IndependentRSA3072-PSS/SHA256verificationPASS77manifestfiles/10outputs,helperactualDesktopbinding,
5archiveapprovals/hostembeddedpublicanchor. ActualpublishedDLLofflinechecksoldlegacyrecovery+newstrict
packagePASS. Installer5scenarios/BrowserHost5scenariosPASS,notrealinstalledmaintenance.

| File | Bytes | SHA256 |
| --- | ---: | --- |
| NxTools.Next-maintenance-0.7.0.exe | 23845757 | 8EF94858DD0DE7829F2E1A6A9C7ED50043F40F0107545E3932771929AD112BA6 |
| NxTools.Next-setup-0.7.0.exe | 23243171 | 4B80EE8938475FCC4C3E4020A8E593BC24AB1F818613F72B70B273F2F0908CED |

BothAuthenticodeNotSigned; innerrelease signed by originalkey. Seq23artifactbackup verified before
intentionalnewpackageoverwrite. No unrelatedsourcechange/reset/commit/push. Historicalnegative/
acceptancereports remainunchanged; only plan/releaseoperations/currentcheckpoint/newoutcome and
in-scope C#/tests/maintenanceInno/gate/artifactmetadata modified. Originalsourcebaselinecapturedafter
firstteststub/authorization, not claimedpristinetaskopening.

## User action and remaining evidence

Run MAINTENANCE installer first, allow explicitadministratorprompt, waitforexplicitsuccess;
then matching NORMAL installer. Maintenanceonlyupdatesupdater,notbusinessreleaseorDesktoplaunch.
Ifanyfailure,stop/retainlogs;donotdeleteProgramData/recoveryfilesorreplacebinariesmanually.

Runtime maintenance NOTRUN: actual bootstraphashremains5BCD...3BEDAE,updaterservicePID5420,
indexerservicePID32360 unchangedbefore/after. RealSCM/admin/reboot/crash/rollback and subsequent
installedupdate require userconfirmation/execution; doNOTclaim现场fixedbeforethat. NoarchiveRegistry/
COM/P208drag/liveGUI/Indexerservicereleasegate; priorA–E/JOB07/SAFE08/userdebts retained.
