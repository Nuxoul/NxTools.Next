"""Small deterministic status-reader regressions; no archive or product process."""
import contextlib
import ctypes as c
from ctypes import wintypes as w
import io
import json
import os
import pathlib
import tempfile
import types
import unittest
from unittest import mock

import control
from control import StopController, read_status
from driver_loop import drive
from test_tools import Child, Native, driver


class StatusReadTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(dir=os.environ['PI_SCRATCH_DIR'])
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name)
        self.path = self.root / 'status.json'
        self.value = dict(schema=2, run_id='status-test', state='Running', identities=[],
                          cleanup=dict(status='Unknown', confirmed=False), stop=dict(requested=False))
        self.path.write_text(json.dumps(self.value), encoding='utf-8')
        self.before = self.path.read_bytes()
        self.kernel = c.WinDLL('kernel32', use_last_error=True)
        self.kernel.CreateFileW.argtypes = (w.LPCWSTR, w.DWORD, w.DWORD, c.c_void_p, w.DWORD, w.DWORD, w.HANDLE)
        self.kernel.CreateFileW.restype = w.HANDLE
        self.kernel.CloseHandle.argtypes = (w.HANDLE,)
        self.kernel.CloseHandle.restype = w.BOOL
        self.held = None
        self.addCleanup(self.release)

    def hold(self):
        self.held = self.kernel.CreateFileW(str(self.path), 0x80000000, 0, None, 3, 0x80, None)
        self.assertNotEqual(c.c_void_p(-1).value, self.held)

    def release(self):
        if self.held is not None:
            self.assertTrue(self.kernel.CloseHandle(self.held))
            self.held = None

    def test_real_reader_recovers_after_exclusive_handle_is_released(self):
        self.hold()
        # Patch only retry scheduling: deterministically release the competing handle on retry.
        with mock.patch('control.time.sleep', side_effect=lambda _: self.release()) as pause:
            value = read_status(self.root, 'status-test')
        self.assertEqual(self.value, value)
        self.assertEqual(1, pause.call_count)
        self.assertEqual(self.before, self.path.read_bytes())

    def test_real_driver_does_not_stop_on_recovered_status_sharing_conflict(self):
        self.hold()
        child = Child()
        controller = StopController(child, self.root, 'status-test')
        sampler = mock.Mock(); sampler.failure = None; sampler.live_targets.return_value = []
        with mock.patch('control.time.sleep', side_effect=lambda _: self.release()):
            observed = drive(child, self.root / 'probe.exe', self.root, 'runner100k', sampler,
                             controller, types.SimpleNamespace(K=driver.K), sleep=lambda _: None)
        self.assertIsNone(observed['failure'])
        self.assertFalse(controller.requested.is_set())
        self.assertFalse(child.terminated)
        self.assertEqual(self.value, observed['status'])
        self.assertEqual(self.before, self.path.read_bytes())


    def test_real_exclusive_handle_exhausts_with_native_code_and_bounded_attempts(self):
        self.hold(); diagnostics = control.StatusReadDiagnostics()
        with mock.patch('control.time.sleep') as pause:
            with self.assertRaises(PermissionError) as caught:
                read_status(self.root, 'status-test', diagnostics)
        self.assertEqual((13, 32, 'open'), (caught.exception.errno, caught.exception.winerror, caught.exception.status_stage))
        self.assertEqual(3, pause.call_count)
        result = diagnostics.result()
        self.assertEqual(dict(reads=1, attempts=4, retries=3, recoveries=0, exhausted=1, failures=1, missing=0), result['callers']['reader'])
        self.assertEqual([32] * 4, [row['native_code'] for row in result['errors']])
        self.release(); self.assertEqual(self.value, read_status(self.root, 'status-test', diagnostics))
        self.assertEqual(self.before, self.path.read_bytes())

    def test_real_access_denied_is_not_retried(self):
        # A directory opened as a normal file produces actual ERROR_ACCESS_DENIED, not sharing.
        self.path.unlink(); self.path.mkdir(); diagnostics = control.StatusReadDiagnostics()
        with mock.patch('control.time.sleep') as pause:
            with self.assertRaises(PermissionError) as caught:
                read_status(self.root, 'status-test', diagnostics)
        self.assertEqual((13, 5), (caught.exception.errno, caught.exception.winerror))
        pause.assert_not_called()
        self.assertEqual(1, diagnostics.result()['callers']['reader']['attempts'])
        self.assertEqual(5, diagnostics.result()['errors'][0]['native_code'])

    def test_errno13_without_native_code_and_other_native_errors_fail_immediately(self):
        for failure in (PermissionError(13, 'not-a-sharing-proof'), c.WinError(5), c.WinError(3), c.WinError(1117)):
            with self.subTest(native=getattr(failure, 'winerror', None)):
                failure.status_stage = 'open'; diagnostics = control.StatusReadDiagnostics()
                with mock.patch.object(control, '_read_status_bytes', side_effect=failure) as read, mock.patch('control.time.sleep') as pause:
                    with self.assertRaises(OSError) as caught:
                        read_status(self.root, 'status-test', diagnostics)
                self.assertIs(failure, caught.exception)
                read.assert_called_once(); pause.assert_not_called()

    def test_native_lock_conflict_recovery_keeps_real_category_and_stage(self):
        failure = c.WinError(33); failure.status_stage = 'read'
        diagnostics = control.StatusReadDiagnostics()
        with mock.patch.object(control, '_read_status_bytes', side_effect=[failure, self.before]) as read, mock.patch('control.time.sleep'):
            self.assertEqual(self.value, read_status(self.root, 'status-test', diagnostics))
        self.assertEqual(2, read.call_count)
        result = diagnostics.result()
        self.assertEqual(1, result['callers']['reader']['recoveries'])
        self.assertEqual(('read', 'lock_conflict', 13, 33), tuple(result['errors'][0][key] for key in ('stage', 'category', 'errno', 'native_code')))

    def test_real_reader_shares_delete_and_closes_before_publisher_replacement(self):
        replacement = dict(self.value, state='Draining'); temporary = self.root / 'status.tmp'
        temporary.write_text(json.dumps(replacement), encoding='utf-8')
        real_read = control._STATUS_KERNEL.ReadFile
        def inspect_sharing_while_reading(*args):
            # DELETE access is compatible only if the real reader grants FILE_SHARE_DELETE.
            compatible = self.kernel.CreateFileW(str(self.path), 0x10000, 7, None, 3, 0x80, None)
            self.assertNotEqual(c.c_void_p(-1).value, compatible)
            self.assertTrue(self.kernel.CloseHandle(compatible))
            return real_read(*args)
        with mock.patch.object(control._STATUS_KERNEL, 'ReadFile', side_effect=inspect_sharing_while_reading):
            self.assertEqual(self.value, read_status(self.root, 'status-test'))
        # Windows replacement may still require target handles to close, even with delete share.
        temporary.replace(self.path)
        self.assertEqual(replacement, read_status(self.root, 'status-test'))

    def test_sampler_recovers_real_sharing_conflict_without_stop(self):
        child = Child(); controller = StopController(child, self.root, 'status-test')
        path = self.root / 'probe.exe'; native = Native(path)
        sampler = driver.Sampler(child, path, self.root / 'samples.jsonl', controller, native=native,
                                 snapshot=lambda: [], guard=lambda: None)
        self.hold()
        with mock.patch('control.time.sleep', side_effect=lambda _: self.release()):
            sampler.discover()
        rows = sampler.finish()
        self.assertIsNone(sampler.failure); self.assertFalse(controller.requested.is_set())
        self.assertTrue(rows[0]['identity_verified'])
        result = controller.status_reads.result()
        self.assertEqual((2, 1, 1), tuple(result['callers']['sampler'][key] for key in ('attempts', 'retries', 'recoveries')))
        self.assertEqual(32, result['errors'][0]['native_code'])

    def test_sampler_exhaustion_requests_original_stop_and_retains_native_error(self):
        child = Child(); controller = StopController(child, self.root, 'status-test')
        path = self.root / 'probe.exe'
        sampler = driver.Sampler(child, path, self.root / 'samples.jsonl', controller,
                                 native=Native(path), snapshot=lambda: [], guard=lambda: None)
        self.hold()
        with mock.patch('control.time.sleep') as pause:
            sampler.run()
        sampler.finish()
        self.assertEqual('PermissionError', sampler.failure)
        self.assertTrue(controller.requested.is_set()); self.assertFalse(child.terminated)
        self.assertEqual(3, pause.call_count)
        result = controller.status_reads.result()
        self.assertEqual(1, result['callers']['sampler']['exhausted'])
        self.assertEqual(32, result['errors'][-1]['native_code'])

    def test_last_good_cleaned_is_not_returned_after_missing_or_exhausted_read(self):
        cleaned = dict(self.value, state='Cleaned', cleanup=dict(status='Confirmed', confirmed=True))
        self.path.write_text(json.dumps(cleaned), encoding='utf-8')
        controller = StopController(Child(), self.root, 'status-test')
        self.assertTrue(controller.read_status('driver')['cleanup']['confirmed'])
        self.hold()
        with mock.patch('control.time.sleep'):
            with self.assertRaises(PermissionError):
                controller.read_status('final')
        self.release(); self.path.unlink()
        self.assertEqual({}, controller.read_status('final'))
        self.assertEqual('Incomplete', drive_verdict_with_missing_status())

    def test_invalid_status_never_retries_or_recovers_from_last_good(self):
        values = [b'{', b'[]', b'null', b' ' * (control.LIMIT + 1),
                  json.dumps(dict(self.value, run_id='other')).encode(),
                  json.dumps(dict(self.value, schema=1)).encode(),
                  json.dumps(dict(self.value, schema=2.0)).encode(),
                  json.dumps(dict(self.value, identities={})).encode(),
                  json.dumps(dict(self.value, identities=[{}] * 33)).encode()]
        diagnostics = control.StatusReadDiagnostics()
        self.assertEqual(self.value, read_status(self.root, 'status-test', diagnostics))
        for data in values:
            with self.subTest(data=data[:30]), mock.patch('control.time.sleep') as pause:
                self.path.write_bytes(data)
                with self.assertRaises((ValueError, TypeError)):
                    read_status(self.root, 'status-test', diagnostics)
                pause.assert_not_called()

    def test_diagnostics_details_are_bounded_and_do_not_copy_payloads(self):
        diagnostics = control.StatusReadDiagnostics(); self.path.write_text('{payload-must-not-be-copied', encoding='utf-8')
        for _ in range(100):
            with self.assertRaises(ValueError):
                read_status(self.root, 'status-test', diagnostics)
        result = diagnostics.result()
        self.assertEqual(100, result['error_count']); self.assertEqual(16, len(result['errors']))
        self.assertTrue(result['errors_truncated'])
        self.assertNotIn('payload-must-not-be-copied', json.dumps(result))
        self.assertNotIn(str(self.root), json.dumps(result))
        self.assertLess(len(json.dumps(result)), 4096)

    def test_driver_clears_last_good_when_current_read_exhausts(self):
        cleaned = dict(self.value, state='Cleaned', cleanup=dict(status='Confirmed', confirmed=True))
        self.path.write_text(json.dumps(cleaned), encoding='utf-8')
        child = Child(); child.returncode = None
        controller = StopController(child, self.root, 'status-test')
        sampler = mock.Mock(); sampler.failure = None; sampler.live_targets.return_value = []
        def next_turn(_):
            self.hold(); child.returncode = 0
        with mock.patch('control.time.sleep'):
            observed = drive(child, self.root / 'probe.exe', self.root, 'runner100k', sampler,
                             controller, types.SimpleNamespace(K=driver.K), sleep=next_turn)
        self.assertEqual({}, observed['status'])
        self.assertIn('PermissionError', observed['failure'])
        self.assertEqual('driver_exception', controller.reason)
        self.assertEqual(1, controller.status_reads.result()['callers']['driver']['exhausted'])
        self.assertFalse(child.terminated)

    def test_final_execute_requires_fresh_status_not_old_cleaned_observation(self):
        import argparse
        from driver_loop import execute
        (self.root / 'owned-resource-fixtures').write_text('v1\n')
        cleaned = dict(self.value, state='Cleaned', cleanup=dict(status='Confirmed', confirmed=True))
        for denied in (False, True):
            with self.subTest(denied=denied):
                attempt = 'denied' if denied else 'missing'
                observed = dict(stdout='', stderr='', events=[], result=None, failure=None, status=cleaned)
                sampler = mock.Mock(); sampler.failure = None; sampler.finish.return_value = []
                def observed_drive(child, executable, root, scenario, actual_sampler, controller, api):
                    final_path = controller.directory / 'status.json'
                    if denied:
                        final_path.write_text(json.dumps(self.value), encoding='utf-8')
                        self.held = self.kernel.CreateFileW(str(final_path), 0x80000000, 0, None, 3, 0x80, None)
                        self.assertNotEqual(c.c_void_p(-1).value, self.held)
                    return observed
                if not self.path.exists():
                    self.path.write_text(json.dumps(self.value), encoding='utf-8')
                state = dict(available=8 << 30, commit_headroom=8 << 30)
                with mock.patch.object(driver, 'system_memory', return_value=state), \
                        mock.patch.object(driver, 'processes', return_value=[]), \
                        mock.patch.object(driver.shutil, 'disk_usage', return_value=mock.Mock(free=8 << 30)), \
                        mock.patch.object(driver.subprocess, 'Popen', return_value=Child()), \
                        mock.patch.object(driver, 'Sampler', return_value=sampler), \
                        mock.patch('driver_loop.uuid.uuid4', return_value='status-test'), \
                        mock.patch('driver_loop.drive', side_effect=observed_drive), \
                        mock.patch.object(driver, 'measurement_result', return_value=dict(status='Complete', reasons=[])), \
                        mock.patch('control.time.sleep'), contextlib.redirect_stdout(io.StringIO()):
                    with self.assertRaises(AssertionError):
                        execute(argparse.Namespace(root=self.root, scenario='engine100k', attempt=attempt), vars(driver))
                self.release()
                result = json.loads((self.root / f'engine100k-{attempt}.json').read_text(encoding='utf-8'))
                self.assertEqual({}, result['control_status'])
                self.assertFalse(result['cleanup']['confirmed'])
                self.assertEqual('Failed' if denied else 'Incomplete', result['resource_verdict'])
                self.assertNotEqual('PASS', result['resource_verdict'])
                final_path = self.root / 'control-status-test' / 'status.json'
                if final_path.exists():
                    final_path.unlink()
                (self.root / 'control-status-test').rmdir()



def drive_verdict_with_missing_status():
    from driver_loop import resource_verdict
    return resource_verdict(dict(status='Succeeded'), dict(status='Complete'), {}, dict(requested=False), None)

if __name__ == '__main__':
    unittest.main()
