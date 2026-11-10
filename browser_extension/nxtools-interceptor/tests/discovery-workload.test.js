"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { pathToFileURL } = require("node:url");
const path = require("node:path");
const helper = import("./discovery-browser.mjs");

test("comparison schedule is three serial interleaved equivalent pairs", async () => {
  const { TRIAL_ORDER, WORKLOAD } = await helper;
  assert.deepEqual(TRIAL_ORDER.map(row => [row.group, row.variant]), [[1, "baseline"], [1, "candidate"], [2, "candidate"], [2, "baseline"], [3, "baseline"], [3, "candidate"]]);
  assert.ok(WORKLOAD.idleMs > 7000);
  assert.equal(WORKLOAD.settings.captureMinimumKiB, 0);
});

test("CPU metric and three-trial summary never substitute resource counts or missing numbers", async () => {
  const { taskDuration, summarizeTrials, TRIAL_ORDER } = await helper;
  assert.throws(() => taskDuration({ metrics: [{ name: "Nodes", value: 100 }] }), /TaskDuration/);
  assert.equal(taskDuration({ metrics: [{ name: "TaskDuration", value: 2.5 }] }), 2.5);
  const rows = TRIAL_ORDER.map((plan, index) => ({ ...plan, active: { taskMs: index + 1, discoveryLatenciesMs: [index, index + 2], candidates: ["a"], observations: ["a", "b"] }, idle: { taskMs: index / 10 } }));
  const summary = summarizeTrials(rows);
  assert.deepEqual(summary.baseline.activeTaskMs, { raw: [1, 4, 5], median: 4, slowest: 5 });
  assert.deepEqual(summary.candidate.idleTaskMs, { raw: [0.1, 0.2, 0.5], median: 0.2, slowest: 0.5 });
  assert.throws(() => summarizeTrials(rows.slice(0, 4)), /three/);
});

test("workload bundle reads real production manifests and every ordered source", async () => {
  const { readSourceBundle } = await helper;
  const root = path.resolve(__dirname, "..");
  const bundle = await readSourceBundle(pathToFileURL(root + path.sep));
  for (const file of [...bundle.manifest.background.scripts, ...bundle.manifest.content_scripts[0].js]) assert.equal(typeof bundle.sources[file], "string");
  assert.equal(bundle.manifest.permissions.includes("debugger"), false);
});

test("failed supplied-Page execution preserves the original error, stage and partial output without navigation", async t => {
  const fs = require("node:fs/promises");
  const scratch = await fs.mkdtemp(path.join(process.env.PI_SCRATCH_DIR || require("node:os").tmpdir(), "b2-helper-failure-"));
  t.after(() => fs.rm(scratch, { recursive: true, force: true }));
  await fs.writeFile(path.join(scratch, "manifest.json"), JSON.stringify({ background: { scripts: [] }, content_scripts: [{ js: [] }] }));
  await fs.writeFile(path.join(scratch, "page-hook.js"), "");
  const outputPath = path.join(scratch, "partial.json"), navigations = [];
  const failure = Object.assign(new Error("fixture evaluate still pending"), { name: "PageEvaluationTimeoutError", executionStopped: false, mayHaveLateEffects: true });
  const page = {
    label: "supplied-page", goto: async value => { navigations.push(value); },
    cdp: async method => { if (method === "Page.getFrameTree") return { frameTree: { frame: { id: "frame" } } }; return {}; },
    evaluate: async () => { throw failure; }, snapshot: async () => "preserved fixture page"
  };
  const { runDiscoveryComparison } = await helper;
  await assert.rejects(runDiscoveryComparison(page, { baselineDirectory: scratch, candidateDirectory: path.resolve(__dirname, ".."), outputPath }), error => error === failure);
  assert.deepEqual(navigations, ["about:blank"], "only initial Popup navigation; never erase failed page");
  const partial = JSON.parse(await fs.readFile(outputPath, "utf8"));
  assert.equal(partial.status, "failed");
  assert.match(partial.stage, /^popup\./);
  assert.equal(partial.failure.name, "PageEvaluationTimeoutError");
  assert.equal(partial.failure.executionStopped, false);
  assert.equal(partial.failure.mayHaveLateEffects, true);
  assert.equal(partial.failure.snapshot, "preserved fixture page");
  assert.deepEqual(partial.trials, []);
});

function vmPage(extra = {}) {
  const vm = require("node:vm"), context = vm.createContext({ console, structuredClone, setTimeout, performance, ...extra });
  context.window = context;
  const evaluate = (fn, argument) => {
    context.argument = argument;
    const result = vm.runInContext(`(${fn.toString()})(argument)`, context);
    assert.equal(typeof result?.then, "undefined", "evaluate must not await a fixture promise");
    return result;
  };
  return { context, evaluate: async (...args) => evaluate(...args),
    waitForFunction: async (fn, argument, options) => {
      assert.ok(options.timeout > 0 && options.timeout <= 12000);
      const until = Date.now() + options.timeout;
      while (!evaluate(fn, argument)) {
        if (Date.now() >= until) throw new Error("fixture observable wait timed out");
        await new Promise(resolve => setImmediate(resolve));
      }
    } };
}

test("named async Page jobs are observed with a bounded wait, preserve errors and stop on pending work", async () => {
  const { runPageJob } = await import("./browser-stages.mjs");
  const page = vmPage();
  assert.equal(await runPageJob(page, "round.1", async value => { await Promise.resolve(); return value + 1; }, 4), 5);
  await assert.rejects(runPageJob(page, "round.2", async () => { throw new Error("actual fixture failure"); }), /actual fixture failure/);
  await assert.rejects(runPageJob(page, "round.3", () => new Promise(() => {}), null, 5), error => error.pendingJob === "round.3" && error.mayHaveLateEffects === true);
  await assert.rejects(runPageJob(page, "round.4", () => 1), /previous fixture job is still pending/);
});

test("chunked active workload runs exactly 24 equivalent serial rounds with partial raw checkpoints", async () => {
  const { activeWork, WORKLOAD } = await helper;
  const resources = new Map(), responses = [], rounds = [], stages = [];
  let choices = 0;
  const work = { responseEvents: 0 };
  const page = vmPage({
    __errors: [], Event: class {}, CustomEvent: class {}, Blob: class {}, URL: { createObjectURL: () => "blob:fixture" },
    dispatchEvent() {}, NxToolsContent: { choices: { backgroundResourceChoiceCandidates: async () => { choices++; } } },
    document: { visibilityState: "visible", body: { appendChild() {} }, createElement: () => ({ style: {}, setAttribute() {}, dispatchEvent() {} }) },
    __discoveryFixture: { work, turn: async () => {},
      bg: { eval: () => ({ 7: [...resources.values()] }), chrome: { webRequest: { onResponseStarted: { fire(value) {
        responses.push(value); resources.set(value.url, { url: value.url, mime: value.responseHeaders[0].value });
      } } } } },
      send: async () => ({ currentResources: [...resources.values()] }) }
  });
  const active = await activeWork(page, { step: async (stage, action) => { stages.push(stage); return action(); }, onRound: async raw => rounds.push(raw) });
  assert.deepEqual(rounds.map(row => row.round), Array.from({ length: 24 }, (_, index) => index + 1));
  assert.equal(stages.filter(stage => stage.startsWith("active.round.")).length, 24);
  assert.equal(responses.length, 24 * (7 + 80 + 24));
  assert.equal(new Set(responses.filter(row => row.url.includes("/part")).map(row => row.url)).size, 80);
  assert.equal(responses.filter(row => row.url === "https://cdn.test/part0").length, 24 * 25);
  assert.equal(choices, 24);
  assert.equal(active.discoveryLatenciesMs.length, WORKLOAD.rounds);
  assert.ok(rounds.every(row => row.observations.length > 0 && row.candidates.length > 0));
  assert.equal(WORKLOAD.idleMs, 8100);
});

test("partial output retains completed rounds even if safe snapshot collection fails", async t => {
  const fs = require("node:fs/promises");
  const { createProgress } = await import("./browser-stages.mjs");
  const scratch = await fs.mkdtemp(path.join(process.env.PI_SCRATCH_DIR || require("node:os").tmpdir(), "b2-progress-failure-"));
  t.after(() => fs.rm(scratch, { recursive: true, force: true }));
  const outputPath = path.join(scratch, "partial.json"), failure = new Error("round failed");
  const report = createProgress({ snapshot: async () => { throw new Error("snapshot unavailable"); } }, { outputPath });
  report.data.currentTrial = { group: 1, variant: "baseline", metrics: { start: 1.5 }, rounds: [{ round: 1, observations: ["raw-observation"] }] };
  await assert.rejects(report.step("trial.1.baseline.active.round.2", async () => { throw failure; }), error => error === failure);
  await report.fail(failure);
  const result = JSON.parse(await fs.readFile(outputPath, "utf8"));
  assert.equal(result.currentTrial.rounds[0].observations[0], "raw-observation");
  assert.equal(result.currentTrial.metrics.start, 1.5);
  assert.equal(result.failure.message, "round failed");
  assert.equal(result.failure.snapshotError.message, "snapshot unavailable");
  assert.equal(result.stages.at(-1).status, "failed");
});
