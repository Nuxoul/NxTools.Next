"""Bounded owned-probe drive loop. Windows ownership stays in the original C# runner."""
import ctypes as c
from ctypes import wintypes as w
import datetime
import json
import os
import pathlib
import struct
import subprocess
import time
import types
import uuid
from control import StopController, read_status


class PipeReader:
    def __init__(self, stream, kernel):
        self.stream = stream; self.kernel = kernel; self.eof = False

    def read(self):
        if self.eof:
            return b''
        if hasattr(self.stream, 'getvalue'):
            data = self.stream.read(65536); self.eof = not data
            return data
        import msvcrt
        available = w.DWORD()
        peek = self.kernel.PeekNamedPipe
        peek.argtypes = (w.HANDLE, c.c_void_p, w.DWORD, c.POINTER(w.DWORD), c.POINTER(w.DWORD), c.POINTER(w.DWORD))
        if not peek(msvcrt.get_osfhandle(self.stream.fileno()), None, 0, None, c.byref(available), None):
            code = c.get_last_error()
            if code in (109, 232):
                self.eof = True
                return b''
            raise c.WinError(code)
        return os.read(self.stream.fileno(), min(available.value, 65536)) if available.value else b''


class DirectVerifier:
    def __init__(self, child, root, send):
        self.child = child; self.send = send; self.operation = str(uuid.uuid4()); self.count = self.batches = 0
        self.first = None; self.completed = False; self.buffer = bytearray(); self.clock = time.perf_counter()
        self.path = root / '100k.zip'
        self.manifest = (root / 'entries.jsonl').open(encoding='utf-8')

    def start(self):
        self.send(self.child.stdin, dict(schema=1, operation_id=self.operation, op='start', operation='open', path=str(self.path),
                  limits=dict(max_entries=100000, max_metadata_bytes=256 << 20, max_working_bytes=64 << 20)))

    def cancel(self):
        self.send(self.child.stdin, dict(version=1, operation_id=self.operation, op='cancel'))

    def feed(self, data):
        self.buffer.extend(data)
        while len(self.buffer) >= 4:
            length, = struct.unpack('<I', self.buffer[:4])
            assert 0 < length <= 1 << 20, 'frame_limit'
            if len(self.buffer) < length + 4:
                break
            frame = json.loads(self.buffer[4:4 + length]); del self.buffer[:4 + length]
            assert frame['operation_id'] == self.operation
            if frame['op'] == 'entry_batch':
                self.first = self.first if self.first is not None else (time.perf_counter() - self.clock) * 1000
                assert frame.get('batch_sequence') is None and frame.get('snapshot_id') is None
                for entry in frame['entries']:
                    expected = json.loads(next(self.manifest))
                    for key in ('ordinal', 'name', 'kind', 'size', 'packed_size'):
                        assert entry[key] == expected[key], (key, self.count)
                    self.count += 1
                self.batches += 1
            elif frame['op'] == 'complete':
                assert self.count == frame['files_verified'] == 100000 and self.batches == 50000
                assert not self.manifest.readline()
                self.completed = True
                self.child.stdin.close()
            elif frame['op'] == 'failure':
                raise RuntimeError('worker_' + frame['error_code'])

    def result(self):
        return dict(scenario='direct-worker100k', operation_id=self.operation, count=self.count, batches=self.batches,
                    completed=self.completed, first_batch_ms=self.first, terminal_ms=(time.perf_counter()-self.clock)*1000,
                    native_job_source_lease=False, snapshot_id='not_exposed_in_legacy_wire', first_paint='not_measured_no_ui')


def resource_verdict(operation, measurement, cleanup, stop, failure):
    if stop.get('forced') or failure or operation['status'] == 'Failed':
        return 'Failed'
    if stop.get('requested'):
        return 'Stopped'
    if measurement['status'] != 'Complete' or not cleanup.get('confirmed'):
        return 'Incomplete'
    return 'PASS'


def drive(child, executable, root, scenario, sampler, controller, api, deadline=150, clock=time.monotonic, sleep=time.sleep):
    stdout = bytearray(); stderr = bytearray(); events = []; line_buffer = bytearray()
    readers = [PipeReader(child.stdout, api.K), PipeReader(child.stderr, api.K)]
    direct = DirectVerifier(child, root, api.send) if scenario == 'direct-worker100k' else None
    failure = None; sent_go = False; direct_cancelled = False; forced_at = None; exited_at = None
    start = clock(); status = {}
    try:
        if direct:
            direct.start()
        while True:
            now = clock()
            if now - start >= deadline:
                controller.request('driver_wait_deadline')
            if sampler.failure:
                controller.request('sampler_failure')
            try:
                status = {}  # Last successful observation must not survive a failed current read.
                status = controller.read_status('driver')
                if status.get('stop', {}).get('requested'):
                    controller.request(status['stop'].get('reason') or 'probe_internal_stop')
                if not sampler.thread.is_alive():
                    sampler.discover()
                if scenario == 'tool-stop-small' and status.get('barrier', {}).get('partial_bytes', 0) > 0:
                    controller.request('injected_partial_threshold')
                for index, reader in enumerate(readers):
                    data = reader.read()
                    if not data:
                        continue
                    if index == 0 and direct:
                        if not controller.requested.is_set():
                            direct.feed(data)
                    else:
                        target = stdout if index == 0 else stderr
                        if len(target) + len(data) > 1 << 20:
                            raise RuntimeError('probe_output_limit')
                        target.extend(data)
                        if index == 0:
                            line_buffer.extend(data)
                            while b'\n' in line_buffer:
                                line, _, tail = line_buffer.partition(b'\n'); line_buffer[:] = tail
                                if line.startswith(b'RESOURCE '):
                                    event = json.loads(line[9:])
                                    if len(events) >= 64:
                                        raise RuntimeError('probe_event_limit')
                                    events.append(event)
                                    if event.get('phase') == 'ready' and not sent_go and executable.name == 'resource_probe.exe':
                                        if not sampler.ready.wait(2):
                                            controller.request('probe_owner_sample_unavailable')
                                        # Release setup barrier even after stop; probe checks stop BEFORE dispatch.
                                        child.stdin.write(b'go\n'); child.stdin.flush(); sent_go = True
                if controller.requested.is_set() and direct and not direct_cancelled:
                    direct_cancelled = True; direct.cancel()
            except (Exception, KeyboardInterrupt) as problem:
                failure = failure or type(problem).__name__ + ':' + str(problem)[:256]
                controller.request('driver_exception')
            exited = child.poll() is not None
            if exited and exited_at is None:
                exited_at = now
            if exited and all(reader.eof for reader in readers):
                if not sampler.live_targets():
                    break
                controller.request('owner_exited_before_descendants')
            if controller.expired() and forced_at is None:
                if not sampler.thread.is_alive():
                    try:
                        sampler.discover()
                    except Exception:
                        pass  # no unverified target is killed; primary failure stays recorded
                controller.force(sampler.live_targets()); forced_at = now
            if forced_at is not None and now - forced_at >= 5:
                failure = failure or 'forced_drain_incomplete'
                break
            if exited_at is not None and now - exited_at >= 1 and not all(reader.eof for reader in readers):
                controller.request('owned_pipe_drain_incomplete')
            sleep(.01)
    finally:
        if direct:
            direct.manifest.close()
    if child.poll() is not None:
        child.wait(timeout=1)
    if direct and not direct.completed and not controller.requested.is_set():
        failure = failure or 'EOF_is_not_completion'
    return dict(stdout=stdout.decode('utf-8', errors='replace'), stderr=stderr.decode('utf-8', errors='replace'),
                events=events, result=direct.result() if direct else None, failure=failure, status=status)


def execute(args, namespace):
    api = types.SimpleNamespace(**namespace)
    root = args.root.resolve(); workspace = pathlib.Path(api.__file__).resolve().parents[3]
    assert (root / 'owned-resource-fixtures').is_file()
    state = api.system_memory()
    assert state['available'] >= 4 << 30 and state['commit_headroom'] >= 6 << 30
    assert api.shutil.disk_usage(root).free >= 4 << 30
    assert not any(name.casefold() in ('cargo.exe', 'rustc.exe') for _, _, name in api.processes()), 'wait for active build'
    label = args.scenario + '-' + args.attempt; output = root / (label + '.json')
    assert not output.exists(), 'retain prior attempts, never overwrite'
    expected = {}
    apphost = workspace / 'csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe'
    if args.scenario == 'direct-worker100k':
        executable = workspace / 'target/debug/nxtools-archive-worker.exe'; command = [str(executable), '--archive-worker']
    elif args.scenario in ('runner100k', 'cancel1gib', 'cancel1gib-hold', 'tool-stop-small'):
        executable = apphost; command = [str(executable), '--archive-resource-probe', args.scenario, str(root)]
        expected['actual_worker'] = str(apphost.parent / 'NxTools.ArchiveWorker.exe')
    else:
        executable = workspace / 'target/debug/examples/resource_probe.exe'; command = [str(executable), args.scenario, str(root)]
        expected['isolated_csharp_host'] = str(apphost)
    run_id = str(uuid.uuid4()); control = root / ('control-' + run_id); control.mkdir()
    env = os.environ.copy(); env['NXTOOLS_ARCHIVE_WORKER_FIXTURE'] = str(workspace / 'target/debug/nxtools-archive-worker.exe')
    env['NXTOOLS_RESOURCE_CONTROL'] = str(control); env['NXTOOLS_RESOURCE_RUN_ID'] = run_id
    clock = time.perf_counter(); child = sampler = controller = None; metrics = []; observed = {}; failure = None
    try:
        child = api.subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, bufsize=0)
        controller = StopController(child, control, run_id)
        sampler = api.Sampler(child, executable, root / (label + '-samples.jsonl'), controller, args.scenario, expected_images=expected)
        sampler.thread.start()
        observed = drive(child, executable, root, args.scenario, sampler, controller, api)
        failure = observed['failure'] or sampler.failure
    except (Exception, KeyboardInterrupt) as problem:
        failure = type(problem).__name__ + ':' + str(problem)[:256]
        if controller:
            controller.request('driver_exception')
            # Constructor/start failure can occur before the ready line is consumed.
            # A stop file already exists; release only setup so the probe can clean, not dispatch.
            try:
                if executable.name == 'resource_probe.exe' and not child.stdin.closed:
                    child.stdin.write(b'go\n'); child.stdin.flush()
                elif args.scenario == 'direct-worker100k':
                    child.stdin.close()  # original private worker EOF cancellation
            except (OSError, ValueError):
                pass  # delivery failure does not erase the primary exception/stop record
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                controller.force(sampler.live_targets() if sampler else ())
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    failure += ':owner_exit_unknown'
    finally:
        if sampler:
            metrics = sampler.finish()
        if child:
            for stream in (child.stdin, child.stdout, child.stderr):
                stream.close()
        if controller:
            controller.close()
    stop = controller.result() if controller else dict(requested=False, forced=False)
    status = {}  # Final status must be freshly read; earlier Cleaned cannot prove current cleanup.
    try:
        status = controller.read_status('final') if controller else read_status(control, run_id)
    except Exception as problem:
        failure = failure or 'final_status_' + type(problem).__name__
    owner_stop = status.get('stop', {})
    if owner_stop.get('requested') or status.get('state') == 'Forced':
        stop['owner_report'] = owner_stop
        stop['requested'] = True
        stop['reason'] = stop.get('reason') or owner_stop.get('reason') or 'probe_internal_stop'
        stop['forced'] = stop['forced'] or owner_stop.get('forced', False) or status.get('state') == 'Forced'
    owner_results = [event for event in observed.get('events', []) if 'operation' in event]
    owner_result = owner_results[-1] if owner_results else status
    cleanup = status.get('cleanup', dict(status='Unknown', confirmed=False))
    if args.scenario == 'direct-worker100k' and child and child.returncode is not None and not stop['forced']:
        cleanup = dict(status='Confirmed', confirmed=True, scope='direct_process_and_pipes_only_no_native_job_or_lease')
    if stop['forced'] or any(row.get('alive_at_end') for row in metrics):
        cleanup = dict(status='Unknown', confirmed=False, reason='forced_or_live_owned_process', owner_report=cleanup)
    operation = dict(status='Stopped' if stop['requested'] else 'Succeeded' if child and child.returncode == 0 and not observed.get('failure') else 'Failed',
                     exit_code=child.returncode if child else None)
    if 'operation' in owner_result:
        operation['owner_report'] = owner_result['operation']
    measurement = api.measurement_result(metrics, api.required_roles(args.scenario))
    if sampler and sampler.failure:
        measurement['status'] = 'Incomplete'
        measurement['reasons'].append('sampler:' + sampler.failure)
    if owner_result.get('measurement', {}).get('status') == 'Incomplete':
        measurement['status'] = 'Incomplete'
        measurement['reasons'].append('probe_internal_counters_incomplete')
    if owner_result.get('measurement'):
        measurement['owner_report'] = owner_result['measurement']
    verdict = resource_verdict(operation, measurement, cleanup, stop, failure)
    ledger = dict(schema=2, run_id=run_id, command=command, attempt=args.attempt, scenario=args.scenario,
                  timestamp=datetime.datetime.now().astimezone().isoformat(), debug=True,
                  cache='ordinary_access_no_cold_warm_guarantee', elapsed_ms=(time.perf_counter()-clock)*1000,
                  operation=operation, measurement=measurement, cleanup=cleanup, stop=stop, resource_verdict=verdict,
                  exit_code=operation['exit_code'], result=observed.get('result'), stdout=observed.get('stdout', ''),
                  stderr=observed.get('stderr', ''), events=observed.get('events', []), failure=failure,
                  control_status=status, system_before=state, system_after=api.system_memory(), processes=metrics,
                  status_reads=controller.status_reads.result() if controller else {},
                  sampling='20ms requested;50ms discovery;actual valid intervals retained;short peaks/processes may be missed')
    with output.open('x', encoding='utf-8') as target:
        json.dump(ledger, target, indent=2)
    print(json.dumps(ledger))
    assert verdict == 'PASS', f'{verdict}: {failure or measurement["reasons"] or cleanup["status"]}'
    return ledger
