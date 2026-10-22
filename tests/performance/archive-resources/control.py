"""Test-tool stop coordination only; not a product process/Job runtime."""
import ctypes as c
from ctypes import wintypes as w
import json
import os
import pathlib
import threading
import time

LIMIT = 16384
STATUS_ATTEMPTS = 4
STATUS_RETRY_SECONDS = .01


if os.name == 'nt':
    _STATUS_KERNEL = c.WinDLL('kernel32', use_last_error=True)
    _STATUS_KERNEL.CreateFileW.argtypes = (w.LPCWSTR, w.DWORD, w.DWORD, c.c_void_p, w.DWORD, w.DWORD, w.HANDLE)
    _STATUS_KERNEL.CreateFileW.restype = w.HANDLE
    _STATUS_KERNEL.ReadFile.argtypes = (w.HANDLE, c.c_void_p, w.DWORD, c.POINTER(w.DWORD), c.c_void_p)
    _STATUS_KERNEL.ReadFile.restype = w.BOOL
    _STATUS_KERNEL.CloseHandle.argtypes = (w.HANDLE,)
    _STATUS_KERNEL.CloseHandle.restype = w.BOOL


def _status_native_error(stage):
    failure = c.WinError(c.get_last_error())
    failure.status_stage = stage
    return failure


def _read_status_bytes(path):
    if os.name != 'nt':
        with path.open('rb') as source:
            return source.read(LIMIT + 1)
    # CRT open can lose native codes. Grant all sharing; always close before returning.
    handle = _STATUS_KERNEL.CreateFileW(str(path), 0x80000000, 7, None, 3, 0x80, None)
    if handle == c.c_void_p(-1).value:
        raise _status_native_error('open')
    primary_failure = None
    try:
        data = bytearray()
        while len(data) <= LIMIT:
            buffer = c.create_string_buffer(LIMIT + 1 - len(data))
            count = w.DWORD()
            if not _STATUS_KERNEL.ReadFile(handle, buffer, len(buffer), c.byref(count), None):
                raise _status_native_error('read')
            if not count.value:
                break
            data.extend(buffer.raw[:count.value])
        return bytes(data)
    except OSError as failure:
        primary_failure = failure
        raise
    finally:
        if not _STATUS_KERNEL.CloseHandle(handle) and primary_failure is None:
            raise _status_native_error('close')


class StatusReadDiagnostics:
    """Per-run bounded counters, never a status cache or a per-poll log sink."""
    def __init__(self):
        self.lock = threading.Lock()
        self.callers = {}
        self.errors = []
        self.error_count = 0

    def count(self, caller, key):
        with self.lock:
            row = self.callers.setdefault(caller, dict(reads=0, attempts=0, retries=0, recoveries=0,
                                                      exhausted=0, failures=0, missing=0))
            row[key] += 1

    def error(self, caller, failure, category, attempt):
        with self.lock:
            self.error_count += 1
            if len(self.errors) < 16:
                self.errors.append(dict(caller=caller, stage=getattr(failure, 'status_stage', 'validate'),
                                        category=category, type=type(failure).__name__,
                                        errno=getattr(failure, 'errno', None), native_code=getattr(failure, 'winerror', None),
                                        attempt=attempt))

    def result(self):
        with self.lock:
            return dict(callers={key: value.copy() for key, value in self.callers.items()},
                        errors=[row.copy() for row in self.errors], error_count=self.error_count,
                        errors_truncated=self.error_count > len(self.errors))


def read_status(directory, run_id, diagnostics=None, caller='reader'):
    if caller not in ('reader', 'driver', 'sampler', 'final'):
        raise ValueError('control_status_caller')
    diagnostics = diagnostics if diagnostics is not None else StatusReadDiagnostics()
    diagnostics.count(caller, 'reads')
    path = pathlib.Path(directory) / 'status.json'
    for attempt in range(1, STATUS_ATTEMPTS + 1):
        diagnostics.count(caller, 'attempts')
        try:
            data = _read_status_bytes(path)
        except OSError as failure:
            native = getattr(failure, 'winerror', None)
            if native == 2 and getattr(failure, 'status_stage', None) == 'open':
                diagnostics.count(caller, 'missing')
                return {}  # Unavailable, never an earlier status/terminal/cleanup.
            transient = native in (32, 33) and getattr(failure, 'status_stage', None) in ('open', 'read')
            category = 'sharing_conflict' if native == 32 else 'lock_conflict' if native == 33 else 'io_error'
            diagnostics.error(caller, failure, category, attempt)
            if not transient or attempt == STATUS_ATTEMPTS:
                diagnostics.count(caller, 'failures')
                if transient:
                    diagnostics.count(caller, 'exhausted')
                raise
            diagnostics.count(caller, 'retries')
            time.sleep(STATUS_RETRY_SECONDS)
            continue
        try:
            if len(data) > LIMIT:
                raise ValueError('control_status_limit')
            value = json.loads(data)
            if not isinstance(value, dict) or value.get('run_id') != run_id or value.get('schema') != 2 or type(value.get('schema')) is not int:
                raise ValueError('control_status_identity')
            if not isinstance(value.get('identities', []), list) or len(value.get('identities', [])) > 32:
                raise ValueError('control_identity_limit')
        except (ValueError, TypeError, RecursionError) as failure:
            diagnostics.count(caller, 'failures')
            diagnostics.error(caller, failure, 'invalid_status', attempt)
            raise
        if attempt > 1:
            diagnostics.count(caller, 'recoveries')
        return value


class StopController:
    def __init__(self, child, directory, run_id, clock=time.monotonic):
        self.child = child
        self.directory = pathlib.Path(directory)
        self.run_id = run_id
        self.clock = clock
        self.lock = threading.Lock()
        self.requested = threading.Event()
        self.reason = None
        self.at = None
        self.errors = []
        self.targets = []
        self.forced = False
        self.closed = False
        self.direct_cancel = None
        self.status_reads = StatusReadDiagnostics()

    def read_status(self, caller):
        return read_status(self.directory, self.run_id, self.status_reads, caller)

    def request(self, reason):
        with self.lock:
            if self.requested.is_set() or self.closed:
                return False
            self.reason = reason
            self.at = self.clock()
            self.requested.set()
            try:
                temporary = self.directory / 'stop.tmp'
                temporary.write_text(json.dumps({'run_id': self.run_id, 'reason': reason}), encoding='utf-8')
                temporary.replace(self.directory / 'stop.json')
                if self.direct_cancel is not None:
                    self.direct_cancel()
            except (OSError, ValueError) as failure:
                self.errors.append({'stage': 'stop_delivery', 'type': type(failure).__name__,
                                    'native_code': getattr(failure, 'winerror', None)})
            return True

    def expired(self, grace=10):
        return self.requested.is_set() and self.clock() - self.at >= grace

    def force(self, descendants=()):
        """Only main driver calls once, after cooperation expires. Retained handles prevent PID reuse."""
        with self.lock:
            if self.forced:
                return
            self.forced = True
            for target in descendants:
                try:
                    target['terminate']()
                    self.targets.append({key: target[key] for key in ('pid', 'executable', 'role')})
                except OSError as failure:
                    self.errors.append({'stage': 'force_descendant', 'pid': target['pid'],
                                        'native_code': getattr(failure, 'winerror', None)})
            if self.child.poll() is None:
                try:
                    self.child.terminate()  # exact Popen handle, NEVER process-name lookup
                    self.targets.append({'pid': self.child.pid, 'role': 'probe_owner', 'identity': 'owned_spawn_handle'})
                except OSError as failure:
                    self.errors.append({'stage': 'force_owner', 'native_code': getattr(failure, 'winerror', None)})

    def result(self):
        return {'requested': self.requested.is_set(), 'reason': self.reason, 'forced': self.forced,
                'targets': self.targets, 'errors': self.errors[:16],
                'state': 'Forced' if self.forced else 'StopRequested' if self.requested.is_set() else 'NotRequested'}

    def close(self):
        with self.lock:
            self.closed = True
