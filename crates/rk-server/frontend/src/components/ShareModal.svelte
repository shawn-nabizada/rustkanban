<script>
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { api } from '../lib/api.js';
  import { currentBoard, activeModal, showFlash } from '../lib/stores.js';

  let shares = [];
  let permission = 'view';
  let loading = true;

  onMount(async () => {
    try {
      shares = await api.listShares($currentBoard.uuid);
    } catch (e) {
      showFlash(e.message);
    }
    loading = false;
  });

  async function createShare() {
    try {
      const share = await api.createShare($currentBoard.uuid, { permission });
      shares = [...shares, share];
    } catch (e) {
      showFlash(e.message);
    }
  }

  async function revokeShare(id) {
    try {
      await api.deleteShare(id);
      shares = shares.filter(s => s.id !== id);
    } catch (e) {
      showFlash(e.message);
    }
  }

  function copyLink(url) {
    navigator.clipboard.writeText(window.location.origin + url);
    showFlash('Link copied!', 'success');
  }

  function close() {
    activeModal.set(null);
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="overlay" on:click={close} role="dialog" transition:fade={{ duration: 150 }}>
  <div class="modal" on:click|stopPropagation role="document"
    in:fly={{ y: 16, duration: 250 }} out:fade={{ duration: 100 }}>
    <h3>Share Board</h3>

    <div class="create-row">
      <select bind:value={permission}>
        <option value="view">View only</option>
        <option value="edit">Can edit (requires login)</option>
      </select>
      <button class="btn-create" on:click={createShare}>Create Link</button>
    </div>

    {#if loading}
      <div class="empty">Loading...</div>
    {:else if shares.length === 0}
      <div class="empty">No share links yet</div>
    {:else}
      <div class="share-list">
        {#each shares as share}
          <div class="share-row">
            <span class="share-perm">{share.permission}</span>
            <button class="sm-btn teal" on:click={() => copyLink(share.url)}>Copy Link</button>
            <button class="sm-btn danger" on:click={() => revokeShare(share.id)}>Revoke</button>
          </div>
        {/each}
      </div>
    {/if}

    <div class="actions">
      <button class="btn-close" on:click={close}>Close</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0; background: #000000aa;
    display: flex; align-items: center; justify-content: center;
    z-index: 100; backdrop-filter: blur(4px);
  }
  .modal {
    background: #16213e; border: 1px solid #1e2d4a;
    border-radius: 12px; padding: 24px;
    width: 440px; max-width: 90vw;
    box-shadow: 0 20px 60px #00000066;
  }
  h3 { margin-bottom: 20px; color: #64ffda; font-size: 18px; }
  .create-row { display: flex; gap: 8px; margin-bottom: 20px; }
  .create-row select {
    flex: 1; background: #0f172a; border: 1px solid #1e2d4a;
    border-radius: 6px; padding: 9px 12px; color: #e0e0e0;
    font-family: inherit; font-size: 16px;
    transition: border-color 0.15s;
  }
  .create-row select:focus { outline: none; border-color: #64ffda44; }
  .btn-create {
    background: #64ffda18; color: #64ffda; border: 1px solid #64ffda30;
    padding: 9px 16px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
    white-space: nowrap;
  }
  .btn-create:hover { background: #64ffda28; }
  .share-list { display: flex; flex-direction: column; gap: 6px; }
  .share-row {
    display: flex; align-items: center; gap: 8px;
    padding: 10px 12px; background: #0f172a;
    border: 1px solid #1e2d4a; border-radius: 6px;
  }
  .share-perm { color: #bb86fc; font-size: 16px; flex: 1; }
  .sm-btn {
    background: none; border: 1px solid #1e2d4a; color: #8b949e;
    padding: 4px 10px; border-radius: 4px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .sm-btn.teal { color: #64ffda; border-color: #64ffda30; }
  .sm-btn.teal:hover { background: #64ffda10; }
  .sm-btn.danger { color: #ff6b6b; border-color: #ff6b6b30; }
  .sm-btn.danger:hover { background: #ff6b6b10; }
  .empty { color: #6b7b8d; font-size: 16px; text-align: center; padding: 16px; }
  .actions { margin-top: 20px; display: flex; justify-content: flex-end; }
  .btn-close {
    background: #ffffff08; color: #8b949e; border: 1px solid #1e2d4a;
    padding: 8px 18px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .btn-close:hover { color: #c9d1d9; background: #ffffff10; }
</style>
