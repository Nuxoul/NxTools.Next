"""Compare real 100%-scale GPUI readbacks for the standalone search surface."""
import argparse
import json
import math
from pathlib import Path

from PIL import Image


def verify(baseline: Path, candidate: Path) -> None:
    before_meta = json.loads((baseline / "frame.json").read_text())
    after_meta = json.loads((candidate / "frame.json").read_text())
    assert before_meta == after_meta, "candidate changed viewport or DPI"
    assert after_meta["scale"] == 1.0, "run this fixed-pixel fixture at 100% DPI"
    before = Image.open(baseline / "frame-premultiplied.png").convert("RGBA")
    after = Image.open(candidate / "frame-premultiplied.png").convert("RGBA")
    assert before.size == after.size
    assert after.width == 652 and after.height in (91, 92)

    # The sample lies 0.5147px outside an 8px circle, on its diagonal arc.
    # A one-pixel fixed ramp discards it. The L1 pixel footprint includes it.
    normal = (6.5, 5.5)
    distance_to_center = math.hypot(*normal)
    distance = distance_to_center - 8.0
    pixel_width = sum(normal) / distance_to_center
    expected_coverage = max(0.0, min(1.0, 0.5 - distance / pixel_width))
    corners = [(17, 18), (634, 18), (17, 73), (634, 73)]
    for point in corners:
        background_alpha = before.getpixel(point)[3] / 255.0
        # At this sample the released group paints one fill and one border quad.
        # Bound their source-over result so a squared-off/expanded corner fails.
        minimum_alpha = math.floor(
            (expected_coverage + background_alpha * (1.0 - expected_coverage)) * 255
        ) - 2
        maximum_alpha = math.ceil(
            (1.0 - (1.0 - background_alpha) * (1.0 - expected_coverage) ** 2) * 255
        ) + 2
        actual = after.getpixel(point)[3]
        assert minimum_alpha <= actual <= maximum_alpha, (
            f"arc pixel {point} coverage={actual}, expected={minimum_alpha}..{maximum_alpha}"
        )

    # Unchanged straight edges and an opaque interior must remain byte-exact.
    for point in [(16, 46), (635, 46), (326, 16), (326, 75), (400, 68)]:
        assert after.getpixel(point) == before.getpixel(point), (
            f"straight edge/interior changed at {point}"
        )
    assert after.getpixel((400, 68))[3] == 255
    assert after.getpixel((0, 0))[3] == 0
    print("PASS GPU arc coverage; straight edges/interior and viewport unchanged")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()
    verify(args.baseline, args.candidate)


if __name__ == "__main__":
    main()
