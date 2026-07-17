# Platform protocol v1

Transport is a duplex Windows named pipe named `nxtools-next-platform-v1`. Each frame is one
UTF-8 JSON object followed by `\n`; the JSON payload before that delimiter may not exceed 1 MiB.

```json
{
  "protocol_version": 1,
  "id": "8df2ce69-8881-48a3-a6b3-8b9c8aa1771c",
  "correlation_id": null,
  "type": "hello",
  "payload": {
    "client_name": "platform_host",
    "client_version": "0.1.0",
    "capabilities": ["foreground_context"]
  }
}
```

The receiver rejects unsupported protocol versions before interpreting the payload. Replies use
a fresh `id` and set `correlation_id` to the request id. Unknown message types receive an error
reply once request/command routing is introduced; unsolicited platform events require no reply.
