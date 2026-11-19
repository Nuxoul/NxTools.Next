# ALZ and EGG interoperability samples

`store_old.alz` is the unchanged `tests/data/store_old.alz` sample from
[alkegi/unalz](https://github.com/alkegi/unalz), revision
`30b4ad40c5452c686df69b3b45554de3073e57a1`.
Its upstream `tests/data/README.md` dedicates all files in that directory to
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).
The source implementation is MIT; no implementation or test code is included here.

The archive contains `text.txt`: the ASCII sentence
`The quick brown fox jumps over the lazy dog. ` repeated80times,3600bytes.
Method0 is Stored, so the actual sample payload is also independently inspectable.
The tests compare literal bytes and the format's own CRC, not an added file digest.

`deflate_empty.alz` is unchanged from the same unalz revision. The unchanged
`store5.egg`, `deflate5.egg`, `mixed_store_deflate5.egg`, `lzma5.egg`,
`bzip2_big.egg` and `empty_noblock.egg` samples come from alkegi/unegg revision
`9929009ca4d010e8cd88bb796f7a1868162026d4`. Its `tests/data/README.md` also
dedicates that directory to CC0 1.0. No upstream implementation or tests are copied.

`expected/{1,2,3,4}.bin` are the independently decoded CC0 contents of
`precompressed.gz`, `prog.sys`, `random.bin` and `text.txt`. CPython zlib/bz2/lzma
validates physical headers, complete block intervals, exact sizes and format CRCs.
The five-member ALZ/EGG samples have identical decoded bytes, including empty.txt.
The large Bzip2 sample contains the same45-byte sentence repeated180,000times,
8,100,000bytes. No sample member is executed.

The unchanged ALZ `split_store.alz/.a00/.a01` and `split.alz/.a00…/.a06`
families come from the same pinned unalz CC0 directory. The unchanged EGG
`split_store.vol1…3.egg` and `split.vol1…8.egg` families come from the pinned
unegg CC0 directory. All22physical volumes are retained, not manufactured by
cutting an ordinary archive. Independently decoded expected CC0 contents are
gzip-encoded under `expected/split/`; `manifest.json` records physical lengths,
logical ranges, actual crossed boundaries, names, sizes and format CRCs.
Identical Stored/large Deflate contents share one expected copy. CPython's
independent struct/zlib oracle validates all19members/21,293,670decoded bytes.
The large Deflate member crosses all7boundaries in each eight-volume family;
the Stored families each contain two members crossing a volume boundary.
ALZ raw CP949 names are decoded independently with CPython cp949. These are
data-only test resources; neither archive members nor reference apps are run.

`enc_store.alz`, `enc_deflate.alz` and `zip20.alz` are unchanged from the same
pinned unalz CC0 directory. Its README identifies the synthetic password
`test1234`. Independent CPython struct/zipfile/zlib validation checks physical
records/end, the twelve-byte traditional-encryption verifier, complete compressed
consumption, decoded lengths and format CRCs:13members/140,748bytes. The
`expected/encrypted/` gzip envelopes contain those independently decoded CC0
bytes; its manifest preserves real names, flags, methods, sizes and offsets.
`zip20.alz` includes an unencrypted empty entry, nested paths and CP949 names.
No encrypted split, non-ASCII password or descriptor-time fixture is claimed.

The14unchanged EGG password samples listed in
`expected/egg-encrypted/manifest.json` come from the same pinned unegg CC0
directory and use `test1234`. Independent CPython struct/zipfile/zlib/bz2,
hashlib/HMAC, PyCryptodome AES and specification-vector-checked LEA verify
62files/700,148bytes. Every block has exact compressed consumption, size and
format CRC; every AES/LEA file has a matching ten-byte ciphertext HMAC.
Gzip envelopes retain the independently decoded CC0 contents. The manifest
records actual encryption/block offsets, names and the method-specific header
length, including producers with overstated lengths. The14samples separately
cover ZipCrypto, AES128/256, LEA128/256 and real Bzip2 combinations. They do not
establish encrypted split, solid+encrypted or encrypted-name support.

Five `spec_egg_*_two_blocks.egg` archives are project-owned specification
fixtures, not additional producer samples. They contain two independent raw
Deflate streams under one continuous per-file ZipCrypto/AES/LEA state. The
first455-byte packed block ends within a cipher block; independent decoding
checks both CRCs, full ciphertext authentication and all214,842literal bytes
per fixture. Their total1,074,210bytes and origin are recorded separately in
the same manifest. No archive member is executed.

`bzip2.alz` and `bzip2_dlz.alz` are unchanged CC0 samples from the pinned
unalz directory. Independent specification parsing checks all Huffman/MTF/BWT/RLE
data, exact DLZ02 termination, packed extents, member sizes and format CRCs.
Each block is reconstructed with its correct Bzip2 CRC and strictly decoded by
CPython bz2:3members/8,100bytes. `expected/dlz/` retains those decoded bytes.
`spec_alz_dlz_multiblock.alz` is separately labelled project-owned specification
evidence, derived from CPython bz2 with the documented DLZ framing. Its three
blocks and1,969,632literal bytes, including long RLE runs, are independently
parsed and strictly reverified before Worker testing. The manifest distinguishes
genuine producer and specification evidence; no split/encrypted DLZ claim follows.

`solid.egg` is unchanged from the same pinned unegg CC0 directory. Its single
raw-Deflate block covers five files/6,342bytes, including an empty file; four
files have no self-owned compressed block. Independent CPython struct/zlib
verification checks the complete stream, exact consumption and block CRC.
`spec_egg_solid_deflate_three_blocks.egg` is separately labelled project-owned
specification evidence: four files/196,656literal bytes share one CPython
raw-Deflate stream across three physical block payloads. Its file spans and
three decoded CRC spans do not coincide. The independent gzip contents and
manifest under `expected/solid/` distinguish both origins and retain exact
framing/offsets. No other solid method, encrypted-solid or producer split-solid
profile is qualified by these samples.

`azo.egg` and `optimal5.egg` are unchanged CC0 samples from the same pinned
unegg directory. An independent Python implementation of public CC0 EGG
AppendixA verifies every internal AZO header/check/terminal, exact packed
consumption, finite26/28-bit arithmetic lookahead, decoded sizes and EGG CRCs:
6files/11,003bytes. All five optimal-sample payloads equal the existing
independently decoded cross-format corpus. The x86-enabled producer sample
contains no address actually changed by that filter.
`spec_egg_azo_multiblock.egg` is separately labelled project-owned specification
evidence, built by an independent arithmetic encoder from literal data and
explicit match tokens: three internal raw/compressed blocks,159,176bytes,
effective x86 restoration across a64KiB output boundary, fresh/reused matches
and a65,539-byte distance. `expected/azo/` contains independent gzip envelopes
and the separate producer/specification manifest. No reference decoder code,
tests or resources are copied; no member is executed. Split, encrypted and
solid AZO producer profiles are not qualified by these inputs.

`azo_pe.egg` is another unchanged CC0 sample from that pinned directory.
Independent AppendixA decode verifies148,781bytes,151actual x86 address
restorations, a140,436-byte match distance, exact packed consumption and
24bits of arithmetic lookahead. `expected/azo-pe/` retains its independently
decoded data and manifest. The embedded `hello.sys` is compared only as bytes,
never loaded or executed. The three genuine inputs total7files/159,784bytes;
with the separately labelled specification input,8files/318,960bytes.

`posix.egg` is unchanged from the same pinned unegg CC0 directory. Its source
README identifies an Android producer. Independent CPython physical framing,
raw Deflate/EOS/consumption and format CRC checks verify three files/98bytes
and real20-byte POSIX mode/uid/gid/Unix-mtime records.
`spec_egg_kind_headers.egg` is project-owned specification evidence: one
ordinary file, one directory, two symbolic links and two FIFOs, with Windows
and POSIX identifiers in both orders. The link/FIFO entries are data-only
catalog records, never filesystem links or devices created by a test.
Four `spec_egg_short_{1,2,5,6}.egg` archives are separately project-owned
specification fixtures for AES128/256 and LEA128/256 salt-plus-verifier
metadata without a full HMAC. Independent PBKDF2/cipher/raw-zlib/CRC checks
verify their ciphertext; the missing authentication is an unsupported product
profile, not an authenticated extraction claim. `expected/kinds-short/`
retains independent gzip contents, physical fields, kinds and origin labels.
No producer locale/relative-name or unauthenticated-header claim follows.

`spec_raw_names.alz/.egg` are project-owned Stored specification fixtures,
not producer samples. Each contains15literal raw names covering traversal,
rooted/drive/UNC paths, empty components, devices, trailing dot/space,
alternate streams, controls and an empty name. Independent CPython struct,
literal payload and format CRC checks verify30members/1,410bytes. The
`expected/names/manifest.json` retains original name bytes and literal data.
Five `spec_egg_locale_*.egg` / `spec_egg_parent.egg` specimens qualify the
locale/parent field order with code pages0/932/949; they are unsupported-name
evidence, not a product locale or parent-resolution claim.
`spec_egg_encrypted_name.egg` and `spec_egg_global_encrypt.egg` qualify only
encrypted-name and reserved Global Encrypt declaration refusal. Their opaque
headers do not establish valid producer encryption or successful decryption.
