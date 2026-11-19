"""Owned fixed-load HTTP fixture for original adaptive tail experiments."""
import json
import re
import socket
import sys
import threading
import time

SIZE = 16 * 1024 * 1024
BODY = (bytes(range(251)) * (SIZE // 251 + 1))[:SIZE]
OUTPUT_LOCK = threading.Lock()


def emit(value):
    with OUTPUT_LOCK:
        print(json.dumps(value, separators=(",", ":")), flush=True)


class Server:
    def __init__(self):
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.scenario = "persistent"
        self.accepted = self.requests = self.active = self.maximum = self.served = 0
        self.open_sockets = self.retired = self.transfers = 0
        self.records = []
        self.failure = None
        self.workers = []
        self.listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen(16)
        self.listener.settimeout(0.02)
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
            stream.settimeout(2)
            with self.lock:
                self.accepted += 1
                self.open_sockets += 1
                if len(self.workers) >= 128:
                    stream.close()
                    self.open_sockets -= 1
                    self.failure = {"stage": "connection_bound", "native": 0}
                    return
                worker = threading.Thread(target=self.serve, args=(stream,))
                self.workers.append(worker)
            worker.start()

    def serve(self, stream):
        record = None
        try:
            request = bytearray()
            while b"\r\n\r\n" not in request and len(request) < 16384:
                part = stream.recv(1024)
                if not part:
                    return
                request.extend(part)
            text = request.decode("ascii")
            method = text.split(" ", 1)[0]
            with self.lock:
                self.requests += 1
            if method == "HEAD":
                stream.sendall(f'HTTP/1.1 200 OK\r\nContent-Length: {SIZE}\r\nAccept-Ranges: bytes\r\nETag: "b6-tail"\r\nConnection: close\r\n\r\n'.encode("ascii"))
                return
            match = re.search(r"(?im)^range: bytes=(\d+)-(\d+)\r?$", text)
            first, last = map(int, match.groups()) if match else (0, SIZE - 1)
            if first > last or last >= SIZE:
                raise ValueError("invalid controlled range")
            length = last - first + 1
            status = "206 Partial Content" if match else "200 OK"
            extra = f"Content-Range: bytes {first}-{last}/{SIZE}\r\n" if match else ""
            stream.sendall(f'HTTP/1.1 {status}\r\n{extra}Content-Length: {length}\r\nAccept-Ranges: bytes\r\nETag: "b6-tail"\r\nConnection: close\r\n\r\n'.encode("ascii"))
            if length > 1:
                with self.lock:
                    self.transfers += 1
                    self.active += 1
                    self.maximum = max(self.maximum, self.active)
                    record = dict(ordinal=self.transfers, first=first, last=last, served=0,
                                  started=time.monotonic(), ended=None, retired=False)
                    self.records.append(record)
                    scenario = self.scenario
            else:
                scenario = "stable"
            started = time.monotonic()
            sent = 0
            next_write = started
            stream.settimeout(0.1)
            while sent < length and not self.stop.is_set():
                count = min(16384, length - sent)
                slow = record is not None and record["ordinal"] == 1 and scenario != "stable"
                if scenario == "recovering" and time.monotonic() - started >= 6:
                    slow = False
                rate = 128 * 1024 if slow else 1024 * 1024
                next_write += count / rate if length > 1 else 0
                if self.stop.wait(max(0, next_write - time.monotonic())):
                    break
                written = 0
                while written < count and not self.stop.is_set():
                    if time.monotonic() - started > 120:
                        raise TimeoutError("fixed transfer deadline")
                    try:
                        size = stream.send(memoryview(BODY)[first + sent + written:first + sent + count])
                    except socket.timeout:
                        continue
                    if size == 0:
                        raise ConnectionResetError("closed transfer")
                    written += size
                    with self.lock:
                        self.served += size
                        if record is not None:
                            record["served"] += size
                sent += written
        except (OSError, ValueError, UnicodeError) as exception:
            if not self.stop.is_set():
                native = getattr(exception, "winerror", None) or getattr(exception, "errno", 0) or 0
                with self.lock:
                    if record is not None and (isinstance(exception, (ConnectionResetError, BrokenPipeError)) or native == 10053):
                        self.retired += 1
                        record["retired"] = True
                    else:
                        self.failure = {"stage": "connection", "native": native}
        finally:
            stream.close()
            with self.lock:
                self.open_sockets -= 1
                if record is not None:
                    self.active -= 1
                    record["ended"] = time.monotonic()

    def stats(self):
        with self.lock:
            return dict(accepted=self.accepted, requests=self.requests, served_bytes=self.served,
                        active=self.active, maximum=self.maximum, open_sockets=self.open_sockets,
                        retired_streams=self.retired, transfer_requests=self.transfers,
                        records=[dict(row) for row in self.records], failure=self.failure)

    def close(self):
        self.stop.set()
        self.listener.close()
        self.acceptor.join(3)
        for worker in self.workers:
            worker.join(3)
        if self.acceptor.is_alive() or any(worker.is_alive() for worker in self.workers):
            raise RuntimeError("owned HTTP worker did not exit")


server = Server()
emit(dict(event="ready", port=server.port))
try:
    for line in sys.stdin:
        command = json.loads(line)
        if command["command"] == "round":
            with server.lock:
                if server.active:
                    raise RuntimeError("previous transfer is live")
                server.scenario = command["scenario"]
            emit(dict(event="round"))
        elif command["command"] == "stats":
            emit(dict(event="stats", **server.stats()))
        elif command["command"] == "stop":
            break
        else:
            raise RuntimeError("unknown fixture control")
finally:
    server.close()
emit(dict(event="stopped", **server.stats()))
