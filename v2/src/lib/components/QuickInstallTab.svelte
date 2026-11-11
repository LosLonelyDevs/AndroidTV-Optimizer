<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import type { QuickAppRow, ShizukuStatus } from "$lib/types";
  let { serial }: { serial: string } = $props();
  let quickApps = $state<QuickAppRow[]>([]);
  let quickBusy = $state<string | null>(null);
  let quickMsg = $state("");
  let quickMsgPkg = $state<string | null>(null);
  let quickOk = $state(false);

  let shizukuBusy = $state(false);
  let shizukuMsg = $state("");
  let shizukuOk = $state(false);
  // Live install/running state for the status dot. null until the first probe
  // (or if a probe fails) — rendered as "unknown" rather than a false negative.
  let shizukuStatus = $state<ShizukuStatus | null>(null);
  let shizukuCheckBusy = $state(false);
  let shizukuStatusLabel = $derived(
    shizukuStatus === null
      ? shizukuCheckBusy
        ? "Checking…"
        : "Status unknown"
      : shizukuStatus.running
        ? "Service running"
        : shizukuStatus.installed
          ? "Installed · service stopped"
          : "Not installed",
  );
  async function loadQuickApps() {
    try {
      quickApps = await api.listQuickApps(serial);
    } catch {
      // Non-fatal — the rest of the tab still works without the catalog.
    }
  }

  async function installQuick(pkg: string) {
    quickBusy = pkg;
    quickMsgPkg = pkg;
    quickMsg = "";
    quickOk = false;
    try {
      const r = await api.installQuickApp(serial, pkg);
      quickOk = r.ok;
      quickMsg = r.ok ? "Installed." : r.message;
      if (r.ok) await loadQuickApps();
    } catch (e) {
      quickMsg = String(e);
    } finally {
      quickBusy = null;
    }
  }

  async function setupShizuku() {
    shizukuBusy = true;
    shizukuMsg = "";
    shizukuOk = false;
    try {
      const r = await api.setupShizuku(serial);
      shizukuOk = r.ok;
      shizukuMsg = r.message;
      await loadQuickApps();
      await checkShizuku();
    } catch (e) {
      shizukuMsg = String(e);
    } finally {
      shizukuBusy = false;
    }
  }

  /// Probe whether Shizuku is installed and its service is live — drives the
  /// status dot. Best-effort: a failed probe leaves the dot in the "unknown"
  /// state rather than reporting a false "stopped".
  async function checkShizuku() {
    shizukuCheckBusy = true;
    try {
      shizukuStatus = await api.shizukuStatus(serial);
    } catch {
      shizukuStatus = null;
    } finally {
      shizukuCheckBusy = false;
    }
  }

  onMount(() => { loadQuickApps(); checkShizuku(); });
</script>
<div class="quick-install">
<div class="card section-card">
  <div class="shizuku-row">
    <div>
      <div class="shizuku-header">
        <h2>Shizuku</h2>
        <span
          class="status-dot"
          class:running={shizukuStatus?.running}
          class:stopped={!!shizukuStatus && shizukuStatus.installed && !shizukuStatus.running}
          class:checking={shizukuCheckBusy}
          aria-hidden="true"
        ></span>
        <span class="shizuku-status muted small">{shizukuStatusLabel}</span>
        <button
          class="icon-check"
          onclick={checkShizuku}
          disabled={shizukuCheckBusy}
          title="Check whether Shizuku's service is running"
          aria-label="Check Shizuku status"
        >
          {shizukuCheckBusy ? "…" : "⟳"}
        </button>
      </div>
      <div class="muted small">
        Installs Shizuku if it's missing, then starts its service so other apps can use
        elevated ADB permissions — no root, and no pairing code (Android TV never shows
        one). The service stops on reboot; press this again to start it back up.
      </div>
      {#if shizukuMsg}
        <div class="install-result" class:ok={shizukuOk} class:bad={!shizukuOk}>
          <span>{shizukuOk ? "✓" : "✕"} {shizukuMsg}</span>
        </div>
      {/if}
    </div>
    <button class="primary" onclick={setupShizuku} disabled={shizukuBusy}>
      {shizukuBusy ? "Starting…" : "Install / Start"}
    </button>
  </div>
</div>

<div class="card section-card">
  <h2>Quick install</h2>
  <p class="muted small">
    We fetch the build that matches your device's CPU from the official source and
    install it. This briefly turns off Play Protect (which flags these apps) and
    restores it afterward.
  </p>
  {#if quickApps.length > 0}
    <ul class="catalog-list">
      {#each quickApps as q (q.package)}
        <li>
          <div>
            <div class="apk-name">{q.name}</div>
            <div class="muted small">{q.description}</div>
            <div class="muted small mono">
              {q.package}
              {#if q.installed}<span class="tag installed">INSTALLED</span>{/if}
            </div>
            {#if quickMsgPkg === q.package && quickMsg}
              <div class="install-result" class:ok={quickOk} class:bad={!quickOk}>
                <span>{quickOk ? "✓" : "✕"} {quickMsg}</span>
              </div>
            {/if}
          </div>
          <button
            class="small-action primary"
            onclick={() => installQuick(q.package)}
            disabled={quickBusy !== null}
          >
            {quickBusy === q.package ? "Installing…" : q.installed ? "Reinstall" : "Install"}
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted small">Catalog not loaded — check the connection and reopen this tab.</p>
  {/if}
</div>
</div>
<style>
  .section-card { margin-top: 1rem; }
  .shizuku-row, .shizuku-header, .catalog-list li { display: flex; gap: 1rem; align-items: center; justify-content: space-between; }
  .catalog-list { padding: 0; list-style: none; }
  .catalog-list li { padding: 0.75rem 0; border-bottom: 1px solid var(--border); }
</style>
