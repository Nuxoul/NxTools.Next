# HZ4-7Z02 — narrowly approved secondary-reader limitation classification

2026-10-29. Decision during bounded Step6 repair, **Phase E — Format expansion and ZIP editing**. This is **not HZ4 acceptance**, a backend/vendor upgrade, writer/metadata permission change, or permission to overwrite [original failures](hz4-acceptance.md). It resolves the implementation session's explicit oracle-contract question only; all other repair/compiled/process checks remain pending.

## Parent independently verified evidence

Parent root `C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\hz4-reader-decision-20261029-181244`; `result.json` and six exact reader stdout/stderr artifacts retained. Parent script `hz4-7z02-reader-check.py` copies three private witness files to parent scratch, verifies their SHA256, signature/start-header CRC, next-header CRC and exact header/body differences, then executes pinned stock7Zip and actual system bsdtar. No product write, process crash, source mutation or compilation occurred.

| Private witness | Stock7Zip test | Actual bsdtar -tf |
| --- | --- | --- |
| valid canonical partial attributes | exit0 | exit1 |
| nonconforming bitmap/external byte order | exit2/HeadersError | exit0 |
| all-defined fixture-only attributes | exit0 | exit0 |

All three use identical packed file-stream bytes. All CRCs pass. The valid and nonconforming headers differ only at header positions164/165, swapping the bitmap/external bytes; signature CRC changes are recomputed. Valid WinAttributes property is **`15 0f 00 e0 00 20 00 00 00 20 00 00 00 10 00 00 00`**: property0x15/length15/all-defined0/MSB bitmapE0/inline external0, values32,32,16 for first three entries, fourth new directory attribute remains **undefined**. All-defined witness privately invents fourth value0x10 solely to discriminate reader behavior; **never transfer that invented metadata to product output**.

Pinned reader binaries: stock7Zip SHA256 `4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F`, system bsdtar SHA256 `92FAFED485AF0FE4C291457DC3C00D0241F6A136731BAB63EF18A3F0C949D37A`.

Parent verified reference7Zip commit `b88c6bc09339de32f7d18bbab900686b244f42ba`: `7zIn.cpp::ReadBoolVector2`1158/1176 and WinAttrib1386–1391 read all-defined+optional bitmap, then stream switch/external, then definedDWORDs. `7zOut.cpp::WriteAlignedBools`482–491 and attribute808–809 write that same order. Exact source SHA256: In `8120BE5248C001FF799C724B4DAF4F1DC22AA4768FE01B355BD9269FF6CB87F8`, Out `5C4B214737EFC80A0615DDC1072D118593539BFD38B287DC9E9B2895B4D13C2C`.

Parent read the locally saved upstream libarchivev3.7.7 `archive_read_support_format_7zip.c`2698–2724: its Attributes case first takes2bytes, then calls read_Bools for partial definitions. Snapshot SHA256 **8C9CA81FBCD15CD12B2E9701B232FC810E00FE83F49FF0DE2556948F5D6A2821**, URL recorded by implementation. This source is a reference snapshot, **not claimed byte-identical source provenance of Microsoft's built executable**. The independently replayed actual-binary reversal above establishes observed installed-reader behavior without that stronger claim.

## Approved contract — narrow and explicit

1. Continue to execute and record bsdtar for each relevant archive. If it succeeds, record actual PASS. Only when its failure matches this **proved well-formed partial inline WinAttributes fixture structure** and pinned reader identity may it be classified **reader-unsupported**, not reader PASS. The demonstrated fixture has4entries/definitions[true,true,true,false], the last is the newly added directory, canonical inline ordering and preserved three original values. No blanket7z/bsdtar failure exemption, extension-only check or error-string-only classifier. If broadening beyond this witnessed structure is required, supply separate format/byte evidence first.
2. Classification is test/oracle bookkeeping only, not product admission/authorization or an output rewrite. Actual encoded/decoded header bounds, CRCs and known metadata structure must be validated; inability to identify the exact construct remains FAIL. A conforming-structure check must not accept the swapped nonconforming witness that stock7Zip rejects.
3. **Mandatory unchanged gates**: stock7Zip full `t` plus exact individual member set/kinds/content/known metadata; original candidate full verification/readback/closed readers/held-source/FILE-only one publication; source old/new integrity and expected crash-seam receipt; current fresh-process refusal and preserved disk facts. Any primary/content/metadata/native/restart failure remains case FAIL regardless of partial attributes.
4. Preserve actual undefined attributes for the new directory and all old member values. No metadata removal, fabricated0x10, default-profile substitution, relaxed CRC/budget/source checks, writer format change, alternate backend/vendor or new oracle binary merely to obtain green.
5. Keep all original7Z02 FAIL evidence immutable. New result accounting must separately show process/integrity verdicts and reader verdicts, e.g. primary process cases PASS with **bsdtar N PASS / M reader-unsupported**. Do not describe this as all readers passing, no limitations, or suppress an unsupported result as an ordinary PASS/zero-skip claim. Other supported/all-defined structures continue mandatory bsdtar checks.
6. Retain executable controls: canonical partial0/1, malformed-order2/0, all-defined0/0, plus at least original product-output full primary manifest/native/refusal evidence. Unknown/new reader versions or hashes are not automatically covered.

Parent approves implementation of only this evidence-driven classification and continued remaining7Z01/Release checks in the current bounded repair. No need for a human behavior/design choice to resolve this test-tool discrepancy; no new destructive or packaging authorization follows. Implementation must deliver current source/hash/counts and stop for final independent HZ4 narrow acceptance. HZ5 and experience packaging remain unstarted pending that stable handoff.
