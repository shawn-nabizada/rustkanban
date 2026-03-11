<script>
  export let params = {};
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { checkAuth } from '../lib/auth.js';
  import { user, currentBoard, tasks, tags, sharedContext, showFlash, activeModal, editingTask } from '../lib/stores.js';
  import Board from '../components/Board.svelte';
  import TaskModal from '../components/TaskModal.svelte';

  let loading = true;
  let notFound = false;

  async function loadSharedBoard() {
    loading = true;
    try {
      const data = await api.getSharedBoard(params.token);
      currentBoard.set({ uuid: data.board.uuid, name: data.board.name, position: 0 });
      tasks.set(data.tasks);
      tags.set(data.tags);
      sharedContext.set({
        permission: data.permission,
        owner_username: data.owner_username,
        token: params.token,
      });
    } catch (e) {
      notFound = true;
    }
    loading = false;
  }

  onMount(async () => {
    const me = await checkAuth();
    if (me) user.set(me);
    await loadSharedBoard();
  });
</script>

{#if loading}
  <div class="loading">Loading shared board...</div>
{:else if notFound}
  <div class="loading">Board not found or share link expired.</div>
{:else}
  <div class="shared-banner">
    Viewing {$sharedContext?.owner_username}'s board
    {#if $sharedContext?.permission === 'edit' && !$user}
      — <a href="/login?redirect_url={encodeURIComponent(`/#/shared/${params.token}`)}">Log in to edit</a>
    {/if}
  </div>
  <Board shared={true} />

  {#if $activeModal === 'task-create' || $activeModal === 'task-edit'}
    <TaskModal shared={true} on:saved={loadSharedBoard} />
  {/if}
{/if}

<style>
  .loading { display: flex; align-items: center; justify-content: center; min-height: 60vh; color: #8b949e; }
  .shared-banner {
    padding: 8px 16px; background: #16213e; text-align: center;
    font-size: 16px; color: #64ffda; border-bottom: 1px solid #30363d;
  }
  .shared-banner a { color: #bb86fc; }
</style>
