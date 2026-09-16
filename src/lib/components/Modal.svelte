<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  let { title, children, onclose }: { title: string; children: Snippet; onclose: () => void } = $props();
  let dialog: HTMLDialogElement;
  onMount(() => {
    dialog.showModal();
    dialog.querySelector<HTMLElement>("input, select, textarea, button:not(.close)")?.focus();
    return () => dialog.close();
  });
</script>
<dialog bind:this={dialog} aria-label={title} oncancel={(event) => { event.preventDefault(); onclose(); }}>
  <header><h2>{title}</h2><button type="button" class="close quiet" aria-label="Close dialog" onclick={onclose}><Icon name="close" size={17} /></button></header>
  {@render children()}
</dialog>
<style>
dialog { width: min(470px, calc(100vw - 32px)); max-height: calc(100dvh - 48px); overflow-y: auto; border: 1px solid var(--line); border-radius: 12px; padding: 24px; background: var(--surface); color: var(--text); box-shadow: var(--shadow); }
  dialog::backdrop { background: #17293770; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
  h2 { font-size: 20px; letter-spacing: -.6px; margin: 0; }
  .close { width: 28px; min-height: 28px; padding: 0; }
  dialog :global(form label) { font-size: 12px; color: var(--muted); }
  dialog :global(form input), dialog :global(form select) { margin-top: 5px; color: var(--text); }
</style>
