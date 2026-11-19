# REF-19 — ordinary EGG per-file encryption

Status: ordinary password slice GREEN, not accepted. REF-19 and B7 remain in progress; REF-20 is pending.

## Verified capabilities

| Encryption | Genuine ordinary compression profiles | Independent producer samples | Boundary evidence |
| --- | --- | --- | --- |
| EGG ZipCrypto | Deflate, Bzip2 | `enc_zip20.egg`, `zip20.egg`, `enc_zip20_bzip2.egg`, `oversized_encrypt_zip20.egg` | Real Worker and production Browse / Test / Browser-bound extraction |
| AES-128 | Deflate, Bzip2 | `enc_aes128.egg`, `aes128.egg`, `oversized_encrypt_aes128.egg` | Same boundaries; full ciphertext HMAC required |
| AES-256 | Deflate, Bzip2 | `enc_aes256.egg`, `aes256.egg`, `enc_aes256_bzip2.egg`, `enc_lea_bzip2.egg`, `oversized_encrypt_aes256.egg` | Same boundaries; full ciphertext HMAC required |
| LEA-128 | Deflate | `lea128.egg` | Same boundaries; full ciphertext HMAC required |
| LEA-256 | Deflate | `lea256.egg` | Same boundaries; full ciphertext HMAC required |

The14unchanged producer samples contain62files/700,148decoded bytes. Their directory is expressly CC0 in pinned alkegi/unegg revision `9929009ca4d010e8cd88bb796f7a1868162026d4`; the synthetic password is `test1234`. The file named `enc_lea_bzip2.egg` identifies AES-256 in its actual Encrypt Info header. No LEA+Bzip2 producer claim follows from its name.

Five additional project-owned specification fixtures contain two independently compressed Deflate blocks under one continuous per-file cipher state. They contribute5files/1,074,210literal bytes, separately labelled in the manifest. The first455-byte packed block ends within a cipher block. They verify counter/key continuity and whole-file authentication across blocks, not additional producer interoperability.

Independent CPython struct/zipfile/zlib/bz2, hashlib/HMAC and PyCryptodome AES validate complete physical records, method metadata, compressed consumption, decoded sizes and format CRCs. Specification-based LEA decoding first passes the published128/256-bit test vectors. AES/LEA tags match the first ten HMAC-SHA1 bytes over every ciphertext block. Expected gzip envelopes contain only independently decoded contents; no member or reference application is executed.

## Original owners and bounded implementation

The verified chain is recorded in [archive format references](../../../docs/reference-code/archive-formats.md) and [REF-19 design](ref19-design.md).

- `worker_alzeg/egg.rs` retains fixed method-specific detached encryption metadata and the original finite metadata/working reservations. It accepts the method-sized record or the sampled UInt16 length including its seven-byte outer header only when the actual payload ends at the mandatory End marker.
- `worker_alzeg/crypto.rs` keeps one operation-private per-file state. Original `formats/zip_crypto.rs` zeroizing keys and `formats/zip_aes.rs` AES primitive are reused. LEA uses fixed zeroizing schedules from the public specification. No dependency, plaintext spool or whole-member buffer is added.
- AES/LEA authenticate the complete ciphertext through a fixed64KiB buffer before decoding. Keys/counter position survive compression-block boundaries. Existing strict codec consumption, expanded-size and block CRC checks precede original member publication.
- `worker_formats`, `worker.rs::job_password_format` and `worker_password::execute` reuse the original private password coordinator, finite read-only preparation, current prompt tickets, cancellation and exactly one subsequent extraction. C# production source/output/Job ownership is unchanged.
- Existing operation/correlation, password/source lifecycle and terminal diagnostics cover the operation. No passwords, payloads, per-block ticks or filenames are added to logs.

Clear Browse preserves actual names, sizes and encrypted flags without claiming payload authentication. Missing passwords enter the original session. Wrong passwords, verifier/tag mismatch and encrypted decode/size/CRC failures remain ambiguous AuthenticationOrDataError; they do not automatically retry as definitive WrongPassword. Resource, cancellation, source and first native failures retain their original categories.

## Behavioral evidence

The final unchanged47-test Worker selection is `b7-ref19-egg-password-red3` → `b7-ref19-egg-password-green3`: nonzero LIST47/RUN47,36PASS/11FAIL/Cargo101 →47PASS/Cargo0, with no ignored or unexecuted test. RED restores only the original EGG supplied-password refusal at the existing format guard. Test and manifest bytes are identical across the authoritative pair.

The two new production selections are `b7-ref19-egg-password-product-red3` → `b7-ref19-egg-password-product-green2`, each individually LIST1/RUN1:0PASS/2FAIL →2PASS. Actual Read v2 Browse, Test Job and Browser-bound extraction produce67files/1,774,358native bytes across the19distinctly labelled fixtures, matching every independent expected byte and the exact output set.

The original prompt owner rejects generation99 stale replies, protects source/helper handles while waiting, and performs no output during preparation. Five actual encryption methods each cover wrong password, cancel, ciphertext damage and verifier/tag damage. All20failed-preparation scenarios retain zero candidate writes and commits, protected sources through `before_cleanup`, terminal cleanup and actual exclusive reopen/tree deletion. Worker controls additionally cover second-block corruption, first-write cancellation, sticky native failure, working-memory admission, unknown methods and invalid encryption-header boundaries. Real children are waited, stderr drained and incomplete owned tokens removed after actual exit.

## Preservation and quality

On final source, the sixteen individually listed production/password selections all pass: the two new EGG selections, twelve prior ordinary/split/readmission/ALZ-password product selections and the original password wait/retry/cancel and encrypted ZIP/7z/RAR extraction selections. Fresh ZIP crypto5, methods13 and boundary2 LIST/RUN selections also pass. Together with the47Worker tests, this is83successful GREEN executions, with zero FAIL/ignored/SKIP/not_run; it is not whole-B7 independent acceptance.

Archive all-target check and strict Clippy under isolated-codecs, default-library strict Clippy, ten-file scoped rustfmt and the source/link/structure documentation gate all exit0. The authoritative matching RED/GREEN worker builds and C# builds exit0; C# reports zero warnings/errors. Commands, environment, actual counts, raw logs, paired inputs and source attribution are retained in `evidence/ref19/egg-password/`. Original B7 baseline ownership remains `b7-start`; this slice's before-copies do not replace it. No dependency, C# production, Protocol/IPC/Core/Desktop/UI, native ownership or deployment changes are included.

## Limits

Encrypted split, solid+encrypted, encrypted filenames/global headers and non-ASCII passwords remain separately unverified or unsupported. Encrypted Stored/LZMA and LEA+Bzip2 producer profiles are not independently qualified. Short AES/LEA metadata without the full authentication tag is outside this authenticated profile and remains a separate framing/typed-outcome qualification.

Native metadata restoration retains the original contract: Windows cannot restore FILETIME0 through the supported API, so it reports `metadata_time_not_supported` rather than claiming restoration. Specification fixtures use the fixed recoverable FILETIME132223104000000000. Physical UI, installed and Release acceptance remain separate.
