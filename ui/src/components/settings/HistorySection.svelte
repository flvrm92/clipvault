<script lang="ts">
  import type { ClipVaultApi } from '../../api/client';
  import type { RetentionDto } from '../../api/bindings/RetentionDto';
  import type { SettingsDto } from '../../api/bindings/SettingsDto';
  import { getLocale, t, tn } from '../../i18n';
  import { errorMessage } from '../../lib/errors';
  import { formatBytes } from '../../lib/format';
  import RestoreDefaults from './RestoreDefaults.svelte';

  /** "Histórico": retention (RET-001/002/004/006), size limits (CAP-006) and clearing (HIST-007). */
  let {
    api,
    settings,
    reload,
  }: { api: ClipVaultApi; settings: SettingsDto; reload: () => Promise<void> } = $props();

  const NONE = 'none';
  const encode = (value: number | null) => (value === null ? NONE : String(value));
  const decode = (value: string) => (value === NONE ? null : Number(value));

  // The selects show the pending choice while it waits for confirmation (RET-004).
  let age = $state('');
  let items = $state('');
  $effect.pre(() => {
    age = encode(settings.retention.maxAgeDays);
    items = encode(settings.retention.maxItems);
  });

  let pending = $state<{ retention: RetentionDto; count: number } | null>(null);
  let confirmingClear = $state(false);
  let status = $state<string | null>(null);
  let error = $state<string | null>(null);

  const ageLabel = (days: number | null) =>
    days === null ? t('settings.history.forever') : tn('settings.history.days', days);
  const itemsLabel = (count: number | null) =>
    count === null
      ? t('settings.history.unlimited')
      : new Intl.NumberFormat(getLocale()).format(count);

  async function attempt(action: () => Promise<void>) {
    error = null;
    status = null;
    try {
      await action();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  async function applyRetention(retention: RetentionDto) {
    pending = null;
    await attempt(async () => {
      const removed = await api.setRetention(retention);
      if (removed > 0) status = tn('settings.history.removed', removed);
      await reload();
    });
  }

  async function changeRetention() {
    const retention = { maxAgeDays: decode(age), maxItems: decode(items) };
    const count = await api.previewRetention(retention);
    if (count > 0) pending = { retention, count };
    else await applyRetention(retention);
  }

  function cancelRetention() {
    pending = null;
    age = encode(settings.retention.maxAgeDays);
    items = encode(settings.retention.maxItems);
  }

  const setClearOnExit = (on: boolean) =>
    attempt(async () => {
      await api.setClearOnExit(on ? 'on' : 'off');
      await reload();
    });

  const setLimit = (limit: 'text' | 'image', bytes: number) =>
    attempt(async () => {
      await api.setSizeLimit(limit, bytes);
      await reload();
    });

  const clear = () =>
    attempt(async () => {
      confirmingClear = false;
      const removed = await api.clearHistory('unpinnedOnly');
      status = tn('settings.history.removed', removed);
    });
</script>

<h1>{t('settings.history.heading')}</h1>

<section>
  <div class="row">
    <div>
      <h2><label for="max-age">{t('settings.history.retention')}</label></h2>
      <p class="hint">{t('settings.history.retentionHint')}</p>
    </div>
    <select id="max-age" bind:value={age} onchange={changeRetention}>
      {#each settings.options.maxAgeDays as days (encode(days))}
        <option value={encode(days)}>{ageLabel(days)}</option>
      {/each}
    </select>
  </div>
  <div class="row">
    <div>
      <h2><label for="max-items">{t('settings.history.maxItems')}</label></h2>
      <p class="hint">{t('settings.history.maxItemsHint')}</p>
    </div>
    <select id="max-items" bind:value={items} onchange={changeRetention}>
      {#each settings.options.maxItems as count (encode(count))}
        <option value={encode(count)}>{itemsLabel(count)}</option>
      {/each}
    </select>
  </div>
  {#if pending}
    <div class="confirm" role="alertdialog" aria-label={t('settings.history.retention')}>
      <span>{tn('settings.history.confirmPurge', pending.count)}</span>
      <button type="button" onclick={cancelRetention}>{t('settings.common.cancel')}</button>
      <button
        type="button"
        class="danger"
        onclick={() => pending && applyRetention(pending.retention)}
      >
        {t('settings.common.continue')}
      </button>
    </div>
  {/if}
  <label class="check">
    <input
      type="checkbox"
      checked={settings.clearOnExit}
      onchange={(e) => setClearOnExit(e.currentTarget.checked)}
    />
    <span>{t('settings.history.clearOnExit')}</span>
  </label>
</section>

<section>
  <div class="row">
    <div>
      <h2><label for="max-text">{t('settings.history.maxText')}</label></h2>
      <p class="hint">{t('settings.history.sizeHint')}</p>
    </div>
    <select
      id="max-text"
      value={settings.maxTextBytes}
      onchange={(e) => setLimit('text', Number(e.currentTarget.value))}
    >
      {#each settings.options.textLimits as bytes (bytes)}
        <option value={bytes}>{formatBytes(bytes, getLocale())}</option>
      {/each}
    </select>
  </div>
  <div class="row">
    <h2><label for="max-image">{t('settings.history.maxImage')}</label></h2>
    <select
      id="max-image"
      value={settings.maxImageBytes}
      onchange={(e) => setLimit('image', Number(e.currentTarget.value))}
    >
      {#each settings.options.imageLimits as bytes (bytes)}
        <option value={bytes}>{formatBytes(bytes, getLocale())}</option>
      {/each}
    </select>
  </div>
</section>

<section>
  <div class="row">
    <div>
      <h2>{t('settings.history.clear')}</h2>
      <p class="hint">{t('settings.history.clearHint')}</p>
    </div>
    <button type="button" onclick={() => (confirmingClear = true)}>
      {t('settings.history.clear')}
    </button>
  </div>
  {#if confirmingClear}
    <div class="confirm" role="alertdialog" aria-label={t('settings.history.clear')}>
      <span>{t('settings.history.clearConfirm')}</span>
      <button type="button" onclick={() => (confirmingClear = false)}>
        {t('settings.common.cancel')}
      </button>
      <button type="button" class="danger" onclick={clear}>{t('settings.history.clear')}</button>
    </div>
  {/if}
</section>

{#if status}
  <p class="status" role="status">{status}</p>
{/if}
{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

<RestoreDefaults
  {api}
  section="history"
  preview={() => api.previewRetention(settings.options.defaultRetention)}
  onrestored={reload}
/>
