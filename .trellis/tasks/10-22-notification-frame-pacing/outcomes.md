# Notification pacing result — 2026-10-22

Implemented public production WindowOptions extraction and inactive_frame_interval=None only for notification popup. OfficialToast/clock/stack/hover/close/easing untouched; nativeprepare/readback/finaldraw/reveal/exit-beforehide retained. New Debug observer never requestsframes, bounds512samples/8milestones, writes one scratch summary.

Historical checkpoint: initial compiled attempt was blocked by unrelatedArchive9compileerrors, notbehavioralred. Production sourceguard red was valid; later actualoptions regression1/1 and fullnotifications35/Desktop1002passed, clearing that compileblocker. Current fullresults: [parent](../10-22-ui-experience-refresh/outcomes.md).

Checked-in `tests/native/notification-cadence.ps1 -AuthorizeNative` actually executed. Scratch evidence `notification-cadence-c61da9a8615342309e0cfd3d617e52b6`:180Hz/96DPI, oldentry13frames/34.131msmedian vsnew79/5.484ms; oldexit9/33.483ms vsnew39/5.486ms; idle0both; reducedentrance1/exit2settlementpaints. NativeACK/clientreadback/nonactivation passed, hideafterofficialexit (~275.77mscurrent). InitialreducedfixtureomittedApp::set_reduce_motion; fixedfixtureandrecollected, productionalreadycorrect.

Paint cadence is not GPU-present/perceivedsmoothness/userapproval. NophysicalIME/pointer/mixedDPI/reader/installed/packageacceptance. Noformalruntime/services/userdata.
