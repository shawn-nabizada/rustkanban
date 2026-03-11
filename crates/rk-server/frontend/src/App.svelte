<script>
  import { onMount } from 'svelte';
  import Router from 'svelte-spa-router';
  import Home from './routes/Home.svelte';
  import BoardView from './routes/BoardView.svelte';
  import SharedBoard from './routes/SharedBoard.svelte';
  import Account from './routes/Account.svelte';
  import LoginToken from './routes/LoginToken.svelte';
  import Header from './components/Header.svelte';
  import { fly, fade } from 'svelte/transition';
  import { flash, user, boards } from './lib/stores.js';
  import { checkAuth } from './lib/auth.js';
  import { api } from './lib/api.js';

  let ready = false;

  const routes = {
    '/': Home,
    '/board/:uuid': BoardView,
    '/shared/:token': SharedBoard,
    '/account': Account,
    '/login-token': LoginToken,
  };

  onMount(async () => {
    const me = await checkAuth();
    if (me) {
      user.set(me);
      const boardList = await api.listBoards();
      boards.set(boardList);
    }
    ready = true;
  });
</script>

{#if $flash}
  <div class="flash flash-{$flash.kind}"
    in:fly={{ y: -16, duration: 250 }}
    out:fade={{ duration: 150 }}>
    {$flash.message}
  </div>
{/if}

{#if ready}
  <div in:fade={{ duration: 200 }}>
    <Header />
    <Router {routes} />
  </div>
{:else}
  <div class="loading">Loading...</div>
{/if}

<style>
  .flash {
    position: fixed;
    top: 12px;
    right: 12px;
    padding: 10px 18px;
    border-radius: 8px;
    font-size: 16px;
    z-index: 1000;
    backdrop-filter: blur(8px);
    box-shadow: 0 8px 24px #00000044;
  }
  .flash-error {
    background: #ff6b6b18; border: 1px solid #ff6b6b40; color: #ff6b6b;
  }
  .flash-success {
    background: #6bcb7718; border: 1px solid #6bcb7740; color: #6bcb77;
  }
  .loading {
    display: flex; align-items: center; justify-content: center;
    min-height: 60vh; color: #6b7b8d; font-size: 16px;
  }
</style>
