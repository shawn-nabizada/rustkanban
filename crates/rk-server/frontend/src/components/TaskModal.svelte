<script>
  import { createEventDispatcher } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { api } from '../lib/api.js';
  import { activeModal, editingTask, currentBoard, tags, sharedContext, showFlash } from '../lib/stores.js';

  export let shared = false;

  const dispatch = createEventDispatcher();

  let title = '';
  let description = '';
  let priority = 'Medium';
  let column = 'todo';
  let due_date = '';
  let selectedTagUuids = [];
  let saving = false;

  const isEdit = $activeModal === 'task-edit';

  if (isEdit && $editingTask) {
    title = $editingTask.title;
    description = $editingTask.description;
    priority = $editingTask.priority;
    column = $editingTask.column;
    due_date = $editingTask.due_date || '';
    selectedTagUuids = $editingTask.tags.map(t => t.uuid);
  }

  function toggleTag(uuid) {
    if (selectedTagUuids.includes(uuid)) {
      selectedTagUuids = selectedTagUuids.filter(u => u !== uuid);
    } else {
      selectedTagUuids = [...selectedTagUuids, uuid];
    }
  }

  async function save() {
    if (!title.trim()) return showFlash('Title is required');
    saving = true;
    try {
      const body = {
        title: title.trim(),
        description,
        priority,
        column,
        due_date: due_date || null,
        tag_uuids: selectedTagUuids,
      };

      if (isEdit) {
        if (shared && $sharedContext) {
          await api.updateSharedTask($sharedContext.token, $editingTask.uuid, body);
        } else {
          await api.updateTask($editingTask.uuid, body);
        }
      } else {
        if (shared && $sharedContext) {
          await api.createSharedTask($sharedContext.token, { ...body, board_uuid: $currentBoard.uuid });
        } else {
          await api.createTask({ ...body, board_uuid: $currentBoard.uuid });
        }
      }
      dispatch('saved');
      close();
    } catch (e) {
      showFlash(e.message);
    }
    saving = false;
  }

  async function deleteTask() {
    if (!isEdit) return;
    try {
      if (shared && $sharedContext) {
        await api.deleteSharedTask($sharedContext.token, $editingTask.uuid);
      } else {
        await api.deleteTask($editingTask.uuid);
      }
      dispatch('saved');
      close();
    } catch (e) {
      showFlash(e.message);
    }
  }

  function close() {
    activeModal.set(null);
    editingTask.set(null);
  }

  function cyclePriority() {
    const order = ['Low', 'Medium', 'High'];
    const idx = order.indexOf(priority);
    priority = order[(idx + 1) % 3];
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="overlay" on:click={close} role="dialog" transition:fade={{ duration: 150 }}>
  <div class="modal" on:click|stopPropagation role="document"
    in:fly={{ y: 16, duration: 250 }} out:fade={{ duration: 100 }}>
    <h3>{isEdit ? 'Edit Task' : 'New Task'}</h3>

    <label>Title
      <input bind:value={title} maxlength="500" placeholder="Task title" />
    </label>

    <label>Description
      <textarea bind:value={description} maxlength="5000" rows="4" placeholder="Optional description"></textarea>
    </label>

    <div class="row">
      <label>Priority
        <button class="priority-btn" on:click={cyclePriority}
          style="color: {priority === 'High' ? '#ff6b6b' : priority === 'Low' ? '#6bcb77' : '#ffd93d'}">
          [{priority}]
        </button>
      </label>

      <label>Column
        <select bind:value={column}>
          <option value="todo">Todo</option>
          <option value="in_progress">In Progress</option>
          <option value="done">Done</option>
        </select>
      </label>

      <label>Due Date
        <input type="date" bind:value={due_date} />
      </label>
    </div>

    {#if $tags.length > 0}
      <label>Tags</label>
      <div class="tag-list">
        {#each $tags as tag}
          <button
            class="tag-chip"
            class:selected={selectedTagUuids.includes(tag.uuid)}
            on:click={() => toggleTag(tag.uuid)}
          >
            {tag.name}
          </button>
        {/each}
      </div>
    {/if}

    <div class="actions">
      {#if isEdit}
        <button class="btn-delete" on:click={deleteTask}>Delete</button>
      {/if}
      <button class="btn-cancel" on:click={close}>Cancel</button>
      <button class="btn-save" on:click={save} disabled={saving}>
        {saving ? 'Saving...' : 'Save'}
      </button>
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
    width: 480px; max-width: 90vw; max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 20px 60px #00000066;
  }
  h3 { margin-bottom: 20px; color: #64ffda; font-size: 18px; }
  label { display: block; margin-bottom: 12px; font-size: 16px; color: #8b949e; }
  input, textarea, select {
    display: block; width: 100%; margin-top: 6px;
    background: #0f172a; border: 1px solid #1e2d4a; border-radius: 6px;
    padding: 9px 12px; color: #e0e0e0;
    font-family: inherit; font-size: 16px;
    transition: border-color 0.15s;
  }
  input:focus, textarea:focus, select:focus { outline: none; border-color: #64ffda44; }
  .row { display: flex; gap: 12px; }
  .row label { flex: 1; }
  .priority-btn {
    display: block; width: 100%; margin-top: 6px; background: #0f172a;
    border: 1px solid #1e2d4a; border-radius: 6px; padding: 9px 12px;
    cursor: pointer; font-family: inherit; font-size: 16px; text-align: left;
    transition: border-color 0.15s;
  }
  .priority-btn:hover { border-color: #ffffff20; }
  .tag-list { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 16px; }
  .tag-chip {
    background: #0f172a; border: 1px solid #1e2d4a; border-radius: 6px;
    padding: 5px 10px; cursor: pointer; font-family: inherit; font-size: 16px;
    color: #8b949e; transition: all 0.15s;
  }
  .tag-chip:hover { border-color: #ffffff20; color: #c9d1d9; }
  .tag-chip.selected { color: #bb86fc; border-color: #bb86fc44; background: #bb86fc10; }
  .actions { display: flex; gap: 8px; margin-top: 20px; justify-content: flex-end; }
  .btn-save {
    background: #64ffda18; color: #64ffda; border: 1px solid #64ffda30;
    padding: 8px 18px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .btn-save:hover { background: #64ffda28; }
  .btn-save:disabled { opacity: 0.5; cursor: default; }
  .btn-cancel {
    background: #ffffff08; color: #8b949e; border: 1px solid #1e2d4a;
    padding: 8px 18px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .btn-cancel:hover { color: #c9d1d9; background: #ffffff10; }
  .btn-delete {
    background: #ff6b6b10; color: #ff6b6b; border: 1px solid #ff6b6b30;
    padding: 8px 18px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; margin-right: auto;
    transition: all 0.15s;
  }
  .btn-delete:hover { background: #ff6b6b20; }
</style>
