"""Check all four opaque 1px/r8 InputGroup corners against one coverage application."""
import argparse
import json
import math
from pathlib import Path

from PIL import Image


def clamp(value: float) -> float:
    return max(0.0, min(1.0, value))


def verify(baseline: Path, candidate: Path) -> None:
    before_meta = json.loads((baseline / "frame.json").read_text())
    after_meta = json.loads((candidate / "frame.json").read_text())
    assert before_meta == after_meta, "viewport/DPI/fixture state changed"
    assert after_meta["scale"] == 1.0 and after_meta["shadow"] is False
    before = Image.open(baseline / "frame-premultiplied.png").convert("RGBA")
    after = Image.open(candidate / "frame-premultiplied.png").convert("RGBA")
    assert before.size == after.size and after.width == 652
    fill = before.getpixel((400, 68))
    border = before.getpixel((16, 46))
    assert fill[3] == border[3] == 255, "this oracle covers opaque fill and solid border"
    for point in [(16, 46), (635, 46), (326, 16), (326, 75), (400, 68)]:
        assert before.getpixel(point) == after.getpixel(point), f"non-arc pixel changed: {point}"
    maximum = (0.0, None, None, None)
    for center_x, center_y, left, top in [
        (24, 24, 16, 16), (628, 24, 628, 16),
        (24, 68, 16, 68), (628, 68, 628, 68),
    ]:
        for y in range(top, top + 8):
            for x in range(left, left + 8):
                distance = math.hypot(x + 0.5 - center_x, y + 0.5 - center_y) - 8.0
                outer = clamp(0.5 - distance)
                inner_border = clamp(1.5 + distance)
                expected = tuple(
                    outer * (fill[channel] * (1.0 - inner_border) + border[channel] * inner_border)
                    for channel in range(3)
                ) + (outer * 255,)
                actual = after.getpixel((x, y))
                error = max(abs(actual[channel] - expected[channel]) for channel in range(4))
                if error > maximum[0]:
                    maximum = (error, (x, y), actual, expected)
    print(f"GPU corner model maximum byte-channel error: {maximum[0]:.3f}; point={maximum[1]}")
    assert maximum[0] <= 3.0, f"fill/border coverage mismatch: {maximum}"
    assert after.getpixel((0, 0)) == (0, 0, 0, 0)
    print("PASS 256 corner pixels; straight edges/interior and viewport unchanged")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()
    verify(args.baseline, args.candidate)
