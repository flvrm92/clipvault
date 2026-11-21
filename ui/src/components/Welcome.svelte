<script lang="ts">
  import { onMount } from 'svelte';
  import type { ClipVaultApi } from '../api/client';
  import type { WelcomeDto } from '../api/bindings/WelcomeDto';
  import { setLocale, t } from '../i18n';
  import { applyAppearance } from '../lib/appearance';
  import BrandMark from '../lib/BrandMark.svelte';
  import { keycaps } from '../lib/shortcuts';
  import { startupLock } from '../lib/startup';

  /** First-run welcome (TRAY-006): open shortcut and consent to start with Windows (CFG-003). */
  let { api }: { api: ClipVaultApi } = $props();

  let welcome = $state<WelcomeDto | null>(null);
  let startWithWindows = $state(true);
  let start = $state<HTMLButtonElement | null>(null);

  const lock = $derived(welcome ? startupLock(welcome.startup) : null);

  async function finish(openSettings: boolean) {
    await api.completeWelcome(lock ? null : startWithWindows, openSettings);
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') void api.completeWelcome(null, false);
  }

  onMount(() => {
    void api.getWelcome().then((loaded) => {
      welcome = loaded;
      setLocale(loaded.locale);
      applyAppearance(loaded.appearance);
      queueMicrotask(() => start?.focus());
    });
  });
</script>

<svelte:window {onkeydown} />

<main class="welcome">
  <BrandMark size={48} />
  <h1>{t('welcome.title')}</h1>
  <p class="lead">{t('welcome.body')}</p>

  {#if welcome}
    <p class="shortcut">
      <span>{t('welcome.shortcut')}</span>
      {#each keycaps(welcome.openShortcut) as cap (cap)}<kbd>{cap}</kbd>{/each}
    </p>

    <label class="check">
      <input
        type="checkbox"
        bind:checked={startWithWindows}
        disabled={lock !== null}
        aria-describedby={lock ? 'startup-lock' : undefined}
      />
      <span>{t('settings.general.startup')}</span>
    </label>
    {#if lock}
      <p id="startup-lock" class="hint">{t(lock)}</p>
    {/if}

    <div class="actions">
      <button type="button" onclick={() => finish(true)}>{t('welcome.openSettings')}</button>
      <button type="button" class="primary" bind:this={start} onclick={() => finish(false)}>
        {t('welcome.start')}
      </button>
    </div>
  {/if}
</main>

<style>
  .welcome {
    display: grid;
    align-content: start;
    gap: 12px;
    height: 100vh;
    padding: 24px 28px;
    background: var(--surface);
    color: var(--text-primary);
  }

  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
  }

  .lead,
  .hint {
    margin: 0;
    color: var(--text-secondary);
  }

  .hint {
    font-size: 12px;
  }

  .shortcut {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    margin: 4px 0;
  }

  .shortcut span {
    margin-right: 4px;
  }

  kbd {
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: var(--surface-raised);
    font-family: inherit;
    font-size: 12px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }

  button {
    padding: 6px 14px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface-raised);
    color: var(--text-primary);
    font: inherit;
  }

  button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-text);
  }

  :focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
</style>
