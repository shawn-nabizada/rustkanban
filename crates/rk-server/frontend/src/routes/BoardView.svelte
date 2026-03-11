<script>
  export let params = {};
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { api } from '../lib/api.js';
  import { redirectToLogin } from '../lib/auth.js';
  import { user, boards, currentBoard, tasks, tags, showFlash } from '../lib/stores.js';
  import Board from '../components/Board.svelte';
  import TaskModal from '../components/TaskModal.svelte';
  import TagManager from '../components/TagManager.svelte';
  import ShareModal from '../components/ShareModal.svelte';
  import { activeModal, editingTask } from '../lib/stores.js';

  let loading = true;

  async function loadBoard(uuid) {
    loading = true;
    try {
      const data = await api.getBoard(uuid);
      currentBoard.set(data.board);
      tasks.set(data.tasks);
      tags.set(data.tags);
    } catch (e) {
      showFlash(e.message);
    }
    loading = false;
  }

  onMount(() => {
    // Auth already checked by App.svelte; redirect if somehow not logged in
    if (!$user) return redirectToLogin();
    loadBoard(params.uuid);
  });

  // Reload when board UUID changes (tab switching)
  let prevUuid = null;
  $: if (params.uuid && params.uuid !== prevUuid) {
    prevUuid = params.uuid;
    loadBoard(params.uuid);
  }
</script>

{#if loading}
  <div class="loading">Loading...</div>
{:else}
  <div in:fade={{ duration: 200 }}>
    <Board />
  </div>
{/if}

{#if $activeModal === 'task-create' || $activeModal === 'task-edit'}
  <TaskModal on:saved={() => loadBoard(params.uuid)} />
{/if}
{#if $activeModal === 'tags'}
  <TagManager on:changed={() => loadBoard(params.uuid)} />
{/if}
{#if $activeModal === 'share'}
  <ShareModal />
{/if}

<style>
  .loading {
    display: flex; align-items: center; justify-content: center;
    min-height: 60vh; color: #8b949e;
  }
</style>
