# Archive browser completion and integration denial repair

2026-10-27. This continues the preceding partial browser correction within the original task and
canonical phase owners. Prior partial outcome, failed edits, compile/lint failures and installation
error1 OPEN history are preserved. No packaging/install/product/real-GUI/Registry/COM/Shell/service
operation, signing, dependency/vendor change, commit or push was performed.

## Delivered source and usage

- **Folder icons:** original Tree and right directory rows now consume the same validated shared
  Shell folder-type PNG at24DIP in original32DIP rows. A Material filled-folder silhouette (Apache2
  adaptation with notice) is the visible loading/failure/offline fallback. Tooltip distinguishes
  pending/failure/offline. Original TypeIcons8pending/128resources/15seconds and snapshot/generation/
  epoch/close/connection guards remain; Tree requests only when folders exist. No member-path probe,
  extraction, embedded executable icon parsing or archive-shortcut resolution.
- **Details:** retained default Name/Packed/Original/Type/Modified, original sorting/column grips/
  horizontal scroll, truthful unknown metadata and minimum240DIP Name. Existing version2 migration
  remains: validv1 missing/false mode promotes five primary columns, preserves widths/sort/geometry/
  history/panels; explicitv1true visibility andv2false choice survive. No whole preference reset.
- **Marquee:** original actual empty-body down/move/up, normal replace/Ctrl fixed-baseline XOR and
  BrowserModel selection/focus/anchor remain. Parent row excluded; original row native drag, header,
  grips, bars, split and doubleclick retained. This round keeps the earlier real-event regressions.
- **Internal read-only preview:** View -> `只读预览栏` opens the original official resizable right pane
  (default280DIP,200..600), close button or same checked menu hides it. Single pointer/keyboard/
  marquee selection shares one generation-scoped request. Plain/code/Markdown use the approved
  passive MarkdownDocument; `原文` / `渲染` is local presentation only, no editor/write-back. Images
  use owned decoded first-frame pixels and GPUI Contain, not a temporary-file path. Long document
  scroll reaches the real final block. Empty/multiple/directory/loading/unsupported/too-large/
  corrupt/password-cancel/source-change/disconnect/failure are distinct; retry exists where useful.
- **Open compatibility:** doubleclick/Enter still use the original explicit Materialize/active-content
  consent/Shell-or-nested chain. Accepted explicit open cancels passive preview first; cancelled
  preview results release their own lease and cannot externally open. Preview-native leases are
  explicitly refused by native Open, independently of the Rust branch.
- **Integration:** the reported message came from permanent product `ProductApplyAuthorized=false`,
  not Windows access-denied. Explicit user Apply/Revert now reaches the original HKCU P206/P207
  service after unchanged approved Desktop/helper release identity checks. Existing desired vs
  observed revision, durable ledger, capture-all/attempt-before-write/reverse compensation and exact
  owned unregister remain. Original detached reader + bounded STA prevents later commands blocking;
  cancellation/timeout is failed/unconfirmed, not applied or a promise Windows is unchanged.
  See [native integration evidence](integration-enable-outcomes.md).

**Deployment distinction:** these are workspace source changes. Installed/package binaries were
not replaced; an existing installed version may still show its old denial until separately built
and delivered. No actual integration Apply/Revert was invoked on this workstation.

## Preview bounds, ownership and privacy

Original ArchiveApplication/EntrySession/worker/source binding/password/cache/lease remain owners.
Preview/PreviewText are explicit purposes of the original materialization contract, not a second
runtime/worker/cache. Native actual output<=8MiB(image)/64KiB(text); Rust independently reads limit+1,
validates regular/nonreparse shape and stable length, UTF8/BOM/control and1200line/65536char budgets.
Supported image families PNG/JPEG/GIF/WebP/BMP; static frame only. Axis<=8192,pixels<=8*1024*1024,
8bit channels only; decoder32MiB allowance with decoded reservation and separate finalRGBA<=32MiB.
Encoded8MiB is separate; this is NOT a measured whole-process64MiB RSS guarantee. Locked JPEG/WebP
require stricter pixel/scratch admission. Animated/ALPH/allocating lossless-transform/cache/meta-
Huffman WebP is explicitly unsupported; bounded static bitstreams are supported. No dependency or
vendor modifications were used to claim a limit the published decoder does not enforce.

At most one preview native/decode task per browser and two global background decode permits; only
latest desired selection is retained. Local token is retired synchronously on selection changes
(including same-turnABA), cross-owner dispatch/result/password work is next-turn and checks token,
row, source, snapshot and browser generation again. Close/refresh/cancel/source/connection retire
work; native actualcleanup and real read/decode completion retain slots and leases until safe release.
Owned image pixels can outlive the temporary lease without referencing it. Incomplete cleanup blocks
replacement rather than creating unlimited jobs. Password retirement clears masked input and only
closes its own top/focused dialog; unrelated newer overlays survive. Original private password reply
transport remains. HTML/scripts/links are never executed; Markdown images are placeholders, no
network/local-resource loading or silent external fallback. Diagnostics accepted/materialized/
decode/terminal carry operation correlation, fixed outcome/elapsed/nativecode only, never payloads,
paths/passwords or move/frame logs. Exact contract: [preview](browser-preview-contract.md).

## Actual verification and retained failures

Evidence root: session scratch `archive-browser-completion/`; integration red/green logs retain their
`integration-*` names in the same session scratch root. All Cargo jobs1/incremental0/codegen256,
locked/offline and checked serial lane; no timeouts, duplicate builds or unrelated process kills.

| Command / log | Observed result |
| --- | --- |
| 01-ui-red: actual browser_completion_ production-view tests before UI repair |0PASS/2FAIL: missing preview menu; folder test first exposed auto-enter fixture setup, corrected before positive size/row assertions. Do not count the fixture issue as a size-specific observed red.|
| 02-preview-ui |CompileFAIL on Scrollable track_scroll ordering, Div/Stateful conditional, Option image bytes assertion; fixed at exact released APIs, not behavioral reds.|
| 03-browser |68PASS/0FAIL real browser/drag/icons/marquee/details/preview/menu/minimum/theme/split boundaries.|
| 04-preview-related |91PASS/0FAIL, including8new actual application lifecycle cases; broad substring includes existing Clipboard/Quick preview tests, not91new archive cases.|
| 09-archive /10-strict |177PASS/0FAIL; strictFAIL unused InteractiveElement added by preview implementation; removed without suppression.|
| 11-preview-ui-final |6PASS/0FAIL, real uniformdocument/source/close/ABA/pointer+keyboard/split/minimumthemes/imagebounds+release/lastblock.|
| 12-explicit-open-red ->13-explicit-open-green |Actual original app entry initially did not Cancel passive request; observed1FAIL then1PASS after original open cancels preview and Browser retires its token.|
| 14-final-archive: cargo test -p nxtools-desktop --bin nxtools-desktop --locked --offline archive:: -- --test-threads=1 |181PASS/0FAIL/0ignored,1188filtered,46.15s.|
| 15-protocol: cargo test -p nxtools-protocol --lib --locked --offline |82PASS/0FAIL/0ignored.|
| 16-strict-tests: cargo clippy -p nxtools-desktop -p nxtools-protocol --tests --locked --offline -- -D warnings |PASS exit0.|
| 17-strict-production: same without --tests |FAIL on pre-existing unrelated main_window.rs SPACING_12 import and tools/floating_assistant/media.rs animated field; no new archive warning. Not globally strict-green.|
| 18-workspace: cargo check --workspace --tests --locked --offline |PASS exit0.|
| 19-full-desktop: cargo test -p nxtools-desktop --locked --offline -- --test-threads=1 |1369unit+10integration PASS,0fail/ignored,507.25s+2.69s. One final fullDesktop run only.|
| 20-final-worker: cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs --locked --offline |PASS; actual fixture worker SHA256 F52CADC910A3862D09E1D16D1207AB9143D065B65F9B21B754C0CAD9D6442CD5.|
| 23-final-csharp-build: dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -v:q |PASS0warnings/0errors.|
| 24-final-native-list ->25-final-native-run: exact six filters below |LIST23=RUN23=PASS23,skip0/fail0/not_run0; includes2preview actualoutput/readonlypurpose,2folder and19integration.|
| integration-red / integration-dispatch-red ->integration-final2 |Original composition permanentdisabled and subsequentreaderblocking each observed red1, then19/19green. No live Registry was involved.|
| 22-fmt-all-final / scoped rustfmt |All touched Rust files PASS after local formatting. Globalcargo fmt --all -- --check FAIL only pre-existing unrelated page_rem_tests/networksettings files; do not format concurrent work to hide it.|

Native exact union filters with original `dotnet run --project ... --no-build --no-restore -- --fixtures
[--list]`: `archive preview fixture`, `archive folder type icon fixture`, `archive Phase F P206`,
`archive Phase F integration`, `archive association settings fixture`,
`archive Phase F P207 future registration plan unapplied`.

Five gates PASS: archive-phase-f-shell, spec-contracts114docs/14owners, componentcatalog79/78,
GPUI0.8.0/pre0.3.9 narrowvendorprovenance, Markdown57fileprovenance. git diff --check PASS (existing
CRLF normalization notices only). Protocol/decoder/native purpose tests authored before repair have
red-capable assertions but were not independently executed pre-fix red; no invented red evidence.

## Changed owners

Desktop archive/browser.rs/browser_preview.rs and actual tests; original browser_icons status and
shared queue; original entry.rs/application.rs plus lifecycle tests and new preview.rs/decode tests;
archive module declaration; icons/assets and Apache notice. Protocol archive_browser adds purpose
validation; C# original EntryContract/EntrySession enforce actual output and no previewleaseOpen;
C# preview test/harness registration. Integration original service/plan/dispatcher/pipe/adapter and
isolatedtests/sourcegate/currentowner specs are recorded in its separate report. No other files were
reset/reverted, no new dependency/vendor patch, no Generated release hash edits.

## Acceptance limits

Source/compiled/synthetic/owned-worker evidence is complete for the delivered slice; no claim of
all A–F/first-release or visual acceptance. Actual product HWND screenshots, user visual approval,
physical mouse/keyboard/Explorer OLE,100/125/150%/mixedDPI,IME/UIA/provider and actual Registry/COM/
Shell integration remain NOT RUN and require separate authorization. Installation-error1 root cause
stays OPEN; current diagnostic installer and signed artifacts unchanged. Production strict/global
fmt have the unrelated blockers described above, not silently waived.

Final documentation/task validation PASS (82implement/79check entries),8existing context-size
warnings retained. Original pre-completion1563-line checkpoint suffix SHA256 remains
7FDEA19E6A365BAF436387F39B212F99DFF9B529308F245B22818AB76AEC3B13,
matching opening #3B13 anchor; prior history is complete and unchanged.
One optional bounded read-only review was started but cancelled after it did not return a report;
no findings/approval were delivered, so it is NOT review-PASS evidence. No replacement review.
