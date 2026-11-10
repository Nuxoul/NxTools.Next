"""Bundle the current archive worker and dependency sources for LGPL relinking."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib
import zipfile

MEMBERS = [f"rust/crates/{name}" for name in ("nxtools-archive", "nxtools-ipc", "nxtools-protocol")]


def worker_workspace(text, members):
    text, changed = re.subn(
        r"(?ms)(^\[workspace\]\s*\n.*?^members\s*=\s*)\[.*?^\]",
        lambda match: match[1] + "[\n" + "".join(f'    "{name}",\n' for name in members) + "]",
        text, count=1,
    )
    if changed != 1:
        raise ValueError("Workspace member declaration not found")
    # Desktop-only patches are not inputs to this standalone worker workspace.
    return re.sub(r"(?ms)^\[patch\.[^\n]+\]\n.*?(?=^\[|\Z)", "", text)


def cargo_tool(root):
    toolchain = tomllib.loads((root / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    rustup = Path(os.environ.get("RUSTUP_HOME", Path.home() / ".rustup"))
    native = rustup / "toolchains" / f"{toolchain}-x86_64-pc-windows-msvc" / "bin/cargo.exe"
    return str(native if native.is_file() else "cargo")


def run_cargo(cargo, arguments, directory):
    environment = os.environ.copy()
    environment.update(CARGO_BUILD_JOBS="1", CARGO_INCREMENTAL="0", RUSTFLAGS="-C codegen-units=256")
    if Path(cargo).is_file():
        environment["PATH"] = str(Path(cargo).parent) + os.pathsep + environment["PATH"]
    result = subprocess.run([cargo, *arguments], cwd=directory, env=environment, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("utf-8", errors="replace")[-6000:])
    return result.stdout


def write_archive(stage, output):
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_suffix(".zip.tmp")
    try:
        with zipfile.ZipFile(temporary, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6, strict_timestamps=False) as archive:
            for path in sorted(stage.rglob("*")):
                if path.is_file():
                    archive.write(path, path.relative_to(stage).as_posix())
        temporary.replace(output)
    finally:
        temporary.unlink(missing_ok=True)


def build_distribution(root, output, work_directory):
    cargo = cargo_tool(root)
    stage = Path(tempfile.mkdtemp(prefix="as-", dir=work_directory))
    try:
        (stage / "Cargo.toml").write_text(worker_workspace((root / "Cargo.toml").read_text(), MEMBERS), encoding="utf-8")
        for name in ["Cargo.lock", "rust-toolchain.toml", "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md"]:
            shutil.copy2(root / name, stage / name)
        for member in MEMBERS:
            target = stage / member
            target.mkdir(parents=True)
            shutil.copy2(root / member / "Cargo.toml", target / "Cargo.toml")
            shutil.copytree(root / member / "src", target / "src")
            build_script = root / member / "build.rs"
            if build_script.is_file():
                shutil.copy2(build_script, target / "build.rs")
        # Start from the product lock, pruning only unrelated workspace packages.
        # Missing published sources may be downloaded; versions must not change.
        metadata = json.loads(run_cargo(cargo, ["metadata", "--format-version", "1", "--features", "nxtools-archive/isolated-codecs"], stage))
        original = tomllib.loads((root / "Cargo.lock").read_text())["package"]
        versions = {(package["name"], package["version"], package.get("source")) for package in original}
        for package in metadata["packages"]:
            if package["source"] and (package["name"], package["version"], package["source"]) not in versions:
                raise ValueError(f"Source bundle changed a locked dependency: {package['name']} {package['version']}")
        vendor = stage / "vendor"
        run_cargo(cargo, ["vendor", "--offline", "--locked", "--versioned-dirs", str(vendor)], stage)
        bundled = []
        for package in metadata["packages"]:
            if package["source"] is None:
                continue
            if not package["source"].startswith("registry+"):
                raise ValueError(f"Unimplemented source kind: {package['source']}")
            destination = vendor / f"{package['name']}-{package['version']}"
            if not (destination / ".cargo-checksum.json").is_file():
                raise ValueError(f"Missing Cargo source metadata: {destination.name}")
            bundled.append({"name": package["name"], "version": package["version"], "license": package["license"]})
        (stage / ".cargo").mkdir()
        (stage / ".cargo/config.toml").write_text(
            '[source.crates-io]\nreplace-with = "bundled-sources"\n'
            '[source.bundled-sources]\ndirectory = "vendor"\n'
            '[net]\noffline = true\n', encoding="utf-8",
        )
        required = {"wim-format", "wim-types", "ms-compress"}
        if not required.issubset({package["name"] for package in bundled}):
            raise ValueError("Incomplete WIM source distribution")
        shutil.copy2(root / "third_party/archive-worker/REBUILD.md", stage / "REBUILD.md")
        (stage / "DEPENDENCIES.json").write_text(json.dumps(bundled, indent=2), encoding="utf-8")
        # This is the delivered offline source configuration, not the machine registry.
        run_cargo(cargo, ["metadata", "--offline", "--locked", "--format-version", "1", "--features", "nxtools-archive/isolated-codecs"], stage)
        write_archive(stage, output)
        materials = output.parent
        shutil.copy2(root / "third_party/archive-worker/REBUILD.md", materials / "REBUILD.md")
        for package_name in sorted(required):
            package = next(package for package in metadata["packages"] if package["name"] == package_name)
            source = Path(package["manifest_path"]).parent
            destination = materials / "licenses" / f"{package_name}-{package['version']}"
            destination.mkdir(parents=True, exist_ok=True)
            for path in source.iterdir():
                if path.is_file() and (path.name.upper().startswith(("LICENSE", "COPYING", "NOTICE")) or path.name == "README.md"):
                    shutil.copy2(path, destination / path.name)
        print(json.dumps({"archive": str(output), "dependency_sources": len(bundled), "application_crates": len(MEMBERS), "offline_resolution": "passed"}))
    finally:
        shutil.rmtree(stage)


def write_distribution(installer, materials, root):
    output = installer.parent / installer.name.replace("-setup-", "-release-").replace(".exe", ".zip")
    temporary = output.with_suffix(".zip.tmp")
    try:
        with zipfile.ZipFile(temporary, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6, strict_timestamps=False) as archive:
            archive.write(installer, installer.name)
            for name in ["LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md"]:
                archive.write(root / name, name)
            for path in sorted(materials.rglob("*")):
                if path.is_file():
                    archive.write(path, path.relative_to(installer.parent).as_posix())
        temporary.replace(output)
    finally:
        temporary.unlink(missing_ok=True)
    print(json.dumps({"distribution": str(output), "bytes": output.stat().st_size}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--installer", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    work = Path(os.environ.get("PI_SCRATCH_DIR", tempfile.gettempdir()))
    if args.installer:
        write_distribution(args.installer.resolve(), args.output.resolve().parent, root)
    else:
        build_distribution(root, args.output.resolve(), work)


if __name__ == "__main__":
    main()
