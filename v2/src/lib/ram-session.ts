import type { RamBenchmarkTrial, RamReport } from "./types";

export function median(values: (number | null)[]): number | null {
  const usable = values.filter((n): n is number => n !== null && Number.isFinite(n)).sort((a, b) => a - b);
  if (!usable.length) return null;
  const mid = Math.floor(usable.length / 2);
  return usable.length % 2 ? usable[mid] : (usable[mid - 1] + usable[mid]) / 2;
}

export function sessionSummary(samples: RamReport[]) {
  const available = samples.map((s) => s.reading.available_mb);
  const known = available.filter((n): n is number => n !== null && Number.isFinite(n));
  return {
    count: samples.length,
    measured: known.length,
    medianAvailable: median(available),
    lowAvailable: known.length ? Math.min(...known) : null,
    medianPressure: median(samples.map((s) => s.reading.pressure_some_avg10)),
    seconds: samples.length > 1 ? Math.max(0, (samples.at(-1)!.sampled_at_ms - samples[0].sampled_at_ms) / 1000) : 0,
  };
}

export function appSummary(samples: RamReport[], packageName: string) {
  const values = samples.map((s) => s.apps.find((a) => a.package === packageName)?.pss_mb ?? null);
  const known = values.filter((n): n is number => n !== null && Number.isFinite(n));
  return { count: known.length, median: median(values), peak: known.length ? Math.max(...known) : null };
}

export function comparable(before: RamReport[], after: RamReport[]): boolean {
  const all = [...before, ...after];
  const first = all[0];
  return before.length >= 3 && after.length >= 3 && !!first && all.every((s) =>
    s.hardware_id === first.hardware_id && s.user === first.user &&
    s.reading.total_mb === first.reading.total_mb && s.reading.available_mb !== null);
}

export function benchmarkSummary(trials: RamBenchmarkTrial[]) {
  const times = trials.map((t) => t.timing.total_ms);
  return { medianMs: median(times), minMs: times.length ? Math.min(...times) : null,
    maxMs: times.length ? Math.max(...times) : null,
    availableMb: median(trials.map((t) => t.reading.available_mb)),
    appPssMb: median(trials.map((t) => t.app_pss_mb)) };
}

export function benchmarkComparable(before: RamBenchmarkTrial[], after: RamBenchmarkTrial[]): boolean {
  const first = before[0];
  return before.length === 3 && after.length === 3 && !!first && [...before, ...after].every((t) =>
    t.hardware_id === first.hardware_id && t.user === first.user && t.package === first.package &&
    t.cold === first.cold && t.firmware === first.firmware && t.app_version === first.app_version &&
    t.timing.launch_state === first.timing.launch_state);
}
