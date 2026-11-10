"""Source boundary only; actual Tree events/worker fixtures require compiled tests."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class TreeRootDragBoundary(unittest.TestCase):
    def test_actual_tree_children_arm_original_candidate(self):
        browser = (ROOT / 'rust/apps/nxtools-desktop/src/archive/browser.rs').read_text('utf-8-sig')
        tree = browser[browser.index('let tree = Tree::new'):browser.index('h_resizable("archive-browser-split")')]
        self.assertEqual(tree.count('tree_drag_down('), 2, 'both official icon and label, never disclosure')
        disclosure = tree[tree.index('.pl(px(entry.depth()'):tree.index('debug_selector(move || icon_selector')]
        self.assertNotIn('tree_drag_down(', disclosure)
        self.assertIn('tree_drag_click(', tree, 'native threshold suppresses Tree click, arrow retains original click')
        self.assertNotIn('on_drag(', browser)

    def test_original_freezer_and_connection_boundary(self):
        drag = (ROOT / 'rust/apps/nxtools-desktop/src/archive/browser_drag.rs').read_text('utf-8-sig')
        model = (ROOT / 'rust/apps/nxtools-desktop/src/archive/model_drag.rs').read_text('utf-8-sig')
        model_production = model.split('#[cfg(test)]')[0]
        self.assertIn('pub fn freeze_drag_tree(', model_production)
        self.assertIn('model.freeze_drag_tree(', drag)
        self.assertIn('send_archive_virtual_drag(command, request, self.drag.connection)', drag)
        self.assertIn('event.pressed_button != Some(MouseButton::Left)', drag)
        self.assertIn('native_window_handle(window)', drag)
        self.assertIn('freeze_drag_roots(', model_production, 'selection and Tree use one expansion owner')
        ipc = (ROOT / 'rust/crates/nxtools-ipc/src/archive_virtual_drag.rs').read_text('utf-8-sig')
        self.assertIn('active.generation == origin', ipc)
        self.assertLess(ipc.index('active.generation == origin'), ipc.index('router.pending.insert('))

    def test_no_new_wire_or_native_framework(self):
        wire = (ROOT / 'rust/crates/nxtools-protocol/src/archive_virtual_drag.rs').read_text('utf-8-sig')
        native = (ROOT / 'csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualDragSession.cs').read_text('utf-8-sig')
        content = (ROOT / 'csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualContent.cs').read_text('utf-8-sig')
        for declaration in ['ARCHIVE_DRAG_MAX_ROOTS', 'ARCHIVE_DRAG_MAX_DESCRIPTORS', 'ARCHIVE_DRAG_MAX_FILES', 'ARCHIVE_DRAG_MAX_BYTES', 'ARCHIVE_DRAG_MAX_COMMAND_BYTES']:
            self.assertNotIn(declaration, wire, 'complete selection uses original resource/frame budgets')
        self.assertIn('struct DragSource', wire, 'original source context is shared, not a per-member authority')
        self.assertIn('> crate::MAX_FRAME_BYTES', wire)
        self.assertIn('ordinary_directory_drag_wire_preserves_256_files_and_all_descriptor_indices', wire)
        self.assertIn('self.version != 1', wire)
        self.assertIn('valid_drag_name_component', wire)
        self.assertIn('ApartmentState.STA', native)
        self.assertIn('TryReleaseSourcePointerCapture', native)
        self.assertIn('GetAsyncKeyState(1)&0x8000', native)
        self.assertIn('entries.CreateVirtualMaterialization(plan,authorizationId,operation,memberProgress)', content)
        self.assertIn('Action<ArchiveEntrySession.VirtualMemberProgress>? memberProgress=null', content)
        self.assertIn('entries.DemandVirtual(context,index,remaining)', content)
        self.assertIn('await entries.CloseVirtual(context)', content)
        self.assertNotIn('items[index].Name', content, 'descriptor wrapper is never an output path')


if __name__ == '__main__':
    unittest.main()
