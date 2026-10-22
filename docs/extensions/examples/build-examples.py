"""Explicit deterministic example ZIP build; never installs or runs extensions."""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import zipfile

ROOT = Path(__file__).resolve().parent
PACKAGES = ("github-public-proxy", "download-link-list", "clipboard-text-tools")
OUTPUT = ROOT.parents[2] / "artifacts" / "extension-examples"
STAMP = (1980, 1, 1, 0, 0, 0)


def unique_object(pairs):
    obj = {}
    for key, value in pairs:
        if key in obj:
            raise ValueError("duplicate manifest key")
        obj[key] = value
    return obj


def archive_bytes(root):
    root = Path(root)
    if root.is_symlink() or not root.is_dir():
        raise ValueError("package must be an ordinary directory")
    manifest_path = root / "extension.json"
    if manifest_path.is_symlink():
        raise ValueError("manifest must be an ordinary file")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    files = manifest.get("files")
    if not isinstance(files, list) or not all(isinstance(name, str) for name in files):
        raise ValueError("manifest files must be explicit paths")
    names = ["extension.json", *sorted(files)]
    if len({name.casefold() for name in names}) != len(names):
        raise ValueError("duplicate archive path")
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as archive:
        for name in names:
            relative = PurePosixPath(name)
            if (not name or "\\" in name or ":" in name or relative.is_absolute()
                    or any(part in ("", ".", "..") for part in name.split("/"))):
                raise ValueError("unsafe manifest path")
            path = root
            for part in relative.parts:
                path = path / part
                if path.is_symlink():
                    raise ValueError("symlinks are not package files")
            if not path.is_file() or not path.resolve().is_relative_to(root.resolve()):
                raise ValueError("manifest-listed regular file missing")
            info = zipfile.ZipInfo(name, STAMP)
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            info.compress_type = zipfile.ZIP_STORED  # Stable across zlib/Python versions.
            archive.writestr(info, path.read_bytes())
    return output.getvalue()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", action="store_true", help="write only the three example ZIPs under artifacts/extension-examples")
    args = parser.parse_args()
    if not args.build:
        parser.error("explicit --build is required; no files written")
    # Validate/snapshot all inputs before any output is written.
    bundles = [(name, archive_bytes(ROOT / name)) for name in PACKAGES]
    if OUTPUT.is_symlink():
        raise ValueError("output directory must not be a symlink")
    OUTPUT.mkdir(parents=True, exist_ok=True)
    for name, data in bundles:
        target = OUTPUT / (name + ".zip")
        if target.is_symlink() or target.exists() and target.read_bytes() != data:
            raise ValueError("refusing to overwrite different existing example artifact")
        if not target.exists():
            with target.open("xb") as stream:
                stream.write(data)
        print(f"{target.relative_to(ROOT.parents[2]).as_posix()}  sha256={hashlib.sha256(data).hexdigest()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
