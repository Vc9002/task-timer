<script lang="ts">
  import { onMount } from "svelte";
  import {
    listClasses,
    createClass,
    archiveClass,
    type ClassRecord,
  } from "$lib/api";

  let classes = $state<ClassRecord[]>([]);
  let courseCode = $state("");
  let name = $state("");
  let semester = $state("");
  let error = $state("");

  async function refresh() {
    try {
      classes = await listClasses();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function addClass(event: Event) {
    event.preventDefault();
    if (!courseCode.trim() || !semester.trim()) return;
    try {
      await createClass({
        course_code: courseCode.trim(),
        name: name.trim() || null,
        semester: semester.trim(),
        color: null,
      });
      courseCode = "";
      name = "";
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(id: number) {
    try {
      await archiveClass(id);
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(refresh);
</script>

<main class="container">
  <h1>Classes</h1>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <form class="row" onsubmit={addClass}>
    <input placeholder="Course code (e.g. LGST 1000)" bind:value={courseCode} />
    <input placeholder="Full name (optional)" bind:value={name} />
    <input placeholder="Semester (e.g. Fall 2026)" bind:value={semester} />
    <button type="submit">Add class</button>
  </form>

  <ul class="class-list">
    {#each classes as c (c.id)}
      <li>
        <span class="code">{c.course_code}</span>
        {#if c.name}<span class="name">{c.name}</span>{/if}
        <span class="semester">{c.semester}</span>
        <button onclick={() => remove(c.id)}>Archive</button>
      </li>
    {/each}
    {#if classes.length === 0}
      <li class="empty">No classes yet — add one above.</li>
    {/if}
  </ul>
</main>

<style>
  :root {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    color: #0f0f0f;
    background-color: #f6f6f6;
  }

  .container {
    margin: 0 auto;
    max-width: 640px;
    padding: 3rem 1.5rem;
  }

  h1 {
    margin-bottom: 1.5rem;
  }

  .row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1.5rem;
  }

  input {
    flex: 1;
    padding: 0.5em 0.75em;
    border-radius: 6px;
    border: 1px solid #ccc;
  }

  button {
    padding: 0.5em 1em;
    border-radius: 6px;
    border: 1px solid transparent;
    background: #396cd8;
    color: white;
    cursor: pointer;
  }

  .class-list {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .class-list li {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6em 0.9em;
    background: white;
    border-radius: 6px;
    border: 1px solid #e5e5e5;
  }

  .class-list li button {
    margin-left: auto;
    background: #e5e5e5;
    color: #333;
  }

  .code {
    font-weight: 600;
  }

  .semester {
    color: #666;
    font-size: 0.9em;
  }

  .empty {
    color: #888;
    font-style: italic;
    border-style: dashed;
  }

  .error {
    color: #b00020;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      color: #f6f6f6;
      background-color: #2f2f2f;
    }
    .class-list li {
      background: #3a3a3a;
      border-color: #4a4a4a;
    }
    input {
      background: #2a2a2a;
      color: #f6f6f6;
      border-color: #555;
    }
  }
</style>
