<script>
  export let col;
  export let shared = false;
  import { dndzone } from 'svelte-dnd-action';
  import { api } from '../lib/api.js';
  import { tasks, sharedContext, showFlash } from '../lib/stores.js';
  import TaskCard from './TaskCard.svelte';

  let items = [];
  $: items = col.tasks.map(t => ({ ...t, id: t.uuid }));

  const flipDurationMs = 200;

  function handleConsider(e) {
    items = e.detail.items;
  }

  async function handleFinalize(e) {
    items = e.detail.items;
    for (const item of items) {
      const original = $tasks.find(t => t.uuid === item.uuid);
      if (original && original.column !== col.key) {
        tasks.update(all => all.map(t =>
          t.uuid === item.uuid ? { ...t, column: col.key } : t
        ));
        try {
          if (shared && $sharedContext) {
            await api.updateSharedTask($sharedContext.token, item.uuid, { column: col.key });
          } else {
            await api.updateTask(item.uuid, { column: col.key });
          }
        } catch (e) {
          tasks.update(all => all.map(t =>
            t.uuid === item.uuid ? { ...t, column: original.column } : t
          ));
          showFlash('Failed to move task');
        }
      }
    }
  }

  $: canEdit = !shared || ($sharedContext?.permission === 'edit');
</script>

<div class="column">
  <div class="column-header">
    <span class="header-dot" style="background: {col.color}"></span>
    <span class="header-label">{col.label}</span>
    <span class="header-count">{col.tasks.length}</span>
  </div>
  {#if canEdit}
    <div
      class="column-body"
      use:dndzone={{ items, flipDurationMs, type: 'task' }}
      on:consider={handleConsider}
      on:finalize={handleFinalize}
    >
      {#each items as task (task.id)}
        <TaskCard {task} {shared} />
      {/each}
    </div>
  {:else}
    <div class="column-body">
      {#each col.tasks as task}
        <TaskCard {task} {shared} readonly={true} />
      {/each}
    </div>
  {/if}
  {#if col.tasks.length === 0}
    <div class="empty">No tasks</div>
  {/if}
</div>

<style>
  .column {
    flex: 1; background: #121e36;
    border: 1px solid #1e2d4a;
    border-radius: 8px; padding: 12px; min-width: 0;
  }
  .column-header {
    display: flex; align-items: center; gap: 8px;
    margin-bottom: 12px; padding-bottom: 10px;
    border-bottom: 1px solid #1e2d4a;
  }
  .header-dot {
    width: 8px; height: 8px; border-radius: 50%;
    flex-shrink: 0;
  }
  .header-label {
    font-weight: 600; font-size: 16px; color: #c9d1d9;
  }
  .header-count {
    font-size: 16px; color: #6b7b8d;
    background: #ffffff08; padding: 1px 7px;
    border-radius: 10px;
  }
  .column-body { min-height: 40px; display: flex; flex-direction: column; gap: 8px; }
  .empty {
    color: #6b7b8d; font-size: 16px; text-align: center;
    padding: 24px; border: 1px dashed #1e2d4a;
    border-radius: 6px;
  }
</style>
