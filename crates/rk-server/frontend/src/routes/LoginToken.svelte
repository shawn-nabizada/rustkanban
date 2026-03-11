<script>
  import { querystring } from 'svelte-spa-router';

  let token = '';
  let deviceId = '';
  let copied = '';

  $: {
    const params = new URLSearchParams($querystring);
    token = params.get('token') || '';
    deviceId = params.get('device_id') || '';
  }

  async function copy(text, label) {
    await navigator.clipboard.writeText(text);
    copied = label;
    setTimeout(() => copied = '', 2000);
  }
</script>

<div class="page">
  <div class="card">
    <h2>CLI Login Successful</h2>
    {#if token}
      <p>Copy this token to complete login in your terminal. You can close this tab after.</p>

      <label>Token
        <div class="token-row">
          <code class="token-value">{token}</code>
          <button on:click={() => copy(token, 'token')}>
            {copied === 'token' ? 'Copied!' : 'Copy'}
          </button>
        </div>
      </label>

      <label>Device ID
        <div class="token-row">
          <code class="token-value">{deviceId}</code>
          <button on:click={() => copy(deviceId, 'device')}>
            {copied === 'device' ? 'Copied!' : 'Copy'}
          </button>
        </div>
      </label>
    {:else}
      <p class="error">No token found. Please try logging in again.</p>
    {/if}
  </div>
</div>

<style>
  .page {
    display: flex; align-items: center; justify-content: center;
    min-height: 80vh; padding: 20px;
  }
  .card {
    background: #16213e; border: 1px solid #1e2d4a; border-radius: 12px;
    padding: 32px; width: 480px; max-width: 90vw;
    box-shadow: 0 20px 60px #00000044;
  }
  h2 { color: #64ffda; font-size: 20px; margin-bottom: 16px; }
  p { color: #8b949e; font-size: 16px; margin-bottom: 20px; line-height: 1.5; }
  label { display: block; margin-bottom: 16px; font-size: 16px; color: #8b949e; }
  .token-row {
    display: flex; gap: 8px; align-items: center; margin-top: 8px;
  }
  .token-value {
    flex: 1; background: #0f172a; border: 1px solid #1e2d4a; border-radius: 6px;
    padding: 10px 12px; color: #e0e0e0; font-size: 16px; word-break: break-all;
    font-family: inherit;
  }
  button {
    background: #64ffda18; color: #64ffda; border: 1px solid #64ffda30;
    padding: 8px 16px; border-radius: 6px; cursor: pointer;
    font-family: inherit; font-size: 16px; white-space: nowrap;
    transition: all 0.15s;
  }
  button:hover { background: #64ffda28; }
  .error { color: #ff6b6b; }
</style>
