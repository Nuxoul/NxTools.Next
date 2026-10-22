# Third-Party Notices

## Google Material Symbols

The SVG icons under `rust/apps/nxtools-desktop/assets/material-symbols/` are selected from
[Google Material Symbols](https://fonts.google.com/icons) and the
[official Google repository](https://github.com/google/material-design-icons).

Copyright Google LLC.

Licensed under the Apache License, Version 2.0. A copy of that license is included in
`LICENSE-APACHE`.

## rust-embed-resource

The Windows desktop build uses
[`embed-resource`](https://crates.io/crates/embed-resource) to compile the generated product icon
resource into `NxTools.exe`.

Copyright (c) 2017 nabijaczleweli.

Licensed under the MIT License. A copy of that license is included in `LICENSE-MIT`.

## GPUI Kit and local Windows backend patch

The desktop uses GPUI Kit0.8.0 and its matched default GPUI Windows backend0.3.9,
published under Apache-2.0. The active repository-local backend retains its
upstream license in `vendor/gpui-pre-windows-039/LICENSE-APACHE`.

NxTools modifies only `WM_MOUSEACTIVATE` handling to respect the live
`WS_EX_NOACTIVATE` window style. Ordinary activating windows retain their original
behavior. Source provenance and the narrow change are recorded in
`vendor/gpui-pre-windows-039/NXTOOLS-PATCH.md`. A copy of Apache-2.0 is included in
`LICENSE-APACHE`; no ShellRS GPL code or assets are included.

The active vendored `gpui-component`0.8.0 snapshot in
`vendor/gpui-component-080/` retains its Apache-2.0 license. The original approved
minimal notification duration builder forwards only into officialToastOptions;
card/stack/clock/pause/close/animation ownership and raw5seconddefault remain.
Provenance and exact boundary: `vendor/gpui-component-080/NXTOOLS-PATCH.md` and
NXTOOLS-UPSTREAM.json. Both patches were carried by the explicit2026-10-21upgrade;
old Component0.7.1/0.5.1 and Windows0.3.8vendors remain inactive historicalsources.

## GPUI Kit Markdown Editor

The unified document surfaces use `gpui-kit-markdown-editor` v0.1.0, fixed revision
`31608fa74bdbeca79d9901bf2daa48ea7ecf1ecc` from
https://github.com/wabzqem/gpui-kit-markdown-editor, licensed Apache-2.0.
The original license is retained in `vendor/gpui-kit-markdown-editor-010/LICENSE-APACHE`.
`NXTOOLS-UPSTREAM.json` records all 57 original file hashes. Approved local changes
are restricted to passive display in `src/element.rs` and exactly three stable-Rust
Fenwick lowbit substitutions in `src/cull.rs`; see `NXTOOLS-PATCH.md`.
Default syntect/all-language features are not enabled. This is separate from the
two existing GPUI Component/Windows framework patches.

The NxTools Graphite Light/Dark theme assets are original project configurations
using public GPUI Kit theme APIs. The optional upstream Catppuccin, Gruvbox and
Solarized assets below remain unchanged.

## pinyin-data

Chinese-name pronunciation data is derived from [pinyin-data](https://github.com/mozillazg/pinyin-data),
Copyright (c) 2016 mozillazg, licensed under MIT. Full license, reviewed input hashes
and regeneration provenance are included in `third_party/pinyin-data/`.
The generated bounded table is under `rust/crates/nxtools-finder/src/pinyin_data/`.
No WinShun C++ implementation is incorporated.

The MIT License (MIT)

Copyright (c) 2016 mozillazg

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## GPUI Kit official theme library

Bundled Catppuccin, Gruvbox and Solarized theme JSON files are from
https://github.com/longbridge/gpui-kit/tree/d2e6bba2fcab8c605e983327207682a397c98060/themes
under the upstream Apache-2.0 license. Their original author/source metadata is
retained in rust/apps/nxtools-desktop/assets/themes/. No theme colors were rewritten.
A copy of Apache-2.0 is included in LICENSE-APACHE.

## Microsoft System.ServiceProcess.ServiceController

The Windows trusted-update service host uses Microsoft's
System.ServiceProcess.ServiceController 8.0.1, from the .NET runtime project,
licensed under MIT. Copyright (c) .NET Foundation and Contributors.
The MIT license text above applies; LICENSE-MIT is included with the package.

## Archive streaming dependencies

The archive engine uses the existing locked ZIP8.6.0 (MIT), flate2 1.1.9
(MIT OR Apache-2.0; zlib-rs backend) and crc32fast 1.5.0 (MIT OR Apache-2.0).
Upstream sources and license notices remain in their published crates:
https://github.com/zip-rs/zip2,
https://github.com/rust-lang/flate2-rs and
https://github.com/srijs/rust-crc32fast.
No Archiver/SupaZip implementation or proprietary Bandizip code/assets was copied.

The optional isolated worker codec build uses sevenz-rust2 0.23.0 (Apache-2.0),
lzma-rust2 0.21.0 (Apache-2.0), and unrar_sys 0.5.8 with bundled UnRAR7.1.
UnRAR Copyright (c) Alexander Roshal. It is read/test/extract only; no RAR creation.
The complete UnRAR notice is retained at
`rust/crates/nxtools-archive/tests/fixtures/upstream/UNRAR-LICENSE.txt` and must
accompany any distributed worker containing UnRAR. In particular:

UnRAR source code may be used in any software to handle RAR archives without
limitations free of charge, but cannot be used to develop RAR (WinRAR) compatible
archiver and to re-create RAR compression algorithm, which is proprietary.
Distribution of modified UnRAR source code in separate form or as a part of other
software is permitted, provided that full text of this paragraph, starting from
"UnRAR source code" words, is included in license, or in documentation if license
is not available, and in source code comments of resulting package.

Upstream test archive provenance/hashes are retained beside these licensed fixtures.
No UnRAR/sevenz upstream source was patched; Rust FFI only supplies the published
UnRAR callbacks for actual-byte accounting and cancellation inside the owned worker.

Rust request/native-call byte buffers and password copies use zeroize1.9.0
(Apache-2.0 OR MIT), already present in the workspace lock, from the RustCrypto
utils project. Clearing owned buffers does not guarantee erasure of immutable
managed strings or third-party internal copies; no secret argv/config/log is used.

Archive private partial read-back uses base64 0.22.1 (MIT OR Apache-2.0),
https://github.com/marshallpierce/rust-base64, already locked by the workspace.
It is pure Rust; only bounded private payloads use it, never diagnostic records.

## ZIP BZip2 decoder

The bounded ZIP12 decoder uses bzip2 0.6.1 (MIT OR Apache-2.0) with the
pure-Rust libbz2-rs-sys 0.2.5 backend (bzip2-1.0.6 license),
https://github.com/trifectatechfoundation/bzip2-rs. No upstream source was changed.
The backend's complete notice follows:

The original program, "bzip2", the associated library "libbzip2", and all
documentation, are

Copyright (C) 1996-2021 Julian R Seward.
Copyright (C) 2019-2020 Federico Mena Quintero
Copyright (C) 2021 Micah Snyder

This Rust translation, "libbzip2-rs" is a derived work based on "bzip2" and
"libbzip2", and is Copyright (C) 2024-2025 Trifecta Tech Foundation and contributors

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

1. Redistributions of source code must retain the above copyright
   notice, this list of conditions and the following disclaimer.

2. The origin of this software must not be misrepresented; you must
   not claim that you wrote the original software. If you use this
   software in a product, an acknowledgment in the product
   documentation would be appreciated but is not required.

3. Altered source versions must be plainly marked as such, and must
   not be misrepresented as being the original software.

4. The name of the author may not be used to endorse or promote
   products derived from this software without specific prior written
   permission.

THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS
OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE
GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

Julian Seward, jseward@acm.org
bzip2/libbzip2 version 1.1.0 of 6 September 2010

## TAR header parsing

The bounded TAR adapter uses tar 0.4.46 (MIT OR Apache-2.0),
https://github.com/composefs/tar-rs, with default xattr support disabled.
Its Windows dependency filetime 0.2.29 is MIT/Apache-2.0,
https://github.com/alexcrichton/filetime. Existing LICENSE-MIT/LICENSE-APACHE
accompany these crates. No upstream source was changed and no unrestricted
native unpack routine is used.

Compound TAR decoding uses existing locked tempfile 3.27.0 (MIT OR Apache-2.0),
https://github.com/Stebalien/tempfile, for bounded nonencrypted disk spools.
Only anonymous delete-on-close temporary creation is used, never persist/replace.

Single-source XZ creation/read-back additionally uses already-locked lzma-rust2 0.21.0
(Apache-2.0) std/xz/encoder features, https://github.com/hasenbanck/lzma-rust2/.
No upstream source changes, no external executable product backend or GPUI/ZIP upgrade.

## Zstandard bounded streaming reads

The archive adapter uses zstd-safe 7.2.4 and zstd-sys 2.0.16+zstd.1.5.7
(MIT OR Apache-2.0 wrappers, https://github.com/gyscos/zstd-rs), defaults disabled,
std only; bundled native Zstandard 1.5.7 is used under its BSD license below.
No upstream edits, bindgen/legacy/dictionary-builder/multithread/experimental features.
Wrapper copyright (c) 2016 Alexandre Bury; the MIT license text above and
LICENSE-MIT apply to the wrappers. The native BSD notice below is distinct.

BSD License

For Zstandard software

Copyright (c) Meta Platforms, Inc. and affiliates. All rights reserved.

Redistribution and use in source and binary forms, with or without modification,
are permitted provided that the following conditions are met:

 * Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.
 * Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.
 * Neither the name Facebook, nor Meta, nor the names of its contributors may
   be used to endorse or promote products derived from this software without
   specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## deflate64

ZIP method9 reading uses deflate640.1.12 (https://github.com/anatawa12/deflate64-rs),
licensed under MIT. Copyright (c) .NET Foundation and Contributors; Copyright (c) anatawa12 2023.
The full MIT license text above and included LICENSE-MIT apply; the original crate retains its
license. No third-party source modifications or checkpoint feature are used.

## ZIP AES legacy interoperability

WinZip AE-1/AE-2 reading uses RustCrypto aes0.8.4 (zeroize),hmac0.12.1,pbkdf20.12.2
and sha10.10.6,licensed MIT OR Apache-2.0. The original published crates retain their
licenses; LICENSE-APACHE is included. SHA-1 is used only where the legacy ZIP PBKDF2/HMAC
format requires it,not as a new general-purpose integrity/security hash. No upstream
source modifications,assembly build feature or product dependency on external7-Zip tools.

The sha1 MIT attribution is Copyright(c)2006-2009 Graydon Hoare,Copyright(c)2009-2013
Mozilla Foundation,Copyright(c)2016 Artyom Pavlov. The full MIT permission/disclaimer
above applies. RustCrypto AES/HMAC/PBKDF2 are Copyright RustCrypto Developers.

Constant-time password-verifier comparison uses subtle2.6.1 (BSD-3-Clause):

Copyright (c) 2016-2017 Isis Agora Lovecruft, Henry de Valence. All rights reserved.
Copyright (c) 2016-2024 Isis Agora Lovecruft. All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

1. Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright
notice, this list of conditions and the following disclaimer in the
documentation and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
contributors may be used to endorse or promote products derived from
this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED
TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## ZIP creation and source filters

ZIP8.6.0's public aes-crypto writer additionally uses aes0.9.3,hmac0.13.0,
pbkdf20.13.0,sha10.11.0,ctutils0.4.3 and cmov0.5.4 (MIT OR Apache-2.0;
Copyright RustCrypto Developers),and constant_time_eq0.4.2
(CC0-1.0 OR MIT-0 OR Apache-2.0,used under Apache-2.0; Copyright ctz).
The included LICENSE-APACHE and original published crate licenses apply.
All are source-unmodified,pure Rust with no native build script; ZIP remains8.6.0.

Source exclusion globs use globset0.4.19 (Unlicense OR MIT), Copyright(c)2015
Andrew Gallant, used under MIT. The full MIT permission/disclaimer above applies.
No original reference application code or proprietary archive backend is bundled.

## Clipboard Markdown and code preview

Markdown parsing uses the existing locked markdown-rs1.0.0 (MIT),
https://github.com/wooorm/markdown-rs. Code highlighting uses GPUI Kit0.8.0's
official Tree-sitter0.26.13 feature and published grammar crates (all MIT):
Bash0.23.3,C0.24.2,C++0.23.4,C#0.23.5,CSS0.23.2,Go0.23.4,HTML0.23.2,
Java0.23.5,JavaScript0.23.1,JSON0.24.8,Markdown0.5.3,Python0.23.6,
Rust0.24.2,Sequel(SQL)0.3.11 and TypeScript/TSX0.23.2.
Tree-sitter runtime/grammar sources and original licenses remain in their published
crates: https://github.com/tree-sitter/tree-sitter,
https://github.com/tree-sitter-grammars/tree-sitter-markdown and
https://github.com/derekstride/tree-sitter-sql. Other grammars are the matching
tree-sitter/tree-sitter-<language> projects. No upstream source was patched and
no TieZ GPL implementation was copied. Retain the original MIT copyright/license
notices when distributing these dependencies; this source change performs no packaging.

## fawazahmed0 Currency API catalog and reference fixtures

The bootstrap code/name catalog and captured reference-rate test fixture derive
from https://github.com/fawazahmed0/exchange-api (CC0-1.0). Original license and
source/snapshot provenance are retained in `third_party/currency-api/`. The
catalog seeds offline selection only; reference rates are fetched as fixed public
bulk data and validated locally. No upstream implementation code is incorporated.
