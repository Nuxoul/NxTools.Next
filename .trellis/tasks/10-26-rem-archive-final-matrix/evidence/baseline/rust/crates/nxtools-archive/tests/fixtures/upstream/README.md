# Independently produced codec fixtures

Small test archives are retained from exact published crates `unrar0.5.8` (MIT OR Apache-2.0 wrapper; UnRAR restriction separately retained) and `sevenz-rust2 0.23.0` (Apache-2.0). `manifest.json` records original crate-relative paths and exact SHA256. No archive was created by NxTools for these fixtures. RAR4/5 signatures were checked; Windows bsdtar independently listed plain RAR4/RAR5 and7z LZMA entries. Actual upstream archive producer version is not asserted unless documented by upstream.

RAR4 VERSION=`unrar-0.4.0` (11bytes); RAR5 Unicode entry=`foobar\n` (7bytes);7z LZMA file.txt=`this is a file\n` (15bytes), independently read by Windows bsdtar. RAR4 encrypted password=`unrar`; RAR5 header password=`password`; encrypted7z password=`sevenz-rust`, from published tests (synthetic only). Current real C# Job→Rust open/test and no-replace native-output fixtures passed for these archives including solid/non-solid/AES. This is small-fixture evidence, not complete P201/format/security acceptance. Solid test/test1.txt SHA256=`dac1a4378fa936d14452116dd08d3da46f9f5eb17a427a57ace6ed7f2629919a`; test/test2.txt=`91a5e97ee41057e078ce25e73f84a1d37f3c7c5c01b8d60a8d4e8c2b779c1392`, independently verified via bsdtar before product extraction.

Full UnRAR handling-source restriction is in UNRAR-LICENSE.txt; it permits archive handling but prohibits reproducing the proprietary compressor. No RAR creation is implemented. The project does not copy Archiver/SupaZip code or proprietary Bandizip DLL/assets. These fixtures are inputs to isolated tests, not evidence that every Phase A compatibility gate passed.

## Encrypted exact-output oracle (2026-10-17 remediation)

Independent tool: official 7-Zip24.09 x64, fetched from `https://www.7-zip.org/a/7z2409-x64.exe`
and extracted in a unique session scratch tree (not installed). Setup SHA256
`bdd1a33de78618d16ee4ce148b849932c05d0015491c34887846d431d29f308e`;
`7z.exe` SHA256 `e2ca3ec168ae9c0b4115cd4fe220145ea9b2dc4b6fc79d765e91f415b34d00de`.
Each extraction exited0; synthetic passwords were supplied through stdin, never argv.
No NxTools worker/backend output was used to create these expected values. Windows
libarchive3.7.7 could list some headers but could not decode these encrypted payloads;
its failed reads were not treated as an oracle.

| Input | Exact regular files | Size / SHA256 |
| --- | --- | --- |
| encrypted.7z |2: encripted/7zFormat.txt; encripted/7ziplogo.png |2247 / `09ff37c5cdb9d30953442827f21457a8263baf7868471e5ae8317c743c84303b`;1417 / `f8ac2201f43c4989eb95b2184eee17a705a3107e43852859d5ce01e239d16ada` |
| rar4-encrypted.rar |1: .gitignore |18 / `f9b1ca6ae27d1c18215265024629a8960c31379f206d9ed20f64e0b2dcf79805` |
| rar5-header-encrypted.rar |1: .gitignore |18 / same hash; exact bytes for both RAR samples: ASCII `target\nCargo.lock\n` |

These are exact small-sample interoperability oracles, not proof for every encrypted method.
Synthetic corruption mutates copies only; failed extraction must retain unrelated existing
user bytes and publish no invalid final or owned partial.

RAR5 corrupted copy flips byte[length-30] in this366byte sample. Independent7-Zip24.09
test returned exit2; mutated-input SHA256
`87bf29d2488f67902420fceb49849ff043645b2726bf22675245261476ff923c`.
The regression supplies the correct synthetic password in private stdin and expects ambiguous
authentication/data failure, zero invalid output/partials and preserved existing sentinel.
