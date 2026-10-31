# P2 — Progress and bounded blob transport outcomes

Status: **source_completed_acceptance_pending**, 2026-10-26. Parent implementation approval retained; no commit/archive/package/install/product launch/service/nativeUI/userstate/delegation performed. This is the canonical P2 slice, not completion of all seven capabilities.

## Delivered source

- Explicit protocol2 once temporary blob methods `blobs.begin/seal/read/release`, methodversion1/minimum_transport2. Legacy23methods, v1/once canonical hash and ordinary1MiBinput/4MiBstdout/256KiBframe including LF remain. Four new methods give27catalog entries. Runtime merges original adapter metadata and truthfully reports transport/data/progress effective limits in initialize/host.describe; v1 callers get unsupported_transport. No future methods advertised.
- One original runtime/broker reader, <=8host futures and4data IO jobs. Host-issued opaque transfer registered before observable reply; <=64KiBdecoded blocks/one ACK outstanding/strict session+run/sequence/offset/length/base64/digest; data budgets16MiBdecoded/24MiBwire independently eachdirection,512combinedchunks/4cumulative transfers. Completion requires quiescence, complete, request_finished and original actualnativecleanup.
- GlobalBlobStore/HostContext invocation lease/retirementguard. Perblob8MiB/live16MiBperrun/global64MiB/4slotsperrun/120s; global4IO/256backing slots bound even zero-byte work. Global AND perrun byte/slot permits survive blockingIO, cancellation and failed physicaldelete. Last backing drop closes owned file and transfers reservations to one signaled cleanup owner. Only exactmanaged regular/nonreparse directchildren are cleaned. Faileddelete retains quota/retries; no replacement worker/timeout-as-cleanup. Atomic sealed/inflight quiescence avoids blocking async/UI on filemutex.
- Original authenticated Rust/C#Start optional data_plane=2, omittedlegacy unchanged. Nativeouter25MiBinput/28MiBstdout, unchanged256KiBframe/stderr64KiB/Job/token/suspendedadmission/originalrunningcapacity and actualpump/treecleanup. V2stdout bounded async publication uses original32queue/2sdeadline; Rust originalreader bounded250msbackpressure cancels exactslowrun. No secondreceiver/unboundedbuffer.
- Strict progress currentidentity/increasingsequence/known integer<=2^53-1 orindeterminate/text512UTF8bytes/256frames. Latest boundedwatch publishes<=4Hz, bursts coalesce without per-tick replies/logs; terminal removes UI-onlytext immediately;100percent neverresult/deadlinerefresh. Original manual/activity generation/package/handler/hash/correlation projection renders officialKit0.8.0Progress value/loading XSmall4DIP. No newpopup/notificationclock/native/focusowner/vendorchange.
- Dependency-free Python/Node/TS helpers progress/blob upload/read/release, exactbytes/hash/ACK checks, one receiver, Python synchronous and Node bounded8accepted serialoperations, noautomatic effectretry/v1negotiation. Original finalflush/stdinexit ownership retained. Schemas/authoring/protocol synchronized. New exactlockedbase640.22.1 directedge only; P1regex1.13.1 unchanged, peerlock changes preserved.
- Metadata-only blob accepted/firstcleanup_pending/actualterminal correlation/run/count/elapsed diagnostics. No path/blobtoken/childoutput/chunk/progress payloadlogs. Nativeerrorcode remains nativeowner.

## Red-capable defects and corrections

1. Original >1MiB realbroker regression failed Protocol at unsupportedblobs.begin; real exact1048576+19bytes and later two8MiB/512chunks now pass without raisinglegacyframe/controlbudgets.
2. Lastpublicstore drop leaked realbacking (3s timeout). Single retained cleanupowner fixed; Windows heldno-delete handle retains permits until actualrelease; blockedspawn_blockingIO proves taskabort is not cleanup and lastlease triggers latecleanup.
3. Real256progressburst failed beforelatestvalue (firsttimeout, then explicit NativeUnavailable0.35s). Originalbounded v2stdout pump/readerbackpressure fixed; no enlargedunboundedqueue. Later burst test passes and observes100percent while processstillrunning, thenerrorcompletion unchanged.
4. Desktop firstcheck lacked InteractiveElement trait for debug_selector; fixedimport, actualGPUI4DIPtest passes. WireInvocation enumgrowth fixed by boxing bothprivatevariants, no warningallow/protocolchange.
5. Latestnative fixturegroup initially RUN7/PASS6/FAIL1/NOTRUN3: StuckAdmission checkeddiagnostics immediatelyafterphysicalDrainAsync. Source proved slotrelease precedes exited/diagnosticcallback. Fixture now boundedwaits4actualexited+4cancelleddiagnostics before unchangedprivacy/assertions; no nativecleanupsemanticchange. Rebuilt and exactsame10group passes. Historicallog retained.
6. New8hosteffect ACKbarrier test initially failedtestcompilation because Notify->Barrier conversion leftoldconstruction; correctedtestonly, finalfullExtensions green.

## Measured verification

All logs under session scratch `70da3267-15fc-4e10-b17c-2d9c0bf62e48`, prefix `extensions-expansion-p2-`. Exit75 busy-preflight attempts are NOT RUN evidence; no peerpause/processkill/reservation. Every launchedCargo uses jobs1/incremental0/codegen-units256/fulltimeout.

| Check | Actual result / log |
| --- | --- |
| Full Extensions `cargo test -p nxtools-extensions --features test-support -- --test-threads=1` | **96PASS/0FAIL/0ignored**,0doc-tests,17resultgroups summed; `extensions-converged.log` |
| Latest preceding lifecycle/resource/session/process group |33PASS (2+6+11+14); `latest-boundaries.log`; finalfull adds requiredtransport+8effectACK test |
| Actual v2 process suite in finalfull |15PASS incl>1MiB,two8MiB/512chunks,8waitinghosteffects+ACK,malformedchunks,5sACKdeadline,zero/incomplete release,fullcancelrevoke/retainedblobstale,noeffectreplay,progressdeadline/burst,currentidentity,Python/NodeSDK |
| Original manual UI group `cargo test -p nxtools-desktop extensions::manual -- --test-threads=1` |**12PASS/0FAIL/0ignored**,1236filtered; `ui-manual.log`; actualscript->service->manualprojection->4DIPpaint/knownunknown/currentgeneration/terminal/footer and retainedcancel/result/notificationtests |
| C#build tests --no-restore -m:1 |0warnings/0errors, `native-race-build.log` |
| C#exact `--fixtures --list 'extension process fixture'`, then samefilter |LIST10/RUN0, then **RUN10/PASS10/SKIP0/FAIL0/NOTRUN0**, `native-process-synchronized.log`; strengthenedv2input+28MiBoutputlimits/legacyJob/UTF8/parentdeath |
| StrictClippy Extensions+Desktop --features nxtools-extensions/test-support --tests -- -Dwarnings |PASS, `final-clippy.log` |
| `cargo check --workspace --tests` latestsource |PASS, `final-workspace.log`,1m02s |
| PythonSDK / NodeSDK / schema parity / examples |**7/8/2/5 PASS**, noexternalnetwork/userstate |
| Scopedrustfmt --check / scopedgitdiff --check |PASS; no workspace/vendorrewrite |
| Spec / componentcatalog / release / editor gates |PASS114documents/14owners, historical79/78catalog/current0.8.0authority, coherentKit0.8.0/pre0.3.9/twoframeworkdeltas, pinnededitorprovenance |

## Remaining / acceptance limits

- No P2 source blocker remains. Canonical next is **P3 — Namespaced persistent storage**; P3–P7/P8 still require implementation, not available from childrecords or declarations.
- Independent prior Corefull **88PASS/1ArchiveFAIL** (`archive_tests::present_invalid_archive_sections_and_missing_required_fields_are_rejected`, archive_tests.rs83) retained; no takeover/fix/fullCoregreen claim. P8 owns finalintegration/fullDesktop; historic1196Desktop is pre-expansion, not this acceptance.
- NativeUI/realpickers/physicalkeyboard/IME/DPI/reader/visual/user/installed/service acceptance **NOT RUN/deferred**. C#selfownedprocess and GPUI TestApp are stronger compiledboundary evidence, not that acceptance. No realClipboard/credentials/config/externalnetwork/thirdpartycode/realvolumescan.
