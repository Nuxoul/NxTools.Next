// One-shot NxTools example. Explicitly release stdin after the final flushed frame.
const readline = require('readline');
const lines = readline.createInterface({ input: process.stdin });
let invoke;
lines.on('line', line => {
  if (!invoke) {
    invoke = JSON.parse(line);
    if (invoke.protocol_version !== 1 || invoke.op !== 'invoke') process.exit(2);
    console.log(JSON.stringify({ protocol_version: 1, op: 'host_call', run_id: invoke.run_id,
      call_id: 'describe', method: 'host.describe', params: {} }));
    return;
  }
  const reply = JSON.parse(line);
  if (reply.op !== 'host_result' || reply.run_id !== invoke.run_id || reply.call_id !== 'describe' || !reply.success) process.exit(2);
  process.stdout.write(JSON.stringify({ protocol_version: 1, op: 'complete', run_id: invoke.run_id,
    status: 'success', value: { text: 'fixture' } }) + '\n', () => {
    lines.close();
    process.stdin.destroy();
  });
});
