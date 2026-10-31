"""Derive approved NxTools icons using the already-installed Pillow (no new dependency).

Run normally to regenerate checked-in resources; --check compares bytes without writing.
Original design PNGs are hash-pinned, read-only inputs. No product, registry or shell APIs.
"""
from __future__ import annotations

import argparse
import hashlib
import io
from pathlib import Path
import struct

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / ".trellis/tasks/10-16-archive-manager/assets"
BRAND = ROOT / "rust/apps/nxtools-desktop/assets/brand"
SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256)
DESIGNS = (
    ("nxtools", "app-icon-blue-nx-approved.png", "17d3a0f8c423595a62cf991c7d62d01abdb39730681f4d8e7e6401352cd3452f"),
    ("nxtools-archive", "archive-icon-orange-nx-approved.png", "b0e3da8afb6d5f3bc7061ade4b9ac8344c01f976e26dcc0f13cc4a3a3952565b"),
)


def png_bytes(image: Image.Image) -> bytes:
    output = io.BytesIO()
    image.save(output, format="PNG", compress_level=9)
    return output.getvalue()


def encode_ico(frames: dict[int, bytes]) -> bytes:
    """Windows PNG ICO: each frame is independently resized, including zero=256 directory size."""
    directory = bytearray(struct.pack("<HHH", 0, 1, len(frames)))
    offset = 6 + 16 * len(frames)
    for size, data in frames.items():
        width = size if size < 256 else 0
        directory.extend(struct.pack("<BBBBHHII", width, width, 0, 0, 1, 32, len(data), offset))
        offset += len(data)
    return bytes(directory) + b"".join(frames.values())


def render_design(source: Path, expected_hash: str) -> dict[int, Image.Image]:
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != expected_hash:
        raise ValueError(f"Approved original changed: {source.name}")
    image = Image.open(io.BytesIO(data))
    if image.mode != "RGBA" or image.size != (1254, 1254):
        raise ValueError(f"Unexpected approved original format: {source.name}")
    # Generation left invisible alpha1 speckles touching the archive canvas edges.
    # Only derivatives discard alpha<8 (<3.2%); all visible artwork/colors/Nx remain intact.
    image.putdata([(red, green, blue, alpha) if alpha >= 8 else (0, 0, 0, 0)
                   for red, green, blue, alpha in image.get_flattened_data()])
    bounds = image.getchannel("A").getbbox()
    if bounds is None:
        raise ValueError(f"Approved original is empty: {source.name}")
    artwork = image.crop(bounds)
    print(f"{source.name}: RGBA1254 alpha crop>=8={bounds}; originals unchanged")
    frames = {}
    for size in SIZES:
        # Common 7/8 occupancy, aspect-preserving and centered. At16px this leaves >=1px
        # transparent breathing room; never crop the visible Nx or stretch the archive box.
        extent = round(size * 7 / 8)
        factor = extent / max(artwork.size)
        dimensions = tuple(max(1, round(value * factor)) for value in artwork.size)
        resized = artwork.convert("RGBa").resize(dimensions, Image.Resampling.LANCZOS).convert("RGBA")
        canvas = Image.new("RGBA", (size, size))
        canvas.paste(resized, ((size - dimensions[0]) // 2, (size - dimensions[1]) // 2))
        frames[size] = canvas
    return frames


def resource_outputs() -> dict[Path, bytes]:
    outputs = {}
    designs = {}
    for name, filename, digest in DESIGNS:
        frames = render_design(SOURCE / filename, digest)
        designs[name] = frames
        encoded = {size: png_bytes(frame) for size, frame in frames.items()}
        for size, data in encoded.items():
            outputs[BRAND / f"{name}-{size}.png"] = data
        outputs[BRAND / f"{name}.ico"] = encode_ico(encoded)
        if name == "nxtools":
            # Preserve original extension entry names/package recursion; no new browser consumer.
            for size in (16, 32, 48, 128):
                outputs[ROOT / f"browser_extension/nxtools-interceptor/icons/icon-{size}.png"] = encoded[size]
    # Offline evidence only, native pixel sizes against both backgrounds, no DPI/visual certification.
    sheet = Image.new("RGB", (800, 1280), "white")
    draw = ImageDraw.Draw(sheet)
    for row, (name, background) in enumerate((name, background) for name in designs for background in ("#f4f4f5", "#18181b")):
        top = row * 320
        draw.rectangle((0, top, 800, top + 319), fill=background)
        foreground = "#18181b" if background == "#f4f4f5" else "#f4f4f5"
        draw.text((12, top + 8), f"{name} / {background} / actual pixels", fill=foreground)
        left = 12
        for size, frame in designs[name].items():
            draw.text((left, top + 28), str(size), fill=foreground)
            sheet.paste(frame, (left, top + 48), frame)
            left += size + 16
    outputs[SOURCE / "approved-icon-size-proof.png"] = png_bytes(sheet)
    return outputs


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Compare exact generated bytes, write nothing")
    args = parser.parse_args()
    outputs = resource_outputs()
    different = []
    for path, data in outputs.items():
        if args.check:
            if not path.is_file() or path.read_bytes() != data:
                different.append(str(path.relative_to(ROOT)))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    if different:
        print("STALE/MISSING approved resources:\n" + "\n".join(different))
        return 1
    print(f"{'Verified' if args.check else 'Generated'} {len(outputs)} resources: 2 ICO/18 PNG/4 extension PNG/1 size proof; 9 real sizes each")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
