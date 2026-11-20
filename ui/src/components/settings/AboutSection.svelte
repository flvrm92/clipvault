<script lang="ts">
  import { onMount } from 'svelte';
  import type { ClipVaultApi } from '../../api/client';
  import type { AboutLinkDto } from '../../api/bindings/AboutLinkDto';
  import { t } from '../../i18n';
  import BrandMark from '../../lib/BrandMark.svelte';

  /** "Sobre" (CFG-005). Links are requested by name; the backend owns the URLs. */
  let { api }: { api: ClipVaultApi } = $props();

  const LINKS: AboutLinkDto[] = ['privacy', 'licenses', 'support'];
  let version = $state('');

  onMount(() => {
    void api.getAbout().then((about) => (version = about.version));
  });
</script>

<h1>{t('settings.about.heading')}</h1>

<section>
  <div class="identity">
    <BrandMark size={56} />
    <div>
      <h2 class="name">{t('app.name')}</h2>
      {#if version}
        <p class="hint">{t('settings.about.version', { version })}</p>
      {/if}
    </div>
  </div>
  <div class="links">
    {#each LINKS as link (link)}
      <button type="button" onclick={() => api.openAboutLink(link)}>
        {t(`settings.about.${link}`)}
      </button>
    {/each}
  </div>
</section>

<section>
  <div class="row">
    <div>
      <h2>{t('settings.about.dataFolder')}</h2>
      <p class="hint">{t('settings.about.dataFolderHint')}</p>
    </div>
    <button type="button" onclick={() => api.openDataFolder()}>
      {t('settings.about.openDataFolder')}
    </button>
  </div>
</section>

<style>
  .identity {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .name {
    font-size: 20px;
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
