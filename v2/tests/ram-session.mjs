import assert from "node:assert/strict";
import { median, sessionSummary, appSummary, comparable, benchmarkSummary, benchmarkComparable, optimizeWithMeasurements, readRamWithTimeout } from "../src/lib/ram-session.ts";

const sample = (time, available, pss = 20) => ({ hardware_id: "test-tv", user: 0, sampled_at_ms: time,
  reading: { total_mb: 3000, available_mb: available, pressure_some_avg10: null },
  apps: [{ package: "org.example.app", pss_mb: pss }] });
const ready = sample(0, 500);
assert.equal(await readRamWithTimeout(() => Promise.resolve(ready), 100), ready);
await assert.rejects(readRamWithTimeout(() => Promise.reject(new Error("Disconnected")), 100), /Disconnected/);
await assert.rejects(readRamWithTimeout(() => new Promise(() => {}), 5), /RAM read timed out/);
assert.equal(median([null, NaN]), null);
assert.equal(median([40, 10, 20, 30]), 25);
const before = [sample(0, 400), sample(15000, 500, null), sample(30000, 600)];
const after = [sample(40000, 600), sample(55000, 700), sample(70000, 800)];
assert.deepEqual(sessionSummary(before), { count: 3, measured: 3, medianAvailable: 500, lowAvailable: 400, medianPressure: null, seconds: 30 });
assert.equal(appSummary(before, "org.example.app").count, 2);
assert.equal(appSummary(before, "missing").median, null);
assert.ok(comparable(before, after));
assert.equal(comparable(before, after.map((s) => ({ ...s, user: 10 }))), false);
assert.equal(comparable(before, [sample(40000, null), ...after.slice(1)]), false);
const trial = (total) => ({ hardware_id: "test-tv", user: 0, package: "org.example.app", cold: false,
  firmware: "test-build", app_version: "1", timing: { total_ms: total, launch_state: "WARM" }, reading: { available_mb: 500 }, app_pss_mb: 80 });
const b = [trial(100), trial(200), trial(300)], a = [trial(80), trial(160), trial(240)];
assert.deepEqual(benchmarkSummary(b), { medianMs: 200, minMs: 100, maxMs: 300, availableMb: 500, appPssMb: 80 });
assert.ok(benchmarkComparable(b, a));
for (const changes of [{ firmware: "changed" }, { app_version: "2" }, { cold: true }, { hardware_id: "another-tv" }, { user: 10 }, { timing: { total_ms: 20, launch_state: "COLD" } }]) {
  assert.equal(benchmarkComparable(b, [{ ...a[0], ...changes }, ...a.slice(1)]), false);
}
assert.equal(benchmarkComparable(b, a.slice(1)), false);
for (const failure of [null, "baseline", "comparison", "stop", "mismatch"]) {
  const calls = [];
  let stopped = false;
  let count = 0;
  const run = optimizeWithMeasurements({
    trial: async () => {
      calls.push("trial"); count++;
      if ((failure === "baseline" && count === 2) || (failure === "comparison" && count === 4)) throw new Error("Reading failed");
      return { ...trial(100), app_version: failure === "mismatch" && count === 3 ? "2" : "1" };
    },
    restrict: async () => { calls.push("restrict"); return "Verified"; },
    cancelled: () => stopped,
    onPhase: () => {},
    onTrial: (phase) => { if (failure === "stop" && phase === "baseline") stopped = true; },
  });
  if (failure === null) {
    assert.equal(await run, "Verified");
    assert.deepEqual(calls, ["trial", "trial", "trial", "restrict", "trial", "trial", "trial"]);
  } else if (failure === "comparison") {
    await assert.rejects(run, /Restriction applied. Comparison incomplete; Undo is available/);
    assert.equal(calls.filter((call) => call === "restrict").length, 1);
  } else {
    await assert.rejects(run, /No restriction was applied/);
    assert.equal(calls.includes("restrict"), false);
  }
}
console.log("PASS: RAM metrics, automatic before/change/after order, cancellation and failure safety");
