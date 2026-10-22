"""One-shot NxTools protocol example. stdout is JSONL only; no external effects."""
import json
import sys

invoke = json.loads(sys.stdin.readline())
assert invoke["protocol_version"] == 1 and invoke["op"] == "invoke"
run_id = invoke["run_id"]

def send(frame):
    frame.update(protocol_version=1, run_id=run_id)
    print(json.dumps(frame, ensure_ascii=False), flush=True)

send({"op": "host_call", "call_id": "describe", "method": "host.describe", "params": {}})
reply = json.loads(sys.stdin.readline())
assert reply["op"] == "host_result" and reply["run_id"] == run_id and reply["call_id"] == "describe"
assert reply["success"]
send({"op": "complete", "status": "success", "value": {"text": "fixture"}})
