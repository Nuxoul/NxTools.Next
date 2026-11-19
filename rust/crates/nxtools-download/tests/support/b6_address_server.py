"""Owned multi-address HTTP listeners and read-only exact-child TCP observation."""
import ctypes as c
from ctypes import wintypes as w
import json
import os
import socket
import sys
import threading
import time

SIZE = 1024 * 1024
PAYLOAD = bytes(index % 251 for index in range(SIZE))


class Row4(c.Structure):
    _fields_ = [(name, w.DWORD) for name in ('state', 'local_addr', 'local_port', 'remote_addr', 'remote_port', 'pid')]


class Row6(c.Structure):
    _fields_ = [('local_addr', c.c_ubyte * 16), ('local_scope', w.DWORD), ('local_port', w.DWORD),
                ('remote_addr', c.c_ubyte * 16), ('remote_scope', w.DWORD), ('remote_port', w.DWORD),
                ('state', w.DWORD), ('pid', w.DWORD)]


api = c.WinDLL('iphlpapi', use_last_error=True)
api.GetExtendedTcpTable.argtypes = [c.c_void_p, c.POINTER(w.DWORD), w.BOOL, w.ULONG, w.DWORD, w.ULONG]
api.GetExtendedTcpTable.restype = w.DWORD


class Server:
    def __init__(self, scenario):
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.listeners = []
        self.fillers = []
        self.workers = []
        self.streams = set()
        self.pending = set()
        self.slow_addresses = set()
        self.current_pending = 0
        self.failure = None
        self.counts = dict(accepted=0, requests=0, served_bytes=0, active=0, maximum=0, proxy_requests=0)
        self.pid = os.getppid()
        if scenario == 'family6':
            first = self.endpoint('::1', '127.0.0.1')
        elif scenario == 'family4':
            first = self.endpoint('127.0.0.2', '::1')
        elif scenario in ('stable', 'unreachable', 'proxy'):
            first = self.endpoint(None, '127.0.0.1')
        else:
            first = self.endpoint('127.0.0.2', '127.0.0.1')
        self.ports = [first]
        if scenario == 'cross_port':
            self.ports.append(self.endpoint('127.0.0.1', '127.0.0.2'))
        if scenario == 'unreachable':
            self.slow_addresses.add((socket.AF_INET, socket.inet_aton('127.0.0.3'), first))
        self.monitor = threading.Thread(target=self.observe)
        self.monitor.start()

    def listener(self, address, port, accepting):
        family = socket.AF_INET6 if ':' in address else socket.AF_INET
        listener = socket.socket(family)
        if family == socket.AF_INET6:
            listener.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 1)
        listener.bind((address, port))
        listener.listen(1 if not accepting else 16)
        self.listeners.append(listener)
        if accepting:
            listener.settimeout(0.05)
            worker = threading.Thread(target=self.accept, args=(listener,))
            self.workers.append(worker)
            worker.start()
        else:
            self.slow_addresses.add((family, socket.inet_pton(family, address), listener.getsockname()[1]))
            filler = socket.socket(family)
            filler.settimeout(2)
            filler.connect(listener.getsockname())
            self.fillers.append(filler)
        return listener.getsockname()[1]

    def endpoint(self, stalled, live):
        port = self.listener(stalled, 0, False) if stalled else 0
        return self.listener(live, port, True)

    def observe(self):
        try:
            while not self.stop.is_set():
                current = 0
                for family, row_type in ((socket.AF_INET, Row4), (socket.AF_INET6, Row6)):
                    size = w.DWORD()
                    code = api.GetExtendedTcpTable(None, c.byref(size), False, family, 5, 0)
                    if code != 122:
                        raise OSError(code, 'TCP size')
                    buffer = c.create_string_buffer(size.value)
                    code = api.GetExtendedTcpTable(buffer, c.byref(size), False, family, 5, 0)
                    if code:
                        raise OSError(code, 'TCP read')
                    for index in range(w.DWORD.from_buffer(buffer).value):
                        row = row_type.from_buffer(buffer, 4 + index * c.sizeof(row_type))
                        remote = socket.ntohs(row.remote_port & 65535)
                        address = row.remote_addr.to_bytes(4, 'little') if family == socket.AF_INET else bytes(row.remote_addr)
                        if row.pid == self.pid and remote in self.ports and row.state == 3 and (family, address, remote) in self.slow_addresses:
                            current += 1
                            with self.lock:
                                self.pending.add((family, row.local_port, remote))
                with self.lock:
                    self.current_pending = current
                self.stop.wait(0.02)
        except Exception as error:
            with self.lock:
                self.failure = dict(stage='tcp_observer', native=getattr(error, 'winerror', None) or getattr(error, 'errno', None))

    def accept(self, listener):
        while not self.stop.is_set():
            try:
                stream, _ = listener.accept()
            except socket.timeout:
                continue
            except OSError:
                return
            stream.settimeout(2)
            with self.lock:
                if len(self.workers) >= 64:
                    stream.close()
                    self.failure = 'worker_bound'
                    return
                self.counts['accepted'] += 1
                self.streams.add(stream)
                worker = threading.Thread(target=self.serve, args=(stream,))
                self.workers.append(worker)
            worker.start()

    def serve(self, stream):
        active = False
        try:
            request = bytearray()
            while b'\r\n\r\n' not in request and len(request) < 16384:
                chunk = stream.recv(1024)
                if not chunk:
                    return
                request.extend(chunk)
            with self.lock:
                self.counts['requests'] += 1
                self.counts['proxy_requests'] += int(request.startswith(b'GET http://'))
                self.counts['active'] += 1
                self.counts['maximum'] = max(self.counts['maximum'], self.counts['active'])
                active = True
            stream.sendall(f'HTTP/1.1 200 OK\r\nContent-Length: {SIZE}\r\nConnection: close\r\nETag: "b6-address"\r\n\r\n'.encode('ascii'))
            stream.sendall(PAYLOAD)
            with self.lock:
                self.counts['served_bytes'] += SIZE
        except (ConnectionResetError, BrokenPipeError, ConnectionAbortedError):
            pass
        except OSError as error:
            if not self.stop.is_set():
                with self.lock:
                    self.failure = dict(stage='http', native=error.winerror or error.errno)
        finally:
            stream.close()
            with self.lock:
                self.streams.discard(stream)
                if active:
                    self.counts['active'] -= 1

    def stats(self):
        with self.lock:
            return dict(self.counts, ports=self.ports, pending_attempts=len(self.pending), pending_now=self.current_pending,
                        open_sockets=len(self.streams), failure=self.failure)

    def close(self):
        self.stop.set()
        for listener in self.listeners:
            listener.close()
        for filler in self.fillers:
            filler.close()
        with self.lock:
            streams = list(self.streams)
        for stream in streams:
            try:
                stream.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
        for worker in self.workers:
            worker.join(3)
        self.monitor.join(3)
        if any(worker.is_alive() for worker in self.workers) or self.monitor.is_alive():
            raise RuntimeError('actual join incomplete')


def emit(value):
    print(json.dumps(value, separators=(',', ':')), flush=True)


server = Server(sys.argv[1])
emit(dict(event='ready', port=server.ports[0], ports=server.ports, owned_fillers=len(server.fillers)))
try:
    for line in sys.stdin:
        command = json.loads(line)['command']
        if command == 'stats':
            emit(dict(event='stats', **server.stats()))
        elif command == 'stop':
            break
        else:
            raise RuntimeError('unknown control')
finally:
    server.close()
emit(dict(event='stopped', **server.stats(), listeners_closed=True, fillers_closed=True, workers_joined=True))
