// Test-only supplied-Page progress. No browser/TaskSpace lifecycle ownership.
import fs from "node:fs/promises";

export function createProgress(page, { outputPath, onStage } = {}, initial = {}) {
  const data = { ...initial, status: "running", stage: "initializing", stages: [], trials: [] };
  const save = async () => {
    if (outputPath) await fs.writeFile(outputPath, JSON.stringify(data, null, 2));
  };
  const step = async (stage, action) => {
    data.stage = stage;
    const entry = { stage, status: "running" }, started = Date.now();
    data.stages.push(entry);
    await save();
    console.log(`[B2 browser] ${stage}`);
    if (onStage) await onStage({ stage });
    try {
      const result = await action();
      entry.status = "completed";
      entry.elapsedMs = Date.now() - started;
      await save();
      return result;
    } catch (error) {
      entry.status = "failed";
      entry.elapsedMs = Date.now() - started;
      throw error;
    }
  };
  const fail = async error => {
    data.status = "failed";
    data.failure = { name: error.name, message: error.message, stage: data.stage,
      executionStopped: error.executionStopped ?? null, mayHaveLateEffects: error.mayHaveLateEffects ?? null,
      pendingJob: error.pendingJob ?? null };
    // Read-only observation only. Never reload a failed or still-pending fixture.
    try { data.failure.snapshot = await page.snapshot(); }
    catch (snapshotError) { data.failure.snapshotError = { name: snapshotError.name, message: snapshotError.message }; }
    try { await save(); }
    catch (writeError) { console.error("B2 partial-output write failed:", writeError.message); }
  };
  return { data, save, step, fail };
}

export async function runPageJob(page, stage, fn, argument = null, timeout = 10000) {
  // The browser owns one named async job. evaluate itself never waits for it.
  await page.evaluate(({ stage, source, argument }) => {
    if (window.__browserJob?.status === "pending") throw new Error("previous fixture job is still pending");
    const job = window.__browserJob = { stage, status: "pending" };
    Promise.resolve().then(() => (0, eval)(`(${source})`)(argument)).then(
      value => { job.value = value ?? null; job.status = "completed"; },
      error => { job.error = { name: error.name, message: error.message }; job.status = "failed"; });
  }, { stage, source: fn.toString(), argument });
  try {
    await page.waitForFunction(() => window.__browserJob.status !== "pending", undefined, { timeout });
  } catch (error) {
    error.pendingJob = stage;
    error.mayHaveLateEffects = true;
    // The asynchronous fixture operation may still finish; no next action is safe.
    throw error;
  }
  const job = await page.evaluate(() => window.__browserJob);
  if (job.status === "failed") throw Object.assign(new Error(job.error.message), { name: job.error.name, fixtureStage: stage });
  return job.value;
}
