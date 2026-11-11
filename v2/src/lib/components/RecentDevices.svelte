<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Device } from "$lib/types";

  type Recent = { address: string; name: string; last_seen_at: number };
  const key = "shieldopt.recentNetworkDevices";
  let { devices, disabled = false, onConnect }: {
    devices: Device[];
    disabled?: boolean;
    onConnect: (address: string) => Promise<void>;
  } = $props();
  let recent = $state<Recent[]>([]);
  let ready = $state(false);
  let busy = $state<string | null>(null);
  let message = $state("");
  let missing = $derived(recent.filter(item => !devices.some(device => device.serial === item.address)));
  function save() {
    try { localStorage.setItem(key, JSON.stringify(recent)); } catch { /* Storage may be unavailable. */ }
  }
  onMount(() => {
    try {
      const value: unknown = JSON.parse(localStorage.getItem(key) ?? "[]");
      if (Array.isArray(value)) recent = value.filter((item): item is Recent =>
        !!item && typeof item.address === "string" && typeof item.name === "string" &&
        typeof item.last_seen_at === "number").slice(0, 8);
    } catch { recent = []; }
    ready = true;
  });
  $effect(() => {
    if (!ready) return;
    const connected = devices.filter(device => device.status === "device" && device.serial.includes(":"));
    untrack(() => {
      const entries = connected.map(device => ({ address: device.serial, name: device.name, last_seen_at: Date.now() }));
      recent = [...entries, ...recent].filter((item, index, all) => all.findIndex(entry => entry.address === item.address) === index).slice(0, 8);
      save();
    });
  });
  function clear(item: Recent) {
    recent = recent.filter(entry => entry.address !== item.address);
    save();
    try { if (localStorage.getItem("shieldopt.lastDeviceSerial") === item.address) localStorage.removeItem("shieldopt.lastDeviceSerial"); } catch { /* Storage may be unavailable. */ }
    message = `Cleared ${item.name} from recent devices.`;
  }
  async function connect(item: Recent) {
    busy = item.address;
    try { await onConnect(item.address); }
    catch (error) { message = String(error); }
    finally { busy = null; }
  }
  export async function reconnect() {
    for (const item of missing) await connect(item);
  }
</script>

{#if missing.length}
  <section class="card recent-devices">
    <h2>Recent devices</h2>
    <p class="muted">Saved addresses are tried before a LAN scan. A saved name describes the last connection; the device must identify itself again.</p>
    <ul>
      {#each missing as item (item.address)}
        <li data-recent-address={item.address}>
          <span>{item.name} <small class="muted">{item.address}</small></span>
          <button disabled={disabled || busy !== null} onclick={() => connect(item)}>{busy === item.address ? "Connecting…" : "Connect"}</button>
          <button disabled={disabled || busy !== null} aria-label={`Clear ${item.name} from recent devices`} onclick={() => clear(item)}>Clear</button>
        </li>
      {/each}
    </ul>
  </section>
{/if}
{#if message}<p role="status">{message}</p>{/if}

<style>
  .recent-devices { margin-top: 1rem; }
  ul { list-style: none; padding: 0; }
  li { display: flex; gap: 0.5rem; align-items: center; margin: 0.75rem 0; }
  li span { flex: 1; overflow-wrap: anywhere; }
</style>
