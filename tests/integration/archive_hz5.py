"""Stored ZIP reclamation scenarios over the original HZ4 exact-process driver."""
import ctypes
from ctypes import wintypes as w
import json
import os
from pathlib import Path
import shutil
import struct
import threading
import time
import uuid
import zipfile

PAYLOADS = {name: (bytes(range(251)) * (4 * 1024 * 1024 // 251 + 1))[:4 * 1024 * 1024]
            for name in ('first.bin', 'second.bin', 'third.bin')}


def matrix():
    cases = [{'name': 'complete', 'mode': 'run', 'point': 'hz5_after_complete', 'expect': 'complete'}]
    for point in ('before_flush', 'before_motw', 'before_metadata', 'metadata_applied',
                  'hz5_journal_ready_before_append', 'hz5_journal_ready_after_flush',
                  'commit_in_flight', 'native_published', 'native_committed',
                  'hz5_journal_pending_before_append', 'hz5_journal_pending_after_append',
                  'hz5_journal_pending_after_flush', 'hz5_journal_pending_after_readback',
                  'hz5_before_punch', 'hz5_after_punch', 'hz5_before_applied',
                  'hz5_journal_applied_before_append', 'hz5_journal_applied_after_append',
                  'hz5_journal_applied_after_flush', 'hz5_before_complete', 'hz5_after_complete',
                  'before_cleanup', 'after_cleanup'):
        incomplete = point in ('before_flush', 'before_motw', 'before_metadata', 'metadata_applied', 'hz5_journal_ready_before_append')
        cases.append({'name': 'crash-' + point, 'mode': 'crash', 'point': point,
                      'expect': 'refuse' if incomplete else 'complete'})
    for visit in (2, 3):
        for point in ('hz5_journal_pending_after_readback', 'hz5_before_punch', 'hz5_after_punch', 'hz5_journal_applied_after_flush'):
            cases.append({'name': 'crash-' + point + '-' + str(visit), 'mode': 'crash', 'point': point, 'visit': visit, 'expect': 'complete'})
    next(c for c in cases if c['name'] == 'crash-before_motw')['copy_motw'] = True
    cases.append({'name': 'complete-motw', 'mode': 'run', 'point': 'hz5_after_complete', 'copy_motw': True, 'expect': 'complete'})
    for point, code in (('before_flush', 112), ('before_flush', 1117),
                        ('hz5_journal_pending_before_append', 112),
                        ('hz5_journal_pending_after_append', 1117),
                        ('hz5_before_punch', 5), ('hz5_before_applied', 112)):
        cases.append({'name': 'fault-' + point + '-' + str(code), 'mode': 'native_failure',
                      'point': point, 'native_code': code,
                      'expect': 'refuse' if point == 'before_flush' else 'complete'})
    for change in ('missing-output', 'replaced-output', 'corrupt-output', 'missing-journal',
                   'corrupt-journal', 'torn-applied', 'missing-marker', 'corrupt-marker', 'replaced-source', 'replaced-root'):
        cases.append({'name': change, 'mode': 'crash', 'point': 'hz5_after_punch',
                      'tamper': change, 'expect': 'complete' if change == 'torn-applied' else 'refuse'})
    cases.append({'name': 'repeated-interruption', 'mode': 'crash', 'point': 'hz5_after_punch', 'repeat': True, 'expect': 'complete'})
    for point in ('before_flush', 'native_published'):
        cases.append({'name': 'ordinary-' + point, 'mode': 'crash', 'point': point, 'ordinary': True, 'expect': 'ordinary'})
    cases.append({'name': 'ordinary-complete', 'mode': 'run', 'point': 'before_cleanup', 'ordinary': True, 'expect': 'ordinary'})
    cases.append({'name': 'ordinary-flush-fault', 'mode': 'native_failure', 'point': 'before_flush', 'native_code': 112, 'ordinary': True, 'expect': 'ordinary'})
    for profile in ('no-allocation', 'deflate'):
        cases.append({'name': 'refuse-' + profile, 'mode': 'run', 'point': 'hz5_after_complete', 'profile': profile, 'expect': 'refuse'})
    for point in ('hz5_before_punch', 'hz5_after_punch'):
        cases.append({'name': 'cancel-' + point, 'mode': 'cancel', 'point': point, 'expect': 'complete'})
    cases.append({'name': 'refuse-source-overlap', 'mode': 'run', 'point': 'hz5_after_complete', 'profile': 'source-overlap', 'destination_archive': True, 'expect': 'refuse'})
    cases.append({'name': 'refuse-existing-output', 'mode': 'run', 'point': 'hz5_after_complete', 'profile': 'existing-output', 'expect': 'refuse'})
    cases.append({'name': 'complete-source-folder', 'mode': 'run', 'point': 'hz5_after_complete', 'profile': 'source-folder', 'destination_archive': True, 'expect': 'complete'})
    return cases


class StandardInfo(ctypes.Structure):
    _fields_ = [('allocation', ctypes.c_longlong), ('length', ctypes.c_longlong),
                ('links', w.DWORD), ('delete_pending', ctypes.c_ubyte), ('directory', ctypes.c_ubyte)]


def setup_native(kernel):
    kernel.CreateFileW.argtypes = [w.LPCWSTR, w.DWORD, w.DWORD, ctypes.c_void_p, w.DWORD, w.DWORD, w.HANDLE]
    kernel.CreateFileW.restype = w.HANDLE
    kernel.GetFileInformationByHandleEx.argtypes = [w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD]
    kernel.GetFileInformationByHandleEx.restype = w.BOOL
    kernel.GetDiskFreeSpaceExW.argtypes = [w.LPCWSTR, ctypes.POINTER(ctypes.c_ulonglong), ctypes.POINTER(ctypes.c_ulonglong), ctypes.POINTER(ctypes.c_ulonglong)]
    kernel.GetDiskFreeSpaceExW.restype = w.BOOL


def allocation(kernel, path):
    handle = kernel.CreateFileW(str(path), 0x80, 7, None, 3, 0x200000, None)
    if handle == ctypes.c_void_p(-1).value: raise OSError(ctypes.get_last_error(), 'CreateFileW attributes')
    try:
        info = StandardInfo()
        if not kernel.GetFileInformationByHandleEx(handle, 1, ctypes.byref(info), ctypes.sizeof(info)):
            raise OSError(ctypes.get_last_error(), 'GetFileInformationByHandleEx allocation')
        assert info.allocation >= 0
        return info.allocation
    finally: kernel.CloseHandle(handle)


def measure(kernel, root):
    source = root / 'archive' / 'source.zip'
    files = [p for folder in (root / 'archive', root / 'output') for p in folder.rglob('*') if p.is_file()]
    allocated = {}; unavailable = []
    for path in files:
        try: allocated[path.relative_to(root).as_posix()] = allocation(kernel, path)
        except OSError as failure: unavailable.append({'path': str(path.relative_to(root)), 'native_code': failure.errno})
    marker = str(source) + ':NxTools.HZ5'
    try: allocated['archive/source.zip:NxTools.HZ5'] = allocation(kernel, marker)
    except OSError as failure:
        if failure.errno not in (2, 3): unavailable.append({'path': 'archive/source.zip:NxTools.HZ5', 'native_code': failure.errno})
    available = ctypes.c_ulonglong(); total = ctypes.c_ulonglong(); free = ctypes.c_ulonglong()
    assert kernel.GetDiskFreeSpaceExW(str(root), ctypes.byref(available), ctypes.byref(total), ctypes.byref(free))
    return {'time_ns': time.time_ns(), 'allocated': allocated, 'total_allocated': sum(allocated.values()),
            'free_bytes': free.value, 'available_bytes': available.value, 'unavailable': unavailable}


def journal(root):
    paths = list((root / 'archive').glob('.nxtools-hz5-*.journal'))
    assert len(paths) == 1, ('one actual journal', paths)
    data = paths[0].read_bytes(); records = []; position = 0
    while position < len(data):
        assert len(data) - position >= 8, 'unexpected torn journal'
        size, = struct.unpack_from('<I', data, position)
        assert 0 < size <= 65536 and position + size + 8 <= len(data)
        assert struct.unpack_from('<I', data, position + 4 + size)[0] == size
        record = json.loads(data[position + 4:position + 4 + size]); records.append(record)
        assert record['schema'] == 1 and record['sequence'] == len(records)
        position += size + 8
    return paths[0], records


def seed(root, profile=None):
    source = root / 'archive' / 'source.zip'
    contents = {name: b'owned' for name in PAYLOADS} if profile == 'no-allocation' else PAYLOADS
    contents = dict(contents)
    if profile == 'source-overlap': contents['source.zip'] = b'owned nested payload' * 4096
    method = zipfile.ZIP_DEFLATED if profile == 'deflate' else zipfile.ZIP_STORED
    with zipfile.ZipFile(source, 'w', compression=method) as writer:
        for name, value in contents.items():
            item = zipfile.ZipInfo(name, (2023, 11, 14, 22, 13, 20)); item.external_attr = 0o100644 << 16
            writer.writestr(item, value, compress_type=method)
        item = zipfile.ZipInfo('empty/', (2023, 11, 14, 22, 13, 20)); item.external_attr = (0o40755 << 16) | 16
        writer.writestr(item, b'')
    structure = []
    with zipfile.ZipFile(source) as reader:
        assert reader.testzip() is None
        with source.open('rb') as stream:
            for item in reader.infolist():
                assert reader.read(item) == contents.get(item.filename, b'')
                stream.seek(item.header_offset); header = stream.read(30)
                name_length, extra_length = struct.unpack_from('<HH', header, 26)
                length = 30 + name_length + extra_length
                stream.seek(item.header_offset); structure.append((item.header_offset, stream.read(length)))
            stream.seek(reader.start_dir); structure.append((reader.start_dir, stream.read()))
    return source, structure


def check_outputs(root, destination_archive=False):
    folder = root / ('archive' if destination_archive else 'output')
    actual = {p.relative_to(folder).as_posix() for p in folder.rglob('*')}
    if destination_archive: actual = {p for p in actual if p != 'source.zip' and not p.startswith('.nxtools-hz5-')}
    assert actual == set(PAYLOADS) | {'empty'}, ('exact outputs, no stranded partial', actual)
    for name, value in PAYLOADS.items():
        assert (folder / name).read_bytes() == value, ('independent exact output bytes', name)
    assert (folder / 'empty').is_dir()


def mutate(root, change):
    source = root / 'archive' / 'source.zip'; output = root / 'output' / 'first.bin'
    path, records = journal(root)
    if change == 'missing-output': output.unlink()
    elif change == 'replaced-output':
        output.rename(root / 'evidence' / 'original-output.bin'); output.write_bytes(PAYLOADS['first.bin'])
    elif change == 'corrupt-output':
        with output.open('r+b') as stream: stream.write(b'CORRUPTED')
    elif change == 'missing-journal': path.rename(root / 'evidence' / 'missing.journal')
    elif change == 'corrupt-journal':
        data = bytearray(path.read_bytes()); data[4] = ord('!'); path.write_bytes(data)
    elif change == 'torn-applied':
        # Simulate an incomplete applied append while preserving the complete pending intent.
        with path.open('ab') as stream: stream.write(struct.pack('<I', 80) + b'{"schema":1'); stream.flush(); os.fsync(stream.fileno())
    elif change == 'missing-marker': os.unlink(str(source) + ':NxTools.HZ5')
    elif change == 'corrupt-marker': Path(str(source) + ':NxTools.HZ5').write_bytes(b'corrupt')
    elif change == 'replaced-source':
        source.rename(root / 'evidence' / 'consumed-source.zip'); source.write_bytes(b'FOREIGN REPLACEMENT')
    elif change == 'replaced-root':
        (root / 'output').rename(root / 'evidence' / 'original-output-root'); (root / 'output').mkdir()
    else: raise AssertionError(change)


def execute(case, args, driver):
    # Native destructive capability deliberately accepts direct session-scratch hz5-GUID roots only.
    root = Path(os.environ['PI_SCRATCH_DIR']).resolve() / ('hz5-' + str(uuid.uuid4()))
    root.mkdir(); (root / 'archive').mkdir(); (root / 'output').mkdir(); (root / 'evidence').mkdir()
    shutil.copyfile(args.worker, root / 'NxTools.ArchiveWorker.exe')
    canary = root / 'canary.bin'; canary.write_bytes(b'UNRELATED OWNED CANARY\x00\xff')
    result = {'case': case, 'root': str(root), 'status': 'FAIL', 'runs': [], 'power_loss_tested': False}
    samples = []; sampler_stop = threading.Event()
    def sample():
        try: samples.append(measure(driver.K, root))
        except OSError: pass  # namespace can change during publication; exact boundary samples remain separately recorded
    sampler = None
    try:
        source, structure = seed(root, case.get('profile')); original_size = source.stat().st_size
        if case.get('copy_motw'): Path(str(source) + ':Zone.Identifier').write_bytes(b'[ZoneTransfer]\r\nZoneId=3\r\n')
        if case.get('profile') == 'existing-output': (root / 'output' / 'third.bin').write_bytes(b'FOREIGN-KEEP')
        original_bytes = source.read_bytes(); original_stat = source.stat()
        result['before'] = measure(driver.K, root); samples.append(result['before'])
        recovery = str(uuid.uuid4())
        def run(mode, point, intent, prior=None, visit=1):
            specification = dict(case, format='Zip', kind='extract' if case.get('ordinary') else 'reclamation', point=point, visit=visit,
                                 reclamation_mode=intent, recovery_id=recovery)
            value = driver.launch(root, args.host, driver.sha(args.worker), specification, mode, prior)
            result['runs'].append(value); sample()
            samples.extend(value.get('native_allocation_samples', []))
            assert value['status'] == 'PASS', value.get('failure')
            return value
        def observe():
            while not sampler_stop.wait(.005): sample()
        sampler = threading.Thread(target=observe, daemon=True); sampler.start()
        first = run(case['mode'], case['point'], 'start', visit=case.get('visit', 1))
        if case['mode'] == 'native_failure':
            assert first['terminal']['outcome'] != 'succeeded', 'injected failure must not report success'
            assert first['receipt'] is not None, 'fault hook actually reached'
            assert first['terminal']['native_error_code'] == case['native_code'], 'owning injected native code is retained'
        elif case['mode'] == 'run':
            assert first['terminal']['outcome'] == ('failed' if case.get('profile') and case['expect'] == 'refuse' else 'succeeded'), first['terminal']
        elif case['mode'] == 'cancel':
            assert first['receipt'] is not None and first['terminal']['outcome'] == 'cancelled', 'original user cancellation remains cancelled'
        if case.get('ordinary'):
            assert source.read_bytes() == original_bytes, 'ordinary mode preserves every source byte'
            current_stat = source.stat()
            assert (current_stat.st_dev, current_stat.st_ino, current_stat.st_size, current_stat.st_mtime_ns) == (original_stat.st_dev, original_stat.st_ino, original_stat.st_size, original_stat.st_mtime_ns)
            with zipfile.ZipFile(source) as reader: assert reader.testzip() is None
            published = [p for p in (root / 'output').rglob('*') if p.is_file() and not p.name.startswith('.nxpart-')]
            assert len(published) == (len(PAYLOADS) if case['mode'] == 'run' else 1 if case['point'] == 'native_published' else 0)
            for path in published: assert path.read_bytes() == PAYLOADS[path.name]
            snapshot = driver.disk_state(root)
            restarted = run('restart', case['point'], 'continue', first['capability'])
            assert restarted['identity']['created'] != first['identity']['created']
            assert driver.disk_state(root) == snapshot, 'ordinary fresh restart remains preserving stale-operation refusal'
            result['recovery_result'] = 'ordinary_source_preserved_restart_refused'
            result['status'] = 'PASS'
            return result
        if case.get('profile') and case['expect'] == 'refuse':
            assert source.read_bytes() == original_bytes, 'rejected plan must preserve every source byte'
            assert first['terminal']['files_committed'] == 0, 'reject the whole plan before any output or source release'
            assert not Path(str(source) + ':NxTools.HZ5').exists(), 'unsealed plan does not mark an intact original'
            if case['profile'] in ('no-allocation', 'deflate'):
                assert not list((root / 'archive').glob('.nxtools-hz5-*.journal')), 'unqualified source has no recovery authority'
            if case['profile'] == 'source-overlap':
                assert not any((root / 'archive' / name).exists() for name in PAYLOADS)
            elif case['profile'] == 'existing-output':
                assert set(p.name for p in (root / 'output').iterdir()) == {'third.bin'}
                assert (root / 'output' / 'third.bin').read_bytes() == b'FOREIGN-KEEP'
            else: assert not list((root / 'output').iterdir()), 'unqualified mode produces no output'
        if case.get('tamper'): mutate(root, case['tamper'])
        if case.get('repeat'):
            second = run('crash', 'hz5_after_punch', 'continue', first['capability'], visit=2)
            assert second['identity']['created'] != first['identity']['created']
        before_continue = source.read_bytes()
        continued = run('run', 'hz5_after_complete', 'continue', first['capability'])
        assert continued['identity']['created'] != first['identity']['created']
        assert continued['capability']['operation_id'] != first['capability']['operation_id']
        assert continued['capability']['session_id'] != first['capability']['session_id']
        assert continued['capability']['recovery_id'] == recovery
        if case['expect'] == 'refuse':
            assert continued['terminal']['outcome'] != 'succeeded', 'unsafe continuation must refuse'
            assert source.read_bytes() == before_continue, 'refusal must not release another source range'
            result['recovery_result'] = 'explicit_preserving_refusal'
        else:
            assert continued['terminal']['outcome'] == 'succeeded', continued['terminal']
            check_outputs(root, case.get('destination_archive', False))
            if case.get('copy_motw'):
                for name in PAYLOADS:
                    assert Path(str(root / 'output' / name) + ':Zone.Identifier').read_bytes() == b'[ZoneTransfer]\r\nZoneId=3\r\n', 'actual output MOTW'
            assert source.stat().st_size == original_size, 'not truncation or source deletion'
            with source.open('rb') as stream:
                for offset, expected in structure:
                    stream.seek(offset); assert stream.read(len(expected)) == expected, 'retained ZIP structure unchanged'
            _, records = journal(root)
            assert records[-1]['kind'] == 'completed_consumed'
            plans = [r['value'] for r in records if r['kind'] == 'plan_entry']
            applied = [r['value'] for r in records if r['kind'] == 'applied']
            assert len(applied) == len(plans) == 3
            assert [r['index'] for r in applied] == list(range(3)), 'no duplicate applied/counting'
            result['recorded_released_bytes'] = sum(r['released_bytes'] for r in applied)
            result['journal_kinds'] = [r['kind'] for r in records]
            with source.open('rb') as stream:
                for plan in plans:
                    stream.seek(plan['release_start'])
                    assert not any(stream.read(plan['release_end'] - plan['release_start'])), 'real consumed payload hole'
            result['after'] = measure(driver.K, root)
            released = result['before']['allocated']['archive/source.zip'] - result['after']['allocated']['archive/source.zip']
            assert released > 0 and released == result['recorded_released_bytes'], ('actual independent allocation reduction', released, result['recorded_released_bytes'])
            result['actual_source_allocation_reduction'] = released
            result['recovery_result'] = 'remaining_extraction_completed'
            # A second entirely fresh invocation verifies the completed receipt rather than replaying extraction.
            verified = run('run', 'hz5_after_complete', 'continue', continued['capability'])
            assert verified['terminal']['outcome'] == 'succeeded'
            assert journal(root)[1] == records, 'completed recognition must not append duplicate authority'
        assert canary.read_bytes() == b'UNRELATED OWNED CANARY\x00\xff'
        result['status'] = 'PASS'
    except Exception as failure:
        result['failure'] = str(failure)
    finally:
        sampler_stop.set()
        if sampler is not None: sampler.join(timeout=5)
        sample()
        complete_samples = [s for s in samples if not s['unavailable']]
        if complete_samples and 'before' in result:
            result['observed_peak_additional_allocation'] = max(s['total_allocated'] for s in complete_samples) - result['before']['total_allocated']
            result['free_space_observed_min'] = min(s['free_bytes'] for s in complete_samples)
            result['free_space_observed_max'] = max(s['free_bytes'] for s in complete_samples)
        result['allocation_sampling_interval_ms'] = 5
        driver.save(root / 'allocation-samples.json', samples)
        driver.save(root / 'case-result.json', result)
    return result


def run(args, driver):
    setup_native(driver.K)
    selected = [c for c in matrix() if args.cases is None or c['name'] in args.cases]
    assert selected and (args.cases is None or set(args.cases) == {c['name'] for c in selected})
    driver.save(args.evidence / 'identities.json', {'host': str(args.host), 'worker': str(args.worker),
        'host_sha256': driver.sha(args.host), 'worker_sha256': driver.sha(args.worker),
        'driver_sha256': driver.sha(__file__), 'process_driver_sha256': driver.sha(driver.__file__)})
    results = []
    for case in selected:
        value = execute(case, args, driver); results.append(value)
        print(case['name'], value['status'], value.get('failure', ''), flush=True)
    driver.save(args.evidence / 'results.json', results)
    accounting = {'selected': len(results), 'pass': sum(v['status'] == 'PASS' for v in results),
                  'fail': sum(v['status'] != 'PASS' for v in results), 'skip': 0, 'power_loss_tested': False}
    driver.save(args.evidence / 'accounting.json', accounting); print(json.dumps(accounting))
    return int(accounting['fail'] != 0)
