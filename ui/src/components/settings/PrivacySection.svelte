<script lang="ts">
  import { onMount } from 'svelte';
  import type { ClipVaultApi } from '../../api/client';
  import type { SettingsDto } from '../../api/bindings/SettingsDto';
  import { t } from '../../i18n';
  import { errorMessage } from '../../lib/errors';
  import Icon from '../Icon.svelte';
  import RestoreDefaults from './RestoreDefaults.svelte';

  /** "Privacidade": what is always respected (CAP-007) and the excluded apps (CAP-009). */
  let {
    api,
    settings,
    reload,
  }: { api: ClipVaultApi; settings: SettingsDto; reload: () => Promise<void> } = $props();

  let name = $state('');
  let running = $state<string[]>([]);
  let chosen = $state('');
  let error = $state<string | null>(null);

  async function refreshRunning() {
    running = await api.listRunningApps();
    chosen = '';
  }

  async function attempt(action: () => Promise<void>) {
    error = null;
    try {
      await action();
      await Promise.all([reload(), refreshRunning()]);
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  const add = (app: string) =>
    attempt(async () => {
      await api.addExcludedApp(app);
      name = '';
    });

  const remove = (app: string) => attempt(() => api.removeExcludedApp(app));

  onMount(() => void refreshRunning());
</script>

<h1>{t('settings.privacy.heading')}</h1>

<section>
  <h2>{t('settings.privacy.respectedTitle')}</h2>
  <p class="hint">{t('settings.privacy.respected')}</p>
</section>

<section>
  <div>
    <h2>{t('settings.privacy.excluded')}</h2>
    <p class="hint">{t('settings.privacy.excludedHint')}</p>
  </div>

  {#if settings.excludedApps.length === 0}
    <p class="empty">{t('settings.privacy.none')}</p>
  {:else}
    <ul class="list" aria-label={t('settings.privacy.excluded')}>
      {#each settings.excludedApps as app (app)}
        <li>
          <span>{app}</span>
          <button
            type="button"
            class="icon"
            aria-label="{t('settings.privacy.remove')}: {app}"
            onclick={() => remove(app)}
          >
            <Icon name="delete" />
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      void add(name);
    }}
  >
    <label for="exe-name">{t('settings.privacy.name')}</label>
    <input
      id="exe-name"
      type="text"
      bind:value={name}
      placeholder={t('settings.privacy.namePlaceholder')}
      autocomplete="off"
      spellcheck="false"
    />
    <button type="submit">{t('settings.privacy.add')}</button>
  </form>

  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault();
      if (chosen) void add(chosen);
    }}
  >
    <label for="running-app">{t('settings.privacy.running')}</label>
    {#if running.length === 0}
      <p class="empty">{t('settings.privacy.noRunning')}</p>
    {:else}
      <select id="running-app" bind:value={chosen}>
        <option value="" disabled>{t('settings.privacy.runningPlaceholder')}</option>
        {#each running as app (app)}
          <option value={app}>{app}</option>
        {/each}
      </select>
      <button type="submit" disabled={!chosen}>{t('settings.privacy.add')}</button>
    {/if}
  </form>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

<RestoreDefaults {api} section="privacy" onrestored={() => attempt(async () => {})} />
