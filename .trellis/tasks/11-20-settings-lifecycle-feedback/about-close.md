# About close and main visibility

## Observed report

User confirms clicking About's upper-right Close hides the main window while the tray remains; this is presentation loss, not application termination. Read-only process observation found the protected Desktop0.7.4. The retained update-admission/About-close log sequence predates the current executable's write time and does not establish a current-source reproduction. Log evidence contains no target HWND/close-origin proof for that sequence.

## Current owner boundary

MainWindow::open_about constructs a separate modeless Normal window through about_window_options. About holds a weak downloader; the application-global dispatcher retains the original downloader/runtime. The official close-only caption stops pointer propagation, retires only About's local update request/task and removes its own window. Accepted installer tasks remain downloader-owned. Native Dialog parent disable/re-enable/activation is not part of this current factory. No global quit-mode change or automatic main-window re-show is applied.

The combined production regression opens MainWindow and About, admits a synthetic newer release through the existing deferred handoff, clicks the actual Close Button, and checks that only About disappears, main remains renderable, the accepted SQLite task remains, and no main-hide command is sent. Close/reopen and retirement before deferred admission share that same real-owner boundary. The fixture's default queue is paused before engine startup and never downloads from the network.

## Diagnostics

Main native-close and caption-close requests record desktop.window.close.requested with correlation, fixed source/native-or-caption, main-or-download_detail surface, target HWND and close_to_tray state. About caption-close records desktop.about.close.requested with the original About correlation and target HWND. These finite lifecycle events do not log titles, paths, clipboard, URLs or pointer streams. Existing close-to-tray, explicit Exit and download completion policy remain authoritative.

## Limitations

Compiled TestWindow coverage is not actual Windows caption/foreground/hide acceptance. The installed report's hiding initiator remains unproved until reproduced on a matching build with target/origin diagnostics. There is no claim that the reported native hiding is resolved from source inspection or a passing synthetic regression alone.
