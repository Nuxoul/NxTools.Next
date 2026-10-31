"""Executable, offline resource/consumer regression. Never loads a product or Windows Shell."""
from __future__ import annotations

import argparse
import hashlib
import io
from pathlib import Path
import sys
import struct
import unittest

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
SOURCES = ROOT / ".trellis/tasks/10-16-archive-manager/assets"
BRAND = ROOT / "rust/apps/nxtools-desktop/assets/brand"
SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256)
ICONS = (
    ("nxtools", "app-icon-blue-nx-approved.png", "17d3a0f8c423595a62cf991c7d62d01abdb39730681f4d8e7e6401352cd3452f"),
    ("nxtools-archive", "archive-icon-orange-nx-approved.png", "b0e3da8afb6d5f3bc7061ade4b9ac8344c01f976e26dcc0f13cc4a3a3952565b"),
)


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8-sig")


def ico_frames(data: bytes) -> dict[int, Image.Image]:
    """Independently parse the actual directory/PNG payload; reject nominal-only sizes."""
    reserved, kind, count = struct.unpack_from("<HHH", data)
    assert (reserved, kind, count) == (0, 1, len(SIZES)), "ICO must have nine real entries"
    frames = {}
    cursor = 6 + count * 16
    for index in range(count):
        width, height, colors, zero, planes, depth, length, offset = struct.unpack_from("<BBBBHHII", data, 6 + index * 16)
        size = width or 256
        assert size == (height or 256) and size not in frames
        assert (colors, zero, planes, depth) == (0, 0, 1, 32)
        assert offset == cursor and length > 0 and offset + length <= len(data)
        payload = data[offset:offset + length]
        assert payload.startswith(b"\x89PNG\r\n\x1a\n"), "real alpha PNG frame required"
        image = Image.open(io.BytesIO(payload))
        image.load()
        assert image.mode == "RGBA" and image.size == (size, size), "directory dimensions must match decoded pixels"
        frames[size] = image
        cursor += length
    assert cursor == len(data) and tuple(frames) == SIZES
    # A second independent decoder must agree, including 256 encoded as zero in ICONDIRENTRY.
    decoded = Image.open(io.BytesIO(data))
    assert decoded.ico.sizes() == {(size, size) for size in SIZES}
    for size, frame in frames.items():
        assert decoded.ico.getimage((size, size)).tobytes() == frame.tobytes()
    return frames


def pe_icon_payloads(data: bytes) -> dict[tuple[int, int, int], bytes]:
    """Read RT_ICON/RT_GROUP_ICON without loading/executing the PE or calling Shell."""
    def unpack(format_string, offset):
        return struct.unpack_from(format_string, data, offset)

    assert data[:2] == b"MZ"
    pe = unpack("<I", 0x3C)[0]
    assert data[pe:pe + 4] == b"PE\0\0"
    section_count = unpack("<H", pe + 6)[0]
    optional_size = unpack("<H", pe + 20)[0]
    optional = pe + 24
    magic = unpack("<H", optional)[0]
    assert magic in (0x10B, 0x20B)
    directories = optional + (112 if magic == 0x20B else 96)
    resource_rva, resource_size = unpack("<II", directories + 16)
    assert resource_rva and 0 < resource_size <= len(data)
    sections = []
    for index in range(section_count):
        section = optional + optional_size + index * 40
        virtual_size, address, raw_size, raw = unpack("<IIII", section + 8)
        sections.append((address, raw_size, raw))

    def physical(rva, length):
        for address, raw_size, raw in sections:
            if address <= rva and rva + length <= address + raw_size:
                offset = raw + rva - address
                assert offset + length <= len(data)
                return offset
        raise AssertionError("resource RVA is outside file-backed sections")

    base = physical(resource_rva, resource_size)
    resources = {}

    def walk(relative, keys):
        assert len(keys) <= 2 and relative + 16 <= resource_size
        named, count = unpack("<HH", base + relative + 12)
        assert relative + 16 + (named + count) * 8 <= resource_size
        for index in range(named + count):
            name, target = unpack("<II", base + relative + 16 + index * 8)
            if name & 0x80000000:
                continue  # Windows' standard icon IDs are numeric, never arbitrary named strings.
            if not keys and name not in (3, 14):
                continue
            key = (*keys, name)
            offset = target & 0x7FFFFFFF
            if target & 0x80000000:
                walk(offset, key)
            else:
                assert len(key) == 3 and offset + 16 <= resource_size and key not in resources
                rva, length = unpack("<II", base + offset)
                start = physical(rva, length)
                resources[key] = data[start:start + length]

    walk(0, ())
    return resources


def verify_desktop_pe(data: bytes) -> None:
    resources = pe_icon_payloads(data)
    for group_id, (name, _, _) in enumerate(ICONS, 1):
        groups = [(language, payload) for (kind, number, language), payload in resources.items() if kind == 14 and number == group_id]
        assert groups, f"Desktop missing RT_GROUP_ICON {group_id} ({name})"
        expected = ico_frames((BRAND / f"{name}.ico").read_bytes())
        for language, group in groups:
            assert struct.unpack_from("<HHH", group) == (0, 1, len(SIZES)), f"Desktop group{group_id} must contain nine real sizes"
            assert len(group) == 6 + 14 * len(SIZES)
            for index, size in enumerate(SIZES):
                width, height, colors, zero, planes, depth, length, image_id = struct.unpack_from("<BBBBHHIH", group, 6 + index * 14)
                assert ((width or 256), (height or 256), colors, zero, planes, depth) == (size, size, 0, 0, 1, 32)
                payload = resources[(3, image_id, language)]
                assert len(payload) == length
                actual = Image.open(io.BytesIO(payload))
                actual.load()
                assert actual.mode == "RGBA" and actual.size == (size, size)
                assert actual.tobytes() == expected[size].tobytes(), f"Desktop group{group_id} {size}px differs from approved pixels"


class ApprovedIconRegression(unittest.TestCase):
    def test_01_approved_originals_unchanged(self):
        for _, source, expected in ICONS:
            with self.subTest(source=source):
                data = (SOURCES / source).read_bytes()
                self.assertEqual(hashlib.sha256(data).hexdigest(), expected)
                image = Image.open(io.BytesIO(data))
                self.assertEqual((image.mode, image.size), ("RGBA", (1254, 1254)))

    def test_02_real_nine_size_ico_and_png_pixels(self):
        for name, _, _ in ICONS:
            with self.subTest(icon=name):
                frames = ico_frames((BRAND / f"{name}.ico").read_bytes())
                for size, frame in frames.items():
                    png = Image.open(BRAND / f"{name}-{size}.png")
                    self.assertEqual(png.tobytes(), frame.tobytes())
                    alpha = frame.getchannel("A")
                    self.assertEqual(alpha.getextrema(), (0, 255))
                    self.assertEqual([alpha.getpixel(point) for point in ((0, 0), (size - 1, 0), (0, size - 1), (size - 1, size - 1))], [0] * 4)
                    # No matte/background, no clipping into outer pixel. Ignore subvisible resample ringing.
                    visible = alpha.point(lambda value: 255 if value >= 8 else 0).getbbox()
                    self.assertIsNotNone(visible)
                    left, top, right, bottom = visible
                    self.assertTrue(left > 0 and top > 0 and right < size and bottom < size, (name, size, visible))
                    self.assertTrue(right - left >= size * 0.65 and bottom - top >= size * 0.65)
                    opaque = [pixel for pixel in frame.get_flattened_data() if pixel[3] >= 128]
                    self.assertGreater(len(opaque), size * size * 0.20)
                    red = sum(pixel[0] for pixel in opaque)
                    blue = sum(pixel[2] for pixel in opaque)
                    self.assertGreater(blue if name == "nxtools" else red, red if name == "nxtools" else blue)

    def test_03_desktop_resource_groups_have_distinct_roles(self):
        build = text("rust/apps/nxtools-desktop/build.rs")
        self.assertRegex(build, r'\(1,\s*"assets/brand/nxtools\.ico"\)')
        self.assertRegex(build, r'\(2,\s*"assets/brand/nxtools-archive\.ico"\)')
        self.assertIn("cargo:rerun-if-changed={path}", build)
        self.assertIn("{id} ICON", build)
        self.assertNotIn("browser_extension/", build)
        self.assertNotEqual((BRAND / "nxtools.ico").read_bytes(), (BRAND / "nxtools-archive.ico").read_bytes())

    def test_04_gpui_alias_and_browser_original_resource_entries(self):
        assets = text("rust/apps/nxtools-desktop/src/assets.rs")
        self.assertRegex(assets, r'"brand/nxtools\.png"\s*=>\s*Some\(include_bytes!\(\s*"\.\./assets/brand/nxtools-32\.png"')
        self.assertRegex(assets, r'"brand/archive\.png"\s*=>\s*Some\(include_bytes!\(\s*"\.\./assets/brand/nxtools-archive-32\.png"')
        self.assertIn('"brand/archive.png",', assets)
        for size in (16, 32, 48, 128):
            self.assertEqual((ROOT / f"browser_extension/nxtools-interceptor/icons/icon-{size}.png").read_bytes(), (BRAND / f"nxtools-{size}.png").read_bytes())

    def test_05_archive_progid_does_not_use_application_icon(self):
        plan = text("csharp/NxTools.PlatformHost/Windows/Archive/ArchiveIntegrationPlan.cs")
        # Negative resource ID, not position 1: robust to resource enumeration order.
        self.assertRegex(plan, r'Add\(changes, root \+ @"\\DefaultIcon", "", Quote\(executablePath\) \+ ",-2", root\)')
        self.assertRegex(plan, r'Add\(changes, root, "Icon", Quote\(executablePath\) \+ ",-2", root\)')
        self.assertNotRegex(plan, r'DefaultIcon[^\n]*",0"')
        fixture = text("csharp/NxTools.PlatformHost.Tests/ArchiveIntegrationPlanTests.cs")
        self.assertIn('NxTools.Archive\\DefaultIcon', fixture)
        self.assertIn(',-2', fixture)

    def test_06_installer_and_packaged_pixel_gate_use_approved_app(self):
        installer = text("installer/nxtools-next.iss")
        self.assertIn('SetupIconFile=..\\rust\\apps\\nxtools-desktop\\assets\\brand\\nxtools.ico', installer)
        self.assertIn('UninstallDisplayIcon={app}\\{#MyAppExeName}', installer)
        gate = text("tests/architecture/desktop-product-icon.ps1")
        self.assertIn('assets\\brand\\nxtools-32.png', gate)
        self.assertNotIn('nxtools-interceptor\\icons', gate)
        self.assertIn("--desktop-pe $desktopPath", gate)
        lifecycle = text("tests/architecture/desktop-shell-lifecycle.ps1")
        self.assertIn("generate-approved-icons.py') --check", lifecycle)
        self.assertIn("tests\\architecture\\approved-icons.py", lifecycle)

    def test_07_existing_window_tray_and_release_owners_are_retained(self):
        self.assertIn('PCWSTR(1 as _)', text('vendor/gpui-pre-windows-039/src/platform.rs'))
        self.assertIn('hIcon: icon_handle', text('vendor/gpui-pre-windows-039/src/window.rs'))
        self.assertIn('DesktopExecutableIcon.ExtractProductIcon(desktopExecutable)', text('csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs'))
        self.assertIn('Icon.ExtractAssociatedIcon(desktopExecutable)', text('csharp/NxTools.PlatformHost/Windows/DesktopExecutableIcon.cs'))
        self.assertIn("desktop-product-icon.ps1", text("scripts/package.ps1"))
        self.assertIn('NxTools.exe', text('csharp/NxTools.PlatformHost/Windows/TrustedReleaseManifest.cs'))
        # Icons are embedded in the original authenticated Desktop, not an unsigned runtime sibling.
        self.assertNotRegex(text('scripts/package.ps1'), r'Copy-PackageFile[^\n]*\.ico')

    def test_08_parser_rejects_nominal_sizes_and_broken_directory(self):
        valid = (BRAND / "nxtools.ico").read_bytes()
        # Execute the same boundary with corrupt resource fixtures; no file/OS mutations.
        invalid_count = bytearray(valid)
        struct.pack_into("<H", invalid_count, 4, 1)
        invalid_size = bytearray(valid)
        invalid_size[6:8] = bytes((20, 20))  # actual first PNG is16, not directory-advertised20
        invalid_offset = bytearray(valid)
        struct.pack_into("<I", invalid_offset, 18, 0)
        for broken in (b"", bytes(invalid_count), bytes(invalid_size), bytes(invalid_offset), valid[:-1]):
            with self.subTest(length=len(broken)):
                with self.assertRaises((AssertionError, struct.error)):
                    ico_frames(broken)

    def test_09_real_pe_resource_directory_positive_and_negative_fixtures(self):
        def fixture(archive_name="nxtools-archive", include_archive=True):
            tree = {3: {}, 14: {}}
            image_id = 1
            for group_id, name in ((1, "nxtools"), (2, archive_name)):
                if group_id == 2 and not include_archive:
                    continue
                group = bytearray(struct.pack("<HHH", 0, 1, len(SIZES)))
                for size in SIZES:
                    payload = (BRAND / f"{name}-{size}.png").read_bytes()
                    tree[3][image_id] = {1033: payload}
                    group.extend(struct.pack("<BBBBHHIH", size % 256, size % 256, 0, 0, 1, 32, len(payload), image_id))
                    image_id += 1
                tree[14][group_id] = {1033: bytes(group)}
            section = bytearray()

            def append_node(node):
                offset = len(section)
                section.extend(bytes(16 + 8 * len(node)))
                struct.pack_into("<HH", section, offset + 12, 0, len(node))
                for index, (name, child) in enumerate(sorted(node.items())):
                    if isinstance(child, dict):
                        target = append_node(child) | 0x80000000
                    else:
                        target = len(section)
                        section.extend(struct.pack("<IIII", 0x1000 + target + 16, len(child), 0, 0))
                        section.extend(child)
                    struct.pack_into("<II", section, offset + 16 + index * 8, name, target)
                return offset

            append_node(tree)
            header = bytearray(512)
            header[:2] = b"MZ"
            struct.pack_into("<I", header, 0x3C, 0x80)
            header[0x80:0x84] = b"PE\0\0"
            struct.pack_into("<HHIIIHH", header, 0x84, 0x8664, 1, 0, 0, 0, 240, 0)
            struct.pack_into("<H", header, 0x98, 0x20B)
            struct.pack_into("<II", header, 0x98 + 112 + 16, 0x1000, len(section))
            header[0x188:0x190] = b".rsrc\0\0\0"
            struct.pack_into("<IIII", header, 0x190, len(section), 0x1000, len(section), 512)
            return bytes(header + section)

        verify_desktop_pe(fixture())
        for invalid in (fixture(archive_name="nxtools"), fixture(include_archive=False)):
            with self.assertRaises(AssertionError):
                verify_desktop_pe(invalid)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--desktop-pe", type=Path, help="Read-only verification of both icon groups/all18 frames in a rebuilt Desktop PE")
    options, remaining = parser.parse_known_args()
    if options.desktop_pe:
        verify_desktop_pe(options.desktop_pe.read_bytes())
        print("PASS Desktop PE: group1 blue/group2 orange;18 decoded frames match approved pixels; no execution/Shell")
    else:
        unittest.main(argv=[sys.argv[0], *remaining], testRunner=unittest.TextTestRunner(stream=sys.stdout, verbosity=2))
