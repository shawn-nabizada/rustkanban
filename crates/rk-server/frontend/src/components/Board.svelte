<script>
  export let shared = false;
  import { tasks } from '../lib/stores.js';
  import Column from './Column.svelte';

  const columns = [
    { key: 'todo', label: 'Todo', color: '#64ffda' },
    { key: 'in_progress', label: 'In Progress', color: '#ffd93d' },
    { key: 'done', label: 'Done', color: '#6bcb77' },
  ];

  $: columnTasks = columns.map(col => ({
    ...col,
    tasks: $tasks.filter(t => t.column === col.key),
  }));
</script>

<div class="board">
  {#each columnTasks as col}
    <Column {col} {shared} />
  {/each}
</div>

<style>
  .board {
    display: flex; gap: 14px; padding: 20px;
    min-height: calc(100vh - 48px);
  }
  @media (max-width: 768px) {
    .board { flex-direction: column; padding: 12px; }
  }
</style>
