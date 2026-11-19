"""Owned self-signed TLS and CONNECT termination, without trust-store changes."""
import datetime
import json
import socket
import ssl
import sys
import tempfile
import threading
from pathlib import Path
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID

SIZE = 1024 * 1024
root = tempfile.TemporaryDirectory(prefix='b6-tls-')
key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
name = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, 'b6.tls.test')])
now = datetime.datetime.now(datetime.timezone.utc)
certificate = (x509.CertificateBuilder().subject_name(name).issuer_name(name).public_key(key.public_key())
               .serial_number(x509.random_serial_number()).not_valid_before(now - datetime.timedelta(minutes=1))
               .not_valid_after(now + datetime.timedelta(days=1))
               .add_extension(x509.SubjectAlternativeName([x509.DNSName('b6.tls.test')]), critical=False)
               .sign(key, hashes.SHA256()))
Path(root.name, 'key.pem').write_bytes(key.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8, serialization.NoEncryption()))
Path(root.name, 'cert.pem').write_bytes(certificate.public_bytes(serialization.Encoding.PEM))
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain(str(Path(root.name, 'cert.pem')), str(Path(root.name, 'key.pem')))
lock = threading.Lock()
stop = threading.Event()
counts = dict(accepted=0, requests=0, served_bytes=0, active=0, open_sockets=0, connect_requests=0, sni_matches=0, tls_rejections=0, origin_canary_connections=0)
workers = []
failure = None


def sni(stream, server_name, initial_context):
    with lock:
        counts['sni_matches'] += int(server_name == 'b6.tls.test')
context.set_servername_callback(sni)
listener = socket.socket()
listener.bind(('127.0.0.1', 0))
listener.listen(8)
listener.settimeout(0.05)
canary = socket.socket()
canary.bind(('127.0.0.1', 0))
canary.listen(8)
canary.setblocking(False)


def headers(stream):
    value = bytearray()
    while b'\r\n\r\n' not in value and len(value) < 16384:
        chunk = stream.recv(1024)
        if not chunk:
            break
        value.extend(chunk)
    return value


def serve(stream):
    global failure
    try:
        stream.settimeout(2)
        if sys.argv[1] == 'proxy':
            request = headers(stream)
            if not request.startswith(b'CONNECT b6.tls.test:'):
                raise RuntimeError('CONNECT authority mismatch')
            with lock:
                counts['connect_requests'] += 1
            stream.sendall(b'HTTP/1.1 200 Connection established\r\n\r\n')
        stream = context.wrap_socket(stream, server_side=True)
        request = headers(stream)
        if not request.startswith(b'GET /file '):
            raise RuntimeError('TLS request mismatch')
        with lock:
            counts['requests'] += 1
        stream.sendall(f'HTTP/1.1 200 OK\r\nContent-Length: {SIZE}\r\nConnection: close\r\n\r\n'.encode('ascii'))
        stream.sendall(bytes(index % 251 for index in range(SIZE)))
        with lock:
            counts['served_bytes'] += SIZE
    except ssl.SSLError:
        with lock:
            counts['tls_rejections'] += 1
    except Exception as error:
        with lock:
            failure = dict(stage=type(error).__name__, native=getattr(error, 'winerror', None))
    finally:
        stream.close()
        with lock:
            counts['active'] -= 1
            counts['open_sockets'] -= 1


def accept():
    while not stop.is_set():
        try:
            stream, _ = listener.accept()
        except socket.timeout:
            continue
        except OSError:
            return
        with lock:
            counts['accepted'] += 1
            counts['active'] += 1
            counts['open_sockets'] += 1
        worker = threading.Thread(target=serve, args=(stream,))
        workers.append(worker)
        worker.start()


def stats():
    try:
        unexpected, _ = canary.accept()
    except BlockingIOError:
        pass
    else:
        unexpected.close()
        counts['origin_canary_connections'] += 1
    with lock:
        return dict(counts, origin_port=canary.getsockname()[1], failure=failure)


def emit(value):
    print(json.dumps(value, separators=(',', ':')), flush=True)


acceptor = threading.Thread(target=accept)
acceptor.start()
emit(dict(event='ready', port=listener.getsockname()[1], origin_port=canary.getsockname()[1]))
try:
    for line in sys.stdin:
        if json.loads(line)['command'] == 'stop':
            break
        emit(stats())
finally:
    stop.set()
    listener.close()
    acceptor.join(3)
    for worker in workers:
        worker.join(3)
    assert not acceptor.is_alive() and all(not worker.is_alive() for worker in workers)
    final = stats()
    canary.close()
    root.cleanup()
emit(dict(final, workers_joined=True, private_pem_removed=True))
