<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { timerStore } from "$lib/stores/timer.svelte";
  import { formatHms } from "$lib/format";

  let { children } = $props();

  onMount(() => {
    timerStore.refresh();
  });

  const links = [
    { href: "/", label: "Today" },
    { href: "/classes", label: "Classes" },
    { href: "/history", label: "History" },
    { href: "/analytics", label: "Analytics" },
    { href: "/settings", label: "Settings" },
  ];

  async function togglePause() {
    if (!timerStore.active) return;
    if (timerStore.active.is_paused) {
      await timerStore.resume();
    } else {
      await timerStore.pause();
    }
  }

  async function finish() {
    await timerStore.finish();
  }
</script>

<div class="shell">
  <nav>
    {#each links as link}
      <a href={link.href} class:active={$page.url.pathname === link.href}>{link.label}</a>
    {/each}
  </nav>

  <div class="content">
    {@render children()}
  </div>

  {#if timerStore.active}
    <div class="timer-bar">
      <span class="dot" class:paused={timerStore.active.is_paused}></span>
      <span class="label">
        {timerStore.active.class_course_code} — {timerStore.active.task_title}
      </span>
      <span class="clock">{formatHms(timerStore.displaySeconds)}</span>
      <button onclick={togglePause}>{timerStore.active.is_paused ? "Resume" : "Pause"}</button>
      <button onclick={finish} class="finish">Finish</button>
    </div>
  {/if}
</div>

<style>
  :global(:root) {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    color: #0f0f0f;
    background-color: #f6f6f6;
  }

  :global(body) {
    margin: 0;
  }

  .shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  nav {
    display: flex;
    gap: 1.25rem;
    padding: 0.9rem 1.5rem;
    border-bottom: 1px solid #e0e0e0;
    background: white;
  }

  nav a {
    text-decoration: none;
    color: #555;
    font-weight: 500;
  }

  nav a.active {
    color: #396cd8;
  }

  .content {
    flex: 1;
    padding-bottom: 4.5rem;
  }

  .timer-bar {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.7rem 1.5rem;
    background: #1f2430;
    color: white;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #4caf50;
  }

  .dot.paused {
    background: #e0a300;
  }

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .clock {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-size: 1.1em;
  }

  .timer-bar button {
    padding: 0.4em 0.9em;
    border-radius: 6px;
    border: 1px solid transparent;
    background: #3a4155;
    color: white;
    cursor: pointer;
  }

  .timer-bar button.finish {
    background: #396cd8;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      color: #f6f6f6;
      background-color: #2f2f2f;
    }
    nav {
      background: #262626;
      border-color: #3a3a3a;
    }
  }
</style>
