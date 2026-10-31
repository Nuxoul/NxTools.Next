"""Dependency-free JSONL client: v1 default, explicit v2 once. Never retries effects."""
from __future__ import annotations
import base64
import hashlib
import json
import sys
from typing import Any, BinaryIO, Iterator, Literal, TypedDict
from uuid import UUID

Completion = Literal['success', 'handled', 'not_handled', 'error']
class Page(TypedDict):
    items: list[dict[str, Any]]
    has_more: bool
    next_cursor: str | None
class HostError(RuntimeError):
    def __init__(self, value: dict[str, Any]):
        self.code = value.get('code', 'host_failed')
        self.retryable = value.get('retryable', False) is True
        self.native_error_code = value.get('native_error_code')
        super().__init__(self.code)  # Do not stringify arbitrary host payloads.

def _object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError('duplicate_key')
        value[key] = item
    return value

class Session:
    def __init__(self, reader: BinaryIO | None = None, writer: BinaryIO | None = None, *, protocol_version: Literal[1, 2] = 1):
        if protocol_version not in (1, 2):
            raise ValueError('unsupported_transport')
        self.reader = reader or sys.stdin.buffer
        self.writer = writer or sys.stdout.buffer
        self.calls = 0
        self.finished = False
        self.protocol_version = protocol_version
        self.session_id = None
        self.run_id = None
        self.progress_sequence = 0
        self.data_chunks = 0
        self.data_sent = 0
        self.data_received = 0
        first = self._read()
        if protocol_version == 2:
            sid = first.get('session_id')
            if first.get('protocol_version') != 2 or first.get('op') != 'initialize' or first.get('mode') != 'once' or not isinstance(sid, str) or UUID(sid).int == 0:
                raise ValueError('invalid_initialize')
            self.session_id = sid
            self._send('ready')
            first = self._read()
        self.invoke = first
        self.run_id = first.get('run_id')
        if first.get('protocol_version') != protocol_version or first.get('op') != 'invoke' or not isinstance(self.run_id, str) or UUID(self.run_id).int == 0 or (protocol_version == 2 and first.get('session_id') != self.session_id):
            raise ValueError('invalid_invoke')

    def _read(self) -> dict[str, Any]:
        raw = self.reader.readline(256 * 1024 + 1)
        if not raw or len(raw) > 256 * 1024 or not raw.endswith(b'\n'):
            raise ValueError('invalid_frame')
        value = json.loads(raw.decode('utf-8'), object_pairs_hook=_object, parse_constant=lambda _: (_ for _ in ()).throw(ValueError('invalid_number')))
        if not isinstance(value, dict):
            raise ValueError('invalid_frame')
        return value

    def _send(self, op: str, **fields: Any) -> None:
        if self.finished:
            raise ValueError('completed')
        identity = {'protocol_version': self.protocol_version}
        if self.run_id is not None:
            identity['run_id'] = self.run_id
        if self.protocol_version == 2:
            identity['session_id'] = self.session_id
        raw = (json.dumps(dict(identity, op=op, **fields), ensure_ascii=False, allow_nan=False, separators=(',', ':')) + '\n').encode('utf-8')
        if len(raw) > 256 * 1024:
            raise ValueError('frame_too_large')
        self.writer.write(raw)
        self.writer.flush()

    def _v2(self) -> None:
        if self.protocol_version != 2:
            raise ValueError('unsupported_transport')

    def _receive(self, op: str, transfer_id: str | None = None) -> dict[str, Any]:
        reply = self._read()
        if reply.get('protocol_version') != self.protocol_version or reply.get('run_id') != self.run_id or (self.protocol_version == 2 and reply.get('session_id') != self.session_id):
            raise ValueError('wrong_identity')
        if reply.get('op') == 'cancel':
            raise HostError({'code': 'cancelled'})
        if reply.get('op') != op or (transfer_id is not None and reply.get('transfer_id') != transfer_id):
            raise ValueError('invalid_reply')
        return reply

    def progress(self, completed: int | None = None, total: int | None = None, message: str | None = None) -> None:
        self._v2()
        if (completed is None) != (total is None) or (total is not None and (type(total) is not int or type(completed) is not int or not 0 <= completed <= total <= 2**53-1 or total == 0)):
            raise ValueError('invalid_progress')
        if message is not None and (not isinstance(message, str) or len(message.encode('utf-8')) > 512 or any(ord(ch) < 32 and ch not in '\n\t' or 127 <= ord(ch) <= 159 for ch in message)):
            raise ValueError('invalid_progress')
        if self.progress_sequence >= 256:
            raise ValueError('progress_limit')
        self.progress_sequence += 1
        fields = dict(sequence=self.progress_sequence)
        if total is not None: fields.update(completed=completed, total=total)
        if message is not None: fields['message'] = message
        self._send('progress', **fields)

    def _chunk_budget(self, count: int, sending: bool) -> None:
        total = self.data_sent if sending else self.data_received
        if count <= 0 or count > 65536 or total + count > 16*1024*1024 or self.data_chunks >= 512:
            raise ValueError('data_limit')
        self.data_chunks += 1
        if sending: self.data_sent += count
        else: self.data_received += count

    def blob_upload(self, data: bytes, media_type: str = 'application/octet-stream') -> dict[str, Any]:
        self._v2()
        if not isinstance(data, bytes) or len(data) > 8*1024*1024: raise ValueError('blob_limit')
        blob = self.call('blobs.begin', dict(length=len(data), media_type=media_type, sha256=hashlib.sha256(data).hexdigest()))
        transfer = blob['transfer_id']
        for sequence, offset in enumerate(range(0, len(data), 65536), 1):
            chunk = data[offset:offset+65536]
            self._chunk_budget(len(chunk), True)
            self._send('data', transfer_id=transfer, sequence=sequence, offset=offset, data=base64.b64encode(chunk).decode('ascii'))
            ack = self._receive('data_ack', transfer)
            if ack.get('sequence') != sequence or ack.get('offset') != offset+len(chunk): raise ValueError('invalid_ack')
        return self.call('blobs.seal', dict(blob_id=blob['blob_id']))

    def blob_read(self, blob: dict[str, Any], *, offset: int = 0, length: int | None = None) -> bytes:
        self._v2()
        params = dict(blob_id=blob['blob_id'], offset=offset)
        if length is not None: params['length'] = length
        span = self.call('blobs.read', params)
        count = span['length']
        if type(count) is not int or not 0 <= count <= 8*1024*1024: raise ValueError('blob_limit')
        data = bytearray()
        sequence = 0
        while len(data) < count:
            part = self._receive('data', span['transfer_id'])
            sequence += 1
            encoded = part.get('data')
            if not isinstance(encoded, str) or len(encoded) > 87384: raise ValueError('invalid_data')
            chunk = base64.b64decode(encoded, validate=True)
            if base64.b64encode(chunk).decode('ascii') != encoded or part.get('sequence') != sequence or part.get('offset') != offset+len(data) or len(data)+len(chunk) > count:
                raise ValueError('invalid_data')
            self._chunk_budget(len(chunk), False)
            data.extend(chunk)
            self._send('data_ack', transfer_id=span['transfer_id'], sequence=sequence, offset=offset+len(data))
        if offset == 0 and count == blob.get('length') and hashlib.sha256(data).hexdigest() != blob.get('sha256'):
            raise ValueError('digest_mismatch')
        return bytes(data)

    def blob_release(self, blob: dict[str, Any]) -> None:
        self._v2()
        self.call('blobs.release', dict(blob_id=blob['blob_id']))

    def storage_get(self, key: str) -> dict[str, Any]:
        self._v2()
        return self.call('storage.get', dict(key=key))

    def storage_set(self, key: str, value: Any, *, expected_revision: int | None = None) -> dict[str, Any]:
        self._v2()
        params = dict(key=key, value=value)
        if expected_revision is not None: params['expected_revision'] = expected_revision
        return self.call('storage.set', params)

    def storage_delete(self, key: str, *, expected_revision: int | None = None) -> dict[str, Any]:
        self._v2()
        params = dict(key=key)
        if expected_revision is not None: params['expected_revision'] = expected_revision
        return self.call('storage.delete', params)

    def storage_list(self, *, prefix: str = '', limit: int = 32, cursor: str | None = None) -> dict[str, Any]:
        self._v2()
        return self.call('storage.list', dict(prefix=prefix, limit=limit, cursor=cursor))

    def call(self, method: str, params: dict[str, Any]) -> Any:
        self.calls += 1
        if self.calls > 64:
            raise ValueError('host_call_limit')
        call_id = f'call-{self.calls}'
        self._send('host_call', call_id=call_id, method=method, params=params)
        reply = self._receive('host_result')
        if reply.get('op') != 'host_result' or reply.get('call_id') != call_id or type(reply.get('success')) is not bool or 'value' not in reply:
            raise ValueError('invalid_reply')
        if not reply['success']:
            raise HostError(reply['value'] if isinstance(reply['value'], dict) else {})
        return reply['value']

    def pages(self, method: Literal['downloads.list_page', 'clipboard.query_page'], **params: Any) -> Iterator[Page]:
        """Bounded by the host-call cap. Mutation/expiry errors propagate; never restart silently."""
        cursor = None
        seen = set()
        while True:
            page = self.call(method, dict(params, cursor=cursor))
            if not isinstance(page, dict) or not isinstance(page.get('items'), list) or type(page.get('has_more')) is not bool:
                raise ValueError('invalid_page')
            yield page
            if not page['has_more']:
                return
            cursor = page.get('next_cursor')
            if not isinstance(cursor, str) or cursor in seen:
                raise ValueError('invalid_cursor')
            seen.add(cursor)

    def complete(self, status: Completion = 'success', value: Any = None) -> None:
        if status not in ('success', 'handled', 'not_handled', 'error'):
            raise ValueError('invalid_status')
        self._send('complete', status=status, value={} if value is None else value)
        if self.protocol_version == 2:
            self._send('request_finished')
        self.finished = True
