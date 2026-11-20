<script lang="ts">
  import { onMount } from 'svelte';
  import type { ClipVaultApi } from '../../api/client';
  import type { LanguageDto } from '../../api/bindings/LanguageDto';
  import type { SettingsDto } from '../../api/bindings/SettingsDto';
  import type { StartupStateDto } from '../../api/bindings/StartupStateDto';
  import { t } from '../../i18n';
  import { errorMessage } from '../../lib/errors';
  import { startupEnabled, startupLock } from '../../lib/startup';
  import RestoreDefaults from './RestoreDefaults.svelte';

  /** "Geral": start with Windows (CFG-003), language (I18N-002) and capture state (CAP-008). */
  let {
    api,
    settings,
    reload,
  }: { api: ClipVaultApi; settings: SettingsDto; reload: () => Promise<void> } = $props();

  const LANGUAGES: LanguageDto[] = ['system', 'ptBr', 'en'];

  let startup = $state<StartupStateDto | null>(null);
  let startupError = $state<string | null>(null);
  const lock = $derived(startup ? startupLock(startup) : null);

  async function setStartup(box: HTMLInputElement) {
    const on = box.checked;
    // The backend decides the resulting state (it may be locked): show the known one until then.
    box.checked = startup !== null && startupEnabled(startup);
    startupError = null;
    try {
      startup = await api.setStartup(on ? 'on' : 'off');
    } catch (failure) {
      startupError = errorMessage(failure);
      startup = await api.getStartup();
    }
  }

  onMount(() => {
    void api.getStartup().then((state) => (startup = state));
  });

  async function setLanguage(language: LanguageDto) {
    await api.setLanguage(language);
    await reload();
  }

  async function setCapture(active: boolean) {
    await api.setCaptureState(active ? 'active' : 'paused');
    await reload();
  }
</script>

<h1>{t('settings.general.heading')}</h1>

<section>
  <div class="row">
    <div>
      <h2>{t('settings.general.startup')}</h2>
      <p class="hint" id="startup-hint">{t(lock ?? 'settings.general.startupHint')}</p>
    </div>
    <label class="check">
      <input
        type="checkbox"
        checked={startup !== null && startupEnabled(startup)}
        disabled={startup === null || lock !== null}
        aria-describedby="startup-hint"
        onchange={(e) => setStartup(e.currentTarget)}
      />
      <span>{t('settings.general.startupToggle')}</span>
    </label>
  </div>
  {#if startupError}
    <p class="error" role="alert">{startupError}</p>
  {/if}
</section>

<section>
  <div class="row">
    <div>
      <h2><label for="language">{t('settings.general.language')}</label></h2>
      <p class="hint">{t('settings.general.languageHint')}</p>
    </div>
    <select
      id="language"
      value={settings.language}
      onchange={(e) => setLanguage(e.currentTarget.value as LanguageDto)}
    >
      {#each LANGUAGES as language (language)}
        <option value={language}>{t(`settings.general.languages.${language}`)}</option>
      {/each}
    </select>
  </div>
</section>

<section>
  <div class="row">
    <div>
      <h2>{t('settings.general.capture')}</h2>
      <p class="hint">{t('settings.general.captureHint')}</p>
    </div>
    <label class="check">
      <input
        type="checkbox"
        checked={!settings.capturePaused}
        onchange={(e) => setCapture(e.currentTarget.checked)}
      />
      <span>{t('settings.general.captureToggle')}</span>
    </label>
  </div>
</section>

<RestoreDefaults {api} section="general" onrestored={reload} />
