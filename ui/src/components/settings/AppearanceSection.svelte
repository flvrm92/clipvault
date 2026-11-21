<script lang="ts">
  import type { ClipVaultApi } from '../../api/client';
  import type { SettingsDto } from '../../api/bindings/SettingsDto';
  import type { ThemeModeDto } from '../../api/bindings/ThemeModeDto';
  import { t } from '../../i18n';
  import RestoreDefaults from './RestoreDefaults.svelte';

  /** "Aparência": theme (THM-001) and Windows accent color (THM-004). */
  let {
    api,
    settings,
    reload,
  }: { api: ClipVaultApi; settings: SettingsDto; reload: () => Promise<void> } = $props();

  const THEMES: ThemeModeDto[] = ['system', 'light', 'dark'];

  async function setTheme(theme: ThemeModeDto) {
    await api.setTheme(theme);
    await reload();
  }

  async function setAccent(on: boolean) {
    await api.setUseAccentColor(on ? 'on' : 'off');
    await reload();
  }
</script>

<h1>{t('settings.appearance.heading')}</h1>

<section>
  <fieldset class="themes">
    <legend><h2>{t('settings.appearance.theme')}</h2></legend>
    {#each THEMES as theme (theme)}
      <label class="card" data-preview={theme}>
        <span class="preview" aria-hidden="true"><span></span><span></span></span>
        <span class="check">
          <input
            type="radio"
            name="theme"
            value={theme}
            checked={settings.theme === theme}
            onchange={() => setTheme(theme)}
          />
          {t(`settings.appearance.themes.${theme}`)}
        </span>
      </label>
    {/each}
  </fieldset>
</section>

<section>
  <div class="row">
    <div>
      <h2>{t('settings.appearance.accent')}</h2>
      <p class="hint">{t('settings.appearance.accentHint')}</p>
    </div>
    <label class="check">
      <input
        type="checkbox"
        checked={settings.useAccentColor}
        onchange={(e) => setAccent(e.currentTarget.checked)}
      />
      <span>{t('settings.appearance.accentToggle')}</span>
    </label>
  </div>
</section>

<RestoreDefaults {api} section="appearance" onrestored={reload} />

<style>
  .themes {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-bottom: 8px;
    padding: 0;
  }

  .card {
    display: grid;
    gap: 8px;
    width: 140px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
  }

  .card:has(input:checked) {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .preview {
    display: grid;
    gap: 6px;
    height: 64px;
    padding: 10px;
    border-radius: 4px;
    background: var(--swatch-light);
  }

  .preview span {
    border-radius: 3px;
    background: var(--swatch-light-raised);
  }

  [data-preview='dark'] .preview {
    background: var(--swatch-dark);
  }

  [data-preview='dark'] .preview span {
    background: var(--swatch-dark-raised);
  }

  [data-preview='system'] .preview {
    background: linear-gradient(135deg, var(--swatch-light) 50%, var(--swatch-dark) 50%);
  }

  [data-preview='system'] .preview span {
    background: color-mix(in srgb, var(--swatch-light-raised) 50%, var(--swatch-dark-raised));
  }

  .check {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  @media (forced-colors: active) {
    .preview,
    .preview span {
      forced-color-adjust: none;
    }
  }
</style>
