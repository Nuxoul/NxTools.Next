"""Deterministic tool regressions: fake counters/guards, never system pressure."""
import argparse
import contextlib
import importlib.util
import io
import json
import os
import pathlib
import tempfile
import threading
import types
import unittest
from unittest import mock

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('resource_driver', HERE / 'run.py')
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)
from control import StopController, read_status
from driver_loop import drive, resource_verdict


class Child:
    pid = 424242
    returncode = 0

    def __init__(self):
        self.stdin = io.BytesIO(); self.stdout = io.BytesIO(); self.stderr = io.BytesIO()
        self.terminated = False

    def poll(self):
        return self.returncode

    def wait(self, timeout=None):
        return self.returncode

    def communicate(self, timeout=None):
        return b'', b''

    def terminate(self):
        self.terminated = True; self.returncode = 1


class Native:
    def __init__(self, path):
        self.path = str(path.resolve()); self.open_failures = 0; self.image_failures = 0
        self.counter_failures = 0; self.closed = []; self.dead = False

    def open(self, process_id):
        if self.open_failures:
            self.open_failures -= 1
            raise PermissionError('injected_open')
        return process_id

    def image(self, handle):
        if self.image_failures:
            self.image_failures -= 1
            raise OSError('injected_image')
        return self.path

    def created(self, handle):
        return 1

    def exited(self, handle):
        return self.dead

    def take(self, handle):
        if self.counter_failures:
            self.counter_failures -= 1
            raise OSError('injected_counter')
        return dict(working_set=12, private_bytes=34, process_lifetime_peak_working_set=56, process_lifetime_peak_commit=78)

    def close(self, handle):
        self.closed.append(handle)


class ToolTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=os.environ['PI_SCRATCH_DIR'])
        self.addCleanup(self.temporary.cleanup)
        self.root = pathlib.Path(self.temporary.name)
        self.child = Child(); self.path = self.root / 'probe.exe'; self.native = Native(self.path)
        self.controller = StopController(self.child, self.root, 'run-1')

    def sampler(self, **kwargs):
        return driver.Sampler(self.child, self.path, self.root / 'samples.jsonl', self.controller,
                              native=self.native, snapshot=lambda: [], guard=lambda: None, **kwargs)

    def test_threshold_requests_stop_without_terminating_owner(self):
        for reason in ('physical_threshold', 'commit_threshold', 'disk_threshold'):
            with self.subTest(reason=reason):
                sampler = self.sampler(); sampler.output = self.root / (reason + '.jsonl')
                controller = StopController(self.child, self.root, reason); sampler.controller = controller
                sampler.guard = lambda: reason
                sampler.run()
                self.assertFalse(self.child.terminated, 'threshold bypassed cooperative probe cleanup')
                self.assertEqual(reason, controller.reason)
                self.assertEqual(reason, json.loads((self.root / 'stop.json').read_text())['reason'])
                sampler.finish()

    def test_exit_zero_with_no_required_samples_is_not_resource_pass(self):
        (self.root / 'owned-resource-fixtures').write_text('v1\n')
        sampler = mock.Mock(); sampler.failure = None; sampler.finish.return_value = []
        observed = dict(stdout='', stderr='', events=[], result=None, failure=None, status={})
        state = {'available': 8 << 30, 'commit_headroom': 8 << 30}
        args = argparse.Namespace(root=self.root, scenario='engine100k', attempt='correctness')
        with mock.patch.object(driver, 'system_memory', return_value=state), \
                mock.patch.object(driver, 'processes', return_value=[]), \
                mock.patch.object(driver.shutil, 'disk_usage', return_value=mock.Mock(free=8 << 30)), \
                mock.patch.object(driver.subprocess, 'Popen', return_value=self.child), \
                mock.patch.object(driver, 'Sampler', return_value=sampler), \
                mock.patch('driver_loop.drive', return_value=observed), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(AssertionError):
                driver.run(args)
        result = json.loads((self.root / 'engine100k-correctness.json').read_text())
        self.assertEqual('Succeeded', result['operation']['status'])
        self.assertEqual('Incomplete', result['measurement']['status'])
        self.assertEqual('Incomplete', result['resource_verdict'])
        self.assertEqual(['probe_owner:Unavailable'], result['measurement']['reasons'])

    def test_root_acquisition_permanent_failure_is_retained(self):
        self.native.open_failures = 100
        sampler = self.sampler()
        for _ in range(50):
            sampler.add(self.child.pid, os.getpid(), 'probe_owner')
        rows = sampler.finish(); row = rows[0]
        self.assertEqual(32, row['acquisition_attempts'])
        self.assertEqual(32, row['acquisition_failures'])
        self.assertEqual(16, len(row['errors']))
        self.assertIsNone(row['peak_working_set'])
        self.assertEqual('Incomplete', driver.measurement_result(rows, ['probe_owner'])['status'])

    def test_worker_image_failure_is_not_silently_omitted(self):
        sampler = self.sampler(); self.native.image_failures = 100
        sampler.add(999, self.child.pid, 'actual_worker', str(self.path))
        rows = sampler.finish()
        self.assertEqual(1, rows[0]['image_failures'])
        self.assertFalse(rows[0]['identity_verified'])
        self.assertEqual('Incomplete', driver.measurement_result(rows, ['actual_worker'])['status'])
        self.assertEqual([999], self.native.closed)

    def test_all_counters_fail_and_zero_valid_samples_have_null_peaks(self):
        sampler = self.sampler(); self.native.counter_failures = 100
        sampler.add(self.child.pid, os.getpid(), 'probe_owner')
        for index in range(31):
            sampler.take(sampler.rows[self.child.pid], index / 100., io.StringIO())
        with self.assertRaisesRegex(RuntimeError, 'required_counter_unavailable'):
            sampler.take(sampler.rows[self.child.pid], .32, io.StringIO())
        rows = sampler.finish(); row = rows[0]
        self.assertEqual((32, 32, 0), (row['counter_attempts'], row['counter_failures'], row['samples']))
        self.assertIsNone(row['baseline']); self.assertIsNone(row['peak_private_bytes'])
        self.assertIn('zero_valid_samples', driver.measurement_result(rows, ['probe_owner'])['reasons'][0])

    def test_transient_acquisition_image_and_counter_failure_recovers(self):
        sampler = self.sampler(); self.native.open_failures = 1; self.native.image_failures = 1
        for _ in range(3):
            sampler.add(self.child.pid, os.getpid(), 'probe_owner')
        self.native.counter_failures = 1
        for now in (0, .02, .05):
            sampler.take(sampler.rows[self.child.pid], now, io.StringIO())
        rows = sampler.finish(); row = rows[0]
        self.assertEqual((3, 2, 1), (row['acquisition_attempts'], row['acquisition_successes'], row['image_successes']))
        self.assertEqual((1, 2, 12, 34), (row['counter_failures'], row['samples'], row['peak_working_set'], row['peak_private_bytes']))
        self.assertAlmostEqual(30, row['max_interval_ms'])
        self.assertEqual('Complete', driver.measurement_result(rows, ['probe_owner'])['status'])

    def test_missing_worker_and_conhost_are_incomplete_not_fake_worker(self):
        sampler = self.sampler(); sampler.scenario = 'runner100k'
        sampler.expected_images = {'actual_worker': str(self.root / 'NxTools.ArchiveWorker.exe')}
        sampler.snapshot = lambda: [(999, self.child.pid, 'conhost.exe')]
        sampler.discover(); sampler.take(sampler.rows[self.child.pid], 0, io.StringIO())
        rows = sampler.finish()
        self.assertEqual([self.child.pid], [row['pid'] for row in rows])
        result = driver.measurement_result(rows, ['probe_owner', 'actual_worker'])
        self.assertEqual(['actual_worker:Unavailable'], result['reasons'])

    def test_existing_record_without_sample_is_unavailable_not_zero(self):
        sampler = self.sampler(); sampler.add(self.child.pid, os.getpid(), 'probe_owner')
        rows = sampler.finish()
        self.assertEqual(0, rows[0]['samples']); self.assertIsNone(rows[0]['peak_working_set'])
        self.assertEqual('Incomplete', driver.measurement_result(rows, ['probe_owner'])['status'])

    def test_wrong_image_and_parent_do_not_certify_worker(self):
        sampler = self.sampler(); sampler.expected_images = {'actual_worker': str(self.root / 'NxTools.ArchiveWorker.exe')}
        sampler.snapshot = lambda: [(998, 12345, 'NxTools.ArchiveWorker.exe'), (999, self.child.pid, 'NxTools.ArchiveWorker.exe')]
        sampler.discover(); rows = sampler.finish()
        self.assertNotIn(998, [row['pid'] for row in rows])
        self.assertFalse(next(row for row in rows if row['pid'] == 999)['identity_verified'])

    def test_sampler_deadline_and_other_exception_request_cooperation(self):
        sampler = self.sampler(clock=iter([0, 181]).__next__)
        sampler.run(); self.assertEqual('sampler_deadline', self.controller.reason)
        self.assertFalse(self.child.terminated); sampler.finish()
        controller = StopController(self.child, self.root, 'other'); sampler = self.sampler(); sampler.controller = controller
        sampler.guard = mock.Mock(side_effect=OSError('injected_guard'))
        sampler.output = self.root / 'other.jsonl'; sampler.run()
        self.assertEqual('OSError', controller.reason); self.assertFalse(self.child.terminated); sampler.finish()

    def test_stop_is_first_wins_concurrent_idempotent_and_late_safe(self):
        barrier = threading.Barrier(3); results = []
        def request(reason):
            barrier.wait(); results.append(self.controller.request(reason))
        threads = [threading.Thread(target=request, args=(reason,)) for reason in ('threshold', 'driver_exception')]
        for thread in threads:
            thread.start()
        barrier.wait()
        for thread in threads:
            thread.join(2); self.assertFalse(thread.is_alive())
        self.assertEqual([False, True], sorted(results))
        self.assertFalse(self.controller.request('late'))
        self.controller.close(); self.assertFalse(self.controller.request('after_close'))
        self.assertFalse(self.child.terminated)

    def test_control_channel_broken_retains_request_and_error(self):
        controller = StopController(self.child, self.root / 'absent', 'run-1')
        self.assertTrue(controller.request('threshold'))
        self.assertTrue(controller.requested.is_set())
        self.assertEqual('stop_delivery', controller.result()['errors'][0]['stage'])
        self.assertFalse(controller.request('retry'))

    def test_status_run_id_and_size_are_checked(self):
        (self.root / 'status.json').write_text(json.dumps({'schema': 2, 'run_id': 'other'}))
        with self.assertRaisesRegex(ValueError, 'identity'):
            read_status(self.root, 'run-1')
        (self.root / 'status.json').write_bytes(b' ' * 16385)
        with self.assertRaisesRegex(ValueError, 'limit'):
            read_status(self.root, 'run-1')

    def test_forced_fallback_records_exact_targets_and_never_passes(self):
        self.child.returncode = None; ticks = [0]
        controller = StopController(self.child, self.root, 'run-1', clock=lambda: ticks[0])
        controller.request('uncooperative'); self.assertFalse(controller.expired())
        self.assertFalse(self.child.terminated)
        ticks[0] = 11; self.assertTrue(controller.expired())
        target = mock.Mock()
        controller.force([dict(pid=99, executable='exact-owned.exe', role='actual_worker', terminate=target)])
        controller.force(); target.assert_called_once(); self.assertTrue(self.child.terminated)
        self.assertEqual([99, self.child.pid], [item['pid'] for item in controller.result()['targets']])
        self.assertEqual('Failed', resource_verdict({'status': 'Stopped'}, {'status': 'Complete'}, {'confirmed': True}, controller.result(), None))

    def test_driver_parse_exception_requests_stop_not_terminate(self):
        self.child.stdout = io.BytesIO(b'RESOURCE not-json\n')
        sampler = mock.Mock(); sampler.failure = None; sampler.live_targets.return_value = []
        observed = drive(self.child, self.path, self.root, 'runner100k', sampler, self.controller, types.SimpleNamespace(K=driver.K))
        self.assertEqual('driver_exception', self.controller.reason)
        self.assertIn('JSONDecodeError', observed['failure']); self.assertFalse(self.child.terminated)

    def test_driver_deadline_requests_stop_before_forcing(self):
        self.child.returncode = None; ticks = [0]
        controller = StopController(self.child, self.root, 'run-1', clock=lambda: ticks[0])
        sampler = mock.Mock(); sampler.failure = None; sampler.live_targets.return_value = []
        def tick(_):
            ticks[0] += 1
            if controller.requested.is_set():
                self.child.returncode = 0
        drive(self.child, self.path, self.root, 'runner100k', sampler, controller,
              types.SimpleNamespace(K=driver.K), deadline=2, clock=lambda: ticks[0], sleep=tick)
        self.assertEqual('driver_wait_deadline', controller.reason)
        self.assertFalse(controller.forced); self.assertFalse(self.child.terminated)


class InternalStopTests(unittest.TestCase):
    def test_probe_internal_stop_starts_original_ten_second_grace(self):
        with tempfile.TemporaryDirectory(dir=os.environ['PI_SCRATCH_DIR']) as text:
            root = pathlib.Path(text); child = Child(); child.returncode = None; ticks = [0]
            (root / 'status.json').write_text(json.dumps(dict(schema=2, run_id='run-1', state='Cancelling',
                stop={'requested':True,'reason':'probe_failure'}, identities=[])))
            controller = StopController(child, root, 'run-1', clock=lambda: ticks[0])
            sampler = mock.Mock(); sampler.failure = None; sampler.live_targets.return_value = []
            drive(child, pathlib.Path('probe.exe'), root, 'runner100k', sampler, controller,
                  types.SimpleNamespace(K=driver.K), clock=lambda: ticks[0], sleep=lambda _: ticks.__setitem__(0,ticks[0]+1))
            self.assertLessEqual(ticks[0], 11, 'internal stop waited for unrelated150s driver deadline')
            self.assertEqual('probe_failure', controller.reason)
            self.assertTrue(controller.forced)


class ExitObservationTests(unittest.TestCase):
    setUp = ToolTests.setUp
    sampler = ToolTests.sampler
    def test_native_exit_query_failure_retains_record_and_closes_handle(self):
        sampler = self.sampler(); sampler.add(self.child.pid, os.getpid(), 'probe_owner')
        self.native.exited = mock.Mock(side_effect=OSError('injected_wait_failure'))
        rows = sampler.finish()
        self.assertEqual([self.child.pid], self.native.closed)
        self.assertIsNone(rows[0]['alive_at_end'])
        self.assertEqual(1, rows[0]['exit_failures'])
        self.assertEqual('Incomplete', driver.measurement_result(rows, ['probe_owner'])['status'])


class StartupBarrierTests(unittest.TestCase):
    def test_stop_before_go_releases_probe_to_its_own_cleanup(self):
        with tempfile.TemporaryDirectory(dir=os.environ['PI_SCRATCH_DIR']) as text:
            root = pathlib.Path(text); child = Child()
            child.stdout = io.BytesIO(b'RESOURCE {"phase":"ready"}\n')
            controller = StopController(child, root, 'run-1')
            sampler = mock.Mock(); sampler.failure = 'root_unavailable'; sampler.ready.wait.return_value = False
            sampler.live_targets.return_value = []
            drive(child, pathlib.Path('resource_probe.exe'), root, 'engine100k', sampler, controller,
                  types.SimpleNamespace(K=driver.K))
            self.assertEqual(b'go\n', child.stdin.getvalue(), 'stopped probe stranded before cleanup barrier')
            self.assertTrue(controller.requested.is_set()); self.assertFalse(child.terminated)


class OwnedProcessTests(unittest.TestCase):
    def test_cooperative_and_forced_processes_are_separate(self):
        import subprocess
        import sys
        import time
        for mode in ('cooperate', 'uncooperative'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory(dir=os.environ['PI_SCRATCH_DIR']) as text:
                root = pathlib.Path(text)
                child = subprocess.Popen([sys.executable, str(HERE / 'owned_stop_fixture.py'), str(root), mode],
                                         stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                controller = StopController(child, root, 'owned-test')
                try:
                    deadline = time.monotonic() + 5
                    while not (root / 'fixture-ready').exists() and time.monotonic() < deadline:
                        time.sleep(.005)
                    self.assertTrue((root / 'fixture-owned.partial').is_file())
                    controller.request('injected_threshold')
                    if mode == 'cooperate':
                        child.wait(timeout=5)
                        self.assertFalse(controller.forced)
                        self.assertFalse((root / 'fixture-owned.partial').exists())
                        self.assertTrue(read_status(root, 'owned-test')['cleanup']['confirmed'])
                    else:
                        child.wait(timeout=.05) if child.poll() is not None else None
                        self.assertIsNone(child.poll())
                        controller.at -= 11  # virtual advance, not an arbitrary long sleep
                        self.assertTrue(controller.expired())
                        controller.force(); child.wait(timeout=5)
                        self.assertTrue(controller.forced)
                        self.assertTrue((root / 'fixture-owned.partial').is_file())
                        self.assertFalse(read_status(root, 'owned-test').get('cleanup', {}).get('confirmed', False))
                finally:
                    if child.poll() is None:
                        controller.at = controller.clock() - 11
                        controller.request('test_finally'); controller.force(); child.wait(timeout=5)
                    controller.close()


if __name__ == '__main__':
    unittest.main()
