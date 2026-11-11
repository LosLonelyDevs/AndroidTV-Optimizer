<script lang="ts">
  import { onMount } from "svelte";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { api } from "$lib/api";
  import { confirmAction } from "$lib/confirm";
  import { formatBytes as fmtBytes } from "$lib/format";
  import type { StorageUsage, HealthReport } from "$lib/types";

  let { serial }: { serial: string } = $props();
  let busy = $state(false);
  let recording = $state(false);
  let message = $state("");
  let recordingPath = $state<string | null>(null);
  let path = $state("/sdcard");
  let usage = $state<StorageUsage | null>(null);
  let report = $state<HealthReport | null>(null);
  const categories = $derived(report ? [
    ["Apps", report.disk.app_bytes],
    ["App data", report.disk.app_data_bytes === null ? null : Math.max(0, report.disk.app_data_bytes - (report.disk.app_cache_bytes ?? 0))],
    ["Cache", report.disk.app_cache_bytes],
    ["Videos", report.disk.videos_bytes],
    ["Photos", report.disk.photos_bytes],
    ["Audio", report.disk.audio_bytes],
    ["Downloads", report.disk.downloads_bytes],
    ["Other", report.disk.other_bytes],
    ["Free", report.disk.data_free_bytes],
  ] as const : []);
  async function loadStorage() {
    try { report = await api.healthReport(serial); }
    catch (error) { message = String(error); }
  }
  onMount(loadStorage);
  async function managePackage(pkg: string, action: "cache" | "data" | "uninstall") {
    const description = action === "data" ? "Clear app data, including accounts, settings and downloads" : action === "uninstall" ? "Uninstall this app" : "Clear cached files";
    if (!await confirmAction(`${description} for ${pkg}?${action === "data" ? " This cannot be undone." : ""}`)) return;
    busy = true;
    try {
      const result = await (action === "cache" ? api.clearAppCache(serial, pkg) : action === "data" ? api.clearAppData(serial, pkg) : api.uninstallPackage(serial, pkg));
      message = result.message;
      if (result.ok) await loadStorage();
    } catch (error) { message = String(error); }
    finally { busy = false; }
  }

  async function toggleRecording() {
    busy = true;
    try {
      if (recording) {
        const result = await api.stopRecording(serial);
        recording = false;
        recordingPath = result.path;
        message = result.message;
      } else {
        const result = await api.startRecording(serial);
        recording = result.ok;
        recordingPath = null;
        message = result.message;
      }
    } catch (error) { message = String(error); }
    finally { busy = false; }
  }
  async function mirror() {
    busy = true;
    try { message = (await api.mirrorScreen(serial)).message; }
    catch (error) { message = String(error); }
    finally { busy = false; }
  }
  async function scan(nextPath = path) {
    busy = true;
    message = "";
    try { usage = await api.storageUsage(serial, nextPath); path = usage.path; }
    catch (error) { message = String(error); }
    finally { busy = false; }
  }
  async function remove(target: string) {
    if (!await confirmAction(`Delete ${target} permanently from the device?`)) return;
    busy = true;
    try {
      const result = await api.deletePath(serial, target);
      if (result.ok) await scan();
      message = result.message;
    } catch (error) { message = String(error); }
    finally { busy = false; }
  }
</script>

<div class="card">
  <h2>Recording & mirroring</h2>
  <div class="controls">
    <button onclick={toggleRecording} disabled={busy}>{recording ? "Stop & save recording" : "Record screen"}</button>
    <button onclick={mirror} disabled={busy}>Mirror with scrcpy</button>
    {#if recordingPath}<button onclick={() => revealItemInDir(recordingPath!)}>Show recording file</button>{/if}
  </div>
  <p class="muted">Recording requires the Google platform-tools ADB engine and saves to this computer. Clips stop after about three minutes; protected video may appear black. Mirroring uses scrcpy installed on this computer.</p>
</div>
<div class="card storage">
  <h2>Storage by category</h2>
  <button onclick={loadStorage} disabled={busy}>Refresh storage report</button>
  {#if report}
    <div class="categories">
      {#each categories as [label, bytes]}
        <div><strong>{label}</strong><span>{bytes === null ? "Unavailable" : fmtBytes(bytes)}</span></div>
      {/each}
    </div>
    <h3>Storage by app</h3>
    <div class="table-scroll">
      <table>
        <thead><tr><th>Package</th><th>App</th><th>Data</th><th>Cache</th><th>Total</th><th>Manage</th></tr></thead>
        <tbody>
          {#each report.disk.packages as item (item.package)}
            <tr>
              <td>{item.package}</td><td>{fmtBytes(item.app_bytes)}</td><td>{fmtBytes(item.data_bytes)}</td><td>{fmtBytes(item.cache_bytes)}</td><td>{fmtBytes(item.total_bytes)}</td>
              <td class="controls">
                <button disabled={busy} onclick={() => managePackage(item.package, "cache")}>Clear cache</button>
                <button disabled={busy} onclick={() => managePackage(item.package, "data")}>Clear data</button>
                <button class="danger" disabled={busy} onclick={() => managePackage(item.package, "uninstall")}>Uninstall</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
<div class="card storage">
  <h2>Storage explorer</h2>
  <div class="controls">
    <input aria-label="Device storage path" bind:value={path} />
    <button onclick={() => scan()} disabled={busy}>{busy ? "Working…" : "Scan storage"}</button>
    <button onclick={() => scan(path.replace(/\/[^/]+\/?$/, "") || "/sdcard")} disabled={busy || path === "/sdcard"}>Parent folder</button>
  </div>
  {#if usage}
    <p>{fmtBytes(usage.total_bytes)} under {usage.path}</p>
    {#each [{ title: "Folders", entries: usage.children }, { title: "Largest files", entries: usage.largest_files }] as group}
      <h3>{group.title}</h3>
      <ul>
        {#each group.entries as entry (entry.path)}
          <li>
            <button onclick={() => scan(entry.path)} disabled={busy}>{entry.path}</button>
            <span>{fmtBytes(entry.bytes)}</span>
            <button class="danger" onclick={() => remove(entry.path)} disabled={busy}>Delete</button>
          </li>
        {/each}
      </ul>
    {/each}
  {/if}
</div>
{#if message}<p role="status">{message}</p>{/if}

<style>
  .controls, li { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }
  .storage { margin-top: 1rem; }
  input { flex: 1; }
  ul { padding: 0; list-style: none; }
  li { margin: 0.5rem 0; overflow-wrap: anywhere; }
  .categories { display: flex; gap: 1rem; flex-wrap: wrap; margin: 1rem 0; }
  .categories span { display: block; }
  .table-scroll { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; }
  td, th { padding: 0.5rem; text-align: left; overflow-wrap: anywhere; }
</style>
