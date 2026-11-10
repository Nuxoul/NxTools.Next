"""Product-owned copy only; this does not certify rendered or native UI acceptance."""
from itertools import chain
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
STRINGS = re.compile(r'"(?:\\.|[^"\\])*"', re.DOTALL)


def product_sources():
    paths = chain(
        ROOT.glob('rust/apps/*/src/**/*.rs'),
        ROOT.glob('rust/crates/*/src/**/*.rs'),
        ROOT.glob('csharp/NxTools.PlatformHost/**/*.cs'),
    )
    for path in paths:
        if path.name == 'tests.rs' or path.name.endswith('_tests.rs'):
            continue
        if {'tests', 'bin', 'obj'} & set(path.relative_to(ROOT).parts):
            continue
        yield path


class ProductTerminology(unittest.TestCase):
    def test_product_strings_use_compressed_package_name(self):
        violations = []
        for path in product_sources():
            source = path.read_text(encoding='utf-8-sig')
            for match in STRINGS.finditer(source):
                if '归档' in match.group():
                    line = source.count('\n', 0, match.start()) + 1
                    violations.append(f'{path.relative_to(ROOT)}:{line}: {match.group()}')
        self.assertFalse(violations, '\n' + '\n'.join(violations))

    def test_settings_and_native_picker_use_the_same_name(self):
        expectations = {
            'rust/apps/nxtools-desktop/src/main_window/archive_page.rs': '压缩包浏览器',
            'rust/apps/nxtools-desktop/src/main_window/clipboard_page.rs': '历史压缩包',
            'csharp/NxTools.PlatformHost/Windows/ClipboardArchiveFilePicker.cs':
                '导出剪贴板压缩包',
        }
        for relative, label in expectations.items():
            with self.subTest(source=relative):
                self.assertIn(label, (ROOT / relative).read_text(encoding='utf-8-sig'))


if __name__ == '__main__':
    unittest.main()
