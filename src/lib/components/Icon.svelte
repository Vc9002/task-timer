<script lang="ts">
  let { name, size = 18 }: { name: string; size?: number } = $props();
  const paths: Record<string, string> = {
    today: "M8 2v4m8-4v4M3 10h18M5 4h14a2 2 0 0 1 2 2v14H3V6a2 2 0 0 1 2-2ZM8 14h3v3H8z",
    classes: "M4 4h6a2 2 0 0 1 2 2v15a3 3 0 0 0-3-2H3V4h1Zm16 0h-6a2 2 0 0 0-2 2v15a3 3 0 0 1 3-2h6V4h-1Z",
    week: "M7 3v4M17 3v4M4 9h16M4 6h16v13H4V6ZM8 13h2m4 0h2M8 16h2m4 0h2",
    history: "M3 11a9 9 0 1 1 2 7M3 4v7h7m2-5v6l4 2",
    analytics: "M4 20V10m8 10V4m8 16v-7",
    settings: "M4 7h16M4 17h16M9 4v6m6 4v6",
    clock: "M12 8v5l3 2M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
    plus: "M12 5v14M5 12h14",
    search: "m21 21-4.35-4.35M10.5 18a7.5 7.5 0 1 1 0-15 7.5 7.5 0 0 1 0 15Z",
    play: "m8 5 11 7-11 7V5Z",
    pause: "M8 5v14M16 5v14",
    check: "m5 12 4 4L19 6",
    close: "m6 6 12 12M6 18 18 6",
  };
</script>
<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={paths[name] ?? paths.clock} /></svg>
