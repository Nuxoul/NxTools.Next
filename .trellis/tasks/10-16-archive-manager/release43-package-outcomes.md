# Formal Release43

2026-10-31. Current source packaged through `scripts/package-local.ps1 -SkipVerification`, reusing [HZ5 independent scoped acceptance](hz5-independent-acceptance.md). All896accepted source/driver inputs remain unchanged. Actual Release compilation, original production signing and normal Inno installer generation completed.

## Artifact and verification

- Version0.7.0 / sequence43 / releaseID`1f864d3a-907e-4c92-9a9e-08b8669ab164`.
- Installer:`artifacts/installer/NxTools.Next-setup-0.7.0.exe`,26,956,659bytes /25.71MiB.
- SHA256:`F904E16B8D21243BF2A8F6E68FC4C8A9A78D52F7CD873DEE1321D2B10ECD645A`; adjacent checksum matches.
- RSA3072-PSS/SHA256 independently verified using the existing public trust anchor. All77signed members and the exact79-file package inventory match.
- All10actual build outputs match the package. Five embedded archive/helper approvals, public trust and helper Desktop identity binding match.
- Manifest SHA256:`68d597e23652592a7682067ff98bf5ed9fa8036898543af0d19923ea69184f51`.
- Installer runtime11and BrowserHost replacement5isolated scenarios passed. No full Rust/C# suite summaries occur in the packaging log; no HZ5matrix rerun. Source drift is zero.
- Installer helper SHA256 remains`4CDFEAC77D33715CD6800AD27CDCC3D60F58E05198500FEB8CECAAD36A76FC1A`, matching the prior [running-upgrade acceptance](installer41-running-upgrade-outcomes.md).
- Windows Authenticode isNotSigned, separate from the verified internal release signature.

Persistent evidence:`.diagnostics/packaging/20261031-094524-f497799b/`, stateCompleted/exit0; `independent-verification.json` and `verification-reuse.json` retain delivery checks and reuse attribution. Source baselines and controller evidence:`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\repackage-20261031-094522`. Original Release42backups and historical records remain retained.

## Included capability

HZ5progressive source-space reclamation is default-off and requires separate irreversible confirmation. The supported slice is ordinary single-volume unencrypted Stored ZIP, complete file units and same-volume localNTFS. Durable output publication and journal authority precede payload-only deallocation. New-process continuation completes remaining qualified extraction; ordinary extraction remains source-preserving.

The existing independent61-case acceptance is reused:61PASS/0FAIL/0SKIP, comprising37remaining-extraction completions,20preserving refusals and4ordinary source-preservation controls. These are prior acceptance results, not newly executed packaging tests.

## Limitations

No installation or product launch was performed. Release41running-upgrade acceptance remains historical installed evidence, not installation proof for Release43. Physical Explorer/GUI/DPI/user acceptance and power-loss durability are not certified by packaging.

HZ5does not enable Deflate, encryption, ZIP64, SFX, split archives, other formats, cross-volume or nonNTFSreclamation. Continuation does not reconstruct the complete original archive or recover missing already-consumed outputs. The canonical development stage remains **Phase E — Format expansion and ZIP editing**.
