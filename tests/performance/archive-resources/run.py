"""Bounded Windows Debug resource probe driver. Does not invoke Cargo or launch product apps."""
import argparse
import ctypes as c
from ctypes import wintypes as w
import json
import os
import pathlib
import shutil
import struct
import subprocess
import threading
import time
import uuid
from control import StopController, read_status

SCHEMA = 2

K = c.WinDLL('kernel32', use_last_error=True)
P = c.WinDLL('psapi', use_last_error=True)
K.OpenProcess.argtypes = (w.DWORD, w.BOOL, w.DWORD)
K.OpenProcess.restype = w.HANDLE
K.CloseHandle.argtypes = (w.HANDLE,)
K.WaitForSingleObject.argtypes = (w.HANDLE, w.DWORD)
K.QueryFullProcessImageNameW.argtypes = (w.HANDLE, w.DWORD, w.LPWSTR, c.POINTER(w.DWORD))


class Memory(c.Structure):
    _fields_ = [('cb', w.DWORD), ('PageFaultCount', w.DWORD)] + [(name, c.c_size_t) for name in
                ('PeakWorkingSetSize', 'WorkingSetSize', 'QuotaPeakPagedPoolUsage', 'QuotaPagedPoolUsage',
                 'QuotaPeakNonPagedPoolUsage', 'QuotaNonPagedPoolUsage', 'PagefileUsage', 'PeakPagefileUsage', 'PrivateUsage')]


class Global(c.Structure):
    _fields_ = [('length', w.DWORD), ('load', w.DWORD)] + [(name, c.c_ulonglong) for name in
                ('total_phys', 'avail_phys', 'total_page', 'avail_page', 'total_virtual', 'avail_virtual', 'avail_extended')]


class Entry(c.Structure):
    _fields_ = [('dwSize', w.DWORD), ('cntUsage', w.DWORD), ('th32ProcessID', w.DWORD),
                ('th32DefaultHeapID', c.c_size_t), ('th32ModuleID', w.DWORD), ('cntThreads', w.DWORD),
                ('th32ParentProcessID', w.DWORD), ('pcPriClassBase', w.LONG), ('dwFlags', w.DWORD), ('szExeFile', w.WCHAR * 260)]


K.CreateToolhelp32Snapshot.argtypes = (w.DWORD, w.DWORD)
K.CreateToolhelp32Snapshot.restype = w.HANDLE
K.Process32FirstW.argtypes = (w.HANDLE, c.POINTER(Entry))
K.Process32NextW.argtypes = (w.HANDLE, c.POINTER(Entry))
P.GetProcessMemoryInfo.argtypes = (w.HANDLE, c.POINTER(Memory), w.DWORD)
K.TerminateProcess.argtypes = (w.HANDLE, w.UINT)
K.GetProcessTimes.argtypes = (w.HANDLE, c.POINTER(w.FILETIME), c.POINTER(w.FILETIME), c.POINTER(w.FILETIME), c.POINTER(w.FILETIME))


def system_memory():
    value = Global(); value.length = c.sizeof(value)
    assert K.GlobalMemoryStatusEx(c.byref(value))
    return dict(physical=value.total_phys, available=value.avail_phys,
                commit_limit=value.total_page, commit_headroom=value.avail_page)


def processes():
    snapshot = K.CreateToolhelp32Snapshot(2, 0)
    if snapshot == c.c_void_p(-1).value:
        raise c.WinError(c.get_last_error())
    try:
        entry = Entry(); entry.dwSize = c.sizeof(entry)
        more = K.Process32FirstW(snapshot, c.byref(entry))
        if not more and c.get_last_error() != 18:
            raise c.WinError(c.get_last_error())
        rows = []
        while more:
            rows.append((entry.th32ProcessID, entry.th32ParentProcessID, entry.szExeFile))
            more = K.Process32NextW(snapshot, c.byref(entry))
            if not more and c.get_last_error() != 18:
                raise c.WinError(c.get_last_error())
        return rows
    finally:
        K.CloseHandle(snapshot)


def image(handle):
    buffer = c.create_unicode_buffer(32768); length = w.DWORD(len(buffer))
    return buffer.value if K.QueryFullProcessImageNameW(handle, 0, buffer, c.byref(length)) else None


def image_key(path):
    value = str(path)
    if value.startswith('\\\\?\\'):
        value = value[4:]
    return os.path.normpath(value).casefold()


class NativeCounters:
    def open(self, process_id):
        handle = K.OpenProcess(0x1000 | 0x10 | 0x100000, False, process_id)
        if not handle:
            raise c.WinError(c.get_last_error())
        return handle

    def image(self, handle):
        value = image(handle)
        if value is None:
            raise c.WinError(c.get_last_error())
        return value

    def created(self, handle):
        values = [w.FILETIME() for _ in range(4)]
        if not K.GetProcessTimes(handle, *(c.byref(value) for value in values)):
            raise c.WinError(c.get_last_error())
        return values[0].dwLowDateTime | values[0].dwHighDateTime << 32

    def exited(self, handle):
        result = K.WaitForSingleObject(handle, 0)
        if result == 0xFFFFFFFF:
            raise c.WinError(c.get_last_error())
        return result == 0

    def take(self, handle):
        memory = Memory(); memory.cb = c.sizeof(memory)
        if not P.GetProcessMemoryInfo(handle, c.byref(memory), memory.cb):
            raise c.WinError(c.get_last_error())
        return dict(working_set=memory.WorkingSetSize, private_bytes=memory.PrivateUsage,
                    process_lifetime_peak_working_set=memory.PeakWorkingSetSize,
                    process_lifetime_peak_commit=memory.PeakPagefileUsage)

    def close(self, handle):
        K.CloseHandle(handle)

    def terminate(self, row):
        # Reopen with termination rights only for this verified, still-retained instance.
        handle = K.OpenProcess(0x1 | 0x1000 | 0x100000, False, row['pid'])
        if not handle:
            raise c.WinError(c.get_last_error())
        try:
            if self.created(handle) != row['created'] or image_key(self.image(handle)) != image_key(row['executable']):
                raise OSError('forced_target_identity_changed')
            if not self.exited(handle) and not K.TerminateProcess(handle, 1):
                raise c.WinError(c.get_last_error())
        finally:
            self.close(handle)


def required_roles(scenario):
    roles = ['probe_owner']
    if scenario != 'engine100k':
        if scenario.startswith('product') or scenario == 'tool-guard-failure':
            roles.append('isolated_csharp_host')
        if scenario != 'tool-guard-failure':
            roles.append('actual_worker')
    return roles


def measurement_result(rows, required):
    reasons = []
    coverage = {}
    for role in required:
        matching = [row for row in rows if role in row.get('roles', [row.get('role')])]
        valid = [row for row in matching if row.get('identity_verified') and row.get('samples', 0) > 0]
        coverage[role] = dict(observed=len(matching), valid=len(valid))
        if not matching:
            reasons.append(role + ':Unavailable')
        for row in matching:
            if not row.get('identity_verified'):
                reasons.append(f"{role}:{row['pid']}:identity_unverified")
            elif not row.get('samples', 0):
                reasons.append(f"{role}:{row['pid']}:zero_valid_samples")
    return dict(status='Incomplete' if reasons else 'Complete', required_roles=required,
                coverage=coverage, reasons=reasons,
                requested_interval_ms=20, discovery_interval_ms=50, peaks='sampled_not_absolute')


class Sampler:
    def __init__(self, child, executable, output, controller=None, scenario='engine100k', native=None,
                 snapshot=None, guard=None, clock=None, deadline=180, expected_images=None):
        self.child = child; self.expected = str(executable.resolve()).casefold()
        self.clock = clock or time.perf_counter; self.start = self.clock()
        self.stop = threading.Event(); self.ready = threading.Event(); self.rows = {}; self.output = output
        self.controller = controller; self.scenario = scenario; self.native = native or NativeCounters()
        self.snapshot = snapshot or processes; self.guard = guard or self.resource_guard
        self.deadline = deadline; self.expected_images = expected_images or {}
        self.failure = None; self.errors = []; self.lock = threading.RLock()
        self.thread = threading.Thread(target=self.run, name='archive-resource-sampler')

    def resource_guard(self):
        state = system_memory()
        if state['available'] < 2 << 30:
            return 'physical_threshold'
        if state['commit_headroom'] < 3 << 30:
            return 'commit_threshold'
        if shutil.disk_usage(self.output.parent).free < 2 << 30:
            return 'disk_threshold'
        return None

    def error(self, row, stage, failure):
        key = stage + '_failures'
        row[key] = row.get(key, 0) + 1
        if len(row['errors']) < 16:
            row['errors'].append(dict(stage=stage, type=type(failure).__name__,
                                      native_code=getattr(failure, 'winerror', None)))

    def add(self, process_id, parent, role, expected=None, operation_id=None):
        with self.lock:
            row = self.rows.get(process_id)
            if row is None:
                if len(self.rows) >= 32:
                    raise RuntimeError('process_record_limit')
                roles = [role]
                if process_id == self.child.pid and self.scenario == 'direct-worker100k':
                    roles.append('actual_worker')
                row = dict(pid=process_id, parent_pid=parent, role=role, roles=roles,
                           expected_executable=expected or self.expected, executable=None, operation_id=operation_id,
                           identity_verified=False, baseline=None, peak_working_set=None, peak_private_bytes=None,
                           samples=0, acquisition_attempts=0, acquisition_successes=0, acquisition_failures=0,
                           image_attempts=0, image_successes=0, image_failures=0, counter_attempts=0,
                           counter_failures=0, max_interval_ms=None, last_ms=None, exit_observed_ms=None, errors=[])
                self.rows[process_id] = row
            if row.get('handle') is not None or row['acquisition_attempts'] >= 32:
                return
            row['acquisition_attempts'] += 1
            try:
                handle = self.native.open(process_id)
                row['acquisition_successes'] += 1
            except OSError as failure:
                self.error(row, 'acquisition', failure)
                return
            try:
                row['image_attempts'] += 1
                path = self.native.image(handle)
                row['image_successes'] += 1
                if image_key(path) != image_key(row['expected_executable']):
                    raise OSError('image_mismatch')
                row['created'] = self.native.created(handle)
                row['handle'] = handle; row['executable'] = path; row['identity_verified'] = True
                row['provenance'] = 'owned_spawn' if process_id == self.child.pid else 'exact_image_and_original_parent'
            except OSError as failure:
                self.error(row, 'image', failure)
                self.native.close(handle)

    def discover(self):
        status = {}
        if self.controller is not None:
            status = self.controller.read_status('sampler')
            self.expected_images.update(status.get('expected_images', {}))
        self.add(self.child.pid, os.getpid(), 'probe_owner')
        snapshot = self.snapshot()
        parents = {process_id: parent for process_id, parent, _ in snapshot}
        for item in status.get('identities', []):
            process_id = item['pid']; parent = item.get('parent_pid')
            role = item['role']
            if process_id == self.child.pid or role not in ('actual_worker', 'isolated_csharp_host'):
                continue
            expected = self.expected_images.get(role)
            if expected and image_key(expected) == image_key(item.get('executable', '')) and parent in self.rows:
                # A captured identity that exited too early remains a zero-sample record.
                if process_id not in parents or parents[process_id] == parent:
                    self.add(process_id, parent, role, expected, item.get('operation_id'))
        for process_id, parent, name in snapshot:
            parent_row = self.rows.get(parent)
            if not parent_row or not parent_row.get('identity_verified') or process_id == self.child.pid:
                continue
            for role, expected in self.expected_images.items():
                if role not in ('actual_worker', 'isolated_csharp_host'):
                    continue
                correct_parent = parent == self.child.pid if role == 'isolated_csharp_host' else (
                    parent == self.child.pid if 'isolated_csharp_host' not in required_roles(self.scenario)
                    else 'isolated_csharp_host' in parent_row['roles'])
                if correct_parent and name.casefold() == pathlib.Path(expected).name.casefold():
                    self.add(process_id, parent, role, expected, status.get('operation_id'))

    def take(self, row, now, stream):
        handle = row.get('handle')
        if handle is None or row['exit_observed_ms'] is not None:
            return
        if self.native.exited(handle):
            row['exit_observed_ms'] = now * 1000
            return
        row['counter_attempts'] += 1
        try:
            point = dict(ms=round(now * 1000, 3), pid=row['pid'], **self.native.take(handle))
        except OSError as failure:
            self.error(row, 'counter', failure)
            if row['counter_failures'] >= 32 and row['samples'] == 0:
                raise RuntimeError('required_counter_unavailable')
            return
        row['baseline'] = row['baseline'] or point
        row['peak_working_set'] = max(row['peak_working_set'] or 0, point['working_set'])
        row['peak_private_bytes'] = max(row['peak_private_bytes'] or 0, point['private_bytes'])
        if row['last_ms'] is not None:
            row['max_interval_ms'] = max(row['max_interval_ms'] or 0, point['ms'] - row['last_ms'])
        row['last_ms'] = point['ms']; row['samples'] += 1; row['last'] = point
        if row['samples'] > 10000:
            raise RuntimeError('sample_limit')
        stream.write(json.dumps(point) + '\n')
        if row['pid'] == self.child.pid:
            self.ready.set()

    def run(self):
        try:
            self.add(self.child.pid, os.getpid(), 'probe_owner')
            next_discovery = next_guard = 0
            with self.output.open('x', encoding='utf-8') as stream:
                while not self.stop.is_set():
                    now = self.clock() - self.start
                    if now > self.deadline:
                        raise RuntimeError('sampler_deadline')
                    if now >= next_discovery:
                        self.discover(); next_discovery = now + .05
                    if now >= next_guard:
                        reason = self.guard()
                        if reason:
                            raise RuntimeError(reason)
                        next_guard = now + 1
                    with self.lock:
                        for row in self.rows.values():
                            self.take(row, now, stream)
                    self.stop.wait(.02)
        except BaseException as failure:
            self.failure = str(failure) if isinstance(failure, RuntimeError) else type(failure).__name__
            if self.controller is not None:
                self.controller.request(self.failure)

    def live_targets(self):
        with self.lock:
            targets = []
            for row in self.rows.values():
                if row['pid'] == self.child.pid or not row.get('identity_verified') or not row.get('handle'):
                    continue
                try:
                    exited = self.native.exited(row['handle'])
                except OSError as failure:
                    self.error(row, 'exit', failure)
                    exited = False  # retained verified instance still needs bounded owned cleanup
                if not exited:
                    targets.append(dict(pid=row['pid'], role=row['role'], executable=row['executable'],
                                        terminate=lambda row=row: self.native.terminate(row)))
            return list(reversed(targets))

    def finish(self):
        self.stop.set()
        if self.thread.ident is not None:
            self.thread.join(10)
            if self.thread.is_alive():
                self.failure = self.failure or 'sampler_join_incomplete'
                return []  # never race the live sampler closing its handles
        result = []
        with self.lock:
            for original in self.rows.values():
                row = original.copy(); handle = row.pop('handle', None)
                try:
                    row['alive_at_end'] = not self.native.exited(handle) if handle else None
                except OSError as failure:
                    row['alive_at_end'] = None
                    self.error(row, 'exit', failure)
                    self.failure = self.failure or 'exit_observation_unavailable'
                finally:
                    if handle:
                        self.native.close(handle)
                result.append(row)
        return result


def send(stream, value):
    data = json.dumps(value, separators=(',', ':')).encode()
    assert len(data) <= 1 << 20
    stream.write(struct.pack('<I', len(data)) + data); stream.flush()


def run(args):
    from driver_loop import execute
    return execute(args, globals())


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=pathlib.Path)
    parser.add_argument('scenario', choices=['engine100k', 'direct-worker100k', 'runner100k', 'cancel1gib', 'cancel1gib-hold', 'product100k', 'product100k-v2', 'product-credit1gib', 'product-pressure1gib', 'product-credit-small', 'product-pressure-small', 'tool-guard-failure', 'tool-stop-small'])
    parser.add_argument('attempt', choices=['correctness', 'repeat', 'tool-repair'])
    run(parser.parse_args())
