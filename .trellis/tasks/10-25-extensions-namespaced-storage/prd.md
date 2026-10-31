# P3 — Namespaced persistent storage

## Authority and scope
Approved parent R3/A3, design section4 and canonical implementation P3; user final开始2026-10-25, P0/P1/P2 source completed. No new product choices/nativeUI/userstate/deployment/service/commit/archive/delegation authority. Independent CoreArchive88/1integrationfailure retained, do not take over unrelated owners. Storage is unencrypted extension data, not CredentialManager/OSsandbox/rawfilesystem access.

## Requirements
- Explicittransport2 methodversion1 storage.get/set/delete/list; namespace hostresolved from installation/dataUUID, never packageID/path parameter. Key<=128UTF8bytes/controlfree; valueJSON<=64KiB/depth16; <=256keys/4MiBserializednamespace. Global128MiBlogical includesorphans,1024metadata slots includingprereservedorphanrecords. No silent eviction.
- One serializedboundedIOowner(queue32/globalblocking<=4), no render/async-thread fileIO. Original lock/read/sync-temp-close-rename pattern, exactalreadylockedfs2=0.4.3 allowed; root injected underappdata outsideimmutablecodestaging, canonicalopaque names/regularno-reparse checks. Globalquota/admissionlock before namespace lock. Boundreadbeforeparse; corruptretained/error, never emptyreset. <=16metadata cache if cachingneeded; physicaltemporarycopy reserve explicitone4MiBnamespace beforewrite.
- get returns found/value/revision, JSONnull distinctmissing. set/delete atomicrevision and optionalexpected_revision CAS; noautomaticretry/multikeytransaction. list<=32key+revisionmetadata/no values/opaque60scurrentnamespace-revisioncursor, mutationretirescursor, boundedresult. Strictunknown/duplicate fields andcurrentfullidentity/trust/cancel/reply gates.
- OptionalomitteddataUUID allocated lazily once throughoriginalCorelockedtransaction forstorageusingpackage; oldconfigload/default/codehash unchanged. Explicitinstalledreplacement preservesdataUUID separately from existingresetsettings/trust semantics. SameIDreimport freshUUID/noauto-inherit.
- Uninstall preparesorphanmetadata durably BEFORECorecheckedremove; failed/stale/cancelledconfigcommit performsnoerase. Retirelogicalaccess then drainactualleases. Defaultretainorphan; optionalclearcheckboxunchecked. Clearfailure leavesregistryremoved/data cleanup_pending; nofalserollback/secureerasepromise. Reconcile recordedpointers againstfreshlockedregistry: currentUUIDactive/absentUUIDorphan, ambiguityretains/no deletion/registryresurrection.
- ExistingSettingsretainedmetadata andofficialrestore/clearconfirmations. Restore onlycurrenttrustedinstalledtarget/emptydestination/reviewedmetadata/exactleasequiescence; no merge/overwrite. Pointerchanges underoriginalCorelock afterfreshreview; failure retainscurrentpointer/data. Explicitrecovery/retry currentidentity gated, noauto effectreplay. Credentialstore untouched.
- Diagnostics correlation/run acrossaccepted/start/state/terminal/elapsed/fixednativecodeatowner; counts/bytes only, no keys/values/paths/secrets. Boundednonblockinglogging neverchangesoutcome.

## Acceptance
- [x] Red-capable actualtempfile/lockedwriter/reopen/revisionCAS/quota/missingnull/listcursor/depth/UTF8/corrupt/renamefailure tests before implementation, no productionuserpaths.
- [x] OriginalCorelazyUUID/upgradepreserve/sameIDreimportfresh/defaultloadnorewrite/stalecheckedremove/failedcommitnoerase, realpersistencebarriers before/aftercommit/reconcile/clearfailure.
- [x] Runtimeactualtrustedstagedscriptstorage/getsetdelete/list andfullcurrentidentity/cancel/noreplay; independentnamespace/crossrunpersistence/globalquota/cursors/actualIOleases.
- [x] ExistingofficialDialog/Checkbox/Settings restoreclear andrealGPUI SaveCancel/stalereview/bounds/currentowner; no newwindow/nativeUI. Registryremoved-butcleanup_pending truthful.
- [x] SDK/schema/docs/discovery/mintransport2/diagnosticparity; focusedExtensions/Corelocked/GPUI/strictlint/workspace/staticgates. FullDesktopP8, priorCoreArchivefailure retained/nativeUIinstalledserviceNOTRUN.

Source evidence and exact acceptance limits: [outcomes.md](outcomes.md). This checklist does not certify deferred native/visual/installed or P8 whole-product acceptance.

## Change boundary
Extensions registry optionaldataidentity/newstorageowner/runtime/API/SDK; Coreoriginalextensions lockedtransactions; DesktopExtensionServiceconfigroot/publication/removal/settingsdataactions only. No ordinarysettingspreservationchange,no credentialcleanup,rawpath APIs,filesystemwalk/Registry/service/network/Clipboard effects. Preserve peerArchive/Downloads/browser/notificationwork and currentdirty files; onlyuniquelyanchorededits/freshbaselines, neverrestore.
