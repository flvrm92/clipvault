<script lang="ts">
  import type { ClipVaultApi } from '../../api/client';
  import type { LanguageDto } from '../../api/bindings/LanguageDto';
  import type { SettingsDto } from '../../api/bindings/SettingsDto';
  import { t } from '../../i18n';
  import RestoreDefaults from './RestoreDefaults.svelte';

  /** "Geral": language (I18N-002) and capture state (CAP-008). Startup arrives with CFG-003. */
  let {
    api,
    settings,
    reload,
  }: { api: ClipVaultApi; settings: SettingsDto; reload: () => Promise<void> } = $props();

  const LANGUAGES: LanguageDto[] = ['system', 'ptBr', 'en'];

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
