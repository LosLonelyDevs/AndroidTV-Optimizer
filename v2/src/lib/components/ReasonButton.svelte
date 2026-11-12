<script lang="ts">
  import { tick, type Snippet } from "svelte";

  let { reason = null, onclick, label, children }: {
    reason?: string | null;
    onclick: () => void;
    label?: string;
    children: Snippet;
  } = $props();
  const id = $props.id();
  let button: HTMLButtonElement;
  let hint = $state<HTMLDivElement>();
  let hovered = $state(false);
  let focused = $state(false);
  let dismissed = $state(false);
  const visible = $derived(!!reason && (hovered || focused) && !dismissed);

  $effect(() => {
    void reason;
    const element = hint;
    if (!element) return;
    if (!visible) { element.hidePopover?.(); return; }
    let cancelled = false;
    void tick().then(() => {
      if (cancelled) return;
      element.showPopover?.();
      const anchor = button.getBoundingClientRect();
      const box = element.getBoundingClientRect();
      element.style.left = `${Math.max(8, Math.min(anchor.left, window.innerWidth - box.width - 8))}px`;
      element.style.top = `${Math.max(8, anchor.bottom + box.height <= window.innerHeight - 8 ? anchor.bottom : anchor.top - box.height)}px`;
    });
    return () => { cancelled = true; element.hidePopover?.(); };
  });

  function activate() {
    if (reason) { button.focus(); focused = true; dismissed = false; return; }
    onclick();
  }
</script>

<svelte:window onkeydown={(event) => { if (event.key === "Escape") dismissed = true; }} onresize={() => dismissed = true} />
<div class="reason-button" role="group" onpointerenter={() => { hovered = true; dismissed = false; }} onpointerleave={() => hovered = false}>
  <button type="button" bind:this={button} aria-label={label} aria-disabled={reason ? "true" : undefined}
    aria-describedby={reason ? `${id}-reason` : undefined} onclick={activate}
    onfocus={() => { focused = true; dismissed = false; }} onblur={() => focused = false}>
    {@render children()}
  </button>
  <div bind:this={hint} id={`${id}-reason`} role="tooltip" popover="manual" data-visible={visible}>{reason}</div>
</div>

<style>
  .reason-button { display: inline-flex; max-width: 100%; }
  button { max-width: 100%; }
  button[aria-disabled="true"]:hover { background: var(--bg-inset); }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
  [popover] {
    display: none;
    position: fixed;
    inset: auto;
    margin: 0;
    box-sizing: border-box;
    width: max-content;
    max-width: min(24rem, calc(100vw - 16px));
    max-height: calc(100vh - 16px);
    overflow: auto;
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    background: var(--bg-surface);
    color: var(--fg-primary);
    box-shadow: 0 4px 16px #0004;
    font-size: 1rem;
    line-height: 1.5;
    white-space: normal;
    overflow-wrap: anywhere;
  }
  [popover][data-visible="true"] { display: block; }
</style>
