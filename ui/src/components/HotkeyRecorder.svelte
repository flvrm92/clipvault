<script lang="ts">
  import type { ClipVaultApi } from '../api/client';
  import { t } from '../i18n';
  import { comboFromEvent, keycaps } from '../lib/shortcuts';

  /** Records a shortcut (HK-002). Global shortcuts are suspended while recording so the
   * combination reaches this field instead of triggering ClipVault. */
  let {
    api,
    value,
    label,
    error = null,
    allowClear = false,
    onchange,
  }: {
    api: ClipVaultApi;
    value: string | null;
    label: string;
    error?: string | null;
    allowClear?: boolean;
    onchange: (shortcut: string | null) => void;
  } = $props();

  let recording = $state(false);
  let field: HTMLButtonElement | undefined = $state();

  async function start() {
    recording = true;
    await api.setHotkeySuspension('suspended');
  }

  async function stop() {
    if (!recording) return;
    recording = false;
    await api.setHotkeySuspension('active');
  }

  function onkeydown(event: KeyboardEvent) {
    if (!recording) {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        void start();
      }
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const plain = !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
    if (plain && event.key === 'Escape') {
      void stop();
      return;
    }
    if (plain && event.key === 'Backspace') {
      if (allowClear) onchange(null);
      void stop();
      return;
    }
    const combo = comboFromEvent(event);
    if (combo) {
      onchange(combo);
      void stop();
    }
  }
</script>

<div class="recorder">
  <button
    bind:this={field}
    type="button"
    class="field"
    class:recording
    class:invalid={!!error}
    aria-label={label}
    aria-describedby={error ? `${label}-error` : undefined}
    onclick={() => (recording ? stop() : start())}
    onblur={stop}
    {onkeydown}
  >
    {#if recording}
      <span class="prompt">{t('settings.recorder.prompt')}</span>
    {:else if value}
      {#each keycaps(value) as cap (cap)}
        <kbd>{cap}</kbd>
      {/each}
    {:else}
      <span class="placeholder">{t('settings.recorder.empty')}</span>
    {/if}
  </button>
  {#if recording}
    <small>{t('settings.recorder.hint')}</small>
  {/if}
  {#if error}
    <small id="{label}-error" class="error" role="alert">{error}</small>
  {/if}
</div>

<style>
  .recorder {
    display: grid;
    gap: 4px;
  }

  .field {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 220px;
    min-height: 34px;
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface-raised);
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
  }

  .field.recording {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .field.invalid {
    border-color: var(--danger);
  }

  .field:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  kbd {
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: var(--surface);
    font-family: inherit;
    font-size: 12px;
  }

  .prompt,
  .placeholder,
  small {
    color: var(--text-secondary);
    font-size: 12px;
  }

  .error {
    color: var(--danger);
  }
</style>
