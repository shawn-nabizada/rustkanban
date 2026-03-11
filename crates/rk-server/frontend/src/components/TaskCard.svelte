<script>
  export let task;
  export let shared = false;
  export let readonly = false;
  import { activeModal, editingTask } from '../lib/stores.js';

  const priorityColors = { High: '#ff6b6b', Medium: '#ffd93d', Low: '#6bcb77' };
  const priorityLabel = { High: 'H', Medium: 'M', Low: 'L' };

  function openEdit() {
    if (readonly) return;
    editingTask.set(task);
    activeModal.set('task-edit');
  }
</script>

<div
  class="card"
  class:done={task.column === 'done'}
  style="border-left-color: {priorityColors[task.priority] || '#ffd93d'}"
  on:click={openEdit}
  on:keypress={openEdit}
  role="button"
  tabindex="0"
>
  <div class="card-title">
    <span class="priority" style="color: {priorityColors[task.priority] || '#ffd93d'}">[{priorityLabel[task.priority] || 'M'}]</span>
    {task.title}
  </div>
  {#if task.tags.length > 0 || task.due_date}
    <div class="card-meta">
      {#each task.tags as tag}
        <span class="tag">{tag.name}</span>
      {/each}
      {#if task.due_date}
        <span class="due">{task.due_date}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .card {
    background: #16213e; border-left: 3px solid;
    border-radius: 6px; padding: 10px 12px;
    cursor: pointer; font-size: 16px;
    transition: background 0.15s, transform 0.1s;
  }
  .card:hover { background: #1c2a4a; transform: translateX(2px); }
  .card.done { opacity: 0.5; }
  .card-title { line-height: 1.5; color: #c9d1d9; }
  .priority { font-weight: 700; margin-right: 2px; }
  .card-meta {
    margin-top: 6px; font-size: 16px;
    display: flex; flex-wrap: wrap; gap: 6px;
    align-items: center;
  }
  .tag {
    color: #bb86fc; background: #bb86fc12;
    padding: 1px 6px; border-radius: 3px;
    font-size: 16px;
  }
  .due {
    color: #ff6b6b; margin-left: auto;
    font-size: 16px; opacity: 0.9;
  }
</style>
