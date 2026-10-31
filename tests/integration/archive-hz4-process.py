#!/usr/bin/env python3
"""HZ4 exact owned process termination + fresh original-session refusal; NOT durable recovery.
Windows only, explicit frozen testapphost/worker/oracles in session scratch. Retains all evidence.
Never targets a process name, launches a product, replays a terminal or cleans a partial by path.
"""
import argparse
import ctypes
from ctypes import wintypes as w
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tarfile
import time
import uuid
import zipfile
_reader_spec = importlib.util.spec_from_file_location('hz4_reader', Path(__file__).with_name('archive_hz4_reader.py'))
_reader = importlib.util.module_from_spec(_reader_spec)
_reader_spec.loader.exec_module(_reader)

K = ctypes.WinDLL('kernel32', use_last_error=True) if os.name == 'nt' else None
TERMINATED = 0x485A3401
FILES = {'first.txt': b'first owned payload\n', 'second.txt': b'second owned payload\n'}
EXT = {'Zip': 'zip', 'SevenZ': '7z', 'Tar': 'tar', 'TarGz': 'tgz', 'TarXz': 'txz'}


def sha(path):
    return hashlib.file_digest(open(path, 'rb'), 'sha256').hexdigest().upper()


def save(path, value):
    with open(path, 'x', encoding='utf-8') as stream:
        json.dump(value, stream, indent=2, ensure_ascii=False)


def native_setup():
    signatures = {
        'GetProcessTimes': (w.BOOL, [w.HANDLE, ctypes.POINTER(w.FILETIME), ctypes.POINTER(w.FILETIME), ctypes.POINTER(w.FILETIME), ctypes.POINTER(w.FILETIME)]),
        'GetProcessId': (w.DWORD, [w.HANDLE]),
        'GetCurrentProcess': (w.HANDLE, []),
        'QueryFullProcessImageNameW': (w.BOOL, [w.HANDLE, w.DWORD, w.LPWSTR, ctypes.POINTER(w.DWORD)]),
        'TerminateProcess': (w.BOOL, [w.HANDLE, w.UINT]),
        'WaitForSingleObject': (w.DWORD, [w.HANDLE, w.DWORD]),
        'OpenProcess': (w.HANDLE, [w.DWORD, w.BOOL, w.DWORD]),
        'CloseHandle': (w.BOOL, [w.HANDLE]),
        'CreateToolhelp32Snapshot': (w.HANDLE, [w.DWORD, w.DWORD]),
    }
    for name, (result, arguments) in signatures.items():
        function = getattr(K, name); function.restype = result; function.argtypes = arguments


class ProcessEntry(ctypes.Structure):
    _fields_ = [('size', w.DWORD), ('usage', w.DWORD), ('pid', w.DWORD), ('heap', ctypes.c_size_t),
                ('module', w.DWORD), ('threads', w.DWORD), ('parent', w.DWORD), ('priority', w.LONG),
                ('flags', w.DWORD), ('exe', w.WCHAR * 260)]


def process_table():
    snapshot = K.CreateToolhelp32Snapshot(2, 0)
    assert snapshot != ctypes.c_void_p(-1).value
    try:
        entry = ProcessEntry(); entry.size = ctypes.sizeof(entry); rows = []
        for name in ('Process32FirstW', 'Process32NextW'):
            getattr(K, name).argtypes = [w.HANDLE, ctypes.POINTER(ProcessEntry)]
            getattr(K, name).restype = w.BOOL
        more = K.Process32FirstW(snapshot, ctypes.byref(entry))
        while more:
            rows.append({'pid': entry.pid, 'parent': entry.parent, 'exe': entry.exe})
            more = K.Process32NextW(snapshot, ctypes.byref(entry))
        return rows
    finally:
        K.CloseHandle(snapshot)


def identity(handle):
    created, exited, kernel, user = (w.FILETIME() for _ in range(4))
    assert K.GetProcessTimes(handle, ctypes.byref(created), ctypes.byref(exited), ctypes.byref(kernel), ctypes.byref(user))
    buffer = ctypes.create_unicode_buffer(32768); size = w.DWORD(len(buffer))
    assert K.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(size))
    return {'pid': int(K.GetProcessId(handle)), 'created': (created.dwHighDateTime << 32) | created.dwLowDateTime,
            'image': buffer.value, 'retained_handle': int(handle)}


def read_json(path):
    try:
        return json.loads(path.read_text(encoding='utf-8-sig'))
    except (OSError, json.JSONDecodeError):
        return None


def external(argv, evidence):
    start = time.time(); run = subprocess.run(list(map(str, argv)), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
    evidence.append({'argv': list(map(str, argv)), 'exit': run.returncode, 'elapsed_ms': round((time.time() - start) * 1000),
                     'stdout_sha256': hashlib.sha256(run.stdout).hexdigest(), 'stderr_sha256': hashlib.sha256(run.stderr).hexdigest()})
    assert run.returncode == 0, ('independent oracle failed', argv, run.returncode, run.stderr.decode(errors='replace'))
    return run.stdout


def inventory(root):
    result = {}
    if not root.exists() and not root.is_symlink():
        return result
    def walk(path, relative):
        info = path.lstat(); attrs = getattr(info, 'st_file_attributes', 0)
        value = {'kind': 'reparse' if attrs & stat.FILE_ATTRIBUTE_REPARSE_POINT else 'directory' if path.is_dir() else 'file',
                 'size': info.st_size, 'mtime_ns': info.st_mtime_ns, 'attributes': attrs, 'device': info.st_dev, 'file_id': info.st_ino,
                 'links': info.st_nlink}
        if value['kind'] == 'file': value['sha256'] = sha(path)
        if value['kind'] == 'reparse': value['target'] = os.readlink(path)
        result[relative] = value
        if value['kind'] == 'directory':
            for item in sorted(path.iterdir(), key=lambda p: p.name): walk(item, relative + '/' + item.name)
    walk(root, '.')
    return result


def disk_state(root):
    return {name: inventory(root / name) for name in ('archive', 'output', 'canary.bin', 'recovery-claim.json', 'redirect', 'output-prior')}


def seed(root, format_name, sevenzip, commands, sevenz_header='compressed'):
    archive = root / 'archive' / ('source.' + EXT[format_name])
    if format_name == 'Zip':
        with zipfile.ZipFile(archive, 'w') as writer:
            for name, content in FILES.items():
                item = zipfile.ZipInfo(name, (2023, 11, 14, 22, 13, 20)); item.external_attr = 0o100640 << 16
                writer.writestr(item, content, compress_type=zipfile.ZIP_DEFLATED)
            item = zipfile.ZipInfo('empty/', (2023, 11, 14, 22, 13, 20)); item.external_attr = (0o40750 << 16) | 16
            writer.writestr(item, b'')
    elif format_name == 'SevenZ':
        inputs = root / 'seed'; inputs.mkdir(); (inputs / 'empty').mkdir()
        for name, content in FILES.items(): (inputs / name).write_bytes(content)
        for item in inputs.iterdir(): os.utime(item, (1700000000, 1700000000))
        options = ['-mhc=off'] if sevenz_header == 'plain' else []
        external([sevenzip, 'a', '-t7z', '-m0=lzma2', '-ms=off', '-mx=1', *options, '-y', archive, str(inputs / '*')], commands)
    else:
        mode = {'Tar': 'w', 'TarGz': 'w:gz', 'TarXz': 'w:xz'}[format_name]
        with tarfile.open(archive, mode, format=tarfile.USTAR_FORMAT) as writer:
            for name, content in FILES.items():
                item = tarfile.TarInfo(name); item.size = len(content); item.mode = 0o640; item.uid = 123; item.gid = 456; item.mtime = 1700000000
                writer.addfile(item, io.BytesIO(content))
            item = tarfile.TarInfo('empty/'); item.type = tarfile.DIRTYPE; item.mode = 0o750; item.uid = 123; item.gid = 456; item.mtime = 1700000000
            writer.addfile(item)
    return archive


def oracle(archive, format_name, sevenzip, bsdtar, commands, secondary_failures=None, secondary_results=None, qualified=False):
    result = {}
    if format_name == 'Zip':
        with zipfile.ZipFile(archive) as reader:
            assert reader.testzip() is None
            for item in reader.infolist():
                content = reader.read(item)
                assert item.filename not in result
                result[item.filename] = {'kind': 'directory' if item.is_dir() else 'file', 'size': item.file_size,
                    'sha256': hashlib.sha256(content).hexdigest().upper(), 'date_time': item.date_time,
                    'attributes': item.external_attr, 'comment_hex': item.comment.hex(), 'crc': item.CRC}
        external([sevenzip, 't', '-y', archive], commands)
    elif format_name == 'SevenZ':
        external([sevenzip, 't', '-y', archive], commands)
        listing = external([sevenzip, 'l', '-slt', '-ba', '-sccUTF-8', archive], commands).decode('utf-8-sig')
        for block in listing.replace('\r', '').strip().split('\n\n'):
            fields = dict(line.split(' = ', 1) for line in block.splitlines() if ' = ' in line)
            if 'Path' not in fields: continue
            directory = fields.get('Folder') == '+' or fields.get('Attributes', '').startswith('D')
            name = fields['Path'].replace('\\', '/') + ('/' if directory and not fields['Path'].endswith('/') else '')
            content = b'' if directory else external([sevenzip, 'x', '-so', '-y', archive, fields['Path']], commands)
            assert name not in result
            result[name] = {'kind': 'directory' if directory else 'file', 'size': len(content), 'sha256': hashlib.sha256(content).hexdigest().upper(),
                            'modified': fields.get('Modified'), 'attributes': fields.get('Attributes'), 'encrypted': fields.get('Encrypted')}
            assert fields.get('Encrypted', '-') == '-', 'unopened encryption is not success'
        secondary = {'status': 'PASS', 'reader': 'bsdtar'}
        try:
            external([bsdtar, '-tf', archive], commands)
        except AssertionError as failure:
            if secondary_failures is None: raise
            secondary_failures.append(str(failure))  # never erase actual failing-reader evidence
            secondary = {'status': 'FAIL', 'reader': 'bsdtar', 'failure': str(failure)}
            if qualified:
                try:
                    assert commands[-1]['exit'] == 1 and 'Damaged 7-Zip archive' in str(failure), 'not the witnessed secondary failure'
                    version = external([bsdtar, '--version'], commands).decode('utf-8').strip()
                    secondary.update(_reader.qualify_bytes(archive.read_bytes(), sha(sevenzip), sha(bsdtar), version))
                    secondary['version'] = version
                except Exception as rejected:
                    secondary['qualification_failure'] = str(rejected)
        if secondary_results is not None: secondary_results.append(secondary)
    else:
        with tarfile.open(archive, 'r:*') as reader:
            for item in reader:
                assert item.isfile() or item.isdir()
                name = item.name + ('/' if item.isdir() and not item.name.endswith('/') else '')
                content = reader.extractfile(item).read() if item.isfile() else b''
                assert name not in result
                result[name] = {'kind': 'directory' if item.isdir() else 'file', 'size': item.size, 'sha256': hashlib.sha256(content).hexdigest().upper(),
                    'mode': item.mode, 'uid': item.uid, 'gid': item.gid, 'mtime': item.mtime, 'uname': item.uname, 'gname': item.gname, 'pax': item.pax_headers}
        external([bsdtar, '-tf', archive], commands)
    assert result, 'independent archive must be readable'
    return result


def check_manifest(actual, published):
    expected = set(FILES) | {'empty/'} | ({'published/'} if published else set())
    assert set(actual) == expected, ('exact independent members', set(actual), expected)
    for name, content in FILES.items():
        assert actual[name]['kind'] == 'file' and actual[name]['size'] == len(content)
        assert actual[name]['sha256'] == hashlib.sha256(content).hexdigest().upper()
    for name in expected - set(FILES): assert actual[name]['kind'] == 'directory'


def launch(root, host, worker_hash, case, mode, prior=None):
    nonce = str(uuid.uuid4()); driver = identity(K.GetCurrentProcess())
    cap = {'schema': 1, 'root': str(root), 'nonce': nonce, 'driver_pid': os.getpid(), 'driver_created': driver['created'],
           'mode': mode, 'format': case['format'], 'kind': case['kind'], 'point': case['point'], 'visit': case.get('visit', 1),
           'worker_sha256': worker_hash, 'logical_id': case.get('recovery_id', (prior or {}).get('logical_id', str(uuid.uuid4()))),
           'operation_id': (prior or {}).get('operation_id', str(uuid.uuid4())) if case['kind'] != 'reclamation' else str(uuid.uuid4()),
           'session_id': (prior or {}).get('session_id', str(uuid.uuid4())) if case['kind'] != 'reclamation' else str(uuid.uuid4())}
    cap.update({key: case[key] for key in ('native_code', 'reclamation_mode', 'recovery_id', 'copy_motw', 'destination_archive') if key in case})
    save(root / ('capability-' + nonce + '.json'), cap)
    result = {'capability': cap, 'argv': [str(host), '--hz4-owned-process', str(root), nonce], 'worker_handles': []}
    stdout = root / 'evidence' / (nonce + '.stdout'); stderr = root / 'evidence' / (nonce + '.stderr')
    owned = []; child = None; started = time.time()
    try:
        with open(stdout, 'xb') as out, open(stderr, 'xb') as err:
            child = subprocess.Popen(result['argv'], stdout=out, stderr=err)
            exact = identity(int(child._handle)); result['identity'] = exact
            assert exact['pid'] == child.pid and Path(exact['image']) == host
            assert any(row['pid'] == child.pid and row['parent'] == os.getpid() for row in process_table()), 'Popen child parent ownership'
            receipt_path = root / 'evidence' / (nonce + '.receipt.json')
            deadline = time.monotonic() + 35
            receipt = None
            while time.monotonic() < deadline:
                receipt = read_json(receipt_path)
                if observer := case.get('_sample'): observer()
                if receipt is not None or child.poll() is not None: break
                time.sleep(.01)
            result['receipt'] = receipt
            if mode == 'crash':
                if receipt is None:
                    child.wait(timeout=10)
                    result['terminal'] = read_json(root / 'evidence' / (nonce + '.terminal.json'))
                    raise AssertionError('requested original-owner hook not visited; exit/terminal is not crashpoint PASS')
                assert child.poll() is None, 'receipt must identify a still-live child'
                assert receipt['nonce'] == nonce and receipt['operation_id'] == cap['operation_id'] and receipt['session_id'] == cap['session_id']
                assert receipt['stage'] == case['point'] and receipt['visit'] == case.get('visit', 1)
                assert receipt['identity']['process_id'] == exact['pid'] and receipt['identity']['created'] == exact['created']
                assert identity(int(child._handle)) == exact, 'same retained process object before termination'
                for row in process_table():
                    if row['parent'] != child.pid: continue
                    handle = K.OpenProcess(0x101000, False, row['pid'])
                    if not handle: continue  # already exited child is not a termination target
                    owned.append(handle); observed = identity(handle)
                    assert Path(observed['image']) == root / 'NxTools.ArchiveWorker.exe', 'only original owned worker child'
                    result['worker_handles'].append(observed)
                assert len(owned) == (0 if case['point'] in ('before_cleanup', 'after_cleanup') else 1), 'actual worker lifetime at selected seam'
                if case['kind'] == 'edit' and case['point'] in ('edit_source_rechecked', 'commit_in_flight'):
                    assert receipt['source_exclusive_read_open'] == (case['point'] == 'commit_in_flight'), 'actual readers/FILE-only handoff'
                assert K.TerminateProcess(int(child._handle), TERMINATED), 'terminate exact retained Popen handle only'
                result['termination'] = {'method': 'TerminateProcess_retained_Popen_handle', 'code': TERMINATED}
            child.wait(timeout=15)
            result['exit'] = child.returncode
            if mode == 'crash': assert child.returncode & 0xffffffff == TERMINATED
            else: assert child.returncode == 0, ('fixture failure is not requested fault', stdout.read_text(errors='replace'))
            for handle in owned: assert K.WaitForSingleObject(handle, 10000) == 0, 'original Job released exact worker after owned Host death'
            result['worker_exit_verified'] = True
            if observer := case.get('_sample'): observer()
            if mode == 'restart':
                result['restart'] = read_json(root / 'evidence' / (nonce + '.restart.json'))
                assert result['restart']['code'] == 'stale_operation' and not result['restart']['replayed'] and not result['restart']['cleanup_attempted']
            else: result['terminal'] = read_json(root / 'evidence' / (nonce + '.terminal.json'))
            metrics_path = root / 'evidence' / (nonce + '.allocation.jsonl')
            if metrics_path.exists():
                result['native_allocation_samples'] = [json.loads(line) for line in metrics_path.read_text(encoding='utf-8-sig').splitlines()]
        result['status'] = 'PASS'
    except Exception as failure:
        result['status'] = 'FAIL'; result['failure'] = str(failure)
    finally:
        if child is not None and child.poll() is None:
            assert identity(int(child._handle)) == result['identity']
            assert K.TerminateProcess(int(child._handle), TERMINATED)
            child.wait(timeout=15)
        for handle in owned: K.CloseHandle(handle)
        result['elapsed_ms'] = round((time.time() - started) * 1000)
        if child is not None: result['exit'] = child.returncode
        save(root / 'evidence' / (nonce + '.driver.json'), result)
    return result


def execute(case, args):
    case = dict(case, seed_profile='stock7zip-' + args.sevenz_header if case['format'] == 'SevenZ' else 'independent-default')
    root = args.evidence / ('archive-product-' + str(uuid.uuid4()))
    root.mkdir(); (root / 'archive').mkdir(); (root / 'output').mkdir(); (root / 'evidence').mkdir()
    (root / 'canary.bin').write_bytes(b'UNRELATED OWNED CANARY\x00\xff')
    shutil.copyfile(args.worker, root / 'NxTools.ArchiveWorker.exe')
    result = {'case': case, 'root': str(root), 'oracle_commands': [], 'secondary_reader_failures': [],
              'secondary_reader_results': [], 'process_integrity_status': 'FAIL', 'status': 'FAIL'}
    try:
        source = seed(root, case['format'], args.sevenzip, result['oracle_commands'], args.sevenz_header)
        original = oracle(source, case['format'], args.sevenzip, args.bsdtar, result['oracle_commands']); check_manifest(original, False)
        result['before'] = disk_state(root); result['original_oracle'] = original
        run = launch(root, args.host, sha(args.worker), case, case.get('mode', 'crash')); result['interruption'] = run
        # Preserve independently inspected actual disk facts even when instrumentation RED never reached its requested hook.
        result['after'] = disk_state(root)
        actual = oracle(source, case['format'], args.sevenzip, args.bsdtar, result['oracle_commands'],
                        result['secondary_reader_failures'], result['secondary_reader_results'],
                        args.qualified_bsdtar_partial_attributes); result['after_oracle'] = actual
        assert result['before']['canary.bin'] == result['after']['canary.bin']
        if run['status'] != 'PASS': raise AssertionError(run['failure'])
        published = case['kind'] == 'edit' and case['point'] in ('native_published', 'before_cleanup', 'after_cleanup')
        check_manifest(actual, published)
        for name, facts in original.items(): assert actual[name] == facts, ('unchanged member metadata', name, facts, actual[name])
        if not published: assert result['before']['archive']['./' + source.name] == result['after']['archive']['./' + source.name], 'old source bytes/native identity/metadata unchanged'
        if case['kind'] == 'extract':
            targets = result['after']['output']; finals = {p[2:]: facts for p, facts in targets.items() if facts['kind'] == 'file' and not p.rsplit('/', 1)[-1].startswith('.nxpart-')}
            for name, facts in finals.items():
                assert name in FILES and facts['sha256'] == hashlib.sha256(FILES[name]).hexdigest().upper()
            wanted = case.get('visit', 1) if case['point'] == 'native_published' else 2 if case['point'] in ('before_cleanup', 'after_cleanup') else 0
            assert len(finals) == wanted, ('actual native-published file count', len(finals), wanted)
            assert all(p == '.' or facts['kind'] != 'directory' or p == './empty' for p, facts in targets.items()), 'no invented wrapper directories'
        snapshot = disk_state(root)
        restart = launch(root, args.host, sha(args.worker), case, 'restart', run['capability']); result['restart'] = restart
        assert restart['status'] == 'PASS' and restart['identity']['created'] != run['identity']['created']
        assert restart['identity']['pid'] != run['identity']['pid'], 'fresh process, not same runtime'
        assert disk_state(root) == snapshot, 'fresh refusal preserves all source/output/partials/claims/canaries'
        result['process_integrity_status'] = 'PASS'
        assert not result['secondary_reader_failures'] or (result['secondary_reader_results'] and
            all(v['status'] == 'reader-unsupported' for v in result['secondary_reader_results'])), (
            'secondary independent reader failed; still FAIL', result['secondary_reader_results'])
        result['status'] = 'PASS'
    except Exception as failure:
        result['failure'] = str(failure)
    save(root / 'case-result.json', result)
    return result


def matrix():
    cases = []
    for format_name in EXT:
        for point in ('edit_verifying', 'candidate_write', 'edit_source_rechecked', 'commit_in_flight', 'native_published', 'before_cleanup', 'after_cleanup'):
            cases.append({'name': 'edit-' + format_name + '-' + point, 'format': format_name, 'kind': 'edit', 'point': point})
    for format_name in ('Zip', 'SevenZ', 'Tar'):
        for point, visit in (('candidate_write', 1), ('commit_in_flight', 1), ('native_published', 1), ('native_published', 2), ('before_cleanup', 1), ('after_cleanup', 1)):
            cases.append({'name': 'extract-' + format_name + '-' + point + '-' + str(visit), 'format': format_name, 'kind': 'extract', 'point': point, 'visit': visit})
    return cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('host', 'worker', 'evidence'): parser.add_argument('--' + name, type=Path, required=True)
    for name in ('sevenzip', 'bsdtar'): parser.add_argument('--' + name, type=Path)
    parser.add_argument('--hz5', action='store_true', help='Stored ZIP actual allocation and durable fresh-process continuation')
    parser.add_argument('--cases', nargs='+')
    parser.add_argument('--sevenz-header', choices=('compressed', 'plain'), default='compressed')
    parser.add_argument('--qualified-bsdtar-partial-attributes', action='store_true',
                        help='opt in to the exact parent-approved HZ4-7Z02 reader-unsupported accounting, never reader PASS')
    args = parser.parse_args(); native_setup()
    scratch = Path(os.environ['PI_SCRATCH_DIR']).resolve()
    args.host = args.host.resolve(strict=True); args.worker = args.worker.resolve(strict=True)
    assert args.host.is_relative_to(scratch) and args.worker.is_relative_to(scratch), 'explicit frozen scratch binaries only'
    assert args.host.name == 'NxTools.PlatformHost.Tests.exe'
    args.evidence = args.evidence.resolve(); assert args.evidence.is_relative_to(scratch)
    args.evidence.mkdir(parents=True, exist_ok=False)
    if args.hz5:
        spec = importlib.util.spec_from_file_location('archive_hz5', Path(__file__).with_name('archive_hz5.py'))
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        return module.run(args, sys.modules[__name__])
    assert args.sevenzip is not None and args.bsdtar is not None, 'ordinary HZ4 requires both independent external readers'
    selected = [case for case in matrix() if args.cases is None or case['name'] in args.cases]
    assert selected and (args.cases is None or set(args.cases) == {case['name'] for case in selected}), 'exact nonempty case selection'
    save(args.evidence / 'identities.json', {'python': {'path': sys.executable, 'sha256': sha(sys.executable)},
        'host': {str(p): sha(p) for p in (args.host, args.host.with_suffix('.dll'), args.host.parent / 'NxTools.PlatformHost.dll')},
        'worker': {'path': str(args.worker), 'sha256': sha(args.worker)},
        'oracles': {str(p): sha(p) for p in (args.sevenzip, args.bsdtar)}, 'driver_sha256': sha(__file__), 'argv': sys.argv})
    results = []
    for case in selected:
        value = execute(case, args); results.append(value)
        print(case['name'], value['status'], value.get('failure', ''), flush=True)
    save(args.evidence / 'results.json', results)
    reader_counts = {key: sum(r['status'] == key for v in results for r in v['secondary_reader_results'])
                     for key in ('PASS', 'reader-unsupported', 'FAIL')}
    accounting = {'selected': len(results), 'pass': sum(v['status'] == 'PASS' for v in results),
                  'fail': sum(v['status'] == 'FAIL' for v in results), 'skip': 0,
                  'process_integrity_pass': sum(v['process_integrity_status'] == 'PASS' for v in results),
                  'secondary_reader': reader_counts}
    save(args.evidence / 'accounting.json', accounting)
    print(json.dumps(accounting))
    return int(any(v['status'] != 'PASS' for v in results))


if __name__ == '__main__':
    raise SystemExit(main())
