<script lang="ts">
  import { applyPlan, getPlanProposal, type PlanProposal, type ProposedBlock } from "$lib/api";
  import { formatMinutesShort } from "$lib/format";
  let { startDate, days, onclose, onapplied }: { startDate: string; days: number; onclose: () => void; onapplied: () => Promise<void> } = $props();
  let proposal = $state<PlanProposal | null>(null); let excluded = $state<string[]>([]); let error = $state(""); let busy = $state(false);
  function key(block: ProposedBlock, index: number) { return `${block.task_id}-${block.planned_date}-${index}`; }
  function included(block: ProposedBlock, index: number) { return !excluded.includes(key(block, index)); }
  async function load() { try { proposal = await getPlanProposal(startDate, days); error = ""; } catch (e) { error = String(e); } }
  async function apply() { if (!proposal || busy) return; busy = true; try { const blocks = proposal.days.flatMap(day => day.blocks.map((block, index) => included(block, index) ? { task_id: block.task_id, planned_date: block.planned_date, planned_minutes: block.planned_minutes } : null).filter((block): block is { task_id: number; planned_date: string; planned_minutes: number } => block !== null)); await applyPlan(blocks); await onapplied(); onclose(); } catch (e) { error = String(e); } finally { busy = false; } }
  $effect(() => { void startDate; void days; void load(); });
</script>

<section class="planner" aria-label="Plan study time">
  <header><div><p class="eyebrow">Deterministic proposal</p><h2>Plan {days === 1 ? "my day" : "my week"}</h2><p class="muted">Existing blocks stay untouched. Review this proposal before adding anything.</p></div><button class="quiet" onclick={onclose}>Close</button></header>
  {#if error}<p class="error">{error}</p>{/if}
  {#if proposal}
    {#if proposal.unschedulable_overdue.length}<p class="warning">Overdue work needs attention first: {proposal.unschedulable_overdue.join(", ")}.</p>{/if}
    {#each proposal.days as day}<div class="day"><div class="day-head"><strong>{day.date}</strong><span>{formatMinutesShort(day.existing_block_minutes)} already planned · {formatMinutesShort(day.proposed_minutes)} proposed</span></div>{#each day.blocks as block, index}<label class="block" class:excluded={!included(block,index)}><input type="checkbox" checked={included(block,index)} onchange={() => { const value = key(block,index); excluded = included(block,index) ? [...excluded, value] : excluded.filter(item => item !== value); }} /><span><b>{block.course_code} · {block.task_title}</b><small>{block.planned_minutes}m · {block.reason}</small></span></label>{:else}<p class="muted">No new block fits this day.</p>{/each}</div>{/each}
    <div class="actions"><button class="secondary" onclick={onclose}>Cancel</button><button disabled={busy} onclick={() => void apply()}>{busy ? "Applying…" : "Apply selected blocks"}</button></div>
  {:else}<p class="muted">Building proposal…</p>{/if}
</section>

<style>
  .planner{margin:0 0 24px;padding:18px;border:1px solid var(--accent);border-radius:10px;background:var(--surface)}header{display:flex;justify-content:space-between;gap:12px}h2{margin:.2rem 0}.muted{color:var(--muted);font-size:12px}.warning{padding:10px;background:var(--accent-soft);font-size:12px}.day{border-top:1px solid var(--line);padding:10px 0}.day-head{display:flex;justify-content:space-between;gap:10px;font-size:12px;margin-bottom:5px}.day-head span{color:var(--muted);font-size:11px}.block{display:flex;align-items:center;gap:9px;padding:8px;background:var(--hover);margin:4px 0;cursor:pointer}.block.excluded{opacity:.45}.block span{display:flex;flex-direction:column;gap:2px}.block small{color:var(--muted);font-size:10px}.actions{display:flex;justify-content:flex-end;gap:8px;margin-top:13px}.error{color:var(--danger);font-size:12px}@media(max-width:600px){.day-head{flex-direction:column;gap:2px}}
</style>
