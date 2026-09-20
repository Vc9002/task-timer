<script lang="ts">
  import { themeStore, type ThemeMode } from "$lib/stores/theme.svelte";

  const presets: { name: string; accent: string; accentSoft: string }[] = [
    { name: "Teal (default)", accent: "#276c65", accentSoft: "#e2eeeb" },
    { name: "Indigo", accent: "#4f5fd6", accentSoft: "#e6e8fb" },
    { name: "Plum", accent: "#8a3f8a", accentSoft: "#f2e4f2" },
    { name: "Amber", accent: "#b6791f", accentSoft: "#f6ecd9" },
    { name: "Slate", accent: "#4a5a6a", accentSoft: "#e6eaee" },
  ];

  function setMode(event: Event) {
    themeStore.setMode((event.currentTarget as HTMLSelectElement).value as ThemeMode);
  }

  function applyPreset(preset: (typeof presets)[number] | null) {
    if (!preset || preset.name === "Teal (default)") themeStore.setAccent(null, null);
    else themeStore.setAccent(preset.accent, preset.accentSoft);
  }
</script>

<section>
  <h2>Appearance</h2>
  <label>Theme
    <select value={themeStore.mode} onchange={setMode}>
      <option value="system">Match system</option>
      <option value="light">Light</option>
      <option value="dark">Dark</option>
    </select>
  </label>
  <label>Window transparency
    <input
      type="range"
      min="10"
      max="100"
      value={themeStore.opacity}
      oninput={(e) => themeStore.setOpacity(Number(e.currentTarget.value))}
    />
    <span class="muted">{themeStore.opacity}%</span>
  </label>
  <p class="muted">Accent color</p>
  <div class="swatches">
    {#each presets as preset (preset.name)}
      <button
        type="button"
        class="swatch"
        class:active={(preset.name === "Teal (default)" && !themeStore.accent) || themeStore.accent === preset.accent}
        style="background:{preset.accent}"
        title={preset.name}
        aria-label={preset.name}
        onclick={() => applyPreset(preset)}
      ></button>
    {/each}
  </div>
</section>

<style>
  .muted { color: var(--muted); font-size: 13px; margin: 14px 0 6px; }
  .swatches { display: flex; gap: 8px; }
  .swatch { width: 28px; height: 28px; padding: 0; border-radius: 50%; border: 2px solid transparent; cursor: pointer; }
  .swatch.active { border-color: var(--text); }
</style>
