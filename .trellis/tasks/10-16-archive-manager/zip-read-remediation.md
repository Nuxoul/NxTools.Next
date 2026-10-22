# ZIP read/encryption remediation

Phase A — Engine contracts and safety foundation, in progress. Overall current state is owned
by phase-a-checkpoint.md. Historical71/1/fmt failure and independently reproduced defects are
preserved in zip-read-acceptance-review.md and research/zip-read-capability.md.

## Scope and fixed invariants

Fix explicit-directory payload extraction integrity and combined EOCD comment metadata budget,
then finish AES reading/authentication plus actual encrypted-method combinations. No UI/general
creation/editing/volumes/overwrite-contract change. Rust owns codecs/state; C# owns Job/lease/
output. Existing accepted atomic-path overwrite/TOCTOU and postcommit ownership stay unchanged.

Public seams: ArchiveEngine.open/execute/extract, current ExtractionPlan and actual C# Job→worker
→public ZIP→native outputs. No internal crypto-only acceptance,secret argv/log/snapshot/disk,
whole-entry/archive Vec,third-party edits,arbitrary ZIP/GPUI upgrade or ignored red tests.

Live zip.rs completely read at this turn's start,tag7949/705lines;previous turn's editing stop
is historical,not a present restriction. Small nonoverlapping grounded edits/new tags required.

Directory repair: record selected explicit-directory IDs in plan;validate only those payloads
once before native directory effects. Derived parent directories have no separate payload.
Comment repair: one combined central+raw comment+worst-case lossy String reservation passed into
projection budget before comment copy/decode;current execution re-admission includes comment.
AES: strict original central/private descriptor,local equality,actual method separate99,checked
salt/verifier/ciphertext/MAC range;RustCrypto public APIs;full constant-time authentication even
empty ciphertext before any publication;AE1CRC vs AE2 exception. Exact dependencies/source and
actual red/green/native/integration counts recorded after execution below.

## Execution evidence

Historical independent71pass/1fail and50C# remain baseline only. This turn's directed evidence:

Directory public red0pass/1fail,exit101(corrupt extract succeeded);selected explicit IDs+payload
verification fix gives public boundary1+plan5pass. Comment red0pass/1fail,exit101(ascii budget1
accepted);combined4x bytes+64 plus central estimate shared with projection yields boundary2+
engine14pass,exit0. Actual native directory/comment5operations:list1/run0,run1/pass1/skip0/fail0,
exit0;normal directory created,invalid directory absent,user file preserved,no partial.

AES exact APIs rechecked in aes0.8.4/cipher0.4.4 KeyInit/BlockEncrypt,zeroize-on-drop source;
pbkdf20.12.2 pbkdf2_hmac;hmac0.12.1/digest0.10.7 Mac::verify_truncated_left uses ct_eq;
sha10.10.6 official source manifest/license and default-features-off pure Rust inspected.
Existing AES/HMAC/PBKDF2 MIT OR Apache-2.0;subtle2.6.1 is BSD-3-Clause(corrects earlier ledger
classification),no build.rs/native/asm/rayon added. Newly resolved sha1 Windows build passed;
ZIP8.6.0/GPUI0.7.1 unchanged. Official WinZip AES document fetched to scratch,not redistributed.
Existing independent AES public red rerun1fail UnsupportedMethod vs PasswordRequired,exit101.
No historical draft restoration;fresh bounded adapter implementation follows verified APIs.

Original AES red unchanged→green1/1;AES fault matrix plus prior ZipCrypto4/4green,then33combo
fixtures(26positive/7malformed) public1/1green. AE1/AE2×128/192/256 Store+Deflate exact bytes;
ZipCrypto and AE2AES256 methods0/8/9/12/14/93/95 public/native tested. Positive ZIP64 sizes,
signed/unsigned descriptors AE1/AE2;duplicate/local-central/truncated/missing extra and valid
authentication with extra compressed tail reject. Matrix covers wrong/salt/verifier/cipher/MAC,
empty file MAC,local equality,unknown version/strength/strong encryption,CRC rules,lower
entry/total/work/metadata,cancel/IO/source change. True IO/resource/cancel/source types retained.

Native3fixtures list3/run0→3run/3pass/0skip/0fail,exit0,then added native IO and directory-MAC
cases:AES fixture1/1green. All use current source lease,exact typed private failure,prior targets/
unrelated user output and owned partial cleanup,correlated payload-free bounded diagnostics.
Clippy one nonminimal_bool repaired without suppression;strict tests Clippy now exit0.

Independent noEOS stock7-Zip26.00 `-mm=LZMA:eos=off` source trace MethodProps→kEndMarker→
CLzmaEncProps.writeEndMark→conditional WriteEndMarker,ZIP writer derives bit1 from actual
IsWriteEndMark. Official source bundle7z2600-src SHA25606a070683bab468ad3e0d00fde09fdde56575034ff87831b57b546581e1315fd;
LZMA encoder public domain,ZIP/CLI LGPL2.1-or-later,not copied/bundled. Direct candidate flags0800,
size1050/packed274/raw265,CRC013b39ac,ZIP SHA25625f64ae048f3c7276adefa0f585e6d54dd5309a163b6a9a4521b55ece831083a.
eos=on control flags0802/raw270 differs at raw byte261(neither stream prefix),not flag-cleared or
guessed truncation.7-Zip t/x verifies exact body SHA256e886ea86544c0dba2061c9369fea8cf668b61423f6ce0c2c15e012d2bb7c53af.
Parent reran self-cleaning checked-in generator and public noEOS test1/1(known size/truncated/
wrong size/EOS-required flag mismatch),exit0. Native noEOS passes final integration.

Oracle difference recorded:7-Zip26 full t rejects encrypted method95 empty-file payload as
trailing data.7-Zip body x-so1050bytes passes;CPython zipfile independent traditional decrypt,
or cryptography+HMAC AES decrypt,then liblzma validates every XZ stream EOF/no unused_data.
Other positive combinations pass7-Zip full t and x-so. No false successful7-Zip whole95claim.

## Integrated checkpoint after all source changes

All commands serial from parent with required3Rust memory environment variables,no timeouts.
External concurrent work caused normal Cargo package/build-lock waits;no duplicate build
started and final process inspection is separate. Logs session scratch remediation-final-*.log.

| Check | Actual result |
| --- | --- |
| archive --features isolated-codecs --offline --no-fail-fast |77pass/0fail/0ignored,exit0;original AES red unchanged now green |
| same-feature actual worker build |exit0 |
| strict archive Clippy --tests/same feature/offline/-Dwarnings |exit0 |
| cargo fmt -p nxtools-archive -- --check |exit0;zip.rs formatting restored |
| cargo fmt --all -- --check |exit1;only unrelated Desktop files below;not changed to force green |
| cargo check --workspace --tests --offline |exit0,NOT workspace tests |
| C#test-host build --no-restore |exit0,0warnings/0errors |
| Original50+new4exact C#union |list selected54/run0;selected54/run54/pass54/skip0/fail0/not_run0,exit0 |
| extensions --test archive_import |7pass/0fail/0ignored,exit0 |
| clipboard --lib archive_ |5pass/0fail/0ignored,39filtered,exit0 |

Four new C#harness fixtures run339actual workers:directory/comment5,AES7inputs×38operations=266,
33combinations×2=66,noEOS2;plus one below-minimum-working-budget reject before child launch.
These are loop operation counts,not339tests. Original50 regressions retained,not skipped.
One bounded read-only reviewer found no definite in-scope correctness/security defects;no
additional review loop. noEOS source/oracle exploration independent and parent generator/test
re-execution completed. Public noEOS and actual native1fixture(list1/run0,1run/1pass/0skip)pass.

Unrelated full-workspace fmt paths at actual check:Desktop downloads/polish_tests.rs,
main_window/extensions_page/{activity,trust_confirmation}.rs,extensions_settings.rs,
quick_launch_page{.rs,/settings_compatibility_tests.rs},ui/components/shortcut.rs,
ui/kit_migration_tests.rs. Earlier fmt output also observed concurrent clipboard edits.
No archive fmt difference remains;whole-workspace green is NOT claimed and no unrelated file
formatted by this turn. General creation not started;Phase A/all A–F exits remain incomplete.

Known limits:bounded small interoperability/fault cases are not large-RSS/performance/100k
timing/4GiB/UNC/release/user-native acceptance. AE1/AE2all strengths extra methods9/12/14/93/95
are mechanically composed but only AE2AES256 such combinations independently executed;
do not generalize that matrix to every encoder/version/strength. No AES creation.
Current20legacyZstd typed rejection unchanged;unknown encryption UnsupportedEncryption and
unknown actual compression UnsupportedMethod,no text matching or silent fallback.

## Document gates and owned cleanup

Actual spec-contracts101docs/14owners,componentcatalog79pages/79rows/78locked,taskcontext
26implement/23check,fixture-selector source gate all exit0.31scoped text/syntax/JSON files,
33combination hashes+noEOS hash and git-notignored checks,all referenced context<=32768 passed.
Reference note32736bytes,archivequality29806bytes. Scoped git diff--check exit0(Program.cs
existing CRLF advisory only);no DEBUG probes/ignored tests. These are document/source hygiene,
not native/release/whole Phase A acceptance.

Final inspection:fixed fixture helper absent,0archive-zip-remediation temp roots,0self-cleaning
noEOS generator temp roots,0archive worker/test-host/7-Zip. Another concurrent Cargo command
`cargo test -p nxtools-desktop followup_ -- --test-threads=1` and its rustc were observed;not
owned by this archive execution,not killed/no duplicate archive Cargo started. No timeout.
Scratch logs,official spec/source reference downloads and independent oracle evidence remain
outside project intentionally;project fixtures/generators are task deliverables,not leftovers.

Final source SHA256 zip.rsACC3DC592F316DE8CDF801183223F5BACF1B766FB06760A9D50BBB3A04F7D73A;
zip_aes.rs1C0949ACE37B9F42381AE697490671C6D4514D507DE5CAE11C76209F3BCB9B46.
No product protocol/native implementation changed;only existing private error enum gains
UnsupportedEncryption and native fixtures assert exact frame type. No registry/service/
association/install/package/user archive/commit/push occurred.
