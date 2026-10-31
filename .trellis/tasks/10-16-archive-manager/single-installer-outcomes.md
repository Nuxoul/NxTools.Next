# Single installer migration — sequence25 outcomes

Date2026-10-23. User rejects two-package maintenance steps and authorizes one installer with detected,
explained, explicit one-time administrator migration then continued installation. No runtime install/
maintenance authorized or executed in this development slice. Same PhaseF, no fullreleaseclaim.

## Smallest change

- `installer/bootstrap-migration.ps1`: read-only fixedNxToolsUpdateV1 registration/imagehash/SPKI /
  ProtocolVersion1 marker decision. Absent/compatible skip; approvedold needsmigration; unknown /
  tampered identity rejects. Actualmachine read-only detection exit10 confirmsoldservicecase.
- `installer/nxtools-next.iss`: before old-file suspension, run detection; require explicit explanation
  and confirmation; verify compiledSHA of embeddedZIP then extract into self-ownedInnotmp. Existing
  preparationShellExec runas invokes maintenance+filepreparation in one process, not two UAC launches.
  No migration when not needed, no file replacement on cancel/detection/maintenancefailure.
- Original `close-owned-processes.ps1` adds Invoke-UpgradeBootstrapMigration before processstop/suspend;
  calls original signed maintenance owner, not a replacement updater. Original rollback/readyguard kept.
- Original maintenance version record adds ProtocolVersion1; originalsamekey/sequence/rollback remains.
- Originalpackagewrapper builds internalZIP from finalsigneddirectory and emits hash/compilerinput;
  onepublicnormalinstaller. No separate maintenanceexe in currentdelivery; previouspairbackedup.

## Evidence

Tests:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\single-installer-20261023-135813`.
NewdetectiontestfirstRED atmissingnormalmigrationentry (01-red.log),thengreen10assertions including
actualoriginalpreparationfunctioncallusingownedfakeexe(success/failure),no SCM/Registrywrite.
C#LIST21=RUN21=PASS21,skip/fail/not_run0; build0warnings/errors. Preparation100,ownedset632,keycleanup20
PASS. RuntimeInnofixtures use actualcallbacks/detectionprocess/ZIPhash/extraction but fake registration/
maintenance and test-only runas/confirmation adapters; NEVERrequest realUAC or maintenanceproduct.
Migration success/cancel/failure eachPASS; original5runtime and5BrowserHostreplacement scenariosPASS
again during packaging. Cancel/failureassertnoapplicationmarkerreplacement; successtestexactly1
migration elevation-request event and continuation. OldBrowserHost-onlyscenarios stubnewpreflight
because they test the original file owner, not newmigration. PowerShellparse/source/lifecycle/spec/
diffgatesPASS. No additionaldelegate/reviewloop/fullDesktop/codec/performance matrix.

Package:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\release-single25-20261023-140952`.
Originalsignedsequence25ID4860186d-21b4-41ca-9a0f-cf004ae59c01,originalSPKI03e979d5...083b69.
Normalinstaller46393858bytes SHA1CF5FCCC2ACD62B96A6128018642A87C2D21DA07BED9BC206355E21C663D07E5.
AuthenticodeNotSigned; originalRSA3072-PSS/SHA256fullmanifestverified77files/10outputs. Internal79-file
ZIP exactbytesmatchfinalsigneddirectory, SHA6095C4031F659AB8A6E6AF085236208909A352E0C76002DE830233E05C4A1990.
Only1publicexe,currentinstaller. Build930sourcebaselineunchanged,headroom19.78/19.73GiB>=6,serial
jobs1/incremental0/codegen256. Seq24pairbackedup; oldstandalonemaintenanceexe/sidecarremovedonlyfrom
currentdeliveryafterbackup. Noinstalledservicechange; updaterPID5420/indexerPID32360 andbootstrap
5BCD8D28...3BEDAE unchanged. No privatekey/regeneration/ledgerreset/commit/push.

## User / remaining

Run only NxTools.Next-setup-0.7.0.exe. Legacycomponentmigration is explained and requests explicit
Windowsadministratorapproval; aftersuccessinstallercontinues. Do not ask usertorunmaintenanceexe.
ActualUAC/SCM/processcapture/installedupgradeNOTRUN; passingfakeeffectsdoesnotcertifyonsitefinish.
Onfailurestopandretainlogs. ExistingarchiveRegistryApply/COM/P208drag/A–E/JOB07/SAFE08 debts unchanged.
