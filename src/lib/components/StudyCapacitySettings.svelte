<script lang="ts">
  import { onMount } from "svelte";
  import { getStudyCapacity, setStudyCapacity } from "$lib/api";

  const weekdayLabels = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
  let minutes = $state<(number | null)[]>(new Array(7).fill(null));
  let error = $state("");

  onMount(async () => {
    try {
      const capacity = await getStudyCapacity();
      minutes = capacity.weekday_minutes;
    } catch (e) {
      error = String(e);
    }
  });

  async function save() {
    try {
      await setStudyCapacity(minutes);
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  function update(i: number, value: string) {
    minutes[i] = value.trim() === "" ? null : Math.max(0, parseInt(value, 10) || 0);
  }
</script>

<section>
  <h2>Study capacity</h2>
  <p class="muted">
    Optional. Set how many minutes you can realistically study each day, and Week view will show
    workload percentages and overload warnings. Leave blank to hide them.
  </p>
  {#if error}<p class="error">{error}</p>{/if}
  <ul class="capacity-grid">
    {#each weekdayLabels as label, i (label)}
      <li>
        <span>{label}</span>
        <input
          type="number"
          min="0"
          step="15"
          placeholder="—"
          value={minutes[i] ?? ""}
          oninput={(e) => update(i, (e.target as HTMLInputElement).value)}
        />
        <span class="unit">min</span>
      </li>
    {/each}
  </ul>
  <button onclick={save}>Save capacity</button>
</section>

<style>
  .capacity-grid { list-style: none; padding: 0; margin: 12px 0; display: flex; flex-direction: column; gap: 6px; }
  .capacity-grid li { display: grid; grid-template-columns: 100px 90px auto; align-items: center; gap: 8px; font-size: 13px; }
  .capacity-grid input { border: 1px solid var(--line); border-radius: 6px; padding: 5px 8px; font-size: 13px; width: 100%; }
  .unit { color: var(--muted); font-size: 11px; }
</style>
