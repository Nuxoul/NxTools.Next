"""In-window overlay source coverage; not native/pixel or interaction acceptance."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "rust/apps/nxtools-desktop/src"
OWNERS = {
    "archive/application.rs": (3, 0),
    "archive/progress.rs": (2, 0),
    "archive/password.rs": (1, 0),
    "archive/forms.rs": (2, 0),
    "archive/entry.rs": (3, 0),
    "archive/browser_find.rs": (1, 0),
    "archive/browser.rs": (2, 0),
    "downloads/new_download_view/template.rs": (1, 0),
    "downloads/paused_url.rs": (1, 0),
    "main_window/extensions_page.rs": (0, 1),
    "main_window/extensions_page/import_confirmation.rs": (1, 0),
    "main_window/extensions_page/trust_confirmation.rs": (1, 0),
    "main_window/extensions_page/storage.rs": (1, 0),
    "main_window/extensions_page/removal.rs": (1, 0),
    "extensions/manual_actions.rs": (1, 0),
    "tools/raw_join.rs": (1, 0),
    "tools/markdown_reader/history.rs": (1, 0),
    "tools/markdown_reader/edit.rs": (1, 0),
}
CALL = re.compile(r"\.open_(dialog|sheet)\s*\(")
CLOSURE = re.compile(
    r"\bwindow\.open_(dialog|sheet)\s*\(\s*cx\s*,\s*(?:move\s*)?"
    r"\|\s*(dialog|sheet)\s*,\s*(?:window|_)\s*,\s*cx\s*\|\s*\{\s*"
    r"let\s+\2\s*=\s*\2\.bg\(crate::theme::overlay_background\(cx\)\);"
)


def covered(text):
    return len(CALL.findall(text)) == len(CLOSURE.findall(text))


def public_outer_slot(text, component):
    styled = re.search(
        rf"impl Styled for {component}\s*\{{\s*"
        r"fn style\(&mut self\) -> &mut gpui::StyleRefinement\s*\{\s*&mut self.style",
        text,
    )
    surface_id = 'surface' if component == 'Dialog' else 'sheet-content'
    outer = re.search(
        rf'\.id\("{surface_id}"\)(?P<chain>.*?)\.refine_style\(&self.style\)',
        text,
        re.S,
    )
    return bool(styled and outer and '.bg(cx.theme().tokens.background)' in outer['chain'])


class OverlaySurfaces(unittest.TestCase):
    def test_production_inventory_and_outer_slot_coverage(self):
        actual = {}
        for path in SOURCE.rglob("*.rs"):
            if path.stem == 'tests' or path.stem.endswith('_tests') or 'tests' in path.parts:
                continue
            text = path.read_text(encoding="utf-8")
            calls = CALL.findall(text)
            if not calls:
                continue
            relative = path.relative_to(SOURCE).as_posix()
            actual[relative] = (calls.count('dialog'), calls.count('sheet'))
            self.assertTrue(covered(text), f"{relative}: set the outer Styled background at every closure entry")
        self.assertEqual(actual, OWNERS, "production overlay ownership changed; review the inventory")
        self.assertEqual(tuple(map(sum, zip(*actual.values()))), (24, 1))

    def test_released_components_refine_actual_outer_background(self):
        for component, path in [
            ('Dialog', 'vendor/gpui-component-0100/src/dialog/dialog.rs'),
            ('Sheet', 'vendor/gpui-component-0100/src/sheet.rs'),
        ]:
            text = (ROOT / path).read_text(encoding="utf-8")
            self.assertTrue(public_outer_slot(text, component), path)
            self.assertFalse(public_outer_slot(text.replace('.refine_style(&self.style)', ''), component))

    def test_coverage_rejects_missing_or_inner_background(self):
        good = 'window.open_dialog(cx, move |dialog, _, cx| { let dialog = dialog.bg(crate::theme::overlay_background(cx)); dialog.title("fixture") });'
        self.assertTrue(covered(good))
        self.assertFalse(covered(good.replace('let dialog = dialog.bg(crate::theme::overlay_background(cx));', '')))
        self.assertFalse(covered(good.replace('let dialog = dialog.bg', 'let body = div().bg')))
        self.assertFalse(covered(good.replace('|dialog, _, cx|', '|dialog, _, _|')))


if __name__ == '__main__':
    unittest.main()
