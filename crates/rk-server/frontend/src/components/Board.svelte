<script>
  export let shared = false;
  import { onMount, onDestroy } from 'svelte';
  import { tasks } from '../lib/stores.js';
  import Column from './Column.svelte';

  let searchQuery = '';
  let searchInput;

  const columns = [
    { key: 'todo', label: 'Todo', color: '#64ffda' },
    { key: 'in_progress', label: 'In Progress', color: '#ffd93d' },
    { key: 'done', label: 'Done', color: '#6bcb77' },
  ];

  $: filteredTasks = searchQuery
    ? $tasks.filter(t => {
        const q = searchQuery.toLowerCase();
        return t.title.toLowerCase().includes(q) || (t.description || '').toLowerCase().includes(q);
      })
    : $tasks;

  $: columnTasks = columns.map(col => ({
    ...col,
    tasks: filteredTasks.filter(t => t.column === col.key),
  }));

  function handleKeydown(e) {
    if (e.key === '/' && document.activeElement?.tagName !== 'INPUT' && document.activeElement?.tagName !== 'TEXTAREA') {
      e.preventDefault();
      searchInput?.focus();
    }
    if (e.key === 'Escape' && document.activeElement === searchInput) {
      searchQuery = '';
      searchInput?.blur();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="search-bar">
  <input
    type="text"
    placeholder="Search tasks... (press / to focus)"
    bind:value={searchQuery}
    bind:this={searchInput}
    class="search-input"
  />
  {#if searchQuery}
    <button class="clear-btn" on:click={() => searchQuery = ''}>&#x00d7;</button>
    <span class="result-count">{filteredTasks.length} result{filteredTasks.length !== 1 ? 's' : ''}</span>
  {/if}
</div>

<div class="board">
  {#each columnTasks as col}
    <Column {col} {shared} />
  {/each}
</div>

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 20px;
    background: rgba(0, 0, 0, 0.2);
  }
  .search-input {
    flex: 1;
    max-width: 400px;
    padding: 6px 12px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.05);
    color: #e0e0e0;
    font-size: 14px;
    font-family: inherit;
    outline: none;
    transition: border-color 0.15s;
  }
  .search-input:focus {
    border-color: #64ffda;
  }
  .search-input::placeholder {
    color: rgba(255, 255, 255, 0.35);
  }
  .clear-btn {
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.5);
    font-size: 18px;
    cursor: pointer;
    padding: 2px 6px;
    line-height: 1;
    transition: color 0.15s;
  }
  .clear-btn:hover {
    color: #fff;
  }
  .result-count {
    color: rgba(255, 255, 255, 0.4);
    font-size: 13px;
    white-space: nowrap;
  }
  .board {
    display: flex; gap: 14px; padding: 20px;
    min-height: calc(100vh - 48px);
  }
  @media (max-width: 768px) {
    .board { flex-direction: column; padding: 12px; }
    .search-bar { padding: 8px 12px; }
    .search-input { max-width: none; }
  }
</style>
