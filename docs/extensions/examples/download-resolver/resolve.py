"""No network effects: demonstrate explicit pass-through for a selected resolver."""
import json
import sys

invoke = json.loads(sys.stdin.readline())
assert invoke["protocol_version"] == 1 and invoke["op"] == "invoke"
# A real resolver returns status='handled', value={'url': 'https://...'} only after
# validating its own site parsing. Do not fabricate a link or silently try peers.
print(json.dumps({"protocol_version": 1, "op": "complete", "run_id": invoke["run_id"],
                  "status": "not_handled", "value": {}}, ensure_ascii=False), flush=True)
