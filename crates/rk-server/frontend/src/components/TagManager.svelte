<script>
  import { createEventDispatcher } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { api } from '../lib/api.js';
  import { tags, activeModal, showFlash } from '../lib/stores.js';

  const dispatch = createEventDispatcher();

  let newTagName = '';
  let editingId = null;
  let editName = '';

  async function addTag() {
    if (!newTagName.trim()) return;
    try {
      await api.createTag({ name: newTagName.trim() });
      newTagName = '';
      dispatch('changed');
    } catch (e) {
      showFlash(e.message);
    }
  }

  function startRename(tag) {
    editingId = tag.uuid;
    editName = tag.name;
  }

  async function saveRename() {
    if (!editName.trim()) return;
    try {
      await api.updateTag(editingId, { name: editName.trim() });
      editingId = null;
      dispatch('changed');
    } catch (e) {
      showFlash(e.message);
    }
  }

  async function deleteTag(uuid) {
    try {
      await api.deleteTag(uuid);
      dispatch('changed');
    } catch (e) {
      showFlash(e.message);
    }
  }

  function close() {
    activeModal.set(null);
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="overlay" on:click={close} role="dialog" transition:fade={{ duration: 150 }}>
  <div class="modal" on:click|stopPropagation role="document"
    in:fly={{ y: 16, duration: 250 }} out:fade={{ duration: 100 }}>
    <h3>Tag Management</h3>

    <div class="add-row">
      <input bind:value={newTagName} placeholder="New tag name" maxlength="50"
        on:keydown={(e) => e.key === 'Enter' && addTag()} />
      <button class="btn-add" on:click={addTag}>Add</button>
    </div>

    <div class="tag-list">
      {#each $tags as tag}
        <div class="tag-row">
          {#if editingId === tag.uuid}
            <input class="edit-input" bind:value={editName}
              on:keydown={(e) => e.key === 'Enter' && saveRename()}
              on:blur={saveRename} />
          {:else}
            <span class="tag-name">{tag.name}</span>
          {/if}
          <div class="tag-actions">
            <button class="sm-btn" on:click={() => startRename(tag)}>Rename</button>
            <button class="sm-btn danger" on:click={() => deleteTag(tag.uuid)}>Delete</button>
          </div>
        </div>
      {/each}
      {#if $tags.length === 0}
        <div class="empty">No tags yet</div>
      {/if}
    </div>

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
    width: 420px; max-width: 90vw;
    box-shadow: 0 20px 60px #00000066;
  }
  h3 { margin-bottom: 20px; color: #64ffda; font-size: 18px; }
  .add-row { display: flex; gap: 8px; margin-bottom: 20px; }
  .add-row input {
    flex: 1; background: #0f172a; border: 1px solid #1e2d4a;
    border-radius: 6px; padding: 9px 12px; color: #e0e0e0;
    font-family: inherit; font-size: 16px;
    transition: border-color 0.15s;
  }
  .add-row input:focus { outline: none; border-color: #64ffda44; }
  .btn-add {
    background: #64ffda18; color: #64ffda; border: 1px solid #64ffda30;
    padding: 9px 16px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .btn-add:hover { background: #64ffda28; }
  .tag-list { display: flex; flex-direction: column; gap: 6px; max-height: 300px; overflow-y: auto; }
  .tag-row {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 10px; background: #0f172a;
    border: 1px solid #1e2d4a; border-radius: 6px;
  }
  .tag-name { color: #bb86fc; font-size: 16px; }
  .tag-actions { display: flex; gap: 4px; }
  .sm-btn {
    background: none; border: 1px solid #1e2d4a; color: #8b949e;
    padding: 3px 10px; border-radius: 4px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .sm-btn:hover { color: #c9d1d9; border-color: #ffffff20; }
  .sm-btn.danger { color: #ff6b6b; border-color: #ff6b6b30; }
  .sm-btn.danger:hover { background: #ff6b6b10; }
  .empty { color: #6b7b8d; font-size: 16px; text-align: center; padding: 16px; }
  .edit-input {
    flex: 1; background: #0f172a; border: 1px solid #64ffda44;
    border-radius: 6px; padding: 5px 10px; color: #e0e0e0;
    font-family: inherit; font-size: 16px;
  }
  .edit-input:focus { outline: none; }
  .actions { margin-top: 20px; display: flex; justify-content: flex-end; }
  .btn-close {
    background: #ffffff08; color: #8b949e; border: 1px solid #1e2d4a;
    padding: 8px 18px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .btn-close:hover { color: #c9d1d9; background: #ffffff10; }
</style>
