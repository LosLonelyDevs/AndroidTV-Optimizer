<script lang="ts">
  import { onDestroy } from "svelte";
  import { api } from "$lib/api";
  import type { RamBenchmarkTrial, RamReport } from "$lib/types";
  import { benchmarkComparable, benchmarkSummary } from "$lib/ram-session";
  import HelpDetails from "./HelpDetails.svelte";
  import ReasonButton from "./ReasonButton.svelte";

  let { serial, hardwareId, report, disabledReason, active, running = $bindable(false) }: {
    serial: string; hardwareId: string | null; report: RamReport | null; disabledReason: string | null; active: boolean; running?: boolean;
  } = $props();
  let packageName = $state("");
  let cold = $state(false);
  let baseline = $state<RamBenchmarkTrial[]>([]);
  let after = $state<RamBenchmarkTrial[]>([]);
  let pending = $state<"baseline" | "after" | null>(null);
  let error = $state("");
  let progress = $state("");
  let cancelled = false;
  let generation = 0;
  const candidates = $derived((report?.apps ?? []).filter((a) => !a.blocked_reason || a.blocked_reason === "Foreground app"));
  const unavailable = $derived(disabledReason ?? (running ? "Wait for the current benchmark to finish, or stop it first."
    : !hardwareId ? "A verified hardware ID is required. Reconnect the TV and refresh its details."
    : !report ? "Refresh RAM successfully before benchmarking." : null));
  const baselineReason = $derived(unavailable ?? (!candidates.length ? "No eligible reviewed apps are available. Check the Resident apps list for each app's restriction reason."
    : !candidates.some((app) => app.package === packageName) ? "Choose an eligible app from the App menu first." : null));
  const before = $derived(benchmarkSummary(baseline));
  const result = $derived(benchmarkSummary(after));
  const matches = $derived(benchmarkComparable(baseline, after));
  const launchDelta = $derived(matches ? result.medianMs! - before.medianMs! : null);
  const overlap = $derived(matches && result.minMs! <= before.maxMs! && before.minMs! <= result.maxMs!);
  const number = (value: number | null, unit: string) => value === null ? "Unavailable" : `${Math.round(value)} ${unit}`;
  $effect(() => { void serial; void hardwareId; generation++; cancelled = true; baseline = []; after = []; pending = null; running = false; progress = ""; error = ""; });
  $effect(() => { if (!active) { cancelled = true; pending = null; } });
  onDestroy(() => { cancelled = true; generation++; });

  async function run() {
    if (!pending || !hardwareId || !report || unavailable) return;
    const phase = pending;
    const selected = phase === "after" ? baseline[0]?.package : packageName;
    const mode = phase === "after" ? baseline[0]?.cold : cold;
    if (!selected || mode === undefined) return;
    const identity = hardwareId, user = report.user, ticket = generation;
    pending = null; running = true; cancelled = false; error = "";
    if (phase === "baseline") { baseline = []; after = []; } else after = [];
    try {
      for (let i = 0; i < 3 && !cancelled; i++) {
        progress = `${phase === "baseline" ? "Baseline" : "Comparison"}: trial ${i + 1} of 3`;
        const trial = await api.ramBenchmarkTrial(serial, identity, user, selected, mode);
        if (ticket !== generation || cancelled) return;
        if (phase === "baseline") baseline = [...baseline, trial]; else after = [...after, trial];
      }
      progress = cancelled ? "Benchmark stopped; incomplete runs are not compared." : "Three trials complete.";
    } catch (e) { if (ticket === generation) error = String(e); }
    finally { if (ticket === generation) { running = false; if (cancelled) progress = "Benchmark stopped; incomplete runs are not compared."; } }
  }
</script>

<div class="card benchmark">
  <h3>Before / after benchmark</h3>
  <div class="benchmark-controls">
    <label>App<select aria-label="Benchmark app" bind:value={packageName} disabled={!!unavailable}>
      <option value="">Choose a reviewed app</option>{#each candidates as app}<option value={app.package}>{app.name}</option>{/each}
    </select></label>
    <label>Launch mode<select aria-label="Benchmark launch mode" bind:value={cold} disabled={!!unavailable}><option value={false}>Resume (pre-open)</option><option value={true}>Cold (force-stop)</option></select></label>
    <div class="benchmark-actions"><ReasonButton reason={baselineReason} onclick={() => pending = "baseline"}>Benchmark baseline</ReasonButton>
      <ReasonButton reason={unavailable ?? (baseline.length !== 3 ? `Complete all three baseline trials first (${baseline.length}/3 completed).` : null)} onclick={() => pending = "after"}>Benchmark comparison</ReasonButton>
      {#if running}<button onclick={() => { cancelled = true; progress = "Stopping after the current trial…"; }}>Stop benchmark</button>{/if}</div>
  </div>
  <HelpDetails label="How the benchmark works">
    <p>Three launches per run, with Home between trials and a three-second settling period before RAM measurements. Resume mode pre-opens the app; Android reports the actual launch state when available. Cold mode force-stops only the chosen app. Keep the same content, app settings, and temperature for both runs.</p>
    <p>Comparison reuses the baseline app and mode. Firmware, app version, Android user, hardware identity, and reported launch state must match. Results show Activity Manager TotalTime, not time to a decoded video frame or fully loaded content. Three trials are a quick check; overlapping ranges are inconclusive.</p>
    <p>Stop takes effect after the current trial. The TV stays on the last launched app. Nothing is installed, cleared, or disabled by this benchmark.</p>
  </HelpDetails>
  {#if pending}
    <div class="benchmark-confirm" role="region" aria-label="Confirm benchmark">
      <p>This switches the TV between Home and <strong>{pending === "after" ? baseline[0]?.package : packageName}</strong> three times and interrupts what is on screen.</p>
      {#if pending === "after" ? baseline[0]?.cold : cold}<p><strong>Cold mode force-stops this app before each trial. Its playback, downloads, and unsaved work may stop.</strong></p>{/if}
      <div class="benchmark-actions"><ReasonButton reason={unavailable} onclick={run}>Start three trials</ReasonButton><button onclick={() => pending = null}>Cancel benchmark</button></div>
    </div>
  {/if}
  {#if progress}<p role="status">{progress}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if baseline.length || after.length}
    <p class="muted">{baseline[0]?.package ?? after[0]?.package} · Android reports {baseline[0]?.timing.launch_state ?? "unknown launch state"}</p>
    <div class="benchmark-table"><table>
      <thead><tr><th>Metric</th><th>Baseline ({baseline.length}/3)</th><th>Comparison ({after.length}/3)</th><th>Delta</th></tr></thead>
      <tbody>
        <tr><th>Median launch</th><td>{number(before.medianMs, "ms")}</td><td>{number(result.medianMs, "ms")}</td><td>{launchDelta === null ? "Not comparable" : `${Math.abs(Math.round(launchDelta))} ms ${launchDelta <= 0 ? "faster" : "slower"}${before.medianMs! > 0 ? ` (${Math.abs(launchDelta / before.medianMs! * 100).toFixed(1)}%)` : ""}`}</td></tr>
        <tr><th>Launch range</th><td>{number(before.minMs, "ms")} – {number(before.maxMs, "ms")}</td><td>{number(result.minMs, "ms")} – {number(result.maxMs, "ms")}</td><td>{overlap ? "Overlapping ranges" : "—"}</td></tr>
        <tr><th>Median available RAM</th><td>{number(before.availableMb, "MB")}</td><td>{number(result.availableMb, "MB")}</td><td>{matches && before.availableMb !== null && result.availableMb !== null ? number(result.availableMb - before.availableMb, "MB") : "Unavailable"}</td></tr>
        <tr><th>Median app PSS</th><td>{number(before.appPssMb, "MB")}</td><td>{number(result.appPssMb, "MB")}</td><td>{matches && before.appPssMb !== null && result.appPssMb !== null ? number(result.appPssMb - before.appPssMb, "MB") : "Unavailable"}</td></tr>
      </tbody>
    </table></div>
    {#if baseline.length === 3 && after.length === 3}<p>{!matches ? "Runs do not match. Check the app version, firmware, user, mode, and reported launch state; record a new baseline." : overlap ? "Launch-time ranges overlap. No clear speed improvement demonstrated." : "Observed delta only. Repeat the same workload to check whether the change persists."}</p>{/if}
  {/if}
</div>

<style>
  h3 { margin-top: 0; }
  .benchmark-controls, .benchmark-actions { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }
  .benchmark-controls > label { display: grid; gap: 0.4rem; }
  select { max-width: 100%; }
  .benchmark-table { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; }
  th, td { text-align: left; vertical-align: middle; padding: 0.75rem; }
  tbody tr { border-top: 1px solid var(--border); }
  .benchmark-confirm { border: 1px solid var(--accent); padding: 1rem; margin-top: 1rem; border-radius: 0.5rem; }
  button:focus-visible, select:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
</style>
