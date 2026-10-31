#!/usr/bin/env python3
"""Opt-in bounded lifecycle probe. Requires a matching scratch-only instrumented host build.
No Desktop, installed files, services, global console signal or recursive cleanup.
"""
import argparse
import ctypes
from ctypes import wintypes as w
import hashlib
import json
import datetime
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import uuid
import zipfile

k = ctypes.WinDLL('kernel32', use_last_error=True) if os.name == 'nt' else None
INVALID = ctypes.c_void_p(-1).value


def native_setup():
    signatures = {
        'CreateEventW': (w.HANDLE, [ctypes.c_void_p, w.BOOL, w.BOOL, w.LPCWSTR]),
        'SetEvent': (w.BOOL, [w.HANDLE]), 'CloseHandle': (w.BOOL, [w.HANDLE]),
        'OpenProcess': (w.HANDLE, [w.DWORD, w.BOOL, w.DWORD]),
        'WaitForSingleObject': (w.DWORD, [w.HANDLE, w.DWORD]),
        'CreateFileW': (w.HANDLE, [w.LPCWSTR, w.DWORD, w.DWORD, ctypes.c_void_p, w.DWORD, w.DWORD, w.HANDLE]),
        'CreateToolhelp32Snapshot': (w.HANDLE, [w.DWORD, w.DWORD]),
        'QueryFullProcessImageNameW': (w.BOOL, [w.HANDLE, w.DWORD, w.LPWSTR, ctypes.POINTER(w.DWORD)]),
        'GetConsoleProcessList': (w.DWORD, [ctypes.POINTER(w.DWORD), w.DWORD]),
        'GenerateConsoleCtrlEvent': (w.BOOL, [w.DWORD, w.DWORD]),
        'TerminateProcess': (w.BOOL, [w.HANDLE, w.UINT]),
    }
    for name, (result, arguments) in signatures.items():
        function = getattr(k, name); function.restype = result; function.argtypes = arguments


class ProcessEntry(ctypes.Structure):
    _fields_ = [('size', w.DWORD), ('usage', w.DWORD), ('pid', w.DWORD),
                ('heap', ctypes.c_size_t), ('module', w.DWORD), ('threads', w.DWORD),
                ('parent', w.DWORD), ('priority', w.LONG), ('flags', w.DWORD), ('exe', w.WCHAR * 260)]


def processes():
    snapshot = k.CreateToolhelp32Snapshot(2, 0)
    if snapshot == INVALID: raise ctypes.WinError(ctypes.get_last_error())
    try:
        entry = ProcessEntry(); entry.size = ctypes.sizeof(entry)
        result = []
        for name in ('Process32FirstW', 'Process32NextW'):
            getattr(k, name).argtypes = [w.HANDLE, ctypes.POINTER(ProcessEntry)]
            getattr(k, name).restype = w.BOOL
        more = k.Process32FirstW(snapshot, ctypes.byref(entry))
        while more:
            result.append({'pid': entry.pid, 'parent': entry.parent, 'name': entry.exe})
            more = k.Process32NextW(snapshot, ctypes.byref(entry))
        return result
    finally: k.CloseHandle(snapshot)


def image(pid):
    handle = k.OpenProcess(0x1000, False, pid)
    if not handle: return None
    try:
        buffer = ctypes.create_unicode_buffer(32768); length = w.DWORD(len(buffer))
        return buffer.value if k.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(length)) else None
    finally: k.CloseHandle(handle)


def exited(pid):
    handle = k.OpenProcess(0x100000, False, pid)
    if not handle:
        native_code = ctypes.get_last_error()
        if native_code == 87: return True  # nonexistent PID, not access-denied-as-success
        raise ctypes.WinError(native_code)
    try: return k.WaitForSingleObject(handle, 0) == 0
    finally: k.CloseHandle(handle)


def exclusive(path):
    handle = k.CreateFileW(str(path), 0xC0000000, 0, None, 3, 0x80, None)
    if handle == INVALID: return {'opened': False, 'native_error_code': ctypes.get_last_error()}
    k.CloseHandle(handle); return {'opened': True, 'native_error_code': None}


def checkpoints(root):
    path = root / 'checkpoints.jsonl'
    if not path.exists(): return []
    values = []
    for line in path.read_text(encoding='utf-8-sig').splitlines():
        try: values.append(json.loads(line))
        except json.JSONDecodeError: pass  # a currently-being-written last line is not evidence yet
    return values


def wait_until(predicate, description, limit=6):
    until = time.monotonic() + limit
    while time.monotonic() < until:
        value = predicate()
        if value: return value
        time.sleep(.005)  # observation only; checkpoint, not elapsed time, determines action
    raise AssertionError('bounded wait: ' + description)


def driver(case_root):
    root = Path(case_root).resolve(strict=True)
    settings = json.loads((root / 'driver-settings.json').read_text(encoding='utf-8'))
    mode = settings['mode']; cli = Path(settings['cli']); source = root / 'tiny.zip'; output = root / 'output'
    native_setup()
    helper_before = hashlib.sha256((cli.parent / 'NxTools.ArchiveWorker.exe').read_bytes()).hexdigest()
    if mode in ('signal', 'partial_signal'):
        handler_type = ctypes.WINFUNCTYPE(w.BOOL, w.DWORD)
        handler = handler_type(lambda event: True)
        k.SetConsoleCtrlHandler.argtypes = [handler_type, w.BOOL]
        k.SetConsoleCtrlHandler.restype = w.BOOL
        assert k.SetConsoleCtrlHandler(handler, True), 'owned driver console handler'
    handles = {name: k.CreateEventW(None, True, False, settings['prefix'] + name)
               for name in ('release_work', 'release_cleanup', 'close_pipe')}
    assert all(handles.values()), 'owned named checkpoint events'
    result = {'mode': mode, 'driver_pid': os.getpid(), 'physical_keyboard': 'NOT RUN', 'fixture': 'scratch-only host checkpoint instrumentation'}
    cli_process = None; owned = []
    stdout = open(root / 'cli.stdout.json', 'wb'); stderr = open(root / 'cli.stderr.txt', 'wb')
    try:
        cli_process = subprocess.Popen([str(cli), 'archive', 'extract', str(source), '--dest', str(output), '--json'], stdout=stdout, stderr=stderr)
        result['cli_pid'] = cli_process.pid
        if mode != 'diagnostics':
            reached = wait_until(lambda: next((x for x in checkpoints(root) if x['stage'] == 'work_checkpoint'), None), 'actual original output checkpoint')
            host_pid = reached['process_id']
            host_path = image(host_pid)
            assert host_path and Path(host_path).parent == cli.parent, 'only owned host image'
            table = processes()
            assert any(p['pid'] == host_pid and p['parent'] == cli_process.pid for p in table), 'actual CLI owns host'
            worker_rows = [p for p in table if p['parent'] == host_pid and p['name'].lower() == 'nxtools.archiveworker.exe']
            assert len(worker_rows) == 1, ('one original worker', worker_rows)
            assert Path(image(worker_rows[0]['pid'])).parent == cli.parent, 'owned worker image'
            owned = [host_pid, worker_rows[0]['pid']]
            result['owned_processes'] = [{'pid': host_pid, 'role': 'host'}, {'pid': worker_rows[0]['pid'], 'role': 'worker'}]
            result['source_before_cleanup'] = exclusive(source)
            result['helper_before_cleanup'] = exclusive(cli.parent / 'NxTools.ArchiveWorker.exe')
            assert not result['source_before_cleanup']['opened'] and not result['helper_before_cleanup']['opened'], 'source/helper held'
            if mode in ('signal', 'partial_signal'):
                console_pids = (w.DWORD * 16)(); count = k.GetConsoleProcessList(console_pids, 16)
                actual = set(console_pids[:count]); expected = {os.getpid(), cli_process.pid}
                result['console_processes_before_signal'] = sorted(actual)
                assert count == 2 and actual == expected, ('REFUSE non-isolated console', actual, expected)
                assert k.GenerateConsoleCtrlEvent(0, 0), 'CTRL_C only to independently created owned console'
                result['native_ctrl_c_sent_once'] = True
                wait_until(lambda: any(x['stage'] == 'cancel_written' for x in checkpoints(root)), 'original worker cancel frame written')
                assert cli_process.poll() is None, 'cancel write/ACK is not terminal'
                result['cli_waited_after_cancel_write'] = True
                k.SetEvent(handles['release_work'])
                wait_until(lambda: any(x['stage'] == 'cleanup_entered' for x in checkpoints(root)), 'actual original cleanup checkpoint')
                assert cli_process.poll() is None, 'no early JSON while cleanup is held'
                result['cli_waited_at_cleanup'] = True
                assert not (root / 'cli.stdout.json').read_bytes(), 'no premature result'
                result['source_during_cleanup'] = exclusive(source)
                result['helper_during_cleanup'] = exclusive(cli.parent / 'NxTools.ArchiveWorker.exe')
                assert not result['source_during_cleanup']['opened'] and not result['helper_during_cleanup']['opened'], 'lease retained until actual cleanup'
                k.SetEvent(handles['release_cleanup'])
            elif mode == 'parent_exit':
                cli_process.terminate()  # only Popen-owned exact CLI; no process-name targeting
                cli_process.wait(timeout=3)
                result['no_parent_json_possible'] = True
                wait_until(lambda: any(x['stage'] == 'cancel_written' for x in checkpoints(root)), 'parent-exit Stop reaches original cancel')
                k.SetEvent(handles['release_work'])
            elif mode == 'eof':
                k.SetEvent(handles['close_pipe'])
                wait_until(lambda: any(x['stage'] == 'pipe_closed_by_fixture' for x in checkpoints(root)), 'private connection closed')
                wait_until(lambda: any(x['stage'] == 'cancel_written' for x in checkpoints(root)), 'connection retirement reaches original cancel')
                k.SetEvent(handles['release_work'])
        cli_process.wait(timeout=20)
        stdout.flush(); stderr.flush()
        result['cli_exit'] = cli_process.returncode
        wait_until(lambda: all(exited(pid) for pid in owned), 'all exact owned host/worker exits', 12)
        result['owned_children_exited'] = True
        result['source_after_cleanup'] = exclusive(source)
        result['helper_after_cleanup'] = exclusive(cli.parent / 'NxTools.ArchiveWorker.exe')
        assert result['source_after_cleanup']['opened'] and result['helper_after_cleanup']['opened'], 'actual source/helper release'
        result['partials'] = [p.name for p in output.glob('.nxpart-*')]
        assert result['partials'] == [], 'original owned partial cleanup'
        raw = (root / 'cli.stdout.json').read_text(encoding='utf-8')
        if mode == 'parent_exit':
            assert not raw, 'dead parent cannot return a result'
            assert (output / 'first.txt').read_bytes() == b'first\n' and not (output / 'second.txt').exists(), 'parent death preserves committed output'
        else:
            value = json.loads(raw); result['json'] = value
            assert not (root / 'cli.stderr.txt').read_bytes(), 'one JSON, empty stderr'
            if mode == 'diagnostics': assert cli_process.returncode == 0 and value['success']
            else:
                assert cli_process.returncode != 0 and not value['success']
                if mode in ('signal', 'partial_signal'):
                    terminal = value['result']['terminal']
                    assert terminal['cleanup_complete']
                    if mode == 'signal':
                        assert terminal['code'] == 'cancelled' and terminal['files_committed'] == 0
                    else:
                        assert terminal['code'] == 'cancelled_with_committed_output', terminal
                        assert terminal['files_committed'] == 1 and terminal['bytes_committed'] == 6
                        assert terminal['commit_state'] == 'Committed' and len(terminal['journal']) == 1
                        assert (output / 'first.txt').read_bytes() == b'first\n' and not (output / 'second.txt').exists()
        if mode == 'diagnostics':
            configured = next(x for x in checkpoints(root) if x['stage'] == 'configured')
            host_pid = configured['process_id']
            control = json.loads((cli.parent / 'lifecycle-control.json').read_text(encoding='utf-8'))
            log_root = root / 'logs' if control['isolated_sink'] else Path(os.environ['LOCALAPPDATA']) / 'NxTools/NxTools.Next/data/logs'
            captured = []
            parse_failures = 0
            started = datetime.datetime.fromisoformat(settings['started_at'])
            dates = {started.date().isoformat(), datetime.datetime.now().astimezone().date().isoformat()}
            for date in dates:
                log_path = log_root / ('platform-host.log.' + date + '.jsonl')
                if not log_path.is_file(): continue
                with log_path.open(encoding='utf-8-sig') as log_stream:
                    for line in log_stream:
                        if '"platform_host_pid":' + str(host_pid) not in line: continue
                        try: record = json.loads(line)
                        except json.JSONDecodeError:
                            parse_failures += 1
                            continue
                        if record.get('platform_host_pid') == host_pid: captured.append(record)
            result['captured_diagnostics'] = captured
            result['matching_diagnostic_parse_failures'] = parse_failures
            lifecycle = [x for x in captured if x.get('event', '').startswith('archive.cli_host.')]
            assert [x['event'] for x in lifecycle] == ['archive.cli_host.start', 'archive.cli_host.connected', 'archive.cli_host.stopped'], 'own host diagnostics must survive selected sink'
            assert len({x['correlation_id'] for x in lifecycle}) == 1
            assert all('elapsed_ms' in x and 'native_error_code' in x for x in lifecycle)
            assert any(x.get('event') == 'archive.job.terminal' and x.get('cleanup_complete') is True for x in captured)
            assert not any(key in x for x in captured for key in ('path', 'file_path', 'password', 'payload', 'content')), 'no diagnostic payload secrets'
        result['source_sha256'] = hashlib.sha256(source.read_bytes()).hexdigest()
        result['helper_sha256_before'] = helper_before
        result['helper_sha256_after'] = hashlib.sha256((cli.parent / 'NxTools.ArchiveWorker.exe').read_bytes()).hexdigest()
        assert result['helper_sha256_after'] == helper_before
        assert result['source_sha256'] == settings['source_sha256']
        result['status'] = 'passed'
    except Exception as failure:
        result['status'] = 'failed'; result['failure'] = str(failure)
        raise
    finally:
        for handle in handles.values(): k.SetEvent(handle)
        if cli_process and cli_process.poll() is None:
            cli_process.terminate(); cli_process.wait(timeout=3)
        for pid in owned:
            if not exited(pid):
                owned_image = image(pid)
                if owned_image and Path(owned_image).parent == cli.parent:
                    handle = k.OpenProcess(1, False, pid)
                    if handle: k.TerminateProcess(handle, 1); k.CloseHandle(handle)
        for handle in handles.values(): k.CloseHandle(handle)
        stdout.close(); stderr.close()
        result['checkpoints'] = checkpoints(root)
        (root / 'result.json').write_text(json.dumps(result, indent=2), encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin', type=Path)
    parser.add_argument('--evidence-root', type=Path)
    parser.add_argument('--case', choices=('signal', 'partial_signal', 'parent_exit', 'eof', 'diagnostics'))
    parser.add_argument('--default-sink', action='store_true')
    parser.add_argument('--driver', type=Path)
    options = parser.parse_args()
    if os.name != 'nt': parser.error('Windows isolated console required; NOT RUN elsewhere')
    if options.driver: return driver(options.driver)
    if not all((options.bin, options.evidence_root, options.case)): parser.error('--bin, --evidence-root, --case required')
    root = options.evidence_root.resolve(strict=True) / (options.case + '-' + str(uuid.uuid4()))
    root.mkdir(); bin_dir = root / 'bin'; shutil.copytree(options.bin.resolve(strict=True), bin_dir)
    source = root / 'tiny.zip'
    with zipfile.ZipFile(source, 'w') as output:
        output.writestr('first.txt', b'first\n'); output.writestr('second.txt', b'second\n')
    (root / 'output').mkdir()
    prefix = 'Local\\NxTools.ArchiveCliLifecycle.' + str(uuid.uuid4()) + '.'
    (bin_dir / 'lifecycle-control.json').write_text(json.dumps({'root': str(root), 'mode': options.case, 'prefix': prefix, 'isolated_sink': not options.default_sink}), encoding='utf-8')
    (root / 'driver-settings.json').write_text(json.dumps({'mode': options.case, 'cli': str(bin_dir/'NxTools.Cli.exe'), 'prefix': prefix, 'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'started_at': datetime.datetime.now().astimezone().isoformat()}), encoding='utf-8')
    with open(root/'driver.stdout.log','wb') as output, open(root/'driver.stderr.log','wb') as errors:
        process = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), '--driver', str(root)], stdout=output, stderr=errors, creationflags=subprocess.CREATE_NEW_CONSOLE)
        try: process.wait(timeout=45)
        except subprocess.TimeoutExpired:
            process.terminate(); process.wait(timeout=3)
            raise AssertionError('isolated driver exceeded bound: ' + str(root))
    print('Evidence: '+str(root))
    if (root/'result.json').exists():
        value=json.loads((root/'result.json').read_text(encoding='utf-8'))
        print(json.dumps(value, indent=2))
    assert process.returncode == 0, (process.returncode, str(root/'driver.stderr.log'))


if __name__ == '__main__': main()
