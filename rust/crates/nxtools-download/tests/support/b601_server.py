"""Owned listener closure with one continuously receiving segment peer."""
import json
import re
import socket
import sys
import threading
import time

TOTAL = 8 * 1024 * 1024
HALF = TOTAL // 2
BODY = (bytes(range(251)) * (TOTAL // 251 + 1))[:TOTAL]
OUTPUT = threading.Lock()


def emit(value):
    with OUTPUT:
        print(json.dumps(value, separators=(",", ":")), flush=True)


class Server:
    def __init__(self):
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.closed = threading.Event()
        self.release = threading.Event()
        self.active = self.maximum = self.accepted = self.requests = 0
        self.status503 = self.peer_requests = self.payload = 0
        self.failure = None
        self.workers = []
        self.listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen(8)
        self.listener.settimeout(0.01)
        self.port = self.listener.getsockname()[1]
        self.acceptor = threading.Thread(target=self.accept, name="b601-listener")
        self.acceptor.start()

    def accept(self):
        while not self.closed.is_set() and not self.stop.is_set():
            try:
                stream, _ = self.listener.accept()
            except socket.timeout:
                continue
            except OSError:
                if self.closed.is_set() or self.stop.is_set():
                    return
                raise
            stream.setblocking(True)
            stream.settimeout(2)
            with self.lock:
                self.accepted += 1
                if self.accepted > 4:
                    stream.close()
                    self.failure = {"stage": "unexpected_accept", "native": 0}
                    return
                worker = threading.Thread(target=self.serve, args=(stream,), name="b601-http")
                self.workers.append(worker)
            worker.start()

    def close_listener(self):
        self.closed.set()
        self.listener.close()
        self.acceptor.join(3)
        if self.acceptor.is_alive():
            raise RuntimeError("listener did not exit")
        emit({"event": "closed", "epoch": 1})

    def serve(self, stream):
        peer = False
        try:
            request = bytearray()
            while b"\r\n\r\n" not in request and len(request) < 16384:
                chunk = stream.recv(1024)
                if not chunk:
                    raise RuntimeError("request ended before headers")
                request.extend(chunk)
            text = request.decode("ascii")
            with self.lock:
                self.requests += 1
            if text.startswith("HEAD "):
                stream.sendall(f'HTTP/1.1 200 OK\r\nContent-Length: {TOTAL}\r\nAccept-Ranges: bytes\r\nETag: "b601-fixed"\r\nConnection: close\r\n\r\n'.encode())
                return
            match = re.search(r"(?im)^range: bytes=(\d+)-(\d+)\r?$", text)
            if not match:
                raise RuntimeError("expected original ranged request")
            first, last = map(int, match.groups())
            if (first, last) == (HALF, TOTAL - 1):
                with self.lock:
                    if self.active != 1 or self.peer_requests != 1:
                        raise RuntimeError("target must have exactly one receiving peer")
                    self.status503 += 1
                self.close_listener()
                stream.sendall(b"HTTP/1.1 503 Controlled\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                return
            if (first, last) not in [(0, 0), (0, HALF - 1)]:
                raise RuntimeError("unexpected range")
            length = last - first + 1
            stream.sendall(f'HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {first}-{last}/{TOTAL}\r\nContent-Length: {length}\r\nAccept-Ranges: bytes\r\nETag: "b601-fixed"\r\nConnection: close\r\n\r\n'.encode())
            if length == 1:
                stream.sendall(BODY[:1])
                return
            peer = True
            with self.lock:
                self.active += 1
                self.maximum = max(self.maximum, self.active)
                self.peer_requests += 1
            sent = 0
            deadline = time.monotonic() + 20
            while sent < length and not self.stop.is_set():
                if time.monotonic() >= deadline:
                    raise RuntimeError("peer fixture deadline")
                released = self.release.is_set()
                size = 65536 if released else 1024
                if not released and sent + size >= length:
                    raise RuntimeError("peer exhausted before terminal refusal")
                end = min(length, sent + size)
                stream.sendall(memoryview(BODY)[sent:end])
                with self.lock:
                    self.payload += end - sent
                sent = end
                if not released:
                    self.stop.wait(0.01)
        except (OSError, UnicodeError, ValueError, RuntimeError) as failure:
            if not self.stop.is_set():
                with self.lock:
                    self.failure = {"stage": "connection", "native": getattr(failure, "winerror", None) or getattr(failure, "errno", 0) or 0}
        finally:
            stream.close()
            if peer:
                with self.lock:
                    self.active -= 1

    def stats(self):
        with self.lock:
            return {"accepted": self.accepted, "requests": self.requests, "status503": self.status503,
                    "peer_requests": self.peer_requests, "served_bytes": self.payload,
                    "active": self.active, "maximum": self.maximum, "failure": self.failure,
                    "listener_exited": not self.acceptor.is_alive()}

    def close(self):
        self.stop.set()
        self.listener.close()
        self.acceptor.join(3)
        for worker in self.workers:
            worker.join(3)
        if self.acceptor.is_alive() or any(worker.is_alive() for worker in self.workers):
            raise RuntimeError("owned fixture threads did not exit")


server = Server()
emit({"event": "ready", "port": server.port})
try:
    for line in sys.stdin:
        command = json.loads(line)["command"]
        if command == "stats":
            emit({"event": "stats", **server.stats()})
        elif command == "release":
            server.release.set()
            emit({"event": "released"})
        elif command == "stop":
            break
        else:
            raise RuntimeError("unknown fixture control")
finally:
    server.close()
emit({"event": "stopped", **server.stats(), "connection_threads_exited": True})
