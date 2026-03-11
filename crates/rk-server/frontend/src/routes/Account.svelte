<script>
  import { onMount } from 'svelte';
  import { api } from '../lib/api.js';
  import { checkAuth, redirectToLogin, logout } from '../lib/auth.js';
  import { user, showFlash } from '../lib/stores.js';
  import { push } from 'svelte-spa-router';

  let devices = [];
  let tokens = [];
  let newToken = null;
  let tokenLabel = '';
  let tokenExpiry = 'never';
  let renamingId = null;
  let renameName = '';
  let showDeleteConfirm = false;
  let loading = true;

  onMount(async () => {
    const me = await checkAuth();
    if (!me) return redirectToLogin();
    user.set(me);
    await loadData();
    loading = false;
  });

  async function loadData() {
    [devices, tokens] = await Promise.all([
      api.listDevices(),
      api.listTokens(),
    ]);
  }

  function startRename(device) {
    renamingId = device.id;
    renameName = device.name;
  }

  async function saveRename(id) {
    try {
      await api.renameDevice(id, { name: renameName });
      renamingId = null;
      await loadData();
    } catch (e) {
      showFlash(e.message);
    }
  }

  function cancelRename() {
    renamingId = null;
  }

  async function revokeDevice(id) {
    if (!confirm('Revoke this device? Its sync token will be invalidated.')) return;
    try {
      await api.revokeDevice(id);
      showFlash('Device revoked', 'success');
      await loadData();
    } catch (e) {
      showFlash(e.message);
    }
  }

  async function createToken() {
    if (!tokenLabel.trim()) return showFlash('Label is required');
    try {
      const result = await api.createToken({ label: tokenLabel.trim(), expires: tokenExpiry });
      newToken = result.token;
      tokenLabel = '';
      tokenExpiry = 'never';
      showFlash('API token created — copy it now, it won\'t be shown again', 'success');
      await loadData();
    } catch (e) {
      showFlash(e.message);
    }
  }

  async function revokeToken(id) {
    if (!confirm('Revoke this API token?')) return;
    try {
      await api.revokeToken(id);
      showFlash('Token revoked', 'success');
      await loadData();
    } catch (e) {
      showFlash(e.message);
    }
  }

  function copyToken() {
    navigator.clipboard.writeText(newToken);
    showFlash('Token copied to clipboard', 'success');
  }

  function exportData() {
    window.open('/api/v1/account/export', '_blank');
  }

  async function deleteAccount() {
    try {
      await api.deleteAccount();
      await logout();
      push('/');
    } catch (e) {
      showFlash(e.message);
    }
  }

  function formatDate(iso) {
    if (!iso) return '—';
    const d = new Date(iso);
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  }
</script>

{#if loading}
  <div class="loading">Loading account...</div>
{:else}
<div class="content">
  <h1>Account Settings</h1>
  <p class="subtitle">Logged in as <strong>{$user?.username}</strong></p>

  <!-- Devices -->
  <section class="section">
    <h2>Devices</h2>
    {#if devices.length === 0}
      <p class="empty">No devices registered. Use <code>rk login</code> to connect a device.</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr><th>Name</th><th>Last Synced</th><th>Status</th><th></th></tr>
          </thead>
          <tbody>
            {#each devices as device}
              <tr>
                <td>
                  {#if renamingId === device.id}
                    <input
                      class="inline-input"
                      bind:value={renameName}
                      on:keydown={(e) => e.key === 'Enter' && saveRename(device.id)}
                    />
                    <button class="sm-btn" on:click={() => saveRename(device.id)}>Save</button>
                    <button class="sm-btn muted" on:click={cancelRename}>Cancel</button>
                  {:else}
                    <span class="device-name" on:dblclick={() => startRename(device)}>{device.name}</span>
                  {/if}
                </td>
                <td class="muted">{formatDate(device.last_synced_at)}</td>
                <td>
                  <span class="badge" class:badge-stale={device.stale}>
                    {device.stale ? 'Stale' : 'Active'}
                  </span>
                </td>
                <td>
                  <button class="sm-btn danger" on:click={() => revokeDevice(device.id)}>Revoke</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>

  <!-- API Tokens -->
  <section class="section">
    <h2>API Tokens</h2>

    {#if newToken}
      <div class="token-banner">
        <code class="token-value">{newToken}</code>
        <button class="sm-btn teal" on:click={copyToken}>Copy</button>
        <button class="sm-btn muted" on:click={() => newToken = null}>Dismiss</button>
      </div>
    {/if}

    <div class="token-form">
      <input class="input" placeholder="Token label" bind:value={tokenLabel} />
      <select class="input" bind:value={tokenExpiry}>
        <option value="never">No expiry</option>
        <option value="30">30 days</option>
        <option value="90">90 days</option>
      </select>
      <button class="btn-primary" on:click={createToken}>Create Token</button>
    </div>

    {#if tokens.length > 0}
      <div class="table-wrap">
        <table>
          <thead>
            <tr><th>Label</th><th>Created</th><th>Last Used</th><th>Expires</th><th></th></tr>
          </thead>
          <tbody>
            {#each tokens as token}
              <tr>
                <td>{token.label}</td>
                <td class="muted">{formatDate(token.created_at)}</td>
                <td class="muted">{formatDate(token.last_used_at)}</td>
                <td class="muted">{token.expires_at ? formatDate(token.expires_at) : 'Never'}</td>
                <td>
                  <button class="sm-btn danger" on:click={() => revokeToken(token.id)}>Revoke</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <p class="empty">No API tokens.</p>
    {/if}
  </section>

  <!-- Export -->
  <section class="section">
    <h2>Export Data</h2>
    <p class="desc">Download all your boards, tasks, and tags as a JSON file.</p>
    <button class="btn-secondary" on:click={exportData}>Download Export</button>
  </section>

  <!-- Danger Zone -->
  <section class="section danger-zone">
    <h2>Danger Zone</h2>
    {#if showDeleteConfirm}
      <p class="desc danger-text">This will permanently delete your account and all data. This cannot be undone.</p>
      <div class="confirm-btns">
        <button class="btn-danger" on:click={deleteAccount}>Yes, Delete My Account</button>
        <button class="btn-secondary" on:click={() => showDeleteConfirm = false}>Cancel</button>
      </div>
    {:else}
      <button class="btn-danger" on:click={() => showDeleteConfirm = true}>Delete Account</button>
    {/if}
  </section>
</div>
{/if}

<style>
  .content {
    max-width: 700px; margin: 0 auto; padding: 32px 20px;
  }
  h1 {
    font-size: 24px; color: #e0e0e0; margin: 0 0 4px;
  }
  .subtitle {
    font-size: 16px; color: #8b949e; margin: 0 0 32px;
  }
  .subtitle strong { color: #bb86fc; }

  .section { margin-bottom: 36px; }
  .section h2 {
    font-size: 17px; color: #e0e0e0; margin: 0 0 12px;
    padding-bottom: 8px; border-bottom: 1px solid #1e2d4a;
  }
  .desc { font-size: 16px; color: #8b949e; margin: 0 0 12px; }
  .empty { font-size: 16px; color: #8b949e; font-style: italic; }
  .empty code { color: #64ffda; font-style: normal; }

  .loading {
    display: flex; align-items: center; justify-content: center;
    min-height: 60vh; color: #8b949e; font-size: 16px;
  }

  .table-wrap { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; font-size: 16px; }
  thead th {
    text-align: left; color: #6b7b8d; font-weight: 500;
    padding: 8px 10px; border-bottom: 1px solid #1e2d4a;
  }
  tbody td {
    padding: 10px; color: #c9d1d9;
    border-bottom: 1px solid #ffffff06;
  }
  tbody tr:hover { background: #ffffff04; }
  .muted { color: #8b949e; }

  .badge {
    display: inline-block; padding: 2px 8px; border-radius: 10px;
    font-size: 16px; font-weight: 500;
    background: #6bcb7718; color: #6bcb77;
  }
  .badge-stale { background: #ffd93d18; color: #ffd93d; }

  .device-name { cursor: pointer; transition: color 0.15s; }
  .device-name:hover { color: #bb86fc; }

  .inline-input {
    background: #0f172a; border: 1px solid #1e2d4a; color: #e0e0e0;
    padding: 4px 8px; border-radius: 4px; font-family: inherit;
    font-size: 16px; width: 140px;
  }
  .inline-input:focus { outline: none; border-color: #64ffda44; }

  .sm-btn {
    background: #ffffff08; color: #c9d1d9; border: 1px solid #1e2d4a;
    padding: 4px 10px; border-radius: 4px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .sm-btn:hover { background: #ffffff10; }
  .sm-btn.muted { color: #6b7b8d; }
  .sm-btn.teal { color: #64ffda; border-color: #64ffda30; }
  .sm-btn.teal:hover { background: #64ffda10; }
  .sm-btn.danger { color: #ff6b6b; border-color: #ff6b6b30; }
  .sm-btn.danger:hover { background: #ff6b6b10; }

  .token-banner {
    background: #0f172a; border: 1px solid #64ffda30; border-radius: 8px;
    padding: 12px 14px; margin-bottom: 16px;
    display: flex; align-items: center; gap: 10px;
  }
  .token-value {
    flex: 1; font-family: inherit;
    font-size: 16px; color: #64ffda; word-break: break-all;
  }

  .token-form {
    display: flex; gap: 8px; margin-bottom: 16px; flex-wrap: wrap;
  }
  .input {
    background: #0f172a; border: 1px solid #1e2d4a; color: #e0e0e0;
    padding: 8px 12px; border-radius: 6px; font-family: inherit; font-size: 16px;
    transition: border-color 0.15s;
  }
  .input:focus { border-color: #64ffda44; outline: none; }

  .btn-primary {
    background: #64ffda18; color: #64ffda; border: 1px solid #64ffda30;
    padding: 8px 16px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .btn-primary:hover { background: #64ffda28; }
  .btn-secondary {
    background: #ffffff08; color: #c9d1d9; border: 1px solid #1e2d4a;
    padding: 8px 16px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .btn-secondary:hover { background: #ffffff10; }
  .btn-danger {
    background: #ff6b6b12; color: #ff6b6b; border: 1px solid #ff6b6b30;
    padding: 8px 16px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; transition: all 0.15s;
  }
  .btn-danger:hover { background: #ff6b6b22; }

  .danger-zone {
    border: 1px solid #ff6b6b20;
    border-radius: 10px;
    padding: 20px;
    background: #ff6b6b06;
  }
  .danger-zone h2 { color: #ff6b6b; border-bottom-color: #ff6b6b20; }
  .danger-text { color: #ff6b6b; }
  .confirm-btns { display: flex; gap: 8px; }

  @media (max-width: 600px) {
    .token-form { flex-direction: column; }
  }
</style>
