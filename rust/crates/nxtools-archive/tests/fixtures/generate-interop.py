"""Small independent x86 BCJ+LZMA2 fixture; uses explicitly supplied stock 7-Zip."""
import argparse
import hashlib
import json
import pathlib
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--sevenzip", required=True, type=pathlib.Path)
parser.add_argument("--output", required=True, type=pathlib.Path)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
payload = b"".join(b"\x55\x8b\xec\xe8" + (4096 - i * 16).to_bytes(4, "little", signed=True)
                   + b"\xe9" + (-i * 16 - 13).to_bytes(4, "little", signed=True)
                   + b"\x90\x5d\xc3" for i in range(256))
assert len(payload) == 4096
(args.output / "x86.bin").write_bytes(payload)
archive = args.output / "x86-bcj-lzma2.7z"
if archive.exists():
    archive.unlink()  # Only this producer's named fixture output.
commands = [
    ["a", "-t7z", "-m0=BCJ", "-m1=LZMA2:d=64k", "-ms=off", "-mhc=off",
     "-mtc=off", "-mta=off", "-mtm=off", "x86-bcj-lzma2.7z", "x86.bin"],
    ["l", "-slt", "-sccUTF-8", "x86-bcj-lzma2.7z"],
    ["t", "x86-bcj-lzma2.7z"],
    ["x", "-y", "-ooracle", "x86-bcj-lzma2.7z"],
]
outputs = []
for command in commands:
    result = subprocess.run([str(args.sevenzip), *command], cwd=args.output,
                            capture_output=True, encoding="utf-8", errors="strict", check=True)
    assert not result.stderr, result.stderr
    outputs.append(result.stdout)
listing = outputs[1]
assert "BCJ" in listing and "LZMA2" in listing and "Blocks = 1" in listing
assert "Method = LZMA2:12 BCJ" in listing and "Method = BCJ LZMA2:12" in listing, listing
assert (args.output / "oracle/x86.bin").read_bytes() == payload
(args.output / "oracle/x86.bin").unlink()
(args.output / "oracle").rmdir()
(args.output / "producer-list.txt").write_text(listing, encoding="utf-8")
manifest = {
    "producer": outputs[0].strip().splitlines()[0],
    "executable_sha256": hashlib.sha256(args.sevenzip.read_bytes()).hexdigest(),
    "commands": commands,
    "coder_chain_proof": "7-Zip -slt archive Method=LZMA2:12 BCJ and member Method=BCJ LZMA2:12; explicit BCJ+LZMA2, dictionary reduced for 4096-byte input",
    "test_exit": 0, "independent_extract_exact": True,
    "files": {name: {"size": (args.output / name).stat().st_size,
                      "sha256": hashlib.sha256((args.output / name).read_bytes()).hexdigest()}
              for name in ["x86.bin", "x86-bcj-lzma2.7z"]},
}
(args.output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
print(json.dumps(manifest, indent=2))
