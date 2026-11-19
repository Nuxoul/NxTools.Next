"""Owned HTTP fault fixture; JSON control frames only, no transfer log on stdout."""
import json
import re
import select
import socket
import sys
import threading
import time

SIZE = 8 * 1024 * 1024
BODY = (bytes(range(251)) * (SIZE // 251 + 1))[:SIZE]
OUTPUT = threading.Lock()
RANGE = re.compile(r"(?im)^range: bytes=(\d+)-(\d+)\r?$")


def emit(frame):
    with OUTPUT:
        print(json.dumps(frame, separators=(",", ":")), flush=True)


class Server:
    def __init__(self):
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.config = {}
        self.records = []
        self.active = self.peak = self.transfers = self.refusals = 0
        self.sockets = set()
        self.workers = []
        self.failure = None
        self.listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen(32)
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
                break
            stream.settimeout(1)
            with self.lock:
                if len(self.workers) >= 256:
                    self.failure = "connection thread bound exceeded"
                    stream.close()
                    continue
                self.sockets.add(stream)
                worker = threading.Thread(target=self.serve, args=(stream,))
                self.workers.append(worker)
            worker.start()

    def serve(self, stream):
        row = None
        receiving = False
        try:
            header = bytearray()
            while b"\r\n\r\n" not in header and len(header) < 16384 and not self.stop.is_set():
                part = stream.recv(2048)
                if not part:
                    return
                header.extend(part)
            text = header.decode("ascii")
            method = text.split(" ", 1)[0]
            match = RANGE.search(text)
            first, last = map(int, match.groups()) if match else (0, SIZE - 1)
            if not (0 <= first <= last < SIZE):
                raise ValueError("invalid requested range")
            probe = method == "HEAD" or (match is not None and first == last == 0)
            if method not in ("GET", "HEAD"):
                raise ValueError("unexpected method")
            if probe:
                ordinal = 0
                config = {}
                status = 206 if match else 200
            else:
                with self.lock:
                    self.transfers += 1
                    ordinal = self.transfers
                    config = dict(self.config)
                    status = 206
                    if config.get("max_conns") and self.active >= config["max_conns"]:
                        status = 503
                    elif config.get("retry_first") and ordinal <= config["retry_first"]:
                        status = 503
                    elif config.get("all_500") or (config.get("first_range_500") and first == 0 and self.refusals < 6):
                        status = 500
                    if status == 500:
                        self.refusals += 1
                    row = dict(ordinal=ordinal, start=first, end=last, status=status,
                               sent=0, outcome="pending", started=time.monotonic(), ended=None)
                    self.records.append(row)
                    if status == 206:
                        self.active += 1
                        receiving = True
                        self.peak = max(self.peak, self.active)
            if status != 206 and status != 200:
                extra = "Retry-After: 1\r\n" if config.get("retry_first") else ""
                stream.sendall(f"HTTP/1.1 {status} Busy\r\nContent-Length: 0\r\n{extra}Connection: close\r\n\r\n".encode("ascii"))
                return
            length = last - first + 1
            range_header = f"Content-Range: bytes {first}-{last}/{SIZE}\r\n" if match else ""
            stream.sendall(f'HTTP/1.1 {status} OK\r\n{range_header}Content-Length: {length}\r\nAccept-Ranges: bytes\r\nETag: "fault-fixture"\r\nConnection: close\r\n\r\n'.encode("ascii"))
            if method == "HEAD":
                return
            drop = row is not None and config.get("drop_every") and ordinal % config["drop_every"] == 0 and length > config.get("drop_after", SIZE)
            slow = row is not None and config.get("slow_every") and ordinal % config["slow_every"] == 0
            rate = config.get("slow_rate", 65536) if slow else config.get("rate", 16 * 1024 * 1024)
            next_send = time.monotonic()
            sent = 0
            while sent < length and not self.stop.is_set():
                if drop and sent >= config["drop_after"]:
                    row["outcome"] = "dropped"
                    return
                size = min(16384, length - sent)
                if drop:
                    size = min(size, config["drop_after"] - sent)
                next_send += size / rate
                if self.stop.wait(max(0, next_send - time.monotonic())):
                    break
                # Catch a peer abandoning a shortened response even if the kernel send buffer is free.
                if select.select([stream], [], [], 0)[0] and stream.recv(1, socket.MSG_PEEK) == b"":
                    row["outcome"] = "aborted"
                    return
                chunk = BODY[first + sent:first + sent + size]
                stream.sendall(chunk)
                sent += size
                if row is not None:
                    with self.lock:
                        row["sent"] = sent
            if row is not None and row["outcome"] == "pending":
                row["outcome"] = "complete" if sent == length else "stopped"
        except (OSError, ValueError, UnicodeError) as error:
            if row is not None and row["outcome"] == "pending" and (isinstance(error, (BrokenPipeError, ConnectionResetError, ConnectionAbortedError)) or getattr(error, "winerror", None) in (10053, 10054)):
                row["outcome"] = "aborted"
            elif not self.stop.is_set():
                with self.lock:
                    self.failure = repr(error)
        finally:
            stream.close()
            with self.lock:
                self.sockets.discard(stream)
                if receiving:
                    self.active -= 1
                if row is not None:
                    if row["outcome"] == "pending":
                        row["outcome"] = "rejected"
                    row["ended"] = time.monotonic()

    def stats(self):
        with self.lock:
            return dict(active=self.active, peak=self.peak, transfers=self.transfers,
                        records=[dict(row) for row in self.records], failure=self.failure)

    def close(self):
        self.stop.set()
        self.listener.close()
        self.acceptor.join(3)
        with self.lock:
            sockets = list(self.sockets)
        for stream in sockets:
            try:
                stream.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
        for worker in self.workers:
            worker.join(3)
        if self.acceptor.is_alive() or any(worker.is_alive() for worker in self.workers):
            raise RuntimeError("owned fixture threads did not join")


server = Server()
emit(dict(event="ready", port=server.port))
try:
    for line in sys.stdin:
        frame = json.loads(line)
        if frame["command"] == "configure":
            with server.lock:
                if server.active:
                    raise RuntimeError("cannot configure during transfer")
                server.config = frame["config"]
                server.records.clear()
                server.transfers = server.refusals = server.peak = 0
            emit(dict(event="configured"))
        elif frame["command"] == "stats":
            emit(dict(event="stats", **server.stats()))
        elif frame["command"] == "stop":
            break
        else:
            raise ValueError("unknown fixture command")
finally:
    server.close()
emit(dict(event="stopped", **server.stats()))
