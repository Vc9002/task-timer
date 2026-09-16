<script lang="ts">
  import { getCalendar, type CalendarDay } from "$lib/api";
  import { localDate } from "$lib/format";
  let month = $state(localDate().slice(0, 7));
  let days = $state<CalendarDay[]>([]);
  let error = $state("");
  async function refresh() { try { days = await getCalendar(month); error = ""; } catch (e) { error = String(e); } }
  $effect(() => { void month; void refresh(); });
  function shift(delta: number) { const [y,m] = month.split("-").map(Number); const d = new Date(y, m - 1 + delta, 1); month = `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,"0")}`; }
  function label() { return new Date(`${month}-01T00:00:00`).toLocaleDateString(undefined, { month: "long", year: "numeric" }); }
  function offset() { const first = new Date(`${month}-01T00:00:00`); return (first.getDay() + 6) % 7; }
</script>
<main class="container">
  <header><div><p class="eyebrow">Plan and deadlines</p><h1>Calendar</h1></div><div class="nav"><button onclick={() => shift(-1)} aria-label="Previous month">←</button><strong>{label()}</strong><button onclick={() => shift(1)} aria-label="Next month">→</button></div></header>
  {#if error}<p class="error">{error}</p>{/if}
  {#if days.length}
    <div class="weekdays">{#each ["Mon","Tue","Wed","Thu","Fri","Sat","Sun"] as day}<span>{day}</span>{/each}</div>
    <div class="grid">
      {#each Array(offset()) as _}<div class="blank"></div>{/each}
      {#each days as day (day.date)}
        <section class="day" class:today={day.date === localDate()}>
          <h2>{Number(day.date.slice(8))}</h2>
          {#if day.planned.length}<p class="kind planned-label">PLANNED</p>{#each day.planned as task (task.id)}<div class="task planned">{task.title}<small>{task.estimated_minutes ? `${task.estimated_minutes}m` : ""}</small></div>{/each}{/if}
          {#if day.due.length}<p class="kind due-label">DUE</p>{#each day.due as task (task.id)}<div class="task due">{task.title}</div>{/each}{/if}
        </section>
      {/each}
    </div>
    <div class="legend"><span><i class="planned-dot"></i>Planned study</span><span><i class="due-dot"></i>Due date</span></div>
  {:else if !error}<p class="page-intro">Loading calendar…</p>{/if}
</main>
<style>
 header{display:flex;justify-content:space-between;align-items:end;margin-bottom:24px;gap:12px}.nav{display:flex;align-items:center;gap:10px}.nav button{border:1px solid var(--line);background:var(--surface);border-radius:6px;padding:6px 10px}.nav strong{font-size:13px;min-width:130px;text-align:center}.weekdays,.grid{display:grid;grid-template-columns:repeat(7,minmax(0,1fr));gap:6px}.weekdays{margin-bottom:6px}.weekdays span{color:var(--muted);font-size:10px;text-transform:uppercase;letter-spacing:.08em;padding:4px}.day{min-height:125px;border:1px solid var(--line);border-radius:8px;padding:8px;background:var(--surface)}.day.today{border-color:var(--accent);box-shadow:inset 0 2px var(--accent)}.day h2{font-size:13px;margin:0 0 8px}.kind{font-size:8px;letter-spacing:.08em;margin:5px 0 3px;font-weight:700}.planned-label{color:var(--accent)}.due-label{color:#b97935}.task{font-size:11px;line-height:1.3;padding:4px 5px;margin:2px 0;border-radius:4px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.task small{float:right;color:var(--muted);font-size:9px}.planned{background:var(--accent-soft)}.due{background:#fff3df}.legend{display:flex;gap:18px;margin-top:14px;color:var(--muted);font-size:11px}.legend i{display:inline-block;width:8px;height:8px;border-radius:2px;margin-right:5px}.planned-dot{background:var(--accent)}.due-dot{background:#d29a4b}@media(max-width:700px){.day{min-height:95px;padding:5px}.task{font-size:10px}.nav strong{min-width:100px}}@media(max-width:520px){.grid,.weekdays{gap:3px}.day{min-height:78px}.task{padding:2px;font-size:9px}.kind{font-size:7px}.day h2{font-size:11px}}
</style>
