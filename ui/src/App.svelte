<script lang="ts">
  import { t } from './i18n';
  import BrandMark from './lib/BrandMark.svelte';
</script>

<main>
  <BrandMark size={72} />
  <h1>{t('app.name')}</h1>
  <p class="tagline">{t('app.tagline')}</p>
  <p class="note">{t('app.foundation')}</p>
</main>

<style>
  main {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 8px;
    height: 100%;
    padding: 24px;
    text-align: center;
  }

  h1 {
    margin: 8px 0 0;
    font-size: 28px;
    font-weight: 600;
  }

  .tagline {
    margin: 0;
    font-size: 16px;
    color: var(--text-primary);
  }

  .note {
    margin: 16px 0 0;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-raised);
    color: var(--text-secondary);
    font-size: 12px;
  }
</style>
