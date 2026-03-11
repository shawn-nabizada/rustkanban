<script>
  import { boards, currentBoard, user, isLoggedIn, activeModal } from '../lib/stores.js';
  import { logout, redirectToLogin } from '../lib/auth.js';
  import { push, location } from 'svelte-spa-router';

  function switchBoard(uuid) {
    push(`/board/${uuid}`);
  }

  async function handleLogout() {
    await logout();
    push('/');
  }
</script>

<header>
  <a href="#/" class="brand">Rust<span class="accent">Kanban</span></a>
  {#if $isLoggedIn}
    <div class="tabs">
      {#each $boards as board}
        <button
          class="tab"
          class:active={$location.startsWith('/board/') && $currentBoard?.uuid === board.uuid}
          on:click={() => switchBoard(board.uuid)}
        >
          {board.name}
        </button>
      {/each}
    </div>
    <div class="actions">
      {#if $location.startsWith('/board/')}
        <div class="board-actions">
          <button class="btn-primary" on:click={() => activeModal.set('task-create')}>+ New Task</button>
          <button class="btn-ghost" on:click={() => activeModal.set('tags')}>Tags</button>
          <button class="btn-ghost" on:click={() => activeModal.set('share')}>Share</button>
        </div>
        <span class="sep"></span>
      {/if}
      <div class="user-area">
        <a href="#/account" class="user-chip">
          <span class="avatar">{$user?.username?.[0]?.toUpperCase() || '?'}</span>
          <span class="uname">{$user?.username}</span>
        </a>
        <button class="btn-logout" on:click={handleLogout}>Sign out</button>
      </div>
    </div>
  {:else}
    <div class="actions">
      <button class="btn-primary" on:click={redirectToLogin}>Login with GitHub</button>
    </div>
  {/if}
</header>

<style>
  header {
    display: flex; align-items: center; gap: 16px;
    padding: 0 20px; height: 48px;
    background: linear-gradient(180deg, #16213e 0%, #142036 100%);
    border-bottom: 1px solid #1e2d4a;
    position: sticky; top: 0; z-index: 50;
  }
  .brand {
    font-weight: 700; color: #c9d1d9; font-size: 16px;
    text-decoration: none; letter-spacing: -0.3px;
    white-space: nowrap; transition: opacity 0.15s;
  }
  .brand:hover { opacity: 0.8; }
  .accent { color: #64ffda; }

  .tabs { display: flex; gap: 2px; margin-left: 8px; }
  .tab {
    background: none; border: none; color: #6b7b8d;
    padding: 6px 12px; cursor: pointer;
    font-family: inherit; font-size: 16px;
    border-radius: 4px; transition: all 0.15s;
  }
  .tab:hover { color: #c9d1d9; background: #ffffff08; }
  .tab.active { color: #64ffda; background: #64ffda10; }

  .actions { margin-left: auto; display: flex; align-items: center; gap: 10px; }

  .board-actions { display: flex; align-items: center; gap: 6px; }

  .sep {
    display: block; width: 1px; height: 20px;
    background: #30363d;
  }

  .btn-primary {
    background: #64ffda12; color: #64ffda;
    border: 1px solid #64ffda28; padding: 5px 14px;
    border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px;
    transition: all 0.15s;
  }
  .btn-primary:hover { background: #64ffda22; border-color: #64ffda44; }

  .btn-ghost {
    background: transparent; color: #8b949e;
    border: 1px solid transparent; padding: 5px 10px;
    border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px;
    transition: all 0.15s;
  }
  .btn-ghost:hover { color: #c9d1d9; background: #ffffff08; }

  .user-area { display: flex; align-items: center; gap: 8px; }

  .user-chip {
    display: flex; align-items: center; gap: 8px;
    padding: 3px 10px 3px 3px; border-radius: 20px;
    background: #ffffff06; border: 1px solid #ffffff0a;
    text-decoration: none; transition: all 0.15s;
  }
  .user-chip:hover { background: #ffffff10; border-color: #ffffff18; }

  .avatar {
    display: flex; align-items: center; justify-content: center;
    width: 22px; height: 22px; border-radius: 50%;
    background: #bb86fc20; color: #bb86fc;
    font-size: 16px; font-weight: 700;
  }
  .uname { color: #9ba4ae; font-size: 16px; }

  .btn-logout {
    background: none; border: none; color: #6b7b8d;
    font-size: 16px; cursor: pointer; font-family: inherit;
    padding: 4px 8px; border-radius: 4px; transition: all 0.15s;
  }
  .btn-logout:hover { color: #ff6b6b; background: #ff6b6b10; }

  @media (max-width: 768px) {
    header { flex-wrap: wrap; height: auto; padding: 8px 16px; }
    .tabs { order: 3; width: 100%; overflow-x: auto; }
    .board-actions { gap: 4px; }
  }
</style>
