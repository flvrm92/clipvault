<script lang="ts" module>
  export const SECTIONS = [
    'general',
    'history',
    'shortcuts',
    'privacy',
    'appearance',
    'about',
  ] as const;
  export type SectionId = (typeof SECTIONS)[number];

  /** Maps a URL fragment or a `settings://navigate` payload to a section. */
  export function sectionOf(value: unknown): SectionId {
    const id = String(value ?? '').replace(/^#/, '');
    return (SECTIONS as readonly string[]).includes(id) ? (id as SectionId) : 'general';
  }
</script>

<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import type { ClipVaultApi } from '../../api/client';
  import type { SettingsDto } from '../../api/bindings/SettingsDto';
  import { setLocale, t } from '../../i18n';
  import { applyAppearance } from '../../lib/appearance';
  import BrandMark from '../../lib/BrandMark.svelte';
  import '../../styles/settings.css';
  import AboutSection from './AboutSection.svelte';
  import AppearanceSection from './AppearanceSection.svelte';
  import GeneralSection from './GeneralSection.svelte';
  import HistorySection from './HistorySection.svelte';
  import PrivacySection from './PrivacySection.svelte';
  import ShortcutsSection from './ShortcutsSection.svelte';

  /** Settings window (CFG-001): side navigation and one section at a time (specs/06 §3). */
  let { api, initial = 'general' }: { api: ClipVaultApi; initial?: string } = $props();

  // The prop only picks the first section; navigation owns it afterwards.
  let section = $state<SectionId>(sectionOf(untrack(() => initial)));
  let settings = $state<SettingsDto | null>(null);

  async function load() {
    const [loaded, popup] = await Promise.all([api.getSettings(), api.getPopupState()]);
    settings = loaded;
    setLocale(loaded.locale);
    applyAppearance(popup.appearance);
  }

  function go(id: SectionId) {
    section = id;
    globalThis.history?.replaceState(null, '', `#${id}`);
  }

  onMount(() => {
    void load();
    const subscriptions = [
      api.on('settings://changed', () => void load()),
      api.on('capture://changed', () => void load()),
      api.on('appearance://changed', () => void load()),
      api.on('settings://navigate', (payload) => go(sectionOf(payload))),
    ];
    return () => subscriptions.forEach((s) => void s.then((unlisten) => unlisten()));
  });
</script>

<div class="settings">
  <nav aria-label={t('settings.nav.label')}>
    <div class="brand"><BrandMark size={20} /> <span>{t('app.name')}</span></div>
    {#each SECTIONS as id (id)}
      <a
        href="#{id}"
        aria-current={section === id ? 'page' : undefined}
        onclick={(event) => {
          event.preventDefault();
          go(id);
        }}
      >
        {t(`settings.nav.${id}`)}
      </a>
    {/each}
  </nav>

  <main id={section}>
    {#if section === 'shortcuts'}
      <ShortcutsSection {api} />
    {:else if section === 'about'}
      <AboutSection {api} />
    {:else if settings}
      {#if section === 'general'}
        <GeneralSection {api} {settings} reload={load} />
      {:else if section === 'history'}
        <HistorySection {api} {settings} reload={load} />
      {:else if section === 'privacy'}
        <PrivacySection {api} {settings} reload={load} />
      {:else}
        <AppearanceSection {api} {settings} reload={load} />
      {/if}
    {/if}
  </main>
</div>
