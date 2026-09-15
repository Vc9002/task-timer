<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  let { title, children, onclose }: { title: string; children: Snippet; onclose: () => void } = $props();
  let dialog: HTMLDialogElement;
  onMount(() => { dialog.showModal(); return () => dialog.close(); });
</script>
<dialog bind:this={dialog} aria-label={title} oncancel={(event) => { event.preventDefault(); onclose(); }}>
  <h2>{title}</h2>
  {@render children()}
</dialog>
<style>
  dialog { max-width: min(480px, 85vw); border: 1px solid #aaa; border-radius: 10px; padding: 1.5rem; color: inherit; background: #fafafa; }
  dialog::backdrop { background: #0007; }
  h2 { margin-top: 0; font-size: 1.2rem; }
  @media (prefers-color-scheme: dark) { dialog { background: #292929; } }
</style>
