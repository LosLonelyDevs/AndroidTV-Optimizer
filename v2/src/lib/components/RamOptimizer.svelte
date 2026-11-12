<script lang="ts">
  import { onDestroy } from "svelte";
  import { api } from "$lib/api";
  import { getKeptPackages, setPackageKept } from "$lib/prefs";
  import { appSummary, comparable, median, readRamWithTimeout, sessionSummary } from "$lib/ram-session";
  import type { RamApp, RamReport, RamBenchmarkTrial } from "$lib/types";
  import HelpDetails from "./HelpDetails.svelte";
  import Icon from "./Icon.svelte";
  import RamBenchmark from "./RamBenchmark.svelte";
  import ReasonButton from "./ReasonButton.svelte";
  import AppSuggestion, { type SuggestionValue } from "./AppSuggestion.svelte";
  import type { SafetyStatus } from "$lib/safety";

  let { serial, hardwareId, active, suggestionFor }: { serial: string; hardwareId: string | null; active: boolean; suggestionFor: (packageName: string, fallback?: SafetyStatus) => SuggestionValue } = $props();
  let report = $state<RamReport | null>(null);
  let busy = $state(false);
  let reading = $state(false);
  let stoppingRead = $state(false);
  let readRequestId: string | null = null;
  let benchmarking = $state(false);
  let benchmarkCold = $state(false);
  let benchmarkProgress = $state("");
  let launchBaseline = $state<RamBenchmarkTrial[]>([]);
  let launchAfter = $state<RamBenchmarkTrial[]>([]);
  let benchmark = $state<{ optimize: (packageName: string, restrict: () => Promise<string>) => Promise<string>; cancel: () => void }>();
  let error = $state("");
  let message = $state("");
  let baseline = $state<RamReport[]>([]);
  let after = $state<RamReport[]>([]);
  let recording = $state<"baseline" | "after" | null>(null);
  let confirmApp = $state<RamApp | null>(null);
  let kept = $state<Set<string>>(new Set());
  let query = $state("");
  let showAll = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let generation = 0;
  const beforeSummary = $derived(sessionSummary(baseline));
  const afterSummary = $derived(sessionSummary(after));
  const comparison = $derived(comparable(baseline, after));
  const delta = $derived(comparison ? afterSummary.medianAvailable! - beforeSummary.medianAvailable! : null);
  const samples = $derived(after.length ? after : baseline);
  const rows = $derived((report?.apps ?? []).filter((a) =>
    (showAll || a.pss_mb !== null || a.can_undo || !a.blocked_reason) &&
    `${a.name} ${a.package}`.toLowerCase().includes(query.toLowerCase())));
  const mb = (n: number | null | undefined) => n == null ? "Unavailable" : `${Math.round(n)} MB`;
  function rowSamples(app: RamApp) {
    const trials = (launchAfter.length ? launchAfter : launchBaseline).filter((trial) => trial.package === app.package);
    if (!trials.length) return appSummary(samples, app.package);
    const values = trials.map((trial) => trial.app_pss_mb).filter((n): n is number => n !== null);
    return { count: values.length, median: median(values), peak: values.length ? Math.max(...values) : null };
  }
  const unavailable = $derived(!hardwareId ? "A verified hardware ID is required. Reconnect the TV and refresh its details."
    : reading ? "Reading RAM and app safety from the TV. No settings are being changed."
    : busy ? "Wait for the current RAM operation to finish."
    : recording ? "Stop RAM recording before starting another action." : null);
  const actionReason = $derived(unavailable ?? (benchmarking ? "Wait for the benchmark to finish, or stop it first." : null));

  function stop() { recording = null; clearTimeout(timer); }
  function discardRead() {
    const requestId = readRequestId;
    readRequestId = null;
    reading = false; stoppingRead = false;
    if (requestId) void api.ramCancelReport(requestId).catch(() => {});
  }
  async function stopReading() {
    stop();
    const requestId = readRequestId;
    if (!requestId || stoppingRead) return;
    stoppingRead = true;
    try { await api.ramCancelReport(requestId); }
    catch (e) { if (readRequestId === requestId) error = `Could not stop the RAM read: ${String(e)}`; }
    finally { if (readRequestId === requestId) stoppingRead = false; }
  }
  $effect(() => {
    void serial; void hardwareId;
    discardRead();
    generation++; stop(); report = null; baseline = []; after = []; confirmApp = null;
    error = ""; message = ""; busy = false; reading = false;
  });
  $effect(() => {
    void serial;
    if (active && hardwareId) kept = getKeptPackages(hardwareId);
    if (!active) { stop(); discardRead(); }
  });
  onDestroy(() => { generation++; stop(); discardRead(); });

  async function refresh(capture: "baseline" | "after" | null = null, preserveMessage = false) {
    if (!active || !hardwareId || busy || reading || benchmarking) return;
    const ticket = generation;
    const expected = hardwareId;
    const requestId = crypto.randomUUID();
    readRequestId = requestId;
    reading = true; stoppingRead = false; error = "";
    if (!preserveMessage) message = "";
    try {
      const next = await readRamWithTimeout(() => api.ramReport(serial, expected, requestId));
      if (ticket !== generation || readRequestId !== requestId) return;
      if (next.hardware_id !== expected) throw new Error("The connected TV changed. Reconnect before continuing.");
      if (report && next.user !== report.user) {
        stop(); baseline = []; after = []; confirmApp = null;
        message = "Android user changed. Session measurements were cleared.";
      } else if (capture && recording === capture) {
        if (capture === "baseline") baseline = [...baseline, next];
        else after = [...after, next];
      }
      report = next;
    } catch (e) {
      if (ticket === generation && readRequestId === requestId) {
        if (String(e).includes("RAM read stopped.")) message = "RAM refresh stopped. No settings were changed.";
        else error = String(e);
        stop();
        if (String(e).includes("RAM read timed out.")) void api.ramCancelReport(requestId).catch(() => {});
      }
    } finally {
      if (ticket === generation && readRequestId === requestId) {
        readRequestId = null; reading = false; stoppingRead = false;
        if (recording) {
          if ((recording === "baseline" ? baseline : after).length >= 12) stop();
          else timer = setTimeout(() => void refresh(recording), 15000);
        }
      }
    }
  }

  function start(kind: "baseline" | "after") {
    stop(); message = "";
    if (kind === "baseline") { baseline = []; after = []; } else after = [];
    recording = kind; void refresh(kind);
  }

  function keep(app: RamApp, value: boolean) {
    if (!hardwareId) return;
    setPackageKept(hardwareId, app.package, value);
    kept = getKeptPackages(hardwareId);
  }

  async function change(app: RamApp, undo = false) {
    if (!hardwareId || !report || busy || reading || benchmarking) return;
    const ticket = generation;
    const identity = hardwareId;
    const user = report.user;
    let failure = "";
    busy = true; stop(); error = ""; message = ""; confirmApp = null;
    try {
      kept = getKeptPackages(identity);
      const transport = serial;
      if (!undo && !benchmark) throw new Error("Benchmark controls are still loading. Try again.");
      const result = undo
        ? await api.ramUndo(transport, identity, user, app.package)
        : await benchmark!.optimize(app.package, () => api.ramRestrict(transport, identity, user, app.package, [...getKeptPackages(identity)]));
      if (ticket === generation) { message = result; after = []; }
    } catch (e) { failure = String(e); }
    finally {
      if (ticket === generation) { busy = false; await refresh(null, true); if (failure) error = failure; }
    }
  }
</script>

<div class="ram-stack" role="tabpanel" id="tabpanel-ram" aria-labelledby="tab-ram" tabindex="0">
  <div class="card">
    <div class="ram-heading">
      <div><h2><Icon name="memory" size={20} /> Reduce background RAM</h2>{#if !reading || recording}<p class="muted">{recording ? `Recording ${recording === "baseline" ? "baseline" : "comparison"} · ${(recording === "baseline" ? baseline : after).length}/12 samples` : report ? "Choose an eligible app. Measurements run automatically." : "Click Refresh RAM to load current RAM usage and app suggestions."}</p>{/if}</div>
      <ReasonButton reason={actionReason} onclick={() => refresh()}>Refresh RAM</ReasonButton>
    </div>
    {#if !hardwareId}<p class="error" role="alert">A verified hardware ID is required.</p>{/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if reading}<div class="ram-progress"><span role="status">{stoppingRead ? "Stopping RAM refresh…" : "Reading RAM and app safety from the TV. No settings are being changed."}</span><button disabled={stoppingRead} onclick={stopReading}>Stop RAM refresh</button></div>{/if}
    {#if message}<p role="status">{message}</p>{/if}
    {#if benchmarking && busy}<div class="ram-progress" role="status"><span>{benchmarkProgress}</span><button onclick={() => benchmark?.cancel()}>Stop optimization</button></div>{/if}
    <div class="ram-metrics">
      <div><span>Available RAM</span><strong>{mb(report?.reading.available_mb)}</strong></div>
      <div><span>File cache</span><strong>{mb(report?.reading.file_cache_mb)}</strong></div>
      <div><span>Swap used</span><strong>{mb(report?.reading.swap_used_mb)}</strong></div>
      <div><span>Memory stalls · 10s</span><strong>{report?.reading.pressure_some_avg10 == null ? "Unavailable" : `${report.reading.pressure_some_avg10.toFixed(1)}%`}</strong></div>
    </div>
    <HelpDetails label="About RAM measurements">
      <p>Refresh RAM reads current RAM and app safety. Opening this tab does not start a read. Stop RAM refresh cancels the read without changing settings. A stalled read also times out so you can retry.</p>
      <p>Cached apps can make switching faster. Available RAM includes memory the kernel estimates it can reclaim. A lower used-RAM number alone does not prove better performance. Swap usage alone does not prove a slowdown.</p>
      <p>Measurements use /proc/meminfo, optional PSI, and PID/UID-verified process PSS. Missing readings stay unavailable. App PSS includes shared memory proportionally; it is not an exact saving if you restrict that app.</p>
      <p>Sessions collect up to 12 samples, 15 seconds apart after each read. Use the same content and app-switching order before and after, and note any reloads or playback problems. Switching tabs stops recording; leaving this device page clears measurements. No raw session data is uploaded.</p>
      {#each report?.notes ?? [] as note}<p>{note}</p>{/each}
    </HelpDetails>
    <HelpDetails label="Record a custom RAM workload">
    <div class="ram-actions">
      <ReasonButton reason={actionReason ?? (!report ? "Refresh RAM successfully before recording a baseline." : null)} onclick={() => start("baseline")}>Record baseline</ReasonButton>
      <ReasonButton reason={actionReason ?? (baseline.length < 3 ? `Record at least three baseline samples first (${baseline.length}/3 collected).` : null)} onclick={() => start("after")}>Record comparison</ReasonButton>
      {#if recording}<button onclick={stopReading}>Stop recording</button>{/if}
    </div>
    </HelpDetails>
    {#if baseline.length || after.length}
      <div class="table-scroll"><table>
        <thead><tr><th>Session</th><th>Samples / span</th><th>Median available</th><th>Lowest available</th></tr></thead>
        <tbody>{#each [["Baseline", beforeSummary], ["Comparison", afterSummary]] as entry}
          {@const summary = entry[1] as ReturnType<typeof sessionSummary>}
          <tr><th>{entry[0] as string}</th><td>{summary.measured}/{summary.count} · {Math.round(summary.seconds)}s</td><td>{mb(summary.medianAvailable)}</td><td>{mb(summary.lowAvailable)}</td></tr>
        {/each}</tbody>
      </table></div>
      <p role="status">{delta === null ? "Collect at least three available-RAM samples in each session on the same TV and Android user." : `${Math.abs(Math.round(delta))} MB ${delta >= 0 ? "more" : "less"} median available RAM in the comparison. This is an observation, not proof that the restriction caused it.`}</p>
    {/if}
  </div>

  <RamBenchmark bind:this={benchmark} {serial} {hardwareId} {report} {active} disabledReason={unavailable} bind:running={benchmarking} bind:cold={benchmarkCold} bind:baseline={launchBaseline} bind:after={launchAfter} bind:progress={benchmarkProgress} />

  {#if confirmApp}
    <div class="card ram-confirm" role="region" aria-label="Confirm background restriction">
      <h3>Restrict {confirmApp.name}?</h3>
      <p>Background downloads, sync, notifications, and playback may stop. You can still open the app. This does not impose a RAM limit or immediately remove cached memory.</p>
      <p>The previous setting will be saved on this computer for Undo.</p>
      <p>Automatically measures three launches of {confirmApp.name}, applies the restriction, then measures three more. This switches the TV between Home and the app and interrupts what is on screen. A failed or stopped baseline applies no restriction.</p>
      {#if benchmarkCold}<p><strong>Cold mode force-stops this app before each trial. Playback, downloads, and unsaved work may stop.</strong></p>{:else}<p>Resume mode pre-opens the app. The TV returns to Home after each trial.</p>{/if}
      <div class="ram-actions"><ReasonButton reason={actionReason} onclick={() => confirmApp && change(confirmApp)}>Measure and restrict</ReasonButton><ReasonButton reason={busy ? "Wait for the current RAM operation to finish." : null} onclick={() => confirmApp = null}>Cancel</ReasonButton></div>
    </div>
  {/if}

  <div class="card">
    <div class="ram-heading"><h3>Apps in RAM</h3><div class="ram-actions"><input aria-label="Search RAM apps" placeholder="Search apps" bind:value={query} /><label><input type="checkbox" bind:checked={showAll} /> Show all apps</label></div></div>
    <HelpDetails label="Which apps can be restricted?">
      <p>Only reviewed safe, enabled, non-system apps with a unique UID are eligible. Home, the foreground app, media sessions, accessibility and foreground services are protected. Unknown safety readings block restrictions. Keep uses the App List's saved choice.</p>
      <p>Restrictions use Android's background AppOp. Firmware may accept the setting without reducing RAM. Undo remains available after a failed write or restart and restores the saved mode, including Default. Other changes to that mode are not overwritten.</p>
      <p>Sampled app memory shows resident PSS, including cached processes. Missing samples do not mean zero usage or a confirmed process exit.</p>
      <p>Suggestion matches Health and the App List. The reason below it tells you whether a background restriction is currently allowed. Restricting limits background activity; it does not disable or uninstall the app. Keep apps whose background playback, downloads, sync, or notifications you need.</p>
    </HelpDetails>
    <div class="table-scroll"><table class="ram-apps">
      <thead><tr><th>App</th><th>RAM</th><th>Suggestion</th><th>Action</th></tr></thead>
      <tbody>{#each rows as app (app.package)}
        {@const stats = rowSamples(app)}
        {@const eligibility = kept.has(app.package) ? "Marked Keep" : app.blocked_reason === "Protected, caution, or unreviewed app" ? app.safety?.reason ?? app.blocked_reason : app.blocked_reason ?? "Eligible to limit background activity"}
        <tr class:ram-eligible={!app.blocked_reason && !kept.has(app.package)}><td><strong>{app.name}</strong><div class="muted package">{app.package}</div>{#if app.can_undo}<span class="ram-saved">Undo available</span>{/if}</td>
          <td>{#if app.pss_mb !== null}<strong class="ram-value">{mb(app.pss_mb)}</strong>{:else}<span class="muted">No RAM reading</span>{/if}
            {#if stats.count}<div class="muted">Median {mb(stats.median)} · Peak {mb(stats.peak)} · {stats.count} readings</div>{/if}
          </td>
          <td class="ram-suggestion"><AppSuggestion value={suggestionFor(app.package, app.safety ? { status: "ready", verdict: app.safety } : undefined)} kept={kept.has(app.package)} /><div class:ram-allowed={!app.blocked_reason && !kept.has(app.package)} class="ram-eligibility">{eligibility}</div></td>
          <td><div class="ram-row-actions"><label><input type="checkbox" aria-label={`Keep ${app.name}`} checked={kept.has(app.package)} disabled={busy} onchange={(event) => keep(app, event.currentTarget.checked)} /> Keep</label>
            {#if app.can_undo}<ReasonButton reason={actionReason} onclick={() => change(app, true)} label={`Undo restriction for ${app.name}`}>Undo restriction</ReasonButton>
            {:else}<ReasonButton reason={actionReason ?? (kept.has(app.package) ? "This app is marked Keep. Uncheck Keep before restricting it." : app.blocked_reason)} onclick={() => confirmApp = app} label={`Restrict ${app.name}`}>Restrict background</ReasonButton>{/if}
          </div></td></tr>
      {:else}<tr><td colspan="4">{reading ? "Reading RAM…" : report ? "No matching apps with verified readings." : "No RAM reading yet."}</td></tr>{/each}</tbody>
    </table></div>
  </div>
</div>

<style>
  .ram-stack { display: grid; grid-template-columns: minmax(0, 1fr); min-width: 0; gap: 1rem; }
  .ram-heading { display: flex; align-items: center; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
  .ram-heading h2, .ram-heading h3 { margin: 0; }
  .ram-heading p { margin-bottom: 0; }
  .ram-metrics { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 10rem), 1fr)); gap: 1rem; margin: 1.5rem 0; }
  .ram-metrics span, .ram-metrics strong { display: block; }
  .ram-metrics strong { margin-top: 0.35rem; font-size: 1.15rem; }
  .ram-actions, .ram-row-actions { display: flex; align-items: center; gap: 0.65rem; flex-wrap: wrap; }
  .ram-actions label, .ram-row-actions label { display: inline-flex; align-items: center; gap: 0.4rem; }
  .table-scroll { overflow-x: auto; margin-top: 1rem; }
  table { width: 100%; border-collapse: collapse; }
  th, td { text-align: left; padding: 0.75rem; vertical-align: middle; }
  tbody tr { border-top: 1px solid var(--border); }
  .package { overflow-wrap: anywhere; }
  .ram-apps td:first-child { min-width: 180px; max-width: 340px; }
  .ram-apps td:nth-child(2) { min-width: 150px; }
  .ram-apps th { color: var(--accent); background: var(--bg-inset); }
  .ram-apps tr:nth-child(even) { background: color-mix(in srgb, var(--accent) 3%, var(--bg-surface)); }
  .ram-apps tr.ram-eligible { box-shadow: inset 3px 0 var(--ok); }
  .ram-value { color: var(--accent); }
  .ram-suggestion { min-width: 180px; max-width: 300px; }
  .ram-eligibility { margin-top: 0.4rem; color: var(--fg-muted); }
  .ram-allowed { color: var(--ok); }
  .ram-saved { display: inline-block; margin-top: 0.4rem; padding: 0.15rem 0.5rem; border-radius: var(--radius-pill); color: var(--accent); background: var(--bg-inset); }
  .ram-progress { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; flex-wrap: wrap; padding: 0.75rem; border: 1px solid var(--accent); border-radius: var(--radius-md); }
  .ram-apps td:last-child { min-width: 170px; }
  .ram-confirm { border: 1px solid var(--accent); }
  input:not([type]) { max-width: 100%; }
  button:focus-visible, input:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
  @media (max-width: 800px) { .ram-heading { align-items: flex-start; } .ram-actions { width: 100%; } }
</style>
