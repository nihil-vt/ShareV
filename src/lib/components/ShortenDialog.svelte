<script lang="ts">
  import type { Config, ShortenProvider } from '../types';
  import { api } from '../api';
  import { toast } from '../state.svelte';

  let { onClose }: { onClose: () => void } = $props();

  const providers: { id: ShortenProvider; label: string; free: boolean }[] = [
    { id: 'isgd', label: 'is.gd', free: true },
    { id: 'vgd', label: 'v.gd', free: true },
    { id: 'tinyurl', label: 'tinyurl', free: true },
    { id: 'zws', label: 'Zero Width Shortener', free: true },
    { id: 'bitly', label: 'bit.ly', free: false },
    { id: 'kutt', label: 'kutt', free: false },
    { id: 'polr', label: 'polr', free: false },
    { id: 'yourls', label: 'YOURLS', free: false },
    { id: 'firebase', label: 'Firebase Dynamic Links', free: false },
    { id: 'custom', label: 'Custom', free: false },
  ];

  let config = $state<Config | null>(null);
  let url = $state('');
  let working = $state(false);
  let short = $state<string | null>(null);

  const provider = $derived<ShortenProvider>(config?.shorten.provider ?? 'isgd');

  async function pick(next: ShortenProvider) {
    if (!config) return;
    config = { ...config, shorten: { ...config.shorten, provider: next } };
    await persist();
  }

  async function persist() {
    if (!config) return;
    try {
      await api.saveConfig(config);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function shortenNow() {
    if (!url.trim() || working || !config) return;
    working = true;
    try {
      short = await api.shortenUrlWith(url, config.shorten.provider, credsFor(config));
      toast('URL shortened', 'success');
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      working = false;
    }
  }

  function credsFor(c: Config) {
    const s = c.shorten;
    switch (c.shorten.provider) {
      case 'bitly':
        return { key: s.bitlyKey, domain: s.bitlyDomain };
      case 'kutt':
        return { host: s.kuttHost, key: s.kuttKey };
      case 'polr':
        return { host: s.polrHost, key: s.polrKey };
      case 'yourls':
        return {
          host: s.yourlsHost,
          key: s.yourlsSignature,
          username: s.yourlsUsername,
          password: s.yourlsPassword,
        };
      case 'firebase':
        return { firebaseKey: s.firebaseKey, firebaseDomain: s.firebaseDomain };
      case 'custom':
        return { customUrl: s.customUrl };
      default:
        return {};
    }
  }

  async function copy() {
    if (!short) return;
    try {
      await api.copyText(short);
      toast('Short URL copied', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function load() {
    try {
      config = await api.getConfig();
    } catch (err) {
      toast(String(err), 'error');
      onClose();
    }
  }

  void load();
</script>

<div
  class="overlay"
  onclick={onClose}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
  role="button"
  tabindex="0"
  aria-label="Close URL shortener"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="URL shortener"
    tabindex="-1"
  >
    <div class="head">
      <h2>Shorten URL</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>

    {#if config}
      <label class="field">
        <span>Provider</span>
        <select value={provider} onchange={(e) => pick(e.currentTarget.value as ShortenProvider)}>
          {#each providers as p (p.id)}
            <option value={p.id}>{p.label}{p.free ? ' (no account needed)' : ''}</option>
          {/each}
        </select>
      </label>

      {#if provider === 'bitly'}
        <label class="field">
          <span>Access token</span>
          <input type="password" bind:value={config.shorten.bitlyKey} onblur={persist} />
        </label>
        <label class="field">
          <span>Custom domain (optional)</span>
          <input type="text" bind:value={config.shorten.bitlyDomain} placeholder="example.bitly.com" onblur={persist} />
        </label>
      {:else if provider === 'kutt'}
        <label class="field">
          <span>Host</span>
          <input type="text" bind:value={config.shorten.kuttHost} placeholder="kutt.it" onblur={persist} />
        </label>
        <label class="field">
          <span>API key</span>
          <input type="password" bind:value={config.shorten.kuttKey} onblur={persist} />
        </label>
      {:else if provider === 'polr'}
        <label class="field">
          <span>Host</span>
          <input type="text" bind:value={config.shorten.polrHost} placeholder="polr.me" onblur={persist} />
        </label>
        <label class="field">
          <span>API key (optional on public installs)</span>
          <input type="password" bind:value={config.shorten.polrKey} onblur={persist} />
        </label>
      {:else if provider === 'yourls'}
        <label class="field">
          <span>Host</span>
          <input type="text" bind:value={config.shorten.yourlsHost} onblur={persist} />
        </label>
        <label class="field">
          <span>Signature token (preferred)</span>
          <input type="password" bind:value={config.shorten.yourlsSignature} onblur={persist} />
        </label>
        <div class="row">
          <input type="text" bind:value={config.shorten.yourlsUsername} placeholder="username" aria-label="YOURLS username" onblur={persist} />
          <input type="password" bind:value={config.shorten.yourlsPassword} placeholder="password" aria-label="YOURLS password" onblur={persist} />
        </div>
      {:else if provider === 'firebase'}
        <label class="field">
          <span>API key</span>
          <input type="password" bind:value={config.shorten.firebaseKey} onblur={persist} />
        </label>
        <label class="field">
          <span>Dynamic link domain</span>
          <input type="text" bind:value={config.shorten.firebaseDomain} placeholder="https://example.page.link" onblur={persist} />
        </label>
      {:else if provider === 'custom'}
        <label class="field">
          <span>URL template</span>
          <input
            type="text"
            bind:value={config.shorten.customUrl}
            placeholder="https://host/api?url=URL_PLACEHOLDER"
            onblur={persist}
          />
        </label>
      {/if}

      <label class="field">
        <span>Long URL</span>
        <textarea rows="3" bind:value={url} placeholder="https://example.com/very/long/path"></textarea>
      </label>

      <button class="primary" disabled={working || !url.trim()} onclick={shortenNow}>
        {working ? 'Shortening…' : 'Shorten'}
      </button>

      {#if short}
        <div class="result">
          <code>{short}</code>
          <button onclick={copy}>Copy</button>
        </div>
      {/if}
    {:else}
      <p class="hint">Loading configuration…</p>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1500;
    padding: 1rem;
  }
  .dialog {
    width: min(440px, 100%);
    max-height: 88vh;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    box-shadow: 0 16px 64px rgba(0, 0, 0, 0.5);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .head h2 {
    margin: 0;
    font-size: 0.9rem;
  }
  .mini {
    padding: 0.2rem 0.6rem;
    font-size: 0.72rem;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
  }
  .mini:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.72rem;
    color: var(--text-dim);
  }
  input,
  select,
  textarea {
    padding: 0.4rem 0.5rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.8rem;
    font-family: inherit;
  }
  textarea {
    resize: vertical;
  }
  .row {
    display: flex;
    gap: 0.4rem;
  }
  .row input {
    flex: 1;
    min-width: 0;
  }
  button {
    padding: 0.4rem 0.7rem;
    font-size: 0.78rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
    white-space: nowrap;
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  .result {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .result code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: var(--surface-3);
    padding: 0.3rem 0.5rem;
    border-radius: 6px;
    font-size: 0.76rem;
  }
  .hint {
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-dim);
  }
</style>
