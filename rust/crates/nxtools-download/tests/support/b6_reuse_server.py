"""Owned full-fallback response fixture; no product or system configuration."""
import json
import re
import socket
import sys
import threading
import time

SIZE = 8 * 1024 * 1024
CHUNK = bytes(i % 251 for i in range(65536))


class Server:
    def __init__(self, scenario):
        self.scenario = scenario
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.workers = []
        self.streams = set()
        self.counts = dict(accepted=0, requests=0, full_gets=0, served_bytes=0,
                           active=0, maximum=0, retired=0, write_timeouts=0)
        self.failure = None
        self.listener = socket.socket()
        self.listener.bind(('127.0.0.1', 0))
        self.listener.listen(16)
        self.listener.settimeout(0.05)
        self.port = self.listener.getsockname()[1]
        self.acceptor = threading.Thread(target=self.accept)
        self.acceptor.start()

    def accept(self):
        while not self.stop.is_set():
            try:
                stream, _ = self.listener.accept()
            except socket.timeout:
                continue
            except OSError:
                return
            stream.settimeout(2)
            with self.lock:
                if len(self.workers) >= 64:
                    stream.close()
                    self.failure = 'connection_bound'
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
            text = request.decode('ascii')
            method = text.split(' ', 1)[0]
            ranged = re.search(r'(?im)^range:', text) is not None
            with self.lock:
                self.counts['requests'] += 1
                if method == 'GET' and not ranged:
                    self.counts['full_gets'] += 1
                ordinal = self.counts['full_gets']
            self.stop.wait(0.2)
            normal = self.scenario == 'ineligible'
            if method == 'HEAD':
                status, length = (200, SIZE) if normal else (405, 0)
            elif ranged:
                status, length = (206, 1) if normal else (416, 0)
            elif self.scenario == 'single_use' and ordinal > 1:
                status, length = 410, 0
            else:
                status, length = 200, SIZE
            headers = f'HTTP/1.1 {status} Controlled\r\nConnection: close\r\nETag: "b6-reuse"\r\n'
            if self.scenario != 'unknown' or method == 'HEAD' or length == 0:
                headers += f'Content-Length: {length}\r\n'
            if status == 206:
                headers += f'Content-Range: bytes 0-0/{SIZE}\r\n'
            stream.sendall((headers + '\r\n').encode('ascii'))
            if method == 'HEAD' or length == 0:
                return
            with self.lock:
                self.counts['active'] += 1
                self.counts['maximum'] = max(self.counts['maximum'], self.counts['active'])
                active = True
            target = min(length, SIZE // 2) if self.scenario == 'truncated' else length
            started = time.monotonic()
            offset = 0
            while offset < target and not self.stop.is_set():
                end = min(target, offset + 65536)
                if length > 1:
                    self.stop.wait(max(0, started + end / (2 * 1024 * 1024) - time.monotonic()))
                payload = bytes((offset + i) % 251 for i in range(end - offset))
                stream.sendall(payload)
                with self.lock:
                    self.counts['served_bytes'] += end - offset
                offset = end
        except socket.timeout:
            with self.lock:
                self.counts['write_timeouts'] += 1
        except (ConnectionResetError, BrokenPipeError, ConnectionAbortedError):
            with self.lock:
                self.counts['retired'] += 1
        except OSError as error:
            if not self.stop.is_set():
                with self.lock:
                    if error.winerror == 10053:
                        self.counts['retired'] += 1
                    else:
                        self.failure = {'native': error.winerror or error.errno}
        finally:
            stream.close()
            with self.lock:
                self.streams.discard(stream)
                if active:
                    self.counts['active'] -= 1

    def stats(self):
        with self.lock:
            return {**self.counts, 'open_sockets': len(self.streams), 'failure': self.failure}

    def close(self):
        self.stop.set()
        self.listener.close()
        with self.lock:
            streams = list(self.streams)
        for stream in streams:
            try:
                stream.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
        self.acceptor.join(3)
        for worker in self.workers:
            worker.join(3)
        if self.acceptor.is_alive() or any(worker.is_alive() for worker in self.workers):
            raise RuntimeError('actual join incomplete')


def emit(value):
    print(json.dumps(value, separators=(',', ':')), flush=True)


server = Server(sys.argv[1])
emit({'event': 'ready', 'port': server.port})
try:
    for line in sys.stdin:
        command = json.loads(line)['command']
        if command == 'stats':
            emit({'event': 'stats', **server.stats()})
        elif command == 'stop':
            break
        else:
            raise RuntimeError('unknown control')
finally:
    server.close()
emit({'event': 'stopped', **server.stats()})
