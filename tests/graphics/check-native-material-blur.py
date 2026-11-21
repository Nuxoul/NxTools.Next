"""Check the owned 100%-DPI GPUI/DWM workbench over its static test backdrop."""
import json
import sys
from PIL import Image


def luminance(pixel):
    red, green, blue = pixel[:3]
    return 0.2126 * red + 0.7152 * green + 0.0722 * blue


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[round((len(ordered) - 1) * fraction)]


def inspect(path):
    image = Image.open(path).convert("RGB")
    if image.size != (900, 600):
        raise AssertionError("Expected the client-only 900x600-DIP native workbench at 100% DPI")
    # These areas contain no GPUI text or controls. The static native backdrop
    # supplies 16px stripe edges above and a large color patch below.
    stripe = [sum(luminance(image.getpixel((x, y))) for y in range(190, 198)) / 8 for x in range(392, 842)]
    edge = percentile([abs(right - left) for left, right in zip(stripe, stripe[1:])], 0.95)
    macro = [sum(luminance(image.getpixel((x, y))) for y in range(540, 548)) / 8 for x in range(442, 842)]
    transmission = percentile(macro, 0.8) - percentile(macro, 0.2)
    result = {"stripe_edge_p95": round(edge, 3), "background_luminance_range": round(transmission, 3)}
    print(json.dumps(result))
    if edge > 4.0:
        raise AssertionError("Native backdrop stripes remain sharp: transparency is not blur")
    if transmission < 3.0:
        raise AssertionError("Backdrop color is not transmitted: an opaque fill is not Acrylic")
    print("PASS: real desktop composition blurs fine stripes and retains broad background color")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: check-native-material-blur.py <owned-native-workbench.png>")
    inspect(sys.argv[1])
