"""Owned loopback HTTP fixture for connection experiments."""
import json
import re
import socket
import sys
import threading
import time

BODY = bytes(range(251)) * (8 * 1024 * 1024 // 251 + 1)
BODY = BODY[:8 * 1024 * 1024]
WRITE_LOCK = threading.Lock()


def emit(value):
    with WRITE_LOCK:
        print(json.dumps(value, separators=(",", ":")), flush=True)


class Server:
    def __init__(self):
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.limit = 2
        self.status = 0
        self.allow_retirement = False
        self.transfer_seconds = 2.5
        self.retired_streams = 0
        self.active = 0
        self.maximum = 0
        self.accepted = 0
        self.requests = 0
        self.proxy_requests = 0
        self.payload = 0
        self.closed_epoch = 0
        self.notified_epoch = 0
        self.workers = []
        self.failure = None
        self.listener = self.bind(0)
        self.port = self.listener.getsockname()[1]
        self.acceptor = threading.Thread(target=self.accept, name="b6-listener")
        self.acceptor.start()

    @staticmethod
    def bind(port):
        listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        listener.bind(("127.0.0.1", port))
        listener.listen(16)
        listener.settimeout(0.01)
        return listener

    def accept(self):
        while not self.stop.is_set():
            with self.lock:
                listener = self.listener
                epoch = self.closed_epoch
                reopen = listener is None and self.active < self.limit
            if listener is None:
                if epoch > self.notified_epoch:
                    self.notified_epoch = epoch
                    emit({"event": "closed", "epoch": epoch})
                if reopen:
                    try:
                        replacement = self.bind(self.port)
                    except OSError as error:
                        self.failure = {"stage": "rebind", "native": error.winerror or error.errno}
                        return
                    with self.lock:
                        self.listener = replacement
                    continue
                self.stop.wait(0.001)
                continue
            try:
                stream, _ = listener.accept()
            except socket.timeout:
                continue
            except OSError:
                continue
            stream.setblocking(True)
            stream.settimeout(2)
            with self.lock:
                self.accepted += 1
                if len(self.workers) >= 256:
                    stream.close()
                    self.failure = {"stage": "connection_bound", "native": 0}
                    return
                admitted = threading.Event()
                worker = threading.Thread(target=self.serve, args=(stream, admitted), name="b6-http")
                self.workers.append(worker)
            worker.start()
            if not admitted.wait(3):
                self.failure = {"stage": "admission_timeout", "native": 0}
                return

    def serve(self, stream, admitted):
        transferring = False
        allow_retirement = False
        try:
            request = bytearray()
            while b"\r\n\r\n" not in request and len(request) < 16384:
                chunk = stream.recv(1024)
                if not chunk:
                    return
                request.extend(chunk)
            text = request.decode("ascii")
            method = text.split(" ", 1)[0]
            with self.lock:
                self.requests += 1
                if text.split(" ", 2)[1].startswith("http://"):
                    self.proxy_requests += 1
                forced_status = self.status
                allow_retirement = self.allow_retirement
                transfer_seconds = self.transfer_seconds
            if forced_status:
                stream.sendall(f"HTTP/1.1 {forced_status} Controlled\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".encode("ascii"))
                return
            if method == "HEAD":
                header = f'HTTP/1.1 200 OK\r\nContent-Length: {len(BODY)}\r\nAccept-Ranges: bytes\r\nETag: "b6-fixed"\r\nConnection: close\r\n\r\n'
                stream.sendall(header.encode("ascii"))
                return
            match = re.search(r"(?im)^range: bytes=(\d+)-(\d+)\r?$", text)
            if match:
                first, last = map(int, match.groups())
                status = "206 Partial Content"
                extra = f"Content-Range: bytes {first}-{last}/{len(BODY)}\r\n"
            else:
                first, last = 0, len(BODY) - 1
                status, extra = "200 OK", ""
            if first > last or last >= len(BODY):
                stream.sendall(b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                return
            length = last - first + 1
            header = f'HTTP/1.1 {status}\r\n{extra}Content-Length: {length}\r\nAccept-Ranges: bytes\r\nETag: "b6-fixed"\r\nConnection: close\r\n\r\n'
            stream.sendall(header.encode("ascii"))
            if length > 1:
                with self.lock:
                    self.active += 1
                    transferring = True
                    self.maximum = max(self.maximum, self.active)
                    if self.active == self.limit:
                        closed = self.listener
                        self.listener = None
                        self.closed_epoch += 1
                    else:
                        closed = None
                if closed is not None:
                    try:
                        closed.shutdown(socket.SHUT_RDWR)
                    except OSError:
                        pass
                    closed.close()
            admitted.set()
            started = time.monotonic()
            sent = 0
            while sent < length and not self.stop.is_set():
                end = min(length, sent + 131072)
                if length > 1:
                    remaining = started + transfer_seconds * end / length - time.monotonic()
                    if remaining > 0:
                        self.stop.wait(remaining)
                stream.sendall(memoryview(BODY)[first + sent:first + end])
                with self.lock:
                    self.payload += end - sent
                sent = end
        except (OSError, UnicodeError, ValueError) as error:
            if not self.stop.is_set():
                with self.lock:
                    native = getattr(error, "winerror", None) or getattr(error, "errno", 0) or 0
                    if allow_retirement and (isinstance(error, (ConnectionResetError, BrokenPipeError)) or native == 10053):
                        self.retired_streams += 1
                    else:
                        self.failure = {"stage": "connection", "native": native}
        finally:
            admitted.set()
            stream.close()
            if transferring:
                with self.lock:
                    self.active -= 1

    def stats(self):
        with self.lock:
            return {"accepted": self.accepted, "requests": self.requests, "served_bytes": self.payload,
                    "maximum": self.maximum, "active": self.active, "failure": self.failure,
                    "retired_streams": self.retired_streams, "proxy_requests": self.proxy_requests}

    def close(self):
        self.stop.set()
        with self.lock:
            listener = self.listener
            self.listener = None
        if listener is not None:
            listener.close()
        self.acceptor.join(3)
        for worker in self.workers:
            worker.join(3)
        if self.acceptor.is_alive() or any(worker.is_alive() for worker in self.workers):
            raise RuntimeError("owned worker cleanup incomplete")


server = Server()
emit({"event": "ready", "port": server.port})
try:
    for line in sys.stdin:
        command = json.loads(line)
        if command["command"] == "round":
            with server.lock:
                if server.active != 0:
                    raise RuntimeError("previous round still active")
                server.limit = command.get("limit", 2)
                server.status = command.get("status", 0)
                server.allow_retirement = command.get("allow_retirement", False)
                server.transfer_seconds = command.get("transfer_seconds", 2.5)
            emit({"event": "round"})
        elif command["command"] == "stats":
            emit({"event": "stats", **server.stats()})
        elif command["command"] == "stop":
            break
        else:
            raise RuntimeError("unknown fixture control")
finally:
    server.close()
emit({"event": "stopped", **server.stats()})
